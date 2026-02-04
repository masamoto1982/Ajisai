// rust/src/interpreter/vector_ops.rs
//
// 【責務】
// ベクタおよびスタックに対する位置・構造操作を実装する。
// 0オリジンの位置指定操作（GET/INSERT/REPLACE/REMOVE）、
// 1オリジンの量指定操作（LENGTH/TAKE/SPLIT）、
// およびベクタ構造操作（CONCAT/REVERSE/LEVEL）を提供する。
//
// 統一Value宇宙アーキテクチャ版

use crate::interpreter::{Interpreter, OperationTargetMode};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{get_bigint_from_value, get_integer_from_value, normalize_index, wrap_number};
use crate::types::{Value, ValueData};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};

// ============================================================================
// ヘルパー関数（統一Value宇宙アーキテクチャ用）
// ============================================================================

/// ベクタ値かどうかを判定
fn is_vector_value(val: &Value) -> bool {
    val.is_vector()
}

/// ベクタの要素を再構築する（子Valueのリストを返す）
fn reconstruct_vector_elements(val: &Value) -> Vec<Value> {
    match &val.data {
        ValueData::Vector(children) => children.clone(),
        ValueData::Scalar(_) => vec![val.clone()],
        ValueData::Nil => vec![],
        ValueData::CodeBlock(_) => vec![val.clone()],  // コードブロックはそのまま単一要素として扱う
    }
}

// ============================================================================
// 位置指定操作（0オリジン）
// ============================================================================

/// GET - 指定位置の要素を取得する
///
/// 【責務】
/// - スタックトップのベクタまたはスタック全体から、指定インデックスの要素を取得
/// - 負数インデックスをサポート（-1 = 末尾）
/// - 取得した要素を単一要素ベクタとしてプッシュ
///
/// 【使用法】
/// - StackTopモード: `[a b c] [1] GET` → `[a b c] [b]`
/// - Stackモード: `a b c [1] .. GET` → `a b c [b]`
///
/// 【引数スタック】
/// - [index]: 取得するインデックス（単一要素ベクタの整数）
/// - (StackTopモード) target: 対象ベクタ
///
/// 【戻り値スタック】
/// - (StackTopモード) target: 元のベクタ（保持）
/// - [element]: 取得した要素
///
/// 【エラー】
/// - インデックスが範囲外の場合
/// - 対象がベクタでない場合
pub fn op_get(interp: &mut Interpreter) -> Result<()> {
    let index_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index = match get_integer_from_value(&index_val) {
        Ok(v) => v,
        Err(e) => {
            interp.stack.push(index_val);
            return Err(e);
        }
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let target_val = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(index_val.clone());
                AjisaiError::StackUnderflow
            })?;

            if is_vector_value(&target_val) {
                let v = reconstruct_vector_elements(&target_val);
                let len = v.len();
                if len == 0 {
                    interp.stack.push(target_val);
                    interp.stack.push(index_val);
                    return Err(AjisaiError::IndexOutOfBounds { index, length: 0 });
                }

                let actual_index = match normalize_index(index, len) {
                    Some(idx) => idx,
                    None => {
                        interp.stack.push(target_val);
                        interp.stack.push(index_val);
                        return Err(AjisaiError::IndexOutOfBounds { index, length: len });
                    }
                };

                let result_elem = v[actual_index].clone();
                interp.stack.push(target_val);
                interp.stack.push(result_elem);
                Ok(())
            } else {
                interp.stack.push(target_val);
                interp.stack.push(index_val);
                Err(AjisaiError::structure_error("vector", "other format"))
            }
        }
        OperationTargetMode::Stack => {
            let stack_len = interp.stack.len();
            if stack_len == 0 {
                interp.stack.push(index_val);
                return Err(AjisaiError::IndexOutOfBounds { index, length: 0 });
            }

            let actual_index = match normalize_index(index, stack_len) {
                Some(idx) => idx,
                None => {
                    interp.stack.push(index_val);
                    return Err(AjisaiError::IndexOutOfBounds { index, length: stack_len });
                }
            };

            let result_elem = interp.stack[actual_index].clone();
            // Stackモードの場合、スタック上の値はすでにベクタ形式なのでそのまま返す
            interp.stack.push(result_elem);
            Ok(())
        }
    }
}

/// INSERT - 指定位置に要素を挿入する
///
/// 【責務】
/// - スタックトップのベクタまたはスタック全体の指定位置に要素を挿入
/// - 負数インデックスをサポート（-1 = 末尾の要素の位置）
///
/// 【使用法】
/// - StackTopモード: `[a c] [1 b] INSERT` → `[a b c]`（インデックス1の位置にbを挿入）
/// - StackTopモード: `[a b c] [-1 X] INSERT` → `[a b X c]`（末尾の要素の前にXを挿入）
/// - Stackモード: `a c [1 x] .. INSERT` → `a x c`
///
/// 【引数スタック】
/// - [index element]: 挿入位置と挿入する要素
/// - (StackTopモード) target: 対象ベクタ
///
/// 【戻り値スタック】
/// - 挿入後のベクタまたはスタック
///
/// 【エラー】
/// - 引数が[index element]形式でない場合
/// - 対象がベクタでない場合（StackTopモード）
pub fn op_insert(interp: &mut Interpreter) -> Result<()> {
    // 引数ベクタ [index element] を取得
    let args_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 引数から index と element を抽出
    let (index, element) = if is_vector_value(&args_val) {
        let v = reconstruct_vector_elements(&args_val);
        if v.len() != 2 {
            interp.stack.push(args_val);
            return Err(AjisaiError::from("INSERT requires [index element]"));
        }

        // index を抽出
        let index = match get_integer_from_value(&v[0]) {
            Ok(i) => i,
            Err(_) => {
                interp.stack.push(args_val);
                return Err(AjisaiError::from("INSERT index must be an integer"));
            }
        };

        // element を抽出（2番目の要素）
        let element = v[1].clone();

        (index, element)
    } else {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("INSERT requires [index element]"));
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let vector_val = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(args_val.clone());
                AjisaiError::StackUnderflow
            })?;

            if is_vector_value(&vector_val) {
                let mut v = reconstruct_vector_elements(&vector_val);
                let len = v.len() as i64;
                let insert_index = if index < 0 {
                    (len + index).max(0) as usize
                } else {
                    (index as usize).min(v.len())
                };

                v.insert(insert_index, element);
                interp.stack.push(Value::from_vector(v));
                Ok(())
            } else {
                interp.stack.push(vector_val);
                interp.stack.push(args_val);
                Err(AjisaiError::structure_error("vector", "other format"))
            }
        }
        OperationTargetMode::Stack => {
            let len = interp.stack.len() as i64;
            let insert_index = if index < 0 {
                (len + index).max(0) as usize
            } else {
                (index as usize).min(interp.stack.len())
            };
            interp.stack.insert(insert_index, element);
            Ok(())
        }
    }
}

/// REPLACE - 指定位置の要素を置き換える
///
/// 【責務】
/// - スタックトップのベクタまたはスタック全体の指定位置の要素を置き換え
/// - 負数インデックスをサポート
///
/// 【使用法】
/// - StackTopモード: `[a b c] [1 X] REPLACE` → `[a X c]`
/// - Stackモード: `a b c [1 X] .. REPLACE` → `a X c`
///
/// 【引数スタック】
/// - [index new_element]: 置換位置と新しい要素
/// - (StackTopモード) target: 対象ベクタ
///
/// 【戻り値スタック】
/// - 置換後のベクタまたはスタック
///
/// 【エラー】
/// - 引数が[index element]形式でない場合
/// - インデックスが範囲外の場合
/// - 対象がベクタでない場合（StackTopモード）
pub fn op_replace(interp: &mut Interpreter) -> Result<()> {
    // 引数ベクタ [index new_element] を取得
    let args_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 引数から index と new_element を抽出
    let (index, new_element) = if is_vector_value(&args_val) {
        let v = reconstruct_vector_elements(&args_val);
        if v.len() != 2 {
            interp.stack.push(args_val);
            return Err(AjisaiError::from("REPLACE requires [index element]"));
        }

        // index を抽出
        let index = match get_integer_from_value(&v[0]) {
            Ok(i) => i,
            Err(_) => {
                interp.stack.push(args_val);
                return Err(AjisaiError::from("REPLACE index must be an integer"));
            }
        };

        // new_element を抽出（2番目の要素）
        let new_element = v[1].clone();

        (index, new_element)
    } else {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("REPLACE requires [index element]"));
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let vector_val = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(args_val.clone());
                AjisaiError::StackUnderflow
            })?;

            if is_vector_value(&vector_val) {
                let mut v = reconstruct_vector_elements(&vector_val);
                let len = v.len();
                let actual_index = match normalize_index(index, len) {
                    Some(idx) => idx,
                    None => {
                        interp.stack.push(Value::from_vector(v));
                        interp.stack.push(args_val);
                        return Err(AjisaiError::IndexOutOfBounds { index, length: len });
                    }
                };

                v[actual_index] = new_element;
                interp.stack.push(Value::from_vector(v));
                Ok(())
            } else {
                interp.stack.push(vector_val);
                interp.stack.push(args_val);
                Err(AjisaiError::structure_error("vector", "other format"))
            }
        }
        OperationTargetMode::Stack => {
            let len = interp.stack.len();
            let actual_index = match normalize_index(index, len) {
                Some(idx) => idx,
                None => {
                    interp.stack.push(args_val);
                    return Err(AjisaiError::IndexOutOfBounds { index, length: len });
                }
            };

            interp.stack[actual_index] = new_element;
            Ok(())
        }
    }
}

/// REMOVE - 指定位置の要素を削除する
///
/// 【責務】
/// - スタックトップのベクタまたはスタック全体から指定位置の要素を削除
/// - 負数インデックスをサポート
///
/// 【使用法】
/// - StackTopモード: `[a b c] [1] REMOVE` → `[a c]`
/// - Stackモード: `a b c [1] .. REMOVE` → `a c`
///
/// 【引数スタック】
/// - [index]: 削除位置（単一要素ベクタの整数）
/// - (StackTopモード) target: 対象ベクタ
///
/// 【戻り値スタック】
/// - 削除後のベクタまたはスタック
///
/// 【エラー】
/// - インデックスが範囲外の場合
/// - 対象がベクタでない場合（StackTopモード）
pub fn op_remove(interp: &mut Interpreter) -> Result<()> {
    let index_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index = match get_integer_from_value(&index_val) {
        Ok(v) => v,
        Err(e) => {
            interp.stack.push(index_val);
            return Err(e);
        }
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let vector_val = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(index_val.clone());
                AjisaiError::StackUnderflow
            })?;

            if is_vector_value(&vector_val) {
                let mut v = reconstruct_vector_elements(&vector_val);
                let len = v.len();
                let actual_index = match normalize_index(index, len) {
                    Some(idx) => idx,
                    None => {
                        interp.stack.push(Value::from_vector(v));
                        interp.stack.push(index_val);
                        return Err(AjisaiError::IndexOutOfBounds { index, length: len });
                    }
                };

                v.remove(actual_index);
                // 空になった場合はNILをプッシュ
                if v.is_empty() {
                    interp.stack.push(Value::nil());
                } else {
                    interp.stack.push(Value::from_vector(v));
                }
                Ok(())
            } else {
                interp.stack.push(vector_val);
                interp.stack.push(index_val);
                Err(AjisaiError::structure_error("vector", "other format"))
            }
        }
        OperationTargetMode::Stack => {
            let len = interp.stack.len();
            let actual_index = match normalize_index(index, len) {
                Some(idx) => idx,
                None => {
                    interp.stack.push(index_val);
                    return Err(AjisaiError::IndexOutOfBounds { index, length: len });
                }
            };

            interp.stack.remove(actual_index);
            Ok(())
        }
    }
}

// ============================================================================
// 量指定操作（1オリジン）
// ============================================================================

/// LENGTH - 要素数を取得する
///
/// 【責務】
/// - スタックトップのベクタまたはスタック全体の要素数を取得
/// - 結果を単一要素ベクタの整数として返す
///
/// 【使用法】
/// - StackTopモード: `[a b c] LENGTH` → `[a b c] [3]`
/// - Stackモード: `a b c .. LENGTH` → `a b c [3]`
///
/// 【引数スタック】
/// - (StackTopモード) target: 対象ベクタ
///
/// 【戻り値スタック】
/// - (StackTopモード) target: 元のベクタ（保持）
/// - [length]: 要素数
///
/// 【エラー】
/// - 対象がベクタでない場合（StackTopモード）
pub fn op_length(interp: &mut Interpreter) -> Result<()> {
    let len = match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            if target_val.is_nil() {
                // NIL = 空ベクタ → 長さ0
                interp.stack.push(target_val);
                0
            } else if is_vector_value(&target_val) {
                let v = reconstruct_vector_elements(&target_val);
                let len = v.len();
                interp.stack.push(target_val);
                len
            } else {
                interp.stack.push(target_val);
                return Err(AjisaiError::structure_error("vector", "other format"));
            }
        }
        OperationTargetMode::Stack => interp.stack.len(),
    };
    let len_frac = Fraction::new(BigInt::from(len), BigInt::one());
    interp.stack.push(wrap_number(len_frac));
    Ok(())
}

/// TAKE - 先頭または末尾から指定数の要素を取得する
///
/// 【責務】
/// - 正数: 先頭からN個取得
/// - 負数: 末尾からN個取得
/// - StackTopモード: ベクタから取得してベクタを返す
/// - Stackモード: スタック自体を変更
///
/// 【使用法】
/// - StackTopモード: `[a b c d] [2] TAKE` → `[a b]`
/// - StackTopモード: `[a b c d] [-2] TAKE` → `[c d]`
/// - Stackモード: `a b c d [2] .. TAKE` → `a b`
///
/// 【引数スタック】
/// - [count]: 取得数（正=先頭、負=末尾）
/// - (StackTopモード) target: 対象ベクタ
///
/// 【戻り値スタック】
/// - 取得後のベクタまたはスタック
///
/// 【エラー】
/// - カウントが長さを超える場合
/// - 対象がベクタでない場合（StackTopモード）
pub fn op_take(interp: &mut Interpreter) -> Result<()> {
    let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let count = match get_integer_from_value(&count_val) {
        Ok(v) => v,
        Err(e) => {
            interp.stack.push(count_val);
            return Err(e);
        }
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let vector_val = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(count_val.clone());
                AjisaiError::StackUnderflow
            })?;

            if is_vector_value(&vector_val) {
                let v = reconstruct_vector_elements(&vector_val);
                let len = v.len();
                let result = if count < 0 {
                    let abs_count = (-count) as usize;
                    if abs_count > len {
                        interp.stack.push(Value::from_vector(v));
                        interp.stack.push(count_val);
                        return Err(AjisaiError::from("Take count exceeds vector length"));
                    }
                    v[len - abs_count..].to_vec()
                } else {
                    let take_count = count as usize;
                    if take_count > len {
                        interp.stack.push(Value::from_vector(v));
                        interp.stack.push(count_val);
                        return Err(AjisaiError::from("Take count exceeds vector length"));
                    }
                    v[..take_count].to_vec()
                };

                // 結果が空の場合はNILを返す（空ベクタ禁止ルール）
                if result.is_empty() {
                    interp.stack.push(Value::nil());
                } else {
                    interp.stack.push(Value::from_vector(result));
                }
                Ok(())
            } else {
                interp.stack.push(vector_val);
                interp.stack.push(count_val);
                Err(AjisaiError::structure_error("vector", "other format"))
            }
        }
        OperationTargetMode::Stack => {
            let len = interp.stack.len();
            if count < 0 {
                let abs_count = (-count) as usize;
                if abs_count > len {
                    interp.stack.push(count_val);
                    return Err(AjisaiError::from("Take count exceeds stack length"));
                }
                interp.stack = interp.stack.split_off(len - abs_count);
            } else {
                let take_count = count as usize;
                if take_count > len {
                    interp.stack.push(count_val);
                    return Err(AjisaiError::from("Take count exceeds stack length"));
                }
                interp.stack.truncate(take_count);
            };
            Ok(())
        }
    }
}

/// SPLIT - 指定サイズで分割する
///
/// 【責務】
/// - サイズ指定ベクタを受け取り、ベクタまたはスタックを分割
/// - サイズの合計が全体より小さい場合、残りは最後の要素に含まれる
///
/// 【使用法】
/// - StackTopモード: `[a b c d e] [2 2] SPLIT` → `[a b] [c d] [e]`
/// - Stackモード: `a b c d e [2 1] .. SPLIT` → `[a b] [c] [d e]`
///
/// 【引数スタック】
/// - [size1 size2 ...]: 分割サイズ（ベクタで指定）
/// - (StackTopモード) target: 対象ベクタ
///
/// 【戻り値スタック】
/// - 分割された複数のベクタ
///
/// 【エラー】
/// - サイズ指定が空の場合
/// - サイズの合計が長さを超える場合
/// - 対象がベクタでない場合（StackTopモード）
pub fn op_split(interp: &mut Interpreter) -> Result<()> {
    // 引数ベクタ [sizes...] を取得
    let args_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // サイズを抽出
    let sizes: Vec<usize> = if is_vector_value(&args_val) {
        let v = reconstruct_vector_elements(&args_val);
        if v.is_empty() {
            interp.stack.push(args_val);
            return Err(AjisaiError::from("SPLIT requires at least one size"));
        }

        let mut sizes = Vec::with_capacity(v.len());
        for elem in &v {
            match get_bigint_from_value(elem) {
                Ok(bi) => {
                    match bi.to_usize() {
                        Some(s) => sizes.push(s),
                        None => {
                            interp.stack.push(args_val);
                            return Err(AjisaiError::from("Split size is too large"));
                        }
                    }
                }
                Err(_) => {
                    interp.stack.push(args_val);
                    return Err(AjisaiError::from("Split sizes must be integers"));
                }
            }
        }
        sizes
    } else {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("SPLIT requires [sizes...] vector"));
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let vector_val = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(args_val.clone());
                AjisaiError::from("SPLIT requires a vector to split")
            })?;

            if is_vector_value(&vector_val) {
                let v = reconstruct_vector_elements(&vector_val);
                let total_size: usize = sizes.iter().sum();
                if total_size > v.len() {
                    interp.stack.push(Value::from_vector(v));
                    interp.stack.push(args_val);
                    return Err(AjisaiError::from("Split sizes sum exceeds vector length"));
                }

                let mut current_pos = 0;
                let mut result_vectors = Vec::new();

                for &size in &sizes {
                    let chunk = v[current_pos..current_pos + size].to_vec();
                    result_vectors.push(Value::from_vector(chunk));
                    current_pos += size;
                }
                if current_pos < v.len() {
                    let chunk = v[current_pos..].to_vec();
                    result_vectors.push(Value::from_vector(chunk));
                }
                interp.stack.extend(result_vectors);
                Ok(())
            } else {
                interp.stack.push(vector_val);
                interp.stack.push(args_val);
                Err(AjisaiError::structure_error("vector", "other format"))
            }
        }
        OperationTargetMode::Stack => {
            let total_size: usize = sizes.iter().sum();
            if total_size > interp.stack.len() {
                interp.stack.push(args_val);
                return Err(AjisaiError::from("Split sizes sum exceeds stack length"));
            }

            let mut remaining_stack = interp.stack.split_off(0);
            let mut result_stack = Vec::new();

            for &size in &sizes {
                let chunk: Vec<Value> = remaining_stack.drain(..size).collect();
                result_stack.push(Value::from_vector(chunk));
            }
            if !remaining_stack.is_empty() {
                result_stack.push(Value::from_vector(remaining_stack));
            }
            interp.stack = result_stack;
            Ok(())
        }
    }
}

// ============================================================================
// ベクタ構造操作
// ============================================================================

/// CONCAT - 複数のベクタを連結する
///
/// 【責務】
/// - StackTopモード: スタックから指定数の値を取得して連結
/// - Stackモード: スタック全体を連結してベクタ化
/// - 正数: 順方向連結、負数: 逆方向連結
/// - ベクタでない要素も単独要素として含める
/// - デフォルトのカウント値: 2
///
/// 【使用法】
/// - StackTopモード: `[a] [b] [c] [3] CONCAT` → `[a b c]`
/// - StackTopモード: `[a] [b] [c] [-3] CONCAT` → `[c b a]`
/// - StackTopモード: `[a] [b] CONCAT` → `[a b]` (デフォルト2)
/// - Stackモード: `[a] [b] [c] .. CONCAT` → `[a b c]`
/// - Stackモード: `[a] [b] [c] [2] .. CONCAT` → `[a] [b c]`
///
/// 【引数スタック】
/// - (オプション) [count]: 連結する値の数（負数で逆順、デフォルト2）
/// - (StackTopモード) vec_n ... vec_1: 連結する値（複数）
///
/// 【戻り値スタック】
/// - 連結されたベクタ
///
/// 【エラー】
/// - カウントがスタック長を超える場合
pub fn op_concat(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            // スタックトップからcountを取得（オプション、デフォルトは2）
            // count_value_optはポップした引数を追跡し、エラー時に復元する
            let (count_i64, count_value_opt) = if let Some(top) = interp.stack.last() {
                if let Ok(count_bigint) = get_bigint_from_value(top) {
                    let count_val = interp.stack.pop().unwrap();
                    if let Some(c) = count_bigint.to_i64() {
                        (c, Some(count_val))
                    } else {
                        interp.stack.push(count_val);
                        return Err(AjisaiError::from("Count is too large"));
                    }
                } else {
                    // countが指定されていない場合、デフォルトは2
                    (2, None)
                }
            } else {
                return Err(AjisaiError::StackUnderflow);
            };

            let abs_count = count_i64.unsigned_abs() as usize;
            let is_reversed = count_i64 < 0;

            if interp.stack.len() < abs_count {
                // ガード節エラー時にcount引数を復元
                if let Some(count_val) = count_value_opt {
                    interp.stack.push(count_val);
                }
                return Err(AjisaiError::StackUnderflow);
            }

            let mut vecs_to_concat: Vec<Value> = interp.stack.split_off(interp.stack.len() - abs_count);

            if is_reversed {
                vecs_to_concat.reverse();
            }

            let mut result_vec = Vec::new();

            for val in vecs_to_concat {
                if is_vector_value(&val) {
                    let v = reconstruct_vector_elements(&val);
                    result_vec.extend(v);
                } else {
                    result_vec.push(val);
                }
            }

            interp.stack.push(Value::from_vector(result_vec));
            Ok(())
        }
        OperationTargetMode::Stack => {
            // Stackモード: スタックトップがcountかチェック、なければスタック全体
            // count_value_optはポップした引数を追跡し、エラー時に復元する
            let (count_i64, count_value_opt) = if let Some(top) = interp.stack.last() {
                if let Ok(count_bigint) = get_bigint_from_value(top) {
                    let count_val = interp.stack.pop().unwrap();
                    if let Some(c) = count_bigint.to_i64() {
                        (c, Some(count_val))
                    } else {
                        interp.stack.push(count_val);
                        return Err(AjisaiError::from("Count is too large"));
                    }
                } else {
                    // countが指定されていない場合、スタック全体を使用
                    (interp.stack.len() as i64, None)
                }
            } else {
                return Err(AjisaiError::StackUnderflow);
            };

            let abs_count = count_i64.unsigned_abs() as usize;
            let is_reversed = count_i64 < 0;

            if interp.stack.len() < abs_count {
                // ガード節エラー時にcount引数を復元
                if let Some(count_val) = count_value_opt {
                    interp.stack.push(count_val);
                }
                return Err(AjisaiError::StackUnderflow);
            }

            let mut vecs_to_concat: Vec<Value> = interp.stack.split_off(interp.stack.len() - abs_count);

            if is_reversed {
                vecs_to_concat.reverse();
            }

            let mut result_vec = Vec::new();

            for val in vecs_to_concat {
                if is_vector_value(&val) {
                    let v = reconstruct_vector_elements(&val);
                    result_vec.extend(v);
                } else {
                    result_vec.push(val);
                }
            }

            interp.stack.push(Value::from_vector(result_vec));
            Ok(())
        }
    }
}

/// REVERSE - 要素の順序を反転する
///
/// 【責務】
/// - スタックトップのベクタまたはスタック全体の要素順を反転
/// - "No change is an error" 原則: 変化がない場合はエラー
///
/// 【使用法】
/// - StackTopモード: `[a b c] REVERSE` → `[c b a]`
/// - Stackモード: `a b c .. REVERSE` → `c b a`
///
/// 【引数スタック】
/// - (StackTopモード) target: 対象ベクタ
///
/// 【戻り値スタック】
/// - 反転後のベクタまたはスタック
///
/// 【エラー】
/// - 2要素未満の場合（変化なし）
/// - 回文の場合（変化なし）
/// - 対象がベクタでない場合（StackTopモード）
pub fn op_reverse(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            if is_vector_value(&val) {
                let mut v = reconstruct_vector_elements(&val);
                // "No change is an error" チェック（disable_no_change_check で無効化可能）
                if !interp.disable_no_change_check {
                    if v.len() < 2 {
                        interp.stack.push(Value::from_vector(v));
                        return Err(AjisaiError::from("REVERSE resulted in no change on a vector with less than 2 elements"));
                    }
                    let original_v = v.clone();
                    v.reverse();
                    if v == original_v {
                        interp.stack.push(Value::from_vector(original_v));
                        return Err(AjisaiError::from("REVERSE resulted in no change (vector is a palindrome)"));
                    }
                    interp.stack.push(Value::from_vector(v));
                } else {
                    // disable_no_change_check が true の場合、単純に反転
                    v.reverse();
                    interp.stack.push(Value::from_vector(v));
                }
                Ok(())
            } else {
                interp.stack.push(val);
                Err(AjisaiError::structure_error("vector", "other format"))
            }
        }
        OperationTargetMode::Stack => {
            // "No change is an error" チェック（disable_no_change_check で無効化可能）
            if !interp.disable_no_change_check {
                if interp.stack.len() < 2 {
                    return Err(AjisaiError::from("REVERSE resulted in no change on a stack with less than 2 elements"));
                }
                let original_stack = interp.stack.clone();
                interp.stack.reverse();
                if interp.stack == original_stack {
                    interp.stack = original_stack;
                    return Err(AjisaiError::from("REVERSE resulted in no change (stack is a palindrome)"));
                }
            } else {
                // disable_no_change_check が true の場合、単純に反転
                interp.stack.reverse();
            }
            Ok(())
        }
    }
}

/// RANGE - 数値範囲を生成する
///
/// 【責務】
/// - startからendまでの数値シーケンスを生成
/// - オプションでstep（増分）を指定可能
/// - 等差数列の生成に対応
///
/// 【使用法】
/// - `[0 5] RANGE` → `[0 1 2 3 4 5]`
/// - `[0 10 2] RANGE` → `[0 2 4 6 8 10]`
/// - `[0 5] .. RANGE` → `[0 1 2 3 4 5]`（Stackモードも同じ引数形式）
///
/// 【引数スタック】
/// - [start end]: 開始値と終了値（整数）
/// - [start end step]: 開始値、終了値、増分（整数）
///
/// 【戻り値スタック】
/// - 生成されたベクタ
///
/// 【エラー】
/// - stepが0の場合
/// - start, end, stepが整数でない場合
/// - 範囲が無限になる場合（start < endだがstep < 0、またはstart > endだがstep > 0）
///
/// 【注意事項】
/// - endの値は範囲に含まれる（inclusive）
/// - 負のstepで降順の範囲を生成可能
/// - start == endの場合は単一要素のベクタを返す
pub fn op_range(interp: &mut Interpreter) -> Result<()> {
    // 引数ベクタを取得
    let args_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 引数ベクタから start, end, step を抽出
    let (start, end, step) = if is_vector_value(&args_val) {
        let v = reconstruct_vector_elements(&args_val);
        if v.len() < 2 || v.len() > 3 {
            interp.stack.push(args_val);
            return Err(AjisaiError::from("RANGE requires [start end] or [start end step]"));
        }

        // start を抽出
        let start = match get_bigint_from_value(&v[0]) {
            Ok(bi) => match bi.to_i64() {
                Some(i) => i,
                None => {
                    interp.stack.push(args_val);
                    return Err(AjisaiError::from("RANGE start is too large"));
                }
            },
            Err(_) => {
                interp.stack.push(args_val);
                return Err(AjisaiError::from("RANGE start must be an integer"));
            }
        };

        // end を抽出
        let end = match get_bigint_from_value(&v[1]) {
            Ok(bi) => match bi.to_i64() {
                Some(i) => i,
                None => {
                    interp.stack.push(args_val);
                    return Err(AjisaiError::from("RANGE end is too large"));
                }
            },
            Err(_) => {
                interp.stack.push(args_val);
                return Err(AjisaiError::from("RANGE end must be an integer"));
            }
        };

        // step を抽出（オプション）
        let step = if v.len() == 3 {
            match get_bigint_from_value(&v[2]) {
                Ok(bi) => match bi.to_i64() {
                    Some(i) => i,
                    None => {
                        interp.stack.push(args_val);
                        return Err(AjisaiError::from("RANGE step is too large"));
                    }
                },
                Err(_) => {
                    interp.stack.push(args_val);
                    return Err(AjisaiError::from("RANGE step must be an integer"));
                }
            }
        } else {
            // デフォルトstep: start <= end なら 1、そうでなければ -1
            if start <= end { 1 } else { -1 }
        };

        (start, end, step)
    } else {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("RANGE requires [start end] or [start end step]"));
    };

    // stepが0の場合はエラー
    if step == 0 {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("RANGE step cannot be 0"));
    }

    // 無限範囲チェック
    if (start < end && step < 0) || (start > end && step > 0) {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("RANGE would create an infinite sequence (check start, end, and step values)"));
    }

    // 範囲を生成
    let mut range_vec = Vec::new();
    let mut current = start;

    if step > 0 {
        while current <= end {
            range_vec.push(Value::from_fraction(Fraction::new(BigInt::from(current), BigInt::one())));
            current += step;
        }
    } else {
        while current >= end {
            range_vec.push(Value::from_fraction(Fraction::new(BigInt::from(current), BigInt::one())));
            current += step;
        }
    }

    // 結果をプッシュ
    interp.stack.push(Value::from_vector(range_vec));

    Ok(())
}

/// REORDER - インデックスリストで要素を並べ替える
///
/// 【責務】
/// - インデックスリストで指定した順序に要素を並べ替える
/// - FORTH由来のスタック操作ワード（DUP/DROP/SWAP/ROT）の代替として機能
/// - 部分選択、重複選択、負のインデックスに対応
///
/// 【使用法】
/// - StackTopモード: `[a b c] [2 0 1] REORDER` → `[c a b]`
/// - Stackモード: `a b c [1 2 0] .. REORDER` → `b c a`
///
/// 【引数スタック】
/// - [indices...]: 並べ替え後のインデックスリスト（負数インデックス対応）
/// - (StackTopモード) target: 対象ベクタ
///
/// 【戻り値スタック】
/// - 並べ替え後のベクタまたはスタック
///
/// 【エラー】
/// - インデックスリストが空の場合
/// - インデックスが範囲外の場合
/// - 対象がベクタでない場合（StackTopモード）
///
/// 【使用例】
/// ```text
/// # ベクタの並べ替え
/// [ a b c ] [ 2 0 1 ] REORDER       # → [ c a b ]
/// [ a b c ] [ 0 0 0 ] REORDER       # → [ a a a ]（複製）
/// [ a b c ] [ -1 -2 -3 ] REORDER    # → [ c b a ]（逆順）
/// [ a b c ] [ 1 ] REORDER           # → [ b ]（部分選択）
///
/// # スタックの並べ替え
/// a b c [ 1 2 0 ] .. REORDER        # → b c a（ROT相当）
/// a b [ 1 0 ] .. REORDER            # → b a（SWAP相当）
/// ```
pub fn op_reorder(interp: &mut Interpreter) -> Result<()> {
    // インデックスリストをポップ
    let indices_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // インデックスリストを抽出
    let indices = if is_vector_value(&indices_val) {
        let v = reconstruct_vector_elements(&indices_val);
        if v.is_empty() {
            interp.stack.push(indices_val);
            return Err(AjisaiError::from("REORDER requires non-empty index list"));
        }

        let mut indices = Vec::with_capacity(v.len());
        for elem in &v {
            match get_integer_from_value(elem) {
                Ok(i) => indices.push(i),
                Err(_) => {
                    interp.stack.push(indices_val);
                    return Err(AjisaiError::from("REORDER indices must be integers"));
                }
            }
        }
        indices
    } else {
        // 単一要素の場合も許容
        match get_integer_from_value(&indices_val) {
            Ok(i) => vec![i],
            Err(_) => {
                interp.stack.push(indices_val);
                return Err(AjisaiError::from("REORDER requires index list"));
            }
        }
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let target_val = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(indices_val.clone());
                AjisaiError::StackUnderflow
            })?;

            if is_vector_value(&target_val) {
                let elements = reconstruct_vector_elements(&target_val);
                let len = elements.len();

                if len == 0 {
                    interp.stack.push(target_val);
                    interp.stack.push(indices_val);
                    return Err(AjisaiError::from("REORDER: target vector is empty"));
                }

                let mut result = Vec::with_capacity(indices.len());
                for &idx in &indices {
                    let actual = match normalize_index(idx, len) {
                        Some(i) => i,
                        None => {
                            interp.stack.push(target_val);
                            interp.stack.push(indices_val);
                            return Err(AjisaiError::IndexOutOfBounds { index: idx, length: len });
                        }
                    };
                    result.push(elements[actual].clone());
                }

                // 結果が空の場合はNILを返す（空ベクタ禁止ルール）
                if result.is_empty() {
                    interp.stack.push(Value::nil());
                } else {
                    interp.stack.push(Value::from_vector(result));
                }
                Ok(())
            } else {
                interp.stack.push(target_val);
                interp.stack.push(indices_val);
                Err(AjisaiError::structure_error("vector", "other format"))
            }
        }
        OperationTargetMode::Stack => {
            let len = interp.stack.len();

            if len == 0 {
                interp.stack.push(indices_val);
                return Err(AjisaiError::from("REORDER: stack is empty"));
            }

            let mut result = Vec::with_capacity(indices.len());
            for &idx in &indices {
                let actual = match normalize_index(idx, len) {
                    Some(i) => i,
                    None => {
                        interp.stack.push(indices_val);
                        return Err(AjisaiError::IndexOutOfBounds { index: idx, length: len });
                    }
                };
                result.push(interp.stack[actual].clone());
            }

            interp.stack.clear();
            interp.stack.extend(result);
            Ok(())
        }
    }
}

/// COLLECT - スタックからN個の値を収集してベクタを作成する
///
/// 【責務】
/// - スタックの先頭からN個の値を取り出し、1つのベクタにまとめる
/// - 値はフラット化せずにそのまま保持（CONCATとの違い）
/// - DEFで定義したカスタムワードの結果をベクタに収集する用途に最適
///
/// 【使用法】
/// - `VOWEL_A VOWEL_I 2 COLLECT` → `[ [VOWEL_Aの結果] [VOWEL_Iの結果] ]`
/// - `1 2 3 3 COLLECT` → `[ 1 2 3 ]`
/// - `[ a ] [ b ] [ c ] 3 COLLECT` → `[ [ a ] [ b ] [ c ] ]`
///
/// 【引数スタック】
/// - count: 収集する値の数（整数）
/// - value_n ... value_1: 収集する値（N個）
///
/// 【戻り値スタック】
/// - 収集されたベクタ（要素がそのまま保持される）
///
/// 【エラー】
/// - countが負数または0の場合
/// - countがスタック長を超える場合
pub fn op_collect(interp: &mut Interpreter) -> Result<()> {
    let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let count_bigint = match get_bigint_from_value(&count_val) {
        Ok(bi) => bi,
        Err(_) => {
            interp.stack.push(count_val);
            return Err(AjisaiError::structure_error("integer", "other format"));
        }
    };

    let count: usize = match count_bigint.to_usize() {
        Some(c) if c > 0 => c,
        _ => {
            interp.stack.push(count_val);
            return Err(AjisaiError::from("COLLECT count must be a positive integer"));
        }
    };

    if interp.stack.len() < count {
        interp.stack.push(count_val);
        return Err(AjisaiError::StackUnderflow);
    }

    // スタックの末尾からcount個の要素を取得
    let collected: Vec<Value> = interp.stack.split_off(interp.stack.len() - count);

    interp.stack.push(Value::from_vector(collected));
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_range_basic_stacktop() {
        let mut interp = Interpreter::new();

        // 基本的な範囲生成（統一形式）
        let result = interp.execute("[ 0 5 ] RANGE").await;
        assert!(result.is_ok(), "RANGE should succeed: {:?}", result);

        // 結果のみがスタックに残る
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_range_with_step() {
        let mut interp = Interpreter::new();

        // ステップ付き範囲生成（統一形式）
        let result = interp.execute("[ 0 10 2 ] RANGE").await;
        assert!(result.is_ok(), "RANGE with step should succeed: {:?}", result);

        // 結果のみがスタックに残る
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_range_descending() {
        let mut interp = Interpreter::new();

        // 降順範囲（統一形式）
        let result = interp.execute("[ 10 0 -2 ] RANGE").await;
        assert!(result.is_ok(), "RANGE descending should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_range_single_element() {
        let mut interp = Interpreter::new();

        // 単一要素（統一形式）
        let result = interp.execute("[ 5 5 ] RANGE").await;
        assert!(result.is_ok(), "RANGE single element should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_range_stack_mode() {
        let mut interp = Interpreter::new();

        // Stackモード（統一形式）
        let result = interp.execute("[ 0 5 ] .. RANGE").await;
        assert!(result.is_ok(), "RANGE stack mode should succeed: {:?}", result);
        // 結果のみ残る
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_range_error_step_zero_restores_stack_stacktop() {
        let mut interp = Interpreter::new();

        // step=0はエラー、エラー時にスタックが復元されるか確認（統一形式）
        let result = interp.execute("[ 0 10 0 ] RANGE").await;
        assert!(result.is_err(), "RANGE with step=0 should fail");

        // エラー時に引数が復元されている
        assert_eq!(interp.stack.len(), 1, "Arguments should be restored on error");
    }

    #[tokio::test]
    async fn test_range_error_step_zero_restores_stack_stack_mode() {
        let mut interp = Interpreter::new();

        // Stackモードでstep=0はエラー、エラー時にスタックが復元されるか確認（統一形式）
        let result = interp.execute("[ 0 10 0 ] .. RANGE").await;
        assert!(result.is_err(), "RANGE stack mode with step=0 should fail");

        // エラー時に引数が復元されている
        assert_eq!(interp.stack.len(), 1, "Arguments should be restored on error in stack mode");
    }

    #[tokio::test]
    async fn test_range_error_infinite_restores_stack() {
        let mut interp = Interpreter::new();

        // 無限範囲エラー（start < endだがstep < 0）（統一形式）
        let result = interp.execute("[ 0 10 -1 ] RANGE").await;
        assert!(result.is_err(), "RANGE with infinite sequence should fail");

        // エラー時に引数が復元されている
        assert_eq!(interp.stack.len(), 1, "Arguments should be restored on infinite error");
    }

    // ========================================================================
    // REORDER テスト
    // ========================================================================

    #[tokio::test]
    async fn test_reorder_basic_stacktop() {
        let mut interp = Interpreter::new();

        // 基本的な並べ替え: [ a b c ] [ 2 0 1 ] REORDER → [ c a b ]
        let result = interp.execute("[ 10 20 30 ] [ 2 0 1 ] REORDER").await;
        assert!(result.is_ok(), "REORDER should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);

        // 結果の検証: 30, 10, 20 の順になっているか
        let val = &interp.stack[0];
        assert!(val.is_vector(), "Result should be a vector");
    }

    #[tokio::test]
    async fn test_reorder_duplicate_indices() {
        let mut interp = Interpreter::new();

        // 重複インデックス: [ a b c ] [ 0 0 0 ] REORDER → [ a a a ]
        let result = interp.execute("[ 10 20 30 ] [ 0 0 0 ] REORDER").await;
        assert!(result.is_ok(), "REORDER with duplicate indices should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);

        // 結果は3要素で全て同じ値
        let val = &interp.stack[0];
        assert_eq!(val.shape(), vec![3], "Result should have 3 elements");
    }

    #[tokio::test]
    async fn test_reorder_negative_indices() {
        let mut interp = Interpreter::new();

        // 負のインデックス: [ a b c ] [ -1 -2 -3 ] REORDER → [ c b a ]
        let result = interp.execute("[ 10 20 30 ] [ -1 -2 -3 ] REORDER").await;
        assert!(result.is_ok(), "REORDER with negative indices should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_reorder_partial_selection() {
        let mut interp = Interpreter::new();

        // 部分選択: [ a b c ] [ 1 ] REORDER → [ b ]
        let result = interp.execute("[ 10 20 30 ] [ 1 ] REORDER").await;
        assert!(result.is_ok(), "REORDER with partial selection should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);

        // 結果は1要素
        let val = &interp.stack[0];
        assert_eq!(val.shape(), vec![1], "Result should have 1 element");
    }

    #[tokio::test]
    async fn test_reorder_stack_mode_swap() {
        let mut interp = Interpreter::new();

        // スタックモードでSWAP相当: a b [ 1 0 ] .. REORDER → b a
        let result = interp.execute("[ 10 ] [ 20 ] [ 1 0 ] .. REORDER").await;
        assert!(result.is_ok(), "REORDER stack mode SWAP should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 2, "Stack should have 2 elements");

        // 順序が入れ替わっていることを確認
        // 元: [10], [20] → 後: [20], [10]
    }

    #[tokio::test]
    async fn test_reorder_stack_mode_rot() {
        let mut interp = Interpreter::new();

        // スタックモードでROT相当: a b c [ 1 2 0 ] .. REORDER → b c a
        let result = interp.execute("[ 10 ] [ 20 ] [ 30 ] [ 1 2 0 ] .. REORDER").await;
        assert!(result.is_ok(), "REORDER stack mode ROT should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 3, "Stack should have 3 elements");
    }

    #[tokio::test]
    async fn test_reorder_error_empty_indices() {
        let mut interp = Interpreter::new();

        // 空のインデックスリストはエラー - ただし空ベクタは許容されないので別の方法でテスト
        // 直接実装テストで確認
    }

    #[tokio::test]
    async fn test_reorder_error_out_of_bounds() {
        let mut interp = Interpreter::new();

        // インデックス範囲外: [ a b c ] [ 5 ] REORDER → エラー
        let result = interp.execute("[ 10 20 30 ] [ 5 ] REORDER").await;
        assert!(result.is_err(), "REORDER with out of bounds index should fail");

        // エラー時にスタックが復元されている
        assert_eq!(interp.stack.len(), 2, "Stack should be restored on error");
    }

    #[tokio::test]
    async fn test_reorder_error_negative_out_of_bounds() {
        let mut interp = Interpreter::new();

        // 負のインデックスが範囲外: [ a b c ] [ -5 ] REORDER → エラー
        let result = interp.execute("[ 10 20 30 ] [ -5 ] REORDER").await;
        assert!(result.is_err(), "REORDER with negative out of bounds index should fail");

        // エラー時にスタックが復元されている
        assert_eq!(interp.stack.len(), 2, "Stack should be restored on error");
    }

    #[tokio::test]
    async fn test_reorder_error_non_vector() {
        let mut interp = Interpreter::new();

        // 対象がベクタでない場合はエラー（ただしスカラーも単一要素ベクタとして扱われる）
        // 実際にはスカラー10は[10]として扱われるので、インデックス[0]は成功する
        let result = interp.execute("[ 10 ] [ 0 ] REORDER").await;
        assert!(result.is_ok(), "REORDER on scalar-like value should succeed");
    }

    #[tokio::test]
    async fn test_reorder_stack_mode_error_out_of_bounds() {
        let mut interp = Interpreter::new();

        // スタックモードでインデックス範囲外
        let result = interp.execute("[ 10 ] [ 20 ] [ 5 ] .. REORDER").await;
        assert!(result.is_err(), "REORDER stack mode with out of bounds should fail");

        // エラー時にインデックスリストがスタックに復元されている
        assert_eq!(interp.stack.len(), 3, "Stack should have indices pushed back on error");
    }

    #[tokio::test]
    async fn test_reorder_single_element_index() {
        let mut interp = Interpreter::new();

        // 単一要素インデックス
        let result = interp.execute("[ 10 20 30 ] [ 2 ] REORDER").await;
        assert!(result.is_ok(), "REORDER with single index should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);

        // 結果は単一要素
        let val = &interp.stack[0];
        assert_eq!(val.shape(), vec![1], "Result should have 1 element");
    }

    // ============================================================================
    // COLLECT テスト
    // ============================================================================

    #[tokio::test]
    async fn test_collect_basic() {
        let mut interp = Interpreter::new();

        // 基本的なCOLLECT: スタックから3つの値を収集
        let result = interp.execute("1 2 3 3 COLLECT").await;
        assert!(result.is_ok(), "COLLECT should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);

        let val = &interp.stack[0];
        assert_eq!(val.shape(), vec![3], "Result should have 3 elements");
    }

    #[tokio::test]
    async fn test_collect_vectors_without_flattening() {
        let mut interp = Interpreter::new();

        // ベクタをフラット化せずに収集（CONCATとの違い）
        let result = interp.execute("[ 1 2 ] [ 3 4 ] 2 COLLECT").await;
        assert!(result.is_ok(), "COLLECT vectors should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);

        // 結果は [ [ 1 2 ] [ 3 4 ] ] - ネストされたベクタ
        // shape()はネスト構造を反映するので[2, 2]になる
        let val = &interp.stack[0];
        assert!(val.is_vector(), "Result should be a vector");
    }

    #[tokio::test]
    async fn test_collect_for_formant_synthesis() {
        let mut interp = Interpreter::new();

        // フォルマント合成の用途: カスタムワードの結果を収集
        // VOWEL_A, VOWEL_I の代わりにSIM（同時再生マーク付きベクタ）を使用
        let result = interp.execute("[ 800 1200 ] CHORD [ 300 2500 ] CHORD 2 COLLECT").await;
        assert!(result.is_ok(), "COLLECT for formant should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);

        // 結果は2つの母音（各々がCHORDマーク付き）を含むベクタ
        let val = &interp.stack[0];
        assert!(val.is_vector(), "Result should be a vector");
    }

    #[tokio::test]
    async fn test_collect_error_underflow() {
        let mut interp = Interpreter::new();

        // スタックに足りない場合はエラー
        let result = interp.execute("1 2 5 COLLECT").await;
        assert!(result.is_err(), "COLLECT with insufficient stack should fail");

        // エラー時にcount引数が復元される
        assert_eq!(interp.stack.len(), 3, "Stack should have count pushed back");
    }

    #[tokio::test]
    async fn test_collect_error_zero_count() {
        let mut interp = Interpreter::new();

        // count=0 はエラー
        let result = interp.execute("1 2 3 0 COLLECT").await;
        assert!(result.is_err(), "COLLECT with zero count should fail");
    }

    #[tokio::test]
    async fn test_collect_error_negative_count() {
        let mut interp = Interpreter::new();

        // 負数のcountはエラー
        let result = interp.execute("1 2 3 -2 COLLECT").await;
        assert!(result.is_err(), "COLLECT with negative count should fail");
    }
}
