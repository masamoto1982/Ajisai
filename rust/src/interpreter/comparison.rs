// rust/src/interpreter/comparison.rs
//
// 【責務】
// 比較演算子（=、<、<=、>、>=）と論理演算子（AND、OR、NOT）を実装する。
// すべての演算は単一要素ベクタを想定し、結果を単一要素ベクタとして返す。

use crate::interpreter::{Interpreter, OperationTarget, error::{AjisaiError, Result}};
use crate::interpreter::helpers::{extract_single_element, get_integer_from_value, wrap_result_value};
use crate::types::{Value, ValueType, BracketType};
use crate::types::fraction::Fraction;

// ============================================================================
// 二項比較演算の汎用実装
// ============================================================================

/// 二項比較演算の汎用ハンドラ
///
/// 【責務】
/// - StackTopモード: 2つの単一要素ベクタから数値を取り出して比較
/// - Stackモード: N個の要素を順に比較し、全ての隣接ペアが条件を満たすかチェック
/// - 比較結果をBoolean値として返す
/// - すべての比較演算（<、<=、>、>=）で共通使用
///
/// 【StackTopモードの動作】
/// - スタックから2つのベクタをポップ
/// - 各ベクタから単一要素を抽出して比較
/// - 例: `[3] [5] <` → `[true]`
///
/// 【Stackモードの動作】
/// - スタックからカウント値をポップ
/// - 指定個数の要素を取得し、全ての隣接ペアが条件を満たすかチェック
/// - 例: `[1] [2] [3] [3] STACK <` → `(1<2) AND (2<3)` → `[false]`
///
/// 【引数】
/// - op: Fraction同士の比較関数
fn binary_comparison_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> bool,
{
    match interp.operation_target {
        // StackTopモード: 2つの単一要素ベクタを比較
        OperationTarget::StackTop => {
            if interp.stack.len() < 2 {
                return Err(AjisaiError::StackUnderflow);
            }

            let b_vec = interp.stack.pop().unwrap();
            let a_vec = interp.stack.pop().unwrap();

            let a_val = extract_single_element(&a_vec)?;
            let b_val = extract_single_element(&b_vec)?;

            let result = match (&a_val.val_type, &b_val.val_type) {
                (ValueType::Number(n1), ValueType::Number(n2)) => {
                    Value { val_type: ValueType::Boolean(op(n1, n2)) }
                },
                _ => {
                    interp.stack.push(a_vec);
                    interp.stack.push(b_vec);
                    return Err(AjisaiError::type_error("number", "other type"));
                }
            };

            interp.stack.push(wrap_result_value(result));
            Ok(())
        },

        // Stackモード: N個の要素を順に比較
        OperationTarget::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            // カウント0, 1はエラー（"No change is an error"原則）
            if count == 0 || count == 1 {
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK comparison with count 0 or 1 results in no change"));
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let items: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();

            // 全ての隣接ペアをチェック
            let mut all_true = true;
            for i in 0..items.len() - 1 {
                let a_val = extract_single_element(&items[i])?;
                let b_val = extract_single_element(&items[i + 1])?;

                match (&a_val.val_type, &b_val.val_type) {
                    (ValueType::Number(n1), ValueType::Number(n2)) => {
                        if !op(n1, n2) {
                            all_true = false;
                            break;
                        }
                    },
                    _ => {
                        interp.stack.extend(items);
                        interp.stack.push(count_val);
                        return Err(AjisaiError::type_error("number", "other type"));
                    }
                }
            }

            let result = Value { val_type: ValueType::Boolean(all_true) };
            interp.stack.push(wrap_result_value(result));
            Ok(())
        }
    }
}

// ============================================================================
// 比較演算子
// ============================================================================

/// < 演算子 - 小なり
///
/// 【責務】
/// - 2つの数値を比較し、左辺が右辺より小さいか判定
///
/// 【使用法】
/// - `[3] [5] <` → `[true]`
/// - `[5] [3] <` → `[false]`
/// - `[3] [3] <` → `[false]`
///
/// 【引数スタック】
/// - [b]: 右オペランド（単一要素ベクタの数値）
/// - [a]: 左オペランド（単一要素ベクタの数値）
///
/// 【戻り値スタック】
/// - [result]: 比較結果（Boolean）
///
/// 【エラー】
/// - オペランドが数値でない場合
/// - オペランドが単一要素ベクタでない場合
pub fn op_lt(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.lt(b))
}

/// <= 演算子 - 小なりイコール
///
/// 【責務】
/// - 2つの数値を比較し、左辺が右辺以下か判定
///
/// 【使用法】
/// - `[3] [5] <=` → `[true]`
/// - `[5] [3] <=` → `[false]`
/// - `[3] [3] <=` → `[true]`
///
/// 【引数スタック】
/// - [b]: 右オペランド（単一要素ベクタの数値）
/// - [a]: 左オペランド（単一要素ベクタの数値）
///
/// 【戻り値スタック】
/// - [result]: 比較結果（Boolean）
///
/// 【エラー】
/// - オペランドが数値でない場合
/// - オペランドが単一要素ベクタでない場合
pub fn op_le(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.le(b))
}

/// > 演算子 - 大なり
///
/// 【責務】
/// - 2つの数値を比較し、左辺が右辺より大きいか判定
///
/// 【使用法】
/// - `[5] [3] >` → `[true]`
/// - `[3] [5] >` → `[false]`
/// - `[3] [3] >` → `[false]`
///
/// 【引数スタック】
/// - [b]: 右オペランド（単一要素ベクタの数値）
/// - [a]: 左オペランド（単一要素ベクタの数値）
///
/// 【戻り値スタック】
/// - [result]: 比較結果（Boolean）
///
/// 【エラー】
/// - オペランドが数値でない場合
/// - オペランドが単一要素ベクタでない場合
pub fn op_gt(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.gt(b))
}

/// >= 演算子 - 大なりイコール
///
/// 【責務】
/// - 2つの数値を比較し、左辺が右辺以上か判定
///
/// 【使用法】
/// - `[5] [3] >=` → `[true]`
/// - `[3] [5] >=` → `[false]`
/// - `[3] [3] >=` → `[true]`
///
/// 【引数スタック】
/// - [b]: 右オペランド（単一要素ベクタの数値）
/// - [a]: 左オペランド（単一要素ベクタの数値）
///
/// 【戻り値スタック】
/// - [result]: 比較結果（Boolean）
///
/// 【エラー】
/// - オペランドが数値でない場合
/// - オペランドが単一要素ベクタでない場合
pub fn op_ge(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.ge(b))
}

/// = 演算子 - 等価比較
///
/// 【責務】
/// - StackTopモード: 2つの値を比較し、完全に等しいか判定
/// - Stackモード: N個の要素を順に比較し、全て等しいか判定
/// - あらゆる型の値を比較可能（Number、String、Boolean、Vector、Nil）
///
/// 【StackTopモードの使用法】
/// - `[3] [3] =` → `[true]`
/// - `[3] [5] =` → `[false]`
/// - `['hello'] ['hello'] =` → `[true]`
/// - `[a b] [a b] =` → `[true]`
///
/// 【Stackモードの使用法】
/// - `[3] [3] [3] [3] STACK =` → `[true]` (全て等しい)
/// - `[1] [2] [1] [3] STACK =` → `[false]` (1≠2)
///
/// 【引数スタック】
/// - StackTopモード: b, a (2つの値)
/// - Stackモード: count (要素数)
///
/// 【戻り値スタック】
/// - [result]: 比較結果（Boolean）
///
/// 【エラー】
/// - なし（すべての型で比較可能）
pub fn op_eq(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        // StackTopモード: 2つの値を比較
        OperationTarget::StackTop => {
            if interp.stack.len() < 2 {
                return Err(AjisaiError::StackUnderflow);
            }

            let b_vec = interp.stack.pop().unwrap();
            let a_vec = interp.stack.pop().unwrap();

            let result = Value { val_type: ValueType::Boolean(a_vec == b_vec) };
            interp.stack.push(wrap_result_value(result));
            Ok(())
        },

        // Stackモード: N個の要素を順に比較
        OperationTarget::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            // カウント0, 1はエラー（"No change is an error"原則）
            if count == 0 || count == 1 {
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK comparison with count 0 or 1 results in no change"));
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let items: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();

            // 全ての隣接ペアをチェック
            let mut all_equal = true;
            for i in 0..items.len() - 1 {
                if items[i] != items[i + 1] {
                    all_equal = false;
                    break;
                }
            }

            let result = Value { val_type: ValueType::Boolean(all_equal) };
            interp.stack.push(wrap_result_value(result));
            Ok(())
        }
    }
}

// ============================================================================
// 論理演算子
// ============================================================================

/// NOT 演算子 - 論理否定
///
/// 【責務】
/// - StackTopモード: ベクタの各要素のBoolean値を反転
/// - Stackモード: 現在未対応（StackTopモードのみ）
///
/// 【使用法】
/// - `[true] NOT` → `[false]`
/// - `[false] NOT` → `[true]`
/// - `[true false true] NOT` → `[false true false]`
///
/// 【引数スタック】
/// - [value]: Boolean値のベクタ
///
/// 【戻り値スタック】
/// - [result]: 反転後の論理値のベクタ
///
/// 【エラー】
/// - Boolean以外の型の場合
pub fn op_not(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            let (vec, bracket_type) = match val.val_type {
                ValueType::Vector(v, b) => (v, b),
                _ => {
                    interp.stack.push(val);
                    return Err(AjisaiError::type_error("vector", "other type"));
                }
            };

            let mut result_vec = Vec::new();
            for elem in &vec {
                match &elem.val_type {
                    ValueType::Boolean(b) => {
                        result_vec.push(Value { val_type: ValueType::Boolean(!b) });
                    },
                    _ => {
                        interp.stack.push(Value { val_type: ValueType::Vector(vec, bracket_type) });
                        return Err(AjisaiError::type_error("boolean", "other type"));
                    }
                }
            }

            let result = Value { val_type: ValueType::Vector(result_vec, bracket_type) };
            interp.stack.push(result);
            Ok(())
        },
        OperationTarget::Stack => {
            // Stackモードは単項演算子では意味が不明確なため未対応
            Err(AjisaiError::from("NOT does not support STACK mode"))
        }
    }
}

/// AND 演算子 - 論理積
///
/// 【責務】
/// - StackTopモード: ベクタ間の要素ごとAND演算、ブロードキャスト対応
/// - Stackモード: N個の要素を左から右へAND畳み込み
///
/// 【真理値表（Boolean同士）】
/// | A     | B     | Result |
/// |-------|-------|--------|
/// | true  | true  | true   |
/// | true  | false | false  |
/// | false | true  | false  |
/// | false | false | false  |
/// | true  | nil   | nil    |
/// | false | nil   | false  |
/// | nil   | true  | nil    |
/// | nil   | false | false  |
/// | nil   | nil   | nil    |
///
/// 【StackTopモードの使用法】
/// - `[true] [true] AND` → `[true]`
/// - `[true false] [false true] AND` → `[false false]`
/// - `[true false true] [true] AND` → `[true false true]` (ブロードキャスト)
///
/// 【Stackモードの使用法】
/// - `[true] [true] [false] [3] STACK AND` → `[false]` (true AND true AND false)
///
/// 【引数スタック】
/// - StackTopモード: b, a (2つのベクタ)
/// - Stackモード: count (要素数)
///
/// 【戻り値スタック】
/// - [result]: ANDの結果
///
/// 【エラー】
/// - オペランドがBoolean/Nilでない場合
/// - ベクタの長さが不一致（ブロードキャスト以外）
pub fn op_and(interp: &mut Interpreter) -> Result<()> {
    /// 2つの論理値のANDを計算（Nil対応）
    fn and_logic(a: &ValueType, b: &ValueType) -> Result<ValueType> {
        match (a, b) {
            (ValueType::Boolean(a), ValueType::Boolean(b)) => {
                Ok(ValueType::Boolean(*a && *b))
            },
            (ValueType::Boolean(false), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(false)) => {
                Ok(ValueType::Boolean(false))
            },
            (ValueType::Boolean(true), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(true)) | (ValueType::Nil, ValueType::Nil) => {
                Ok(ValueType::Nil)
            },
            _ => Err(AjisaiError::type_error("boolean or nil", "other types")),
        }
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            let (a_vec, a_bracket) = match a_val.val_type {
                ValueType::Vector(v, b) => (v, b),
                _ => {
                    interp.stack.push(a_val);
                    interp.stack.push(b_val);
                    return Err(AjisaiError::type_error("vector", "other type"));
                }
            };
            let (b_vec, _) = match b_val.val_type {
                ValueType::Vector(v, b) => (v, b),
                _ => {
                    interp.stack.push(Value { val_type: ValueType::Vector(a_vec, a_bracket) });
                    interp.stack.push(b_val);
                    return Err(AjisaiError::type_error("vector", "other type"));
                }
            };

            let a_len = a_vec.len();
            let b_len = b_vec.len();

            let mut result_vec = Vec::new();

            // ブロードキャスト判定と要素ごと演算
            if a_len > 1 && b_len == 1 {
                // aがベクタ、bがスカラー: bを各要素にブロードキャスト
                let scalar = &b_vec[0];
                for elem in &a_vec {
                    let res_type = and_logic(&elem.val_type, &scalar.val_type)?;
                    result_vec.push(Value { val_type: res_type });
                }
            } else if a_len == 1 && b_len > 1 {
                // aがスカラー、bがベクタ: aを各要素にブロードキャスト
                let scalar = &a_vec[0];
                for elem in &b_vec {
                    let res_type = and_logic(&scalar.val_type, &elem.val_type)?;
                    result_vec.push(Value { val_type: res_type });
                }
            } else {
                // 要素数が等しい、または両方とも単一要素
                if a_len != b_len {
                    interp.stack.push(Value { val_type: ValueType::Vector(a_vec, a_bracket) });
                    interp.stack.push(Value { val_type: ValueType::Vector(b_vec, BracketType::Square) });
                    return Err(AjisaiError::VectorLengthMismatch{ len1: a_len, len2: b_len });
                }
                for (a, b) in a_vec.iter().zip(b_vec.iter()) {
                    let res_type = and_logic(&a.val_type, &b.val_type)?;
                    result_vec.push(Value { val_type: res_type });
                }
            }

            let result = Value { val_type: ValueType::Vector(result_vec, a_bracket) };
            interp.stack.push(result);
            Ok(())
        },

        OperationTarget::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            if count == 0 {
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK operation with count 0 results in no change"));
            }

            if count == 1 {
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK operation with count 1 results in no change"));
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let items: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();

            // 最初の要素から開始
            let first = extract_single_element(&items[0])?;
            let mut acc_type = first.val_type.clone();

            // 残りの要素を順にAND
            for item in items.iter().skip(1) {
                let elem = extract_single_element(item)?;
                acc_type = and_logic(&acc_type, &elem.val_type)?;
            }

            interp.stack.push(wrap_result_value(Value { val_type: acc_type }));
            Ok(())
        }
    }
}

/// OR 演算子 - 論理和
///
/// 【責務】
/// - StackTopモード: ベクタ間の要素ごとOR演算、ブロードキャスト対応
/// - Stackモード: N個の要素を左から右へOR畳み込み
///
/// 【真理値表（Boolean同士）】
/// | A     | B     | Result |
/// |-------|-------|--------|
/// | true  | true  | true   |
/// | true  | false | true   |
/// | false | true  | true   |
/// | false | false | false  |
/// | true  | nil   | true   |
/// | false | nil   | nil    |
/// | nil   | true  | true   |
/// | nil   | false | nil    |
/// | nil   | nil   | nil    |
///
/// 【StackTopモードの使用法】
/// - `[true] [false] OR` → `[true]`
/// - `[true false] [false true] OR` → `[true true]`
/// - `[true false true] [false] OR` → `[true false true]` (ブロードキャスト)
///
/// 【Stackモードの使用法】
/// - `[false] [false] [true] [3] STACK OR` → `[true]` (false OR false OR true)
///
/// 【引数スタック】
/// - StackTopモード: b, a (2つのベクタ)
/// - Stackモード: count (要素数)
///
/// 【戻り値スタック】
/// - [result]: ORの結果
///
/// 【エラー】
/// - オペランドがBoolean/Nilでない場合
/// - ベクタの長さが不一致（ブロードキャスト以外）
pub fn op_or(interp: &mut Interpreter) -> Result<()> {
    /// 2つの論理値のORを計算（Nil対応）
    fn or_logic(a: &ValueType, b: &ValueType) -> Result<ValueType> {
        match (a, b) {
            (ValueType::Boolean(a), ValueType::Boolean(b)) => {
                Ok(ValueType::Boolean(*a || *b))
            },
            (ValueType::Boolean(true), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(true)) => {
                Ok(ValueType::Boolean(true))
            },
            (ValueType::Boolean(false), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(false)) | (ValueType::Nil, ValueType::Nil) => {
                Ok(ValueType::Nil)
            },
            _ => Err(AjisaiError::type_error("boolean or nil", "other types")),
        }
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            let (a_vec, a_bracket) = match a_val.val_type {
                ValueType::Vector(v, b) => (v, b),
                _ => {
                    interp.stack.push(a_val);
                    interp.stack.push(b_val);
                    return Err(AjisaiError::type_error("vector", "other type"));
                }
            };
            let (b_vec, _) = match b_val.val_type {
                ValueType::Vector(v, b) => (v, b),
                _ => {
                    interp.stack.push(Value { val_type: ValueType::Vector(a_vec, a_bracket) });
                    interp.stack.push(b_val);
                    return Err(AjisaiError::type_error("vector", "other type"));
                }
            };

            let a_len = a_vec.len();
            let b_len = b_vec.len();

            let mut result_vec = Vec::new();

            // ブロードキャスト判定と要素ごと演算
            if a_len > 1 && b_len == 1 {
                // aがベクタ、bがスカラー: bを各要素にブロードキャスト
                let scalar = &b_vec[0];
                for elem in &a_vec {
                    let res_type = or_logic(&elem.val_type, &scalar.val_type)?;
                    result_vec.push(Value { val_type: res_type });
                }
            } else if a_len == 1 && b_len > 1 {
                // aがスカラー、bがベクタ: aを各要素にブロードキャスト
                let scalar = &a_vec[0];
                for elem in &b_vec {
                    let res_type = or_logic(&scalar.val_type, &elem.val_type)?;
                    result_vec.push(Value { val_type: res_type });
                }
            } else {
                // 要素数が等しい、または両方とも単一要素
                if a_len != b_len {
                    interp.stack.push(Value { val_type: ValueType::Vector(a_vec, a_bracket) });
                    interp.stack.push(Value { val_type: ValueType::Vector(b_vec, BracketType::Square) });
                    return Err(AjisaiError::VectorLengthMismatch{ len1: a_len, len2: b_len });
                }
                for (a, b) in a_vec.iter().zip(b_vec.iter()) {
                    let res_type = or_logic(&a.val_type, &b.val_type)?;
                    result_vec.push(Value { val_type: res_type });
                }
            }

            let result = Value { val_type: ValueType::Vector(result_vec, a_bracket) };
            interp.stack.push(result);
            Ok(())
        },

        OperationTarget::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            if count == 0 {
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK operation with count 0 results in no change"));
            }

            if count == 1 {
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK operation with count 1 results in no change"));
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let items: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();

            // 最初の要素から開始
            let first = extract_single_element(&items[0])?;
            let mut acc_type = first.val_type.clone();

            // 残りの要素を順にOR
            for item in items.iter().skip(1) {
                let elem = extract_single_element(item)?;
                acc_type = or_logic(&acc_type, &elem.val_type)?;
            }

            interp.stack.push(wrap_result_value(Value { val_type: acc_type }));
            Ok(())
        }
    }
}
