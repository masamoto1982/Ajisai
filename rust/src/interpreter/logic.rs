// rust/src/interpreter/logic.rs
//
// 統一Value宇宙アーキテクチャ版の論理演算
//
// 論理演算（0 = false、非0 = true）

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::get_integer_from_value;
use crate::types::{Value, ValueData, DisplayHint};
use crate::types::fraction::Fraction;

// ============================================================================
// ヘルパー関数
// ============================================================================

/// Valueが「truthy」かどうかを判定（再帰的に任意の非ゼロ要素があるか）
fn value_has_any_truthy(val: &Value) -> bool {
    match &val.data {
        ValueData::Nil => false,
        ValueData::Scalar(f) => !f.is_zero(),
        ValueData::Vector(children) => children.iter().any(|c| value_has_any_truthy(c)),
    }
}

/// Valueの全要素にNOT演算を適用（再帰的）
fn apply_not_to_value(val: &Value) -> Value {
    match &val.data {
        ValueData::Nil => Value::nil(),
        ValueData::Scalar(f) => {
            let result = if f.is_zero() {
                Fraction::from(1)
            } else {
                Fraction::from(0)
            };
            Value {
                data: ValueData::Scalar(result),
                display_hint: DisplayHint::Boolean,
                audio_hint: None,
            }
        }
        ValueData::Vector(children) => {
            let new_children: Vec<Value> = children.iter()
                .map(|c| apply_not_to_value(c))
                .collect();
            Value {
                data: ValueData::Vector(new_children),
                display_hint: DisplayHint::Boolean,
                audio_hint: None,
            }
        }
    }
}

/// 二項論理演算をブロードキャスト付きで適用（再帰的）
fn apply_binary_logic<F>(a: &Value, b: &Value, op: F) -> Result<Value>
where
    F: Fn(bool, bool) -> bool + Copy,
{
    match (&a.data, &b.data) {
        // NIL処理は呼び出し元で処理済み
        (ValueData::Nil, _) | (_, ValueData::Nil) => {
            Err(AjisaiError::from("Cannot apply logic operation with NIL"))
        }

        // 両方スカラー
        (ValueData::Scalar(fa), ValueData::Scalar(fb)) => {
            let a_truthy = !fa.is_zero();
            let b_truthy = !fb.is_zero();
            let result = op(a_truthy, b_truthy);
            Ok(Value {
                data: ValueData::Scalar(Fraction::from(if result { 1 } else { 0 })),
                display_hint: DisplayHint::Boolean,
                audio_hint: None,
            })
        }

        // スカラー対ベクター（ブロードキャスト）
        (ValueData::Scalar(fa), ValueData::Vector(vb)) => {
            let a_truthy = !fa.is_zero();
            let new_children: Result<Vec<Value>> = vb.iter()
                .map(|bi| apply_binary_logic(&Value::from_bool(a_truthy), bi, op))
                .collect();
            Ok(Value {
                data: ValueData::Vector(new_children?),
                display_hint: DisplayHint::Boolean,
                audio_hint: None,
            })
        }

        // ベクター対スカラー（ブロードキャスト）
        (ValueData::Vector(va), ValueData::Scalar(fb)) => {
            let b_truthy = !fb.is_zero();
            let new_children: Result<Vec<Value>> = va.iter()
                .map(|ai| apply_binary_logic(ai, &Value::from_bool(b_truthy), op))
                .collect();
            Ok(Value {
                data: ValueData::Vector(new_children?),
                display_hint: DisplayHint::Boolean,
                audio_hint: None,
            })
        }

        // 両方ベクター
        (ValueData::Vector(va), ValueData::Vector(vb)) => {
            if va.len() != vb.len() {
                return Err(AjisaiError::VectorLengthMismatch { len1: va.len(), len2: vb.len() });
            }
            let new_children: Result<Vec<Value>> = va.iter().zip(vb.iter())
                .map(|(ai, bi)| apply_binary_logic(ai, bi, op))
                .collect();
            Ok(Value {
                data: ValueData::Vector(new_children?),
                display_hint: DisplayHint::Boolean,
                audio_hint: None,
            })
        }
    }
}

// ============================================================================
// 論理演算子
// ============================================================================

/// NOT 演算子 - 論理否定
///
/// Kleene三値論理:
/// - ゼロなら 1、非ゼロなら 0
/// - NIL なら NIL を返す（不明の否定は不明）
pub fn op_not(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            // NILの場合はNILを返す（Kleene三値論理: NOT NIL = NIL）
            if val.is_nil() {
                interp.stack.push(Value::nil());
                return Ok(());
            }

            // 再帰的にNOT演算を適用
            let result = apply_not_to_value(&val);
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
/// Kleene三値論理:
/// - 両方が非ゼロなら 1、それ以外は 0
/// - FALSE AND NIL = FALSE（FALSEが確定的に偽）
/// - TRUE AND NIL = NIL（不明が伝播）
/// - NIL AND NIL = NIL（不明が伝播）
pub fn op_and(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            let a_is_nil = a_val.is_nil();
            let b_is_nil = b_val.is_nil();

            // Kleene三値論理でのAND処理
            if a_is_nil && b_is_nil {
                // NIL AND NIL = NIL
                interp.stack.push(Value::nil());
                return Ok(());
            } else if a_is_nil {
                // NIL AND b: bがfalsy（全て0）ならFALSE、それ以外はNIL
                let b_has_any_truthy = value_has_any_truthy(&b_val);
                if b_has_any_truthy {
                    interp.stack.push(Value::nil());
                } else {
                    interp.stack.push(Value::from_bool(false));
                }
                return Ok(());
            } else if b_is_nil {
                // a AND NIL: aがfalsy（全て0）ならFALSE、それ以外はNIL
                let a_has_any_truthy = value_has_any_truthy(&a_val);
                if a_has_any_truthy {
                    interp.stack.push(Value::nil());
                } else {
                    interp.stack.push(Value::from_bool(false));
                }
                return Ok(());
            }

            // 両方がNILでない場合は通常のAND演算
            let result = apply_binary_logic(&a_val, &b_val, |a, b| a && b)?;
            interp.stack.push(result);
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

            // Kleene三値論理でのSTACKモードAND:
            // - どれかがNILでどれかがtruthyなら NIL
            // - どれかがfalsy（非NILで全て0）なら FALSE
            // - 全てがtruthyなら TRUE
            let has_nil = items.iter().any(|v| v.is_nil());
            let has_falsy_non_nil = items.iter().any(|v| !v.is_nil() && !v.is_truthy());
            let all_truthy = items.iter().all(|v| v.is_truthy());

            if has_falsy_non_nil {
                interp.stack.push(Value::from_bool(false));
            } else if has_nil {
                interp.stack.push(Value::nil());
            } else if all_truthy {
                interp.stack.push(Value::from_bool(true));
            } else {
                interp.stack.push(Value::from_bool(false));
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
            let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            let a_is_nil = a_val.is_nil();
            let b_is_nil = b_val.is_nil();

            // Kleene三値論理でのOR処理
            if a_is_nil && b_is_nil {
                // NIL OR NIL = NIL
                interp.stack.push(Value::nil());
                return Ok(());
            } else if a_is_nil {
                // NIL OR b: bがtruthy（どれか非0）ならTRUE、それ以外はNIL
                let b_has_any_truthy = value_has_any_truthy(&b_val);
                if b_has_any_truthy {
                    interp.stack.push(Value::from_bool(true));
                } else {
                    interp.stack.push(Value::nil());
                }
                return Ok(());
            } else if b_is_nil {
                // a OR NIL: aがtruthy（どれか非0）ならTRUE、それ以外はNIL
                let a_has_any_truthy = value_has_any_truthy(&a_val);
                if a_has_any_truthy {
                    interp.stack.push(Value::from_bool(true));
                } else {
                    interp.stack.push(Value::nil());
                }
                return Ok(());
            }

            // 両方がNILでない場合は通常のOR演算
            let result = apply_binary_logic(&a_val, &b_val, |a, b| a || b)?;
            interp.stack.push(result);
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

            // Kleene三値論理でのSTACKモードOR:
            // - どれかがtruthy（非NILで非0要素あり）なら TRUE
            // - どれかがNILで残りがfalsy（全て0）なら NIL
            // - 全てがfalsy（非NILで全て0）なら FALSE
            let has_nil = items.iter().any(|v| v.is_nil());
            let has_truthy = items.iter().any(|v| v.is_truthy());

            if has_truthy {
                interp.stack.push(Value::from_bool(true));
            } else if has_nil {
                interp.stack.push(Value::nil());
            } else {
                interp.stack.push(Value::from_bool(false));
            }
            Ok(())
        }
    }
}
