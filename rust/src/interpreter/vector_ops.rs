// rust/src/interpreter/vector_ops.rs
//
// 【責務】
// ベクタおよびスタックに対する位置・構造操作を実装する。
// 0オリジンの位置指定操作（GET/INSERT/REPLACE/REMOVE）、
// 1オリジンの量指定操作（LENGTH/TAKE/SPLIT）、
// およびベクタ構造操作（CONCAT/REVERSE/LEVEL）を提供する。

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{get_bigint_from_value, normalize_index, unwrap_single_element, wrap_value, wrap_number};
use crate::types::{Value, ValueType};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};
use std::collections::VecDeque;

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
    let index_bigint = match get_bigint_from_value(&index_val) {
        Ok(v) => v,
        Err(e) => {
            interp.stack.push(index_val);
            return Err(e);
        }
    };
    let index = match index_bigint.to_i64() {
        Some(v) => v,
        None => {
            interp.stack.push(index_val);
            return Err(AjisaiError::from("Index is too large"));
        }
    };

    match interp.operation_target {
        OperationTarget::StackTop => {
            let target_val = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(index_val.clone());
                AjisaiError::StackUnderflow
            })?;

            match &target_val.val_type {
                ValueType::Vector(v) => {
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
                    // 結果を単一要素Vectorでラップ
                    interp.stack.push(wrap_value(result_elem));
                    Ok(())
                },
                _ => {
                    interp.stack.push(target_val);
                    interp.stack.push(index_val);
                    Err(AjisaiError::type_error("vector", "other type"))
                }
            }
        }
        OperationTarget::Stack => {
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
/// - 単一要素ベクタは自動的にアンラップして挿入
///
/// 【使用法】
/// - StackTopモード: `[a c] [1] [b] INSERT` → `[a b c]`（インデックス1の位置に挿入）
/// - StackTopモード: `[a b c] [-1] [X] INSERT` → `[a b X c]`（末尾の要素の前に挿入）
/// - Stackモード: `a c [1] x .. INSERT` → `a x c`
///
/// 【引数スタック】
/// - element: 挿入する要素
/// - [index]: 挿入位置（単一要素ベクタの整数）
/// - (StackTopモード) target: 対象ベクタ
///
/// 【戻り値スタック】
/// - 挿入後のベクタまたはスタック
///
/// 【エラー】
/// - 対象がベクタでない場合（StackTopモード）
pub fn op_insert(interp: &mut Interpreter) -> Result<()> {
    let element = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index_val = interp.stack.pop().ok_or_else(|| {
        interp.stack.push(element.clone());
        AjisaiError::StackUnderflow
    })?;
    let index_bigint = match get_bigint_from_value(&index_val) {
        Ok(v) => v,
        Err(e) => {
            interp.stack.push(index_val);
            interp.stack.push(element);
            return Err(e);
        }
    };
    let index = match index_bigint.to_i64() {
        Some(v) => v,
        None => {
            interp.stack.push(index_val);
            interp.stack.push(element);
            return Err(AjisaiError::from("Index is too large"));
        }
    };

    match interp.operation_target {
        OperationTarget::StackTop => {
            let vector_val = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(index_val.clone());
                interp.stack.push(element.clone());
                AjisaiError::StackUnderflow
            })?;

            let element_to_insert = unwrap_single_element(element.clone());

            match vector_val.val_type {
                ValueType::Vector(mut v) => {
                    let len = v.len() as i64;
                    let insert_index = if index < 0 {
                        // 負数インデックス: -1は末尾、-2は末尾の1つ前
                        // これは他の位置指定操作（GET/REPLACE/REMOVE）と一貫
                        (len + index).max(0) as usize
                    } else {
                        // 正数インデックス: lengthまで許容（末尾への追加を可能にする）
                        (index as usize).min(v.len())
                    };

                    if let ValueType::Vector(elems) = element_to_insert.val_type {
                        v.splice(insert_index..insert_index, elems);
                    } else {
                        v.insert(insert_index, element_to_insert);
                    }
                    interp.stack.push(Value { val_type: ValueType::Vector(v) });
                    Ok(())
                },
                _ => {
                    interp.stack.push(vector_val);
                    interp.stack.push(index_val);
                    interp.stack.push(element);
                    Err(AjisaiError::type_error("vector", "other type"))
                }
            }
        }
        OperationTarget::Stack => {
            // REPLACEと同様に単一要素ベクタをアンラップする
            let element_to_insert = unwrap_single_element(element);

            let len = interp.stack.len() as i64;
            let insert_index = if index < 0 {
                // 負数インデックス: -1は末尾、-2は末尾の1つ前
                // これは他の位置指定操作（GET/REPLACE/REMOVE）と一貫
                (len + index).max(0) as usize
            } else {
                // 正数インデックス: lengthまで許容（末尾への追加を可能にする）
                (index as usize).min(interp.stack.len())
            };
            interp.stack.insert(insert_index, element_to_insert);
            Ok(())
        }
    }
}

/// REPLACE - 指定位置の要素を置き換える
///
/// 【責務】
/// - スタックトップのベクタまたはスタック全体の指定位置の要素を置き換え
/// - 負数インデックスをサポート
/// - 単一要素ベクタは自動的にアンラップして置換
///
/// 【使用法】
/// - StackTopモード: `[a b c] [1] [X] REPLACE` → `[a X c]`
/// - Stackモード: `a b c [1] X .. REPLACE` → `a X c`
///
/// 【引数スタック】
/// - new_element: 新しい要素
/// - [index]: 置換位置（単一要素ベクタの整数）
/// - (StackTopモード) target: 対象ベクタ
///
/// 【戻り値スタック】
/// - 置換後のベクタまたはスタック
///
/// 【エラー】
/// - インデックスが範囲外の場合
/// - 対象がベクタでない場合（StackTopモード）
pub fn op_replace(interp: &mut Interpreter) -> Result<()> {
    let new_element = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index_val = interp.stack.pop().ok_or_else(|| {
        interp.stack.push(new_element.clone());
        AjisaiError::StackUnderflow
    })?;
    let index_bigint = match get_bigint_from_value(&index_val) {
        Ok(v) => v,
        Err(e) => {
            interp.stack.push(index_val);
            interp.stack.push(new_element);
            return Err(e);
        }
    };
    let index = match index_bigint.to_i64() {
        Some(v) => v,
        None => {
            interp.stack.push(index_val);
            interp.stack.push(new_element);
            return Err(AjisaiError::from("Index too large"));
        }
    };

    match interp.operation_target {
        OperationTarget::StackTop => {
            let vector_val = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(index_val.clone());
                interp.stack.push(new_element.clone());
                AjisaiError::StackUnderflow
            })?;

            let replace_element = unwrap_single_element(new_element.clone());

            match vector_val.val_type {
                ValueType::Vector(mut v) => {
                    let len = v.len();
                    let actual_index = match normalize_index(index, len) {
                        Some(idx) => idx,
                        None => {
                            interp.stack.push(Value { val_type: ValueType::Vector(v) });
                            interp.stack.push(index_val);
                            interp.stack.push(new_element);
                            return Err(AjisaiError::IndexOutOfBounds { index, length: len });
                        }
                    };

                    v[actual_index] = replace_element;
                    interp.stack.push(Value { val_type: ValueType::Vector(v) });
                    Ok(())
                },
                _ => {
                    interp.stack.push(vector_val);
                    interp.stack.push(index_val);
                    interp.stack.push(new_element);
                    Err(AjisaiError::type_error("vector", "other type"))
                }
            }
        }
        OperationTarget::Stack => {
            let len = interp.stack.len();
            let actual_index = match normalize_index(index, len) {
                Some(idx) => idx,
                None => {
                    interp.stack.push(index_val);
                    interp.stack.push(new_element);
                    return Err(AjisaiError::IndexOutOfBounds { index, length: len });
                }
            };

            let replace_element = unwrap_single_element(new_element);
            interp.stack[actual_index] = replace_element;
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
    let index_bigint = match get_bigint_from_value(&index_val) {
        Ok(v) => v,
        Err(e) => {
            interp.stack.push(index_val);
            return Err(e);
        }
    };
    let index = match index_bigint.to_i64() {
        Some(v) => v,
        None => {
            interp.stack.push(index_val);
            return Err(AjisaiError::from("Index too large"));
        }
    };

    match interp.operation_target {
        OperationTarget::StackTop => {
            let vector_val = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(index_val.clone());
                AjisaiError::StackUnderflow
            })?;
            match vector_val.val_type {
                ValueType::Vector(mut v) => {
                    let len = v.len();
                    let actual_index = match normalize_index(index, len) {
                        Some(idx) => idx,
                        None => {
                            interp.stack.push(Value { val_type: ValueType::Vector(v) });
                            interp.stack.push(index_val);
                            return Err(AjisaiError::IndexOutOfBounds { index, length: len });
                        }
                    };

                    v.remove(actual_index);
                    interp.stack.push(Value { val_type: ValueType::Vector(v) });
                    Ok(())
                },
                _ => {
                    interp.stack.push(vector_val);
                    interp.stack.push(index_val);
                    Err(AjisaiError::type_error("vector", "other type"))
                }
            }
        }
        OperationTarget::Stack => {
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
    let len = match interp.operation_target {
        OperationTarget::StackTop => {
            let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            match &target_val.val_type {
                ValueType::Vector(v) => {
                    let len = v.len();
                    interp.stack.push(target_val);
                    len
                }
                _ => {
                    interp.stack.push(target_val);
                    return Err(AjisaiError::type_error("vector", "other type"));
                }
            }
        }
        OperationTarget::Stack => interp.stack.len(),
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
    let count_bigint = match get_bigint_from_value(&count_val) {
        Ok(v) => v,
        Err(e) => {
            interp.stack.push(count_val);
            return Err(e);
        }
    };
    let count = match count_bigint.to_i64() {
        Some(v) => v,
        None => {
            interp.stack.push(count_val);
            return Err(AjisaiError::from("Count is too large"));
        }
    };

    match interp.operation_target {
        OperationTarget::StackTop => {
            let vector_val = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(count_val.clone());
                AjisaiError::StackUnderflow
            })?;
            match vector_val.val_type {
                ValueType::Vector(v) => {
                    let len = v.len();
                    let result = if count < 0 {
                        let abs_count = (-count) as usize;
                        if abs_count > len {
                            interp.stack.push(Value { val_type: ValueType::Vector(v) });
                            interp.stack.push(count_val);
                            return Err(AjisaiError::from("Take count exceeds vector length"));
                        }
                        v[len - abs_count..].to_vec()
                    } else {
                        let take_count = count as usize;
                        if take_count > len {
                            interp.stack.push(Value { val_type: ValueType::Vector(v) });
                            interp.stack.push(count_val);
                            return Err(AjisaiError::from("Take count exceeds vector length"));
                        }
                        v[..take_count].to_vec()
                    };

                    interp.stack.push(Value::from_vector(result));
                    Ok(())
                },
                _ => {
                    interp.stack.push(vector_val);
                    interp.stack.push(count_val);
                    Err(AjisaiError::type_error("vector", "other type"))
                }
            }
        }
        OperationTarget::Stack => {
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
/// - 複数のサイズ指定を受け取り、ベクタまたはスタックを分割
/// - サイズの合計が全体より小さい場合、残りは最後の要素に含まれる
///
/// 【使用法】
/// - StackTopモード: `[a b c d e] [2] [2] SPLIT` → `[a b] [c d] [e]`
/// - Stackモード: `a b c d e [2] [1] .. SPLIT` → `[a b] [c] [d e]`
///
/// 【引数スタック】
/// - [size_n] ... [size_1]: 分割サイズ（複数指定可能）
/// - (StackTopモード) target: 対象ベクタ
///
/// 【戻り値スタック】
/// - 分割された複数のベクタ
///
/// 【エラー】
/// - サイズ指定がない場合
/// - サイズの合計が長さを超える場合
/// - 対象がベクタでない場合（StackTopモード）
pub fn op_split(interp: &mut Interpreter) -> Result<()> {
    let mut sizes_values = VecDeque::new();
    while let Some(top) = interp.stack.last() {
        if get_bigint_from_value(top).is_ok() {
            sizes_values.push_front(interp.stack.pop().unwrap());
        } else {
            break;
        }
    }

    if sizes_values.is_empty() {
        return Err(AjisaiError::from("SPLIT requires at least one size"));
    }

    let sizes: Vec<usize> = match sizes_values.iter()
        .map(|v| get_bigint_from_value(v).and_then(|bi| {
            bi.to_usize().ok_or_else(|| AjisaiError::from("Split size is too large"))
        }))
        .collect::<Result<Vec<_>>>() {
        Ok(v) => v,
        Err(e) => {
            // Restore all size values to the stack
            for val in sizes_values {
                interp.stack.push(val);
            }
            return Err(e);
        }
    };

    match interp.operation_target {
        OperationTarget::StackTop => {
            let vector_val = interp.stack.pop().ok_or_else(|| {
                // Restore all size values to the stack
                for val in &sizes_values {
                    interp.stack.push(val.clone());
                }
                AjisaiError::from("SPLIT requires a vector to split")
            })?;
            match vector_val.val_type {
                ValueType::Vector(v) => {
                    let total_size: usize = sizes.iter().sum();
                    if total_size > v.len() {
                        interp.stack.push(Value { val_type: ValueType::Vector(v) });
                        for val in &sizes_values {
                            interp.stack.push(val.clone());
                        }
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
                },
                _ => {
                    interp.stack.push(vector_val);
                    for val in &sizes_values {
                        interp.stack.push(val.clone());
                    }
                    Err(AjisaiError::type_error("vector", "other type"))
                }
            }
        }
        OperationTarget::Stack => {
            let total_size: usize = sizes.iter().sum();
            if total_size > interp.stack.len() {
                // Restore all size values to the stack
                for val in sizes_values {
                    interp.stack.push(val);
                }
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
    match interp.operation_target {
        OperationTarget::StackTop => {
            // スタックトップからcountを取得（オプション、デフォルトは2）
            let count_i64 = if let Some(top) = interp.stack.last() {
                if let Ok(count_bigint) = get_bigint_from_value(top) {
                    if let Some(c) = count_bigint.to_i64() {
                        interp.stack.pop();
                        c
                    } else {
                        return Err(AjisaiError::from("Count is too large"));
                    }
                } else {
                    // countが指定されていない場合、デフォルトは2
                    2
                }
            } else {
                return Err(AjisaiError::StackUnderflow);
            };

            let abs_count = count_i64.unsigned_abs() as usize;
            let is_reversed = count_i64 < 0;

            if interp.stack.len() < abs_count {
                return Err(AjisaiError::StackUnderflow);
            }

            let mut vecs_to_concat: Vec<Value> = interp.stack.split_off(interp.stack.len() - abs_count);

            if is_reversed {
                vecs_to_concat.reverse();
            }

            let mut result_vec = Vec::new();

            for val in vecs_to_concat {
                if let ValueType::Vector(v) = val.val_type {
                    result_vec.extend(v);
                } else {
                    result_vec.push(val);
                }
            }

            interp.stack.push(Value { val_type: ValueType::Vector(result_vec) });
            Ok(())
        }
        OperationTarget::Stack => {
            // Stackモード: スタックトップがcountかチェック、なければスタック全体
            let count_i64 = if let Some(top) = interp.stack.last() {
                if let Ok(count_bigint) = get_bigint_from_value(top) {
                    if let Some(c) = count_bigint.to_i64() {
                        interp.stack.pop();
                        c
                    } else {
                        return Err(AjisaiError::from("Count is too large"));
                    }
                } else {
                    // countが指定されていない場合、スタック全体を使用
                    interp.stack.len() as i64
                }
            } else {
                return Err(AjisaiError::StackUnderflow);
            };

            let abs_count = count_i64.unsigned_abs() as usize;
            let is_reversed = count_i64 < 0;

            if interp.stack.len() < abs_count {
                return Err(AjisaiError::StackUnderflow);
            }

            let mut vecs_to_concat: Vec<Value> = interp.stack.split_off(interp.stack.len() - abs_count);

            if is_reversed {
                vecs_to_concat.reverse();
            }

            let mut result_vec = Vec::new();

            for val in vecs_to_concat {
                if let ValueType::Vector(v) = val.val_type {
                    result_vec.extend(v);
                } else {
                    result_vec.push(val);
                }
            }

            interp.stack.push(Value { val_type: ValueType::Vector(result_vec) });
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
    match interp.operation_target {
        OperationTarget::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            match val.val_type {
                ValueType::Vector(mut v) => {
                    if v.len() < 2 {
                        interp.stack.push(Value { val_type: ValueType::Vector(v) });
                        return Err(AjisaiError::from("REVERSE resulted in no change on a vector with less than 2 elements"));
                    }
                    let original_v = v.clone();
                    v.reverse();
                    if v == original_v {
                        interp.stack.push(Value { val_type: ValueType::Vector(original_v) });
                        return Err(AjisaiError::from("REVERSE resulted in no change (vector is a palindrome)"));
                    }
                    interp.stack.push(Value { val_type: ValueType::Vector(v) });
                    Ok(())
                },
                _ => {
                    interp.stack.push(val);
                    Err(AjisaiError::type_error("vector", "other type"))
                }
            }
        }
        OperationTarget::Stack => {
            if interp.stack.len() < 2 {
                return Err(AjisaiError::from("REVERSE resulted in no change on a stack with less than 2 elements"));
            }
            let original_stack = interp.stack.clone();
            interp.stack.reverse();
            if interp.stack == original_stack {
                interp.stack = original_stack;
                return Err(AjisaiError::from("REVERSE resulted in no change (stack is a palindrome)"));
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
/// - StackTopモード（2引数）: `[0] [5] RANGE` → `[0] [5] [0 1 2 3 4 5]`
/// - StackTopモード（3引数）: `[0] [10] [2] RANGE` → `[0] [10] [2] [0 2 4 6 8 10]`
/// - Stackモード（2引数）: `0 5 .. RANGE` → `[0 1 2 3 4 5]`
/// - Stackモード（3引数）: `0 10 2 .. RANGE` → `[0 2 4 6 8 10]`
///
/// 【引数スタック】
/// - [start]: 開始値（整数）
/// - [end]: 終了値（整数、この値を含む）
/// - (オプション) [step]: 増分（整数、デフォルトは自動判定: start <= end なら 1、そうでなければ -1）
///
/// 【戻り値スタック】
/// - (StackTopモード) 元の引数 + 生成されたベクタ
/// - (Stackモード) 生成されたベクタ
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
    match interp.operation_target {
        OperationTarget::StackTop => {
            // スタックから引数を取得（2個または3個）
            if interp.stack.len() < 2 {
                return Err(AjisaiError::from("RANGE requires at least 2 arguments: [start] [end] or [start] [end] [step]"));
            }

            // 最後の引数を確認（endまたはstep）
            let last_val = interp.stack.pop().unwrap();
            let last_bigint = get_bigint_from_value(&last_val)?;
            let last_i64 = last_bigint.to_i64()
                .ok_or_else(|| AjisaiError::from("RANGE argument is too large"))?;

            // 2番目の引数を確認（startまたはend）
            let second_val = interp.stack.pop().unwrap();
            let second_bigint = get_bigint_from_value(&second_val)?;
            let second_i64 = second_bigint.to_i64()
                .ok_or_else(|| AjisaiError::from("RANGE argument is too large"))?;

            let (start, end, step, start_val, end_val, step_val) = if interp.stack.is_empty() {
                // 2引数モード: start, end
                let step = if second_i64 <= last_i64 { 1 } else { -1 };
                (second_i64, last_i64, step, second_val, last_val, None)
            } else {
                // 3引数モード: start, end, step
                let first_val = interp.stack.pop().unwrap();
                let first_bigint = get_bigint_from_value(&first_val)?;
                let first_i64 = first_bigint.to_i64()
                    .ok_or_else(|| AjisaiError::from("RANGE argument is too large"))?;
                (first_i64, second_i64, last_i64, first_val, second_val, Some(last_val))
            };

            // stepが0の場合はエラー
            if step == 0 {
                return Err(AjisaiError::from("RANGE step cannot be 0"));
            }

            // 無限範囲チェック
            if (start < end && step < 0) || (start > end && step > 0) {
                return Err(AjisaiError::from("RANGE would create an infinite sequence (check start, end, and step values)"));
            }

            // 範囲を生成
            let mut range_vec = Vec::new();
            let mut current = start;

            if step > 0 {
                while current <= end {
                    range_vec.push(Value {
                        val_type: ValueType::Number(Fraction::new(BigInt::from(current), BigInt::one())),
                    });
                    current += step;
                }
            } else {
                while current >= end {
                    range_vec.push(Value {
                        val_type: ValueType::Number(Fraction::new(BigInt::from(current), BigInt::one())),
                    });
                    current += step;
                }
            }

            // 元の引数をスタックに戻す
            interp.stack.push(start_val);
            interp.stack.push(end_val);
            if let Some(sv) = step_val {
                interp.stack.push(sv);
            }

            // 結果をプッシュ
            interp.stack.push(Value {
                val_type: ValueType::Vector(range_vec),
            });

            Ok(())
        }
        OperationTarget::Stack => {
            // スタックから引数を取得（2個または3個）
            if interp.stack.len() < 2 {
                return Err(AjisaiError::from("RANGE requires at least 2 arguments"));
            }

            // 最後の引数を確認
            let last_val = interp.stack.pop().unwrap();
            let last_bigint = get_bigint_from_value(&last_val)?;
            let last_i64 = last_bigint.to_i64()
                .ok_or_else(|| AjisaiError::from("RANGE argument is too large"))?;

            // 2番目の引数を確認
            let second_val = interp.stack.pop().unwrap();
            let second_bigint = get_bigint_from_value(&second_val)?;
            let second_i64 = second_bigint.to_i64()
                .ok_or_else(|| AjisaiError::from("RANGE argument is too large"))?;

            let (start, end, step) = if let Some(top) = interp.stack.last() {
                // 3番目の引数があるかチェック
                if let Ok(third_bigint) = get_bigint_from_value(top) {
                    if let Some(third_i64) = third_bigint.to_i64() {
                        // 3引数モード
                        interp.stack.pop();
                        (third_i64, second_i64, last_i64)
                    } else {
                        // 2引数モード（3番目が整数でない）
                        let step = if second_i64 <= last_i64 { 1 } else { -1 };
                        (second_i64, last_i64, step)
                    }
                } else {
                    // 2引数モード
                    let step = if second_i64 <= last_i64 { 1 } else { -1 };
                    (second_i64, last_i64, step)
                }
            } else {
                // スタックが空なので2引数モード
                let step = if second_i64 <= last_i64 { 1 } else { -1 };
                (second_i64, last_i64, step)
            };

            // stepが0の場合はエラー
            if step == 0 {
                return Err(AjisaiError::from("RANGE step cannot be 0"));
            }

            // 無限範囲チェック
            if (start < end && step < 0) || (start > end && step > 0) {
                return Err(AjisaiError::from("RANGE would create an infinite sequence (check start, end, and step values)"));
            }

            // 範囲を生成
            let mut range_vec = Vec::new();
            let mut current = start;

            if step > 0 {
                while current <= end {
                    range_vec.push(Value {
                        val_type: ValueType::Number(Fraction::new(BigInt::from(current), BigInt::one())),
                    });
                    current += step;
                }
            } else {
                while current >= end {
                    range_vec.push(Value {
                        val_type: ValueType::Number(Fraction::new(BigInt::from(current), BigInt::one())),
                    });
                    current += step;
                }
            }

            // 結果をプッシュ
            interp.stack.push(Value {
                val_type: ValueType::Vector(range_vec),
            });

            Ok(())
        }
    }
}
