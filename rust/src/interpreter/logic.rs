use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::extract_integer_from_value;
use crate::interpreter::tensor_ops::{apply_binary_broadcast, apply_unary_flat, FlatTensor};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};

fn check_value_has_truthy(val: &Value) -> bool {
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

fn compute_inverted_fraction(f: &Fraction) -> Fraction {
    if f.is_zero() {
        Fraction::from(1)
    } else {
        Fraction::from(0)
    }
}

fn compute_inverted_value(val: &Value) -> Result<Value> {
    if val.is_nil() {
        return Ok(Value::nil());
    }
    if let Some(f) = val.as_scalar() {
        return Ok(Value::from_fraction(compute_inverted_fraction(f)));
    }
    apply_unary_flat(val, compute_inverted_fraction)
}

pub fn op_not(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let val = if is_keep_mode {
                interp
                    .stack
                    .last()
                    .cloned()
                    .ok_or(AjisaiError::StackUnderflow)?
            } else {
                interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
            };

            let result = match compute_inverted_value(&val) {
                Ok(v) => v,
                Err(e) => {
                    if !is_keep_mode {
                        interp.stack.push(val);
                    }
                    return Err(e);
                }
            };

            interp.stack.push(result);
            Ok(())
        }
        OperationTargetMode::Stack => {
            let source: Vec<Value> = interp.stack.iter().cloned().collect();
            let mut results: Vec<Value> = Vec::with_capacity(source.len());
            for value in &source {
                results.push(compute_inverted_value(value)?);
            }

            if is_keep_mode {
                interp.stack.extend(results);
            } else {
                interp.stack = results;
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
                (
                    interp.stack[stack_len - 2].clone(),
                    interp.stack[stack_len - 1].clone(),
                )
            } else {
                let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
                let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
                (a_val, b_val)
            };

            let a_is_nil: bool = a_val.is_nil();
            let b_is_nil: bool = b_val.is_nil();

            if a_is_nil && b_is_nil {
                interp.stack.push(Value::nil());
                return Ok(());
            } else if a_is_nil {
                if check_value_has_truthy(&b_val) {
                    interp.stack.push(Value::nil());
                } else {
                    interp.stack.push(Value::from_bool(false));
                }
                return Ok(());
            } else if b_is_nil {
                if check_value_has_truthy(&a_val) {
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
        }

        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = extract_integer_from_value(&count_val)? as usize;

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

            let has_nil: bool = items.iter().any(|v| v.is_nil());
            let has_falsy_non_nil: bool = items.iter().any(|v| !v.is_nil() && !v.is_truthy());
            let all_truthy: bool = items.iter().all(|v| v.is_truthy());

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
                (
                    interp.stack[stack_len - 2].clone(),
                    interp.stack[stack_len - 1].clone(),
                )
            } else {
                let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
                let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
                (a_val, b_val)
            };

            let a_is_nil: bool = a_val.is_nil();
            let b_is_nil: bool = b_val.is_nil();

            if a_is_nil && b_is_nil {
                interp.stack.push(Value::nil());
                return Ok(());
            } else if a_is_nil {
                if check_value_has_truthy(&b_val) {
                    interp.stack.push(Value::from_bool(true));
                } else {
                    interp.stack.push(Value::nil());
                }
                return Ok(());
            } else if b_is_nil {
                if check_value_has_truthy(&a_val) {
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
        }

        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = extract_integer_from_value(&count_val)? as usize;

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

            let has_nil: bool = items.iter().any(|v| v.is_nil());
            let has_truthy: bool = items.iter().any(|v| v.is_truthy());

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
