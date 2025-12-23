// rust/src/interpreter/logic.rs
//
// 【責務】
// 論理演算子（AND、OR、NOT）を実装する。
// ベクタ間の要素ごと論理演算とブロードキャスト機能を提供する。

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::get_integer_from_value;
use crate::types::{Value, ValueType};

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
/// - `[TRUE] NOT` → `[FALSE]`
/// - `[FALSE] NOT` → `[TRUE]`
/// - `[TRUE FALSE TRUE] NOT` → `[FALSE TRUE FALSE]`
/// - `[NIL] NOT` → `[NIL]` (Kleene論理: NOT unknown = unknown)
///
/// 【引数スタック】
/// - [value]: Boolean値またはNilのベクタ
///
/// 【戻り値スタック】
/// - [result]: 反転後の論理値のベクタ
///
/// 【エラー】
/// - Boolean/Nil以外の型の場合
pub fn op_not(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            let vec = match val.val_type {
                ValueType::Vector(v) => v,
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
                    ValueType::Nil => {
                        // NOT nil = nil (Kleene論理: NOT unknown = unknown)
                        result_vec.push(Value { val_type: ValueType::Nil });
                    },
                    _ => {
                        interp.stack.push(Value { val_type: ValueType::Vector(vec) });
                        return Err(AjisaiError::type_error("boolean or nil", "other type"));
                    }
                }
            }

            let result = Value { val_type: ValueType::Vector(result_vec) };
            interp.stack.push(result);
            Ok(())
        },
        OperationTarget::Stack => {
            // Stackモードは単項演算子では意味が不明確なため未対応
            Err(AjisaiError::from("NOT does not support Stack (..) mode"))
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
/// | TRUE  | TRUE  | TRUE   |
/// | TRUE  | FALSE | FALSE  |
/// | FALSE | TRUE  | FALSE  |
/// | FALSE | FALSE | FALSE  |
/// | TRUE  | NIL   | NIL    |
/// | FALSE | NIL   | FALSE  |
/// | NIL   | TRUE  | NIL    |
/// | NIL   | FALSE | FALSE  |
/// | NIL   | NIL   | NIL    |
///
/// 【StackTopモードの使用法】
/// - `[TRUE] [TRUE] AND` → `[TRUE]`
/// - `[TRUE FALSE] [FALSE TRUE] AND` → `[FALSE FALSE]`
/// - `[TRUE FALSE TRUE] [TRUE] AND` → `[TRUE FALSE TRUE]` (ブロードキャスト)
///
/// 【Stackモードの使用法】
/// - `[TRUE] [TRUE] [FALSE] [3] STACK AND` → `[FALSE]` (TRUE AND TRUE AND FALSE)
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

            let a_vec = match a_val.val_type {
                ValueType::Vector(v) => v,
                _ => {
                    interp.stack.push(a_val);
                    interp.stack.push(b_val);
                    return Err(AjisaiError::type_error("vector", "other type"));
                }
            };
            let b_vec = match b_val.val_type {
                ValueType::Vector(v) => v,
                _ => {
                    interp.stack.push(Value { val_type: ValueType::Vector(a_vec) });
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
                    interp.stack.push(Value { val_type: ValueType::Vector(a_vec) });
                    interp.stack.push(Value { val_type: ValueType::Vector(b_vec) });
                    return Err(AjisaiError::VectorLengthMismatch{ len1: a_len, len2: b_len });
                }
                for (a, b) in a_vec.iter().zip(b_vec.iter()) {
                    let res_type = and_logic(&a.val_type, &b.val_type)?;
                    result_vec.push(Value { val_type: res_type });
                }
            }

            let result = Value { val_type: ValueType::Vector(result_vec) };
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
            use crate::interpreter::helpers::{extract_single_element, wrap_result_value};
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
/// | TRUE  | TRUE  | TRUE   |
/// | TRUE  | FALSE | TRUE   |
/// | FALSE | TRUE  | TRUE   |
/// | FALSE | FALSE | FALSE  |
/// | TRUE  | NIL   | TRUE   |
/// | FALSE | NIL   | NIL    |
/// | NIL   | TRUE  | TRUE   |
/// | NIL   | FALSE | NIL    |
/// | NIL   | NIL   | NIL    |
///
/// 【StackTopモードの使用法】
/// - `[TRUE] [FALSE] OR` → `[TRUE]`
/// - `[TRUE FALSE] [FALSE TRUE] OR` → `[TRUE TRUE]`
/// - `[TRUE FALSE TRUE] [FALSE] OR` → `[TRUE FALSE TRUE]` (ブロードキャスト)
///
/// 【Stackモードの使用法】
/// - `[FALSE] [FALSE] [TRUE] [3] STACK OR` → `[TRUE]` (FALSE OR FALSE OR TRUE)
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

            let a_vec = match a_val.val_type {
                ValueType::Vector(v) => v,
                _ => {
                    interp.stack.push(a_val);
                    interp.stack.push(b_val);
                    return Err(AjisaiError::type_error("vector", "other type"));
                }
            };
            let b_vec = match b_val.val_type {
                ValueType::Vector(v) => v,
                _ => {
                    interp.stack.push(Value { val_type: ValueType::Vector(a_vec) });
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
                    interp.stack.push(Value { val_type: ValueType::Vector(a_vec) });
                    interp.stack.push(Value { val_type: ValueType::Vector(b_vec) });
                    return Err(AjisaiError::VectorLengthMismatch{ len1: a_len, len2: b_len });
                }
                for (a, b) in a_vec.iter().zip(b_vec.iter()) {
                    let res_type = or_logic(&a.val_type, &b.val_type)?;
                    result_vec.push(Value { val_type: res_type });
                }
            }

            let result = Value { val_type: ValueType::Vector(result_vec) };
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
            use crate::interpreter::helpers::{extract_single_element, wrap_result_value};
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
