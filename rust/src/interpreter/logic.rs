use crate::interpreter::{Interpreter, OperationTargetMode, ConsumptionMode};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::get_integer_from_value;
use crate::interpreter::tensor_ops::{apply_binary_broadcast, apply_unary_flat, FlatTensor};
use crate::types::{Value, ValueData};
use crate::types::fraction::Fraction;

fn value_has_any_truthy(val: &Value) -> bool {
    match &val.data {
        ValueData::Nil => false,
        ValueData::Scalar(f) => !f.is_zero(),
        ValueData::CodeBlock(_) => true,
        ValueData::Vector(_) | ValueData::Record { .. } => {
            if let Ok(tensor) = FlatTensor::from_value(val) {
                tensor.data.iter().any(|f| !f.is_zero())
            } else {
                false
            }
        }
    }
}

pub fn op_not(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let val = if is_keep_mode {
                interp.stack.last().cloned().ok_or(AjisaiError::StackUnderflow)?
            } else {
                interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
            };

            if val.is_nil() {
                interp.stack.push(Value::nil());
                return Ok(());
            }

            if val.is_scalar() {
                if let Some(f) = val.as_scalar() {
                    let result = if f.is_zero() { Fraction::from(1) } else { Fraction::from(0) };
                    interp.stack.push(Value::from_fraction(result));
                    return Ok(());
                }
            }

            let result = apply_unary_flat(&val, |f| {
                if f.is_zero() { Fraction::from(1) } else { Fraction::from(0) }
            })?;
            interp.stack.push(result);
            Ok(())
        },
        OperationTargetMode::Stack => {
            if is_keep_mode {
                let original: Vec<Value> = interp.stack.iter().cloned().collect();
                let results: Vec<Value> = original.iter().map(|v| {
                    if v.is_nil() {
                        Value::nil()
                    } else if let Some(f) = v.as_scalar() {
                        let r = if f.is_zero() { Fraction::from(1) } else { Fraction::from(0) };
                        Value::from_fraction(r)
                    } else {
                        apply_unary_flat(v, |f| {
                            if f.is_zero() { Fraction::from(1) } else { Fraction::from(0) }
                        }).unwrap_or_else(|_| v.clone())
                    }
                }).collect();
                interp.stack.extend(results);
            } else {
                let elements: Vec<Value> = interp.stack.drain(..).collect();
                for v in elements {
                    if v.is_nil() {
                        interp.stack.push(Value::nil());
                    } else if let Some(f) = v.as_scalar() {
                        let r = if f.is_zero() { Fraction::from(1) } else { Fraction::from(0) };
                        interp.stack.push(Value::from_fraction(r));
                    } else {
                        let result = apply_unary_flat(&v, |f| {
                            if f.is_zero() { Fraction::from(1) } else { Fraction::from(0) }
                        }).unwrap_or_else(|_| v.clone());
                        interp.stack.push(result);
                    }
                }
            }
            Ok(())
        }
    }
}

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

            if a_is_nil && b_is_nil {
                interp.stack.push(Value::nil());
                return Ok(());
            } else if a_is_nil {
                if value_has_any_truthy(&b_val) {
                    interp.stack.push(Value::nil());
                } else {
                    interp.stack.push(Value::from_bool(false));
                }
                return Ok(());
            } else if b_is_nil {
                if value_has_any_truthy(&a_val) {
                    interp.stack.push(Value::nil());
                } else {
                    interp.stack.push(Value::from_bool(false));
                }
                return Ok(());
            }

            let result = apply_binary_broadcast(&a_val, &b_val, |a, b| {
                let a_truthy = !a.is_zero();
                let b_truthy = !b.is_zero();
                Ok(Fraction::from(if a_truthy && b_truthy { 1 } else { 0 }))
            })?;
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

            if a_is_nil && b_is_nil {
                interp.stack.push(Value::nil());
                return Ok(());
            } else if a_is_nil {
                if value_has_any_truthy(&b_val) {
                    interp.stack.push(Value::from_bool(true));
                } else {
                    interp.stack.push(Value::nil());
                }
                return Ok(());
            } else if b_is_nil {
                if value_has_any_truthy(&a_val) {
                    interp.stack.push(Value::from_bool(true));
                } else {
                    interp.stack.push(Value::nil());
                }
                return Ok(());
            }

            let result = apply_binary_broadcast(&a_val, &b_val, |a, b| {
                let a_truthy = !a.is_zero();
                let b_truthy = !b.is_zero();
                Ok(Fraction::from(if a_truthy || b_truthy { 1 } else { 0 }))
            })?;
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
