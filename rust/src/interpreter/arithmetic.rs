use crate::interpreter::{Interpreter, OperationTargetMode, ConsumptionMode};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{extract_integer_from_value, extract_operands_with_flow, push_result, push_flow_result};
use crate::interpreter::simd_ops;
use crate::interpreter::tensor_ops::apply_binary_broadcast;
use crate::types::{Value, ValueData};
use crate::types::fraction::Fraction;

fn extract_scalar_from_value(val: &Value) -> Option<&Fraction> {
    match &val.data {
        ValueData::Scalar(f) => Some(f),
        ValueData::Vector(children) if children.len() == 1 => {
            extract_scalar_from_value(&children[0])
        }
        _ => None
    }
}

fn is_scalar_value(val: &Value) -> bool {
    extract_scalar_from_value(val).is_some()
}

fn apply_binary_arithmetic<F>(interp: &mut Interpreter, op: F, op_name: &str) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let (operands, flow_tokens) = extract_operands_with_flow(interp, 2)?;
            let a_val = &operands[0];
            let b_val = &operands[1];

            let result = match apply_binary_broadcast(a_val, b_val, op) {
                Ok(r) => r,
                Err(e) => {
                    if !is_keep_mode {
                        for val in operands {
                            interp.stack.push(val);
                        }
                    }
                    return Err(e);
                }
            };

            if !interp.disable_no_change_check && (result == *a_val || result == *b_val) {
                if !is_keep_mode {
                    for val in operands {
                        interp.stack.push(val);
                    }
                }
                return Err(AjisaiError::NoChange { word: op_name.into() });
            }

            if let Some(ref tokens) = flow_tokens {
                let consumed: Vec<Fraction> = tokens.iter().map(|t| t.total.clone()).collect();
                push_flow_result(interp, result, Some(tokens), &consumed);
            } else {
                push_result(interp, result);
            }
        },

        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = extract_integer_from_value(&count_val)? as usize;

            if count == 0 || count == 1 {
                interp.stack.push(count_val);
                return Err(AjisaiError::NoChange { word: op_name.into() });
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

            if items.iter().any(|v| !is_scalar_value(v)) {
                if !is_keep_mode {
                    interp.stack.extend(items);
                }
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK mode requires single-element values"));
            }

            let first_scalar = extract_scalar_from_value(&items[0]).unwrap().clone();
            let mut acc = first_scalar.clone();
            let original_first = acc.clone();

            for item in items.iter().skip(1) {
                if let Some(f) = extract_scalar_from_value(item) {
                    acc = op(&acc, f)?;
                }
            }

            if !interp.disable_no_change_check && acc == original_first {
                if !is_keep_mode {
                    interp.stack.extend(items);
                }
                interp.stack.push(count_val);
                return Err(AjisaiError::NoChange { word: op_name.into() });
            }

            push_result(interp, Value::from_fraction(acc));
        }
    }
    Ok(())
}

pub fn op_add(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::StackTop && interp.stack.len() >= 2 {
        let stack_len = interp.stack.len();
        let a = &interp.stack[stack_len - 2];
        let b = &interp.stack[stack_len - 1];

        if let Some(result) = simd_ops::apply_simd_add(a, b) {
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            interp.stack.push(result);
            return Ok(());
        }

        if let Some(result) = simd_ops::apply_simd_scalar_add(a, b)
            .or_else(|| simd_ops::apply_simd_scalar_add(b, a))
        {
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            interp.stack.push(result);
            return Ok(());
        }
    }
    apply_binary_arithmetic(interp, |a, b| Ok(a.add(b)), "+")
}

pub fn op_sub(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::StackTop && interp.stack.len() >= 2 {
        let stack_len = interp.stack.len();
        let a = &interp.stack[stack_len - 2];
        let b = &interp.stack[stack_len - 1];

        if let Some(result) = simd_ops::apply_simd_sub(a, b) {
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            interp.stack.push(result);
            return Ok(());
        }
    }
    apply_binary_arithmetic(interp, |a, b| Ok(a.sub(b)), "-")
}

pub fn op_mul(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::StackTop && interp.stack.len() >= 2 {
        let stack_len = interp.stack.len();
        let a = &interp.stack[stack_len - 2];
        let b = &interp.stack[stack_len - 1];

        if let Some(result) = simd_ops::apply_simd_mul(a, b) {
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            interp.stack.push(result);
            return Ok(());
        }

        if let Some(result) = simd_ops::apply_simd_scalar_mul(a, b)
            .or_else(|| simd_ops::apply_simd_scalar_mul(b, a))
        {
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            interp.stack.push(result);
            return Ok(());
        }
    }
    apply_binary_arithmetic(interp, |a, b| Ok(a.mul(b)), "*")
}

pub fn op_div(interp: &mut Interpreter) -> Result<()> {
    apply_binary_arithmetic(interp, |a, b| {
        if b.is_zero() {
            Err(AjisaiError::DivisionByZero)
        } else {
            Ok(a.div(b))
        }
    }, "/")
}
