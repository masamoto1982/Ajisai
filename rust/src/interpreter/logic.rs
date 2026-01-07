// rust/src/interpreter/logic.rs
//
// 統一分数アーキテクチャ版の論理演算
//
// 論理演算（0 = false、非0 = true）

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::get_integer_from_value;
use crate::types::{Value, DisplayHint};
use crate::types::fraction::Fraction;

// ============================================================================
// 論理演算子
// ============================================================================

/// NOT 演算子 - 論理否定
///
/// ゼロなら 1、非ゼロなら 0
pub fn op_not(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            // NIL（空）の場合はエラー
            if val.data.is_empty() {
                interp.stack.push(val);
                return Err(AjisaiError::from("Cannot apply NOT to NIL"));
            }

            // 各要素を反転
            let result_data: Vec<Fraction> = val.data.iter()
                .map(|f| {
                    if f.is_zero() {
                        Fraction::from(1)
                    } else {
                        Fraction::from(0)
                    }
                })
                .collect();

            interp.stack.push(Value {
                data: result_data,
                display_hint: DisplayHint::Boolean,
            });
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
/// 両方が非ゼロなら 1、それ以外は 0
pub fn op_and(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            // NILチェック
            if a_val.data.is_empty() || b_val.data.is_empty() {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
                return Err(AjisaiError::from("Cannot apply AND to NIL"));
            }

            let a_len = a_val.data.len();
            let b_len = b_val.data.len();

            let result_data = if a_len > 1 && b_len == 1 {
                // aがベクタ、bがスカラー: bを各要素にブロードキャスト
                let b_truthy = !b_val.data[0].is_zero();
                a_val.data.iter()
                    .map(|a| {
                        let a_truthy = !a.is_zero();
                        Fraction::from(if a_truthy && b_truthy { 1 } else { 0 })
                    })
                    .collect()
            } else if a_len == 1 && b_len > 1 {
                // aがスカラー、bがベクタ: aを各要素にブロードキャスト
                let a_truthy = !a_val.data[0].is_zero();
                b_val.data.iter()
                    .map(|b| {
                        let b_truthy = !b.is_zero();
                        Fraction::from(if a_truthy && b_truthy { 1 } else { 0 })
                    })
                    .collect()
            } else if a_len != b_len {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
                return Err(AjisaiError::VectorLengthMismatch { len1: a_len, len2: b_len });
            } else {
                // 要素ごと演算
                a_val.data.iter().zip(b_val.data.iter())
                    .map(|(a, b)| {
                        let result = !a.is_zero() && !b.is_zero();
                        Fraction::from(if result { 1 } else { 0 })
                    })
                    .collect()
            };

            interp.stack.push(Value {
                data: result_data,
                display_hint: DisplayHint::Boolean,
            });
            Ok(())
        },

        OperationTarget::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            if count == 0 || count == 1 {
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK operation with count 0 or 1 results in no change"));
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let items: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();

            // 全ての要素が truthy かチェック
            let all_truthy = items.iter().all(|v| v.is_truthy());

            interp.stack.push(Value::from_bool(all_truthy));
            Ok(())
        }
    }
}

/// OR 演算子 - 論理和
///
/// どちらかが非ゼロなら 1
pub fn op_or(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            // NILチェック
            if a_val.data.is_empty() || b_val.data.is_empty() {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
                return Err(AjisaiError::from("Cannot apply OR to NIL"));
            }

            let a_len = a_val.data.len();
            let b_len = b_val.data.len();

            let result_data = if a_len > 1 && b_len == 1 {
                // aがベクタ、bがスカラー: bを各要素にブロードキャスト
                let b_truthy = !b_val.data[0].is_zero();
                a_val.data.iter()
                    .map(|a| {
                        let a_truthy = !a.is_zero();
                        Fraction::from(if a_truthy || b_truthy { 1 } else { 0 })
                    })
                    .collect()
            } else if a_len == 1 && b_len > 1 {
                // aがスカラー、bがベクタ: aを各要素にブロードキャスト
                let a_truthy = !a_val.data[0].is_zero();
                b_val.data.iter()
                    .map(|b| {
                        let b_truthy = !b.is_zero();
                        Fraction::from(if a_truthy || b_truthy { 1 } else { 0 })
                    })
                    .collect()
            } else if a_len != b_len {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
                return Err(AjisaiError::VectorLengthMismatch { len1: a_len, len2: b_len });
            } else {
                // 要素ごと演算
                a_val.data.iter().zip(b_val.data.iter())
                    .map(|(a, b)| {
                        let result = !a.is_zero() || !b.is_zero();
                        Fraction::from(if result { 1 } else { 0 })
                    })
                    .collect()
            };

            interp.stack.push(Value {
                data: result_data,
                display_hint: DisplayHint::Boolean,
            });
            Ok(())
        },

        OperationTarget::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            if count == 0 || count == 1 {
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK operation with count 0 or 1 results in no change"));
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let items: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();

            // どれかが truthy かチェック
            let any_truthy = items.iter().any(|v| v.is_truthy());

            interp.stack.push(Value::from_bool(any_truthy));
            Ok(())
        }
    }
}
