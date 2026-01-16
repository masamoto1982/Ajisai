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
/// Kleene三値論理:
/// - ゼロなら 1、非ゼロなら 0
/// - NIL（空Vector）なら NIL を返す（不明の否定は不明）
pub fn op_not(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let val = interp.stack_pop().ok_or(AjisaiError::StackUnderflow)?;

            // NIL（空）の場合はNILを返す（Kleene三値論理: NOT NIL = NIL）
            if val.data.is_empty() {
                interp.stack_push(Value::nil());
                return Ok(());
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

            let len = result_data.len();
            interp.stack_push(Value {
                data: result_data,
                display_hint: DisplayHint::Boolean,
                shape: vec![len],
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
/// Kleene三値論理:
/// - 両方が非ゼロなら 1、それ以外は 0
/// - FALSE AND NIL = FALSE（FALSEが確定的に偽）
/// - TRUE AND NIL = NIL（不明が伝播）
/// - NIL AND NIL = NIL（不明が伝播）
pub fn op_and(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let b_val = interp.stack_pop().ok_or(AjisaiError::StackUnderflow)?;
            let a_val = interp.stack_pop().ok_or(AjisaiError::StackUnderflow)?;

            let a_is_nil = a_val.data.is_empty();
            let b_is_nil = b_val.data.is_empty();

            // Kleene三値論理でのAND処理
            if a_is_nil && b_is_nil {
                // NIL AND NIL = NIL
                interp.stack_push(Value::nil());
                return Ok(());
            } else if a_is_nil {
                // NIL AND b: bがfalsy（全て0）ならFALSE、それ以外はNIL
                let b_has_any_truthy = b_val.data.iter().any(|f| !f.is_zero());
                if b_has_any_truthy {
                    interp.stack_push(Value::nil());
                } else {
                    interp.stack_push(Value::from_bool(false));
                }
                return Ok(());
            } else if b_is_nil {
                // a AND NIL: aがfalsy（全て0）ならFALSE、それ以外はNIL
                let a_has_any_truthy = a_val.data.iter().any(|f| !f.is_zero());
                if a_has_any_truthy {
                    interp.stack_push(Value::nil());
                } else {
                    interp.stack_push(Value::from_bool(false));
                }
                return Ok(());
            }

            let a_len = a_val.data.len();
            let b_len = b_val.data.len();

            let result_data: Vec<Fraction> = if a_len > 1 && b_len == 1 {
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
                interp.stack_push(a_val);
                interp.stack_push(b_val);
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

            let len = result_data.len();
            interp.stack_push(Value {
                data: result_data,
                display_hint: DisplayHint::Boolean,
                shape: vec![len],
            });
            Ok(())
        },

        OperationTarget::Stack => {
            let count_val = interp.stack_pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            if count == 0 || count == 1 {
                interp.stack_push(count_val);
                return Err(AjisaiError::from("STACK operation with count 0 or 1 results in no change"));
            }

            if interp.stack_len() < count {
                interp.stack_push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let elements = interp.stack_elements();
            let items: Vec<Value> = elements[elements.len() - count..].to_vec();
            let remaining: Vec<Value> = elements[..elements.len() - count].to_vec();
            interp.stack_set(remaining);

            // Kleene三値論理でのSTACKモードAND:
            // - どれかがNILでどれかがtruthyなら NIL
            // - どれかがfalsy（非NILで全て0）なら FALSE
            // - 全てがtruthyなら TRUE
            let has_nil = items.iter().any(|v| v.data.is_empty());
            let has_falsy_non_nil = items.iter().any(|v| !v.data.is_empty() && !v.is_truthy());
            let all_truthy = items.iter().all(|v| v.is_truthy());

            if has_falsy_non_nil {
                interp.stack_push(Value::from_bool(false));
            } else if has_nil {
                interp.stack_push(Value::nil());
            } else if all_truthy {
                interp.stack_push(Value::from_bool(true));
            } else {
                interp.stack_push(Value::from_bool(false));
            }
            Ok(())
        }
    }
}

/// OR 演算子 - 論理和
///
/// Kleene三値論理:
/// - どちらかが非ゼロなら 1
/// - TRUE OR NIL = TRUE（TRUEが確定的に真）
/// - FALSE OR NIL = NIL（不明が伝播）
/// - NIL OR NIL = NIL（不明が伝播）
pub fn op_or(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let b_val = interp.stack_pop().ok_or(AjisaiError::StackUnderflow)?;
            let a_val = interp.stack_pop().ok_or(AjisaiError::StackUnderflow)?;

            let a_is_nil = a_val.data.is_empty();
            let b_is_nil = b_val.data.is_empty();

            // Kleene三値論理でのOR処理
            if a_is_nil && b_is_nil {
                // NIL OR NIL = NIL
                interp.stack_push(Value::nil());
                return Ok(());
            } else if a_is_nil {
                // NIL OR b: bがtruthy（どれか非0）ならTRUE、それ以外はNIL
                let b_has_any_truthy = b_val.data.iter().any(|f| !f.is_zero());
                if b_has_any_truthy {
                    interp.stack_push(Value::from_bool(true));
                } else {
                    interp.stack_push(Value::nil());
                }
                return Ok(());
            } else if b_is_nil {
                // a OR NIL: aがtruthy（どれか非0）ならTRUE、それ以外はNIL
                let a_has_any_truthy = a_val.data.iter().any(|f| !f.is_zero());
                if a_has_any_truthy {
                    interp.stack_push(Value::from_bool(true));
                } else {
                    interp.stack_push(Value::nil());
                }
                return Ok(());
            }

            let a_len = a_val.data.len();
            let b_len = b_val.data.len();

            let result_data: Vec<Fraction> = if a_len > 1 && b_len == 1 {
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
                interp.stack_push(a_val);
                interp.stack_push(b_val);
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

            let len = result_data.len();
            interp.stack_push(Value {
                data: result_data,
                display_hint: DisplayHint::Boolean,
                shape: vec![len],
            });
            Ok(())
        },

        OperationTarget::Stack => {
            let count_val = interp.stack_pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            if count == 0 || count == 1 {
                interp.stack_push(count_val);
                return Err(AjisaiError::from("STACK operation with count 0 or 1 results in no change"));
            }

            if interp.stack_len() < count {
                interp.stack_push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let elements = interp.stack_elements();
            let items: Vec<Value> = elements[elements.len() - count..].to_vec();
            let remaining: Vec<Value> = elements[..elements.len() - count].to_vec();
            interp.stack_set(remaining);

            // Kleene三値論理でのSTACKモードOR:
            // - どれかがtruthy（非NILで非0要素あり）なら TRUE
            // - どれかがNILで残りがfalsy（全て0）なら NIL
            // - 全てがfalsy（非NILで全て0）なら FALSE
            let has_nil = items.iter().any(|v| v.data.is_empty());
            let has_truthy = items.iter().any(|v| v.is_truthy());

            if has_truthy {
                interp.stack_push(Value::from_bool(true));
            } else if has_nil {
                interp.stack_push(Value::nil());
            } else {
                interp.stack_push(Value::from_bool(false));
            }
            Ok(())
        }
    }
}
