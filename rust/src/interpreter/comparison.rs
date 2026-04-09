use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::extract_integer_from_value;
use crate::interpreter::tensor_ops::FlatTensor;
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};

fn extract_scalar_for_comparison(val: &Value) -> Result<Fraction> {
    match &val.data {
        ValueData::Scalar(f) => Ok(f.clone()),
        ValueData::Vector(_) | ValueData::Record { .. } => {
            let tensor = FlatTensor::from_value(val)?;
            if tensor.data.len() != 1 {
                return Err(AjisaiError::create_structure_error(
                    "scalar value",
                    "non-scalar value",
                ));
            }
            Ok(tensor.data[0].clone())
        }
        ValueData::Nil => Err(AjisaiError::create_structure_error(
            "scalar value",
            "non-scalar value",
        )),
        ValueData::CodeBlock(_) => Err(AjisaiError::create_structure_error(
            "scalar value",
            "non-scalar value",
        )),
    }
}

fn check_all_adjacent_pairs<F>(items: &[Value], op: F) -> Result<bool>
where
    F: Fn(&Fraction, &Fraction) -> bool,
{
    for pair in items.windows(2) {
        let a_scalar: Fraction = extract_scalar_for_comparison(&pair[0])?;
        let b_scalar: Fraction = extract_scalar_for_comparison(&pair[1])?;
        if !op(&a_scalar, &b_scalar) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn check_all_adjacent_equal(items: &[Value]) -> bool {
    items.windows(2).all(|pair| pair[0].data == pair[1].data)
}

fn apply_binary_comparison<F>(interp: &mut Interpreter, op: F, _op_name: &str) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> bool,
{
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            if interp.stack.len() < 2 {
                return Err(AjisaiError::StackUnderflow);
            }

            let (a_val, b_val) = if is_keep_mode {
                let stack_len = interp.stack.len();
                let a_val = interp.stack[stack_len - 2].clone();
                let b_val = interp.stack[stack_len - 1].clone();
                (a_val, b_val)
            } else {
                let b_val = interp.stack.pop().unwrap();
                let a_val = interp.stack.pop().unwrap();
                (a_val, b_val)
            };

            let a_scalar: Fraction = match extract_scalar_for_comparison(&a_val) {
                Ok(f) => f,
                Err(e) => {
                    if !is_keep_mode {
                        interp.stack.push(a_val);
                        interp.stack.push(b_val);
                    }
                    return Err(e);
                }
            };
            let b_scalar: Fraction = match extract_scalar_for_comparison(&b_val) {
                Ok(f) => f,
                Err(e) => {
                    if !is_keep_mode {
                        interp.stack.push(a_val);
                        interp.stack.push(b_val);
                    }
                    return Err(e);
                }
            };

            let result: bool = op(&a_scalar, &b_scalar);
            if interp.gui_mode {
                interp
                    .stack
                    .push(Value::from_vector(vec![Value::from_bool(result)]));
            } else {
                interp.stack.push(Value::from_bool(result));
            }
            Ok(())
        }

        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = extract_integer_from_value(&count_val)? as usize;

            if count == 0 || count == 1 {
                interp.stack.push(count_val);
                return Ok(());
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

            let all_true: bool = match check_all_adjacent_pairs(&items, op) {
                Ok(v) => v,
                Err(e) => {
                    if !is_keep_mode {
                        interp.stack.extend(items);
                    }
                    interp.stack.push(count_val);
                    return Err(e);
                }
            };

            interp.stack.push(Value::from_bool(all_true));
            Ok(())
        }
    }
}

pub fn op_lt(interp: &mut Interpreter) -> Result<()> {
    apply_binary_comparison(interp, |a, b| a.lt(b), "<")
}

pub fn op_le(interp: &mut Interpreter) -> Result<()> {
    apply_binary_comparison(interp, |a, b| a.le(b), "<=")
}

pub fn op_eq(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            if interp.stack.len() < 2 {
                return Err(AjisaiError::StackUnderflow);
            }

            let (a_val, b_val) = if is_keep_mode {
                let stack_len = interp.stack.len();
                let a_val = interp.stack[stack_len - 2].clone();
                let b_val = interp.stack[stack_len - 1].clone();
                (a_val, b_val)
            } else {
                let b_val = interp.stack.pop().unwrap();
                let a_val = interp.stack.pop().unwrap();
                (a_val, b_val)
            };

            let result: bool = if a_val.data == b_val.data {
                true
            } else {

                match (&a_val.data, &b_val.data) {
                    (ValueData::Scalar(_), ValueData::Vector(children))
                        if children.len() == 1 =>
                    {
                        a_val.data == children[0].data
                    }
                    (ValueData::Vector(children), ValueData::Scalar(_))
                        if children.len() == 1 =>
                    {
                        children[0].data == b_val.data
                    }
                    _ => false,
                }
            };
            if interp.gui_mode {
                interp
                    .stack
                    .push(Value::from_vector(vec![Value::from_bool(result)]));
            } else {
                interp.stack.push(Value::from_bool(result));
            }
            Ok(())
        }

        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = extract_integer_from_value(&count_val)? as usize;

            if count == 0 || count == 1 {
                interp.stack.push(count_val);
                return Ok(());
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

            let all_equal: bool = check_all_adjacent_equal(&items);
            interp.stack.push(Value::from_bool(all_equal));
            Ok(())
        }
    }
}
