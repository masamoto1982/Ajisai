// rust/src/interpreter/logic.rs
//
// 統一Value宇宙アーキテクチャ版の論理演算
//
// 論理演算（0 = false、非0 = true）

use crate::interpreter::{Interpreter, OperationTargetMode, ConsumptionMode};
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
        ValueData::CodeBlock(_) => true,  // コードブロックは常にtruthy
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
        ValueData::CodeBlock(_) => val.clone(),  // コードブロックにはNOTを適用しない
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

        // CodeBlock処理
        (ValueData::CodeBlock(_), _) | (_, ValueData::CodeBlock(_)) => {
            Err(AjisaiError::from("Cannot apply logic operation with code blocks"))
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

/// NOT 演算子 - 論理否定（Map型）
///
/// 【消費モード】
/// - Consume（デフォルト）: オペランドを消費し、結果をプッシュ
/// - Keep（,,）: オペランドを保持し、結果を追加
pub fn op_not(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let val = if is_keep_mode {
                interp.stack.last().cloned().ok_or(AjisaiError::StackUnderflow)?
            } else {
                interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
            };

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
        OperationTargetMode::Stack => {
            if is_keep_mode {
                // Keep mode: preserve original stack, push NOT results
                let original: Vec<Value> = interp.stack.iter().cloned().collect();
                let results: Vec<Value> = original.iter().map(|v| {
                    if v.is_nil() {
                        Value::nil()
                    } else {
                        apply_not_to_value(v)
                    }
                }).collect();
                interp.stack.extend(results);
            } else {
                // Consume mode: replace each element with its NOT
                let elements: Vec<Value> = interp.stack.drain(..).collect();
                for v in elements {
                    if v.is_nil() {
                        interp.stack.push(Value::nil());
                    } else {
                        interp.stack.push(apply_not_to_value(&v));
                    }
                }
            }
            Ok(())
        }
    }
}

/// AND 演算子 - 論理積（Fold型）
///
/// 【消費モード】
/// - Consume（デフォルト）: オペランドを消費し、結果をプッシュ
/// - Keep（,,）: オペランドを保持し、結果を追加
pub fn op_and(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            if interp.stack.len() < 2 {
                return Err(AjisaiError::StackUnderflow);
            }

            let (a_val, b_val) = if is_keep_mode {
                let stack_len = interp.stack.len();
                (interp.stack[stack_len - 2].clone(), interp.stack[stack_len - 1].clone())
            } else {
                let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
                let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
                (a_val, b_val)
            };

            let a_is_nil = a_val.is_nil();
            let b_is_nil = b_val.is_nil();

            // Kleene三値論理でのAND処理
            if a_is_nil && b_is_nil {
                interp.stack.push(Value::nil());
                return Ok(());
            } else if a_is_nil {
                let b_has_any_truthy = value_has_any_truthy(&b_val);
                if b_has_any_truthy {
                    interp.stack.push(Value::nil());
                } else {
                    interp.stack.push(Value::from_bool(false));
                }
                return Ok(());
            } else if b_is_nil {
                let a_has_any_truthy = value_has_any_truthy(&a_val);
                if a_has_any_truthy {
                    interp.stack.push(Value::nil());
                } else {
                    interp.stack.push(Value::from_bool(false));
                }
                return Ok(());
            }

            let result = apply_binary_logic(&a_val, &b_val, |a, b| a && b)?;
            interp.stack.push(result);
            Ok(())
        },

        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            if count == 0 || count == 1 {
                interp.stack.push(count_val);
                return Err(AjisaiError::NoChange { word: "AND".into() });
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let items: Vec<Value> = if is_keep_mode {
                let stack_len = interp.stack.len();
                interp.stack[stack_len - count..].iter().cloned().collect()
            } else {
                interp.stack.drain(interp.stack.len() - count..).collect()
            };

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

/// OR 演算子 - 論理和（Fold型）
///
/// 【消費モード】
/// - Consume（デフォルト）: オペランドを消費し、結果をプッシュ
/// - Keep（,,）: オペランドを保持し、結果を追加
pub fn op_or(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            if interp.stack.len() < 2 {
                return Err(AjisaiError::StackUnderflow);
            }

            let (a_val, b_val) = if is_keep_mode {
                let stack_len = interp.stack.len();
                (interp.stack[stack_len - 2].clone(), interp.stack[stack_len - 1].clone())
            } else {
                let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
                let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
                (a_val, b_val)
            };

            let a_is_nil = a_val.is_nil();
            let b_is_nil = b_val.is_nil();

            // Kleene三値論理でのOR処理
            if a_is_nil && b_is_nil {
                interp.stack.push(Value::nil());
                return Ok(());
            } else if a_is_nil {
                let b_has_any_truthy = value_has_any_truthy(&b_val);
                if b_has_any_truthy {
                    interp.stack.push(Value::from_bool(true));
                } else {
                    interp.stack.push(Value::nil());
                }
                return Ok(());
            } else if b_is_nil {
                let a_has_any_truthy = value_has_any_truthy(&a_val);
                if a_has_any_truthy {
                    interp.stack.push(Value::from_bool(true));
                } else {
                    interp.stack.push(Value::nil());
                }
                return Ok(());
            }

            let result = apply_binary_logic(&a_val, &b_val, |a, b| a || b)?;
            interp.stack.push(result);
            Ok(())
        },

        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            if count == 0 || count == 1 {
                interp.stack.push(count_val);
                return Err(AjisaiError::NoChange { word: "OR".into() });
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let items: Vec<Value> = if is_keep_mode {
                let stack_len = interp.stack.len();
                interp.stack[stack_len - count..].iter().cloned().collect()
            } else {
                interp.stack.drain(interp.stack.len() - count..).collect()
            };

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
