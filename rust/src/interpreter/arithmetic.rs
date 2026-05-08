use crate::error::{AjisaiError, Result};
use crate::interpreter::interval_ops::{interval_to_value, value_to_interval};
use crate::interpreter::simd_ops;
use crate::interpreter::tensor_ops::apply_binary_broadcast_with_metrics;
use crate::interpreter::value_extraction_helpers::{
    extract_integer_from_value, extract_operands, nil_passthrough_binary, push_result,
};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};

fn extract_scalar_from_value(val: &Value) -> Option<Fraction> {
    match &val.data {
        ValueData::Scalar(f) => Some(f.clone()),
        ValueData::Vector(children) if children.len() == 1 => {
            extract_scalar_from_value(&children[0])
        }
        ValueData::Vector(_) => None,
        ValueData::Tensor { data, .. } if data.len() == 1 => data.get_small_fraction(0),
        ValueData::Tensor { .. } => None,
        ValueData::Nil => None,
        ValueData::Record { .. } => None,
        ValueData::CodeBlock(_) | ValueData::ProcessHandle(_) | ValueData::SupervisorHandle(_) => {
            None
        }
    }
}

fn is_scalar_value(val: &Value) -> bool {
    extract_scalar_from_value(val).is_some()
}

fn apply_binary_arithmetic<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let operands = extract_operands(interp, 2)?;
            let a_val = &operands[0];
            let b_val = &operands[1];

            let result = match apply_binary_broadcast_with_metrics(
                a_val,
                b_val,
                op,
                Some(&mut interp.runtime_metrics),
            ) {
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

            push_result(interp, result);
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
                interp.stack[stack_len - count..]
                    .iter()
                    .cloned()
                    .collect::<Vec<Value>>()
            } else {
                interp
                    .stack
                    .drain(interp.stack.len() - count..)
                    .collect::<Vec<Value>>()
            };

            if items.iter().any(|v| v.is_nil()) {
                push_result(interp, Value::nil());
                return Ok(());
            }

            if items.iter().any(|v| !is_scalar_value(v)) {
                if !is_keep_mode {
                    interp.stack.extend(items);
                }
                interp.stack.push(count_val);
                return Err(AjisaiError::from("+: expected scalar values in Stack mode"));
            }

            let first_scalar: Fraction = extract_scalar_from_value(&items[0]).unwrap();
            let mut acc = first_scalar.clone();

            for item in items.iter().skip(1) {
                if let Some(f) = extract_scalar_from_value(item) {
                    acc = op(&acc, &f)?;
                }
            }

            push_result(interp, Value::from_fraction(acc));
        }
    }
    Ok(())
}

pub fn op_add(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::StackTop
        && nil_passthrough_binary(interp)
    {
        return Ok(());
    }
    if interp.operation_target_mode == OperationTargetMode::StackTop && interp.stack.len() >= 2 {
        let stack_len = interp.stack.len();
        let a = interp.stack[stack_len - 2].clone();
        let b = interp.stack[stack_len - 1].clone();
        if let (Some(ai), Some(bi)) = (value_to_interval(&a), value_to_interval(&b)) {
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            interp.stack.push(interval_to_value(ai.add(&bi)));
            return Ok(());
        }
    }
    if interp.operation_target_mode == OperationTargetMode::StackTop && interp.stack.len() >= 2 {
        let stack_len = interp.stack.len();
        let a = &interp.stack[stack_len - 2];
        let b = &interp.stack[stack_len - 1];

        if let Some(result) = simd_ops::apply_simd_add(a, b) {
            interp.runtime_metrics.vtu_simd_kernel_use_count = interp
                .runtime_metrics
                .vtu_simd_kernel_use_count
                .saturating_add(1);
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            interp.stack.push(result);
            return Ok(());
        }

        if let Some(result) =
            simd_ops::apply_simd_scalar_add(a, b).or_else(|| simd_ops::apply_simd_scalar_add(b, a))
        {
            interp.runtime_metrics.vtu_simd_kernel_use_count = interp
                .runtime_metrics
                .vtu_simd_kernel_use_count
                .saturating_add(1);
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            interp.stack.push(result);
            return Ok(());
        }
    }
    apply_binary_arithmetic(interp, |a, b| Ok(a.add(b)))
}

pub fn op_sub(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::StackTop
        && nil_passthrough_binary(interp)
    {
        return Ok(());
    }
    if interp.operation_target_mode == OperationTargetMode::StackTop && interp.stack.len() >= 2 {
        let stack_len = interp.stack.len();
        let a = interp.stack[stack_len - 2].clone();
        let b = interp.stack[stack_len - 1].clone();
        if let (Some(ai), Some(bi)) = (value_to_interval(&a), value_to_interval(&b)) {
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            interp.stack.push(interval_to_value(ai.sub(&bi)));
            return Ok(());
        }
    }
    if interp.operation_target_mode == OperationTargetMode::StackTop && interp.stack.len() >= 2 {
        let stack_len = interp.stack.len();
        let a = &interp.stack[stack_len - 2];
        let b = &interp.stack[stack_len - 1];

        if let Some(result) = simd_ops::apply_simd_sub(a, b) {
            interp.runtime_metrics.vtu_simd_kernel_use_count = interp
                .runtime_metrics
                .vtu_simd_kernel_use_count
                .saturating_add(1);
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            interp.stack.push(result);
            return Ok(());
        }
    }
    apply_binary_arithmetic(interp, |a, b| Ok(a.sub(b)))
}

pub fn op_mul(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::StackTop
        && nil_passthrough_binary(interp)
    {
        return Ok(());
    }
    if interp.operation_target_mode == OperationTargetMode::StackTop && interp.stack.len() >= 2 {
        let stack_len = interp.stack.len();
        let a = interp.stack[stack_len - 2].clone();
        let b = interp.stack[stack_len - 1].clone();
        if let (Some(ai), Some(bi)) = (value_to_interval(&a), value_to_interval(&b)) {
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            interp.stack.push(interval_to_value(ai.mul(&bi)));
            return Ok(());
        }
    }
    if interp.operation_target_mode == OperationTargetMode::StackTop && interp.stack.len() >= 2 {
        let stack_len = interp.stack.len();
        let a = &interp.stack[stack_len - 2];
        let b = &interp.stack[stack_len - 1];

        if let Some(result) = simd_ops::apply_simd_mul(a, b) {
            interp.runtime_metrics.vtu_simd_kernel_use_count = interp
                .runtime_metrics
                .vtu_simd_kernel_use_count
                .saturating_add(1);
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            interp.stack.push(result);
            return Ok(());
        }

        if let Some(result) =
            simd_ops::apply_simd_scalar_mul(a, b).or_else(|| simd_ops::apply_simd_scalar_mul(b, a))
        {
            interp.runtime_metrics.vtu_simd_kernel_use_count = interp
                .runtime_metrics
                .vtu_simd_kernel_use_count
                .saturating_add(1);
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            interp.stack.push(result);
            return Ok(());
        }
    }
    apply_binary_arithmetic(interp, |a, b| Ok(a.mul(b)))
}

pub fn op_div(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::StackTop
        && nil_passthrough_binary(interp)
    {
        return Ok(());
    }
    if interp.operation_target_mode == OperationTargetMode::StackTop && interp.stack.len() >= 2 {
        let stack_len = interp.stack.len();
        let a = interp.stack[stack_len - 2].clone();
        let b = interp.stack[stack_len - 1].clone();
        if let (Some(ai), Some(bi)) = (value_to_interval(&a), value_to_interval(&b)) {
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            let result = ai.div(&bi)?;
            interp.stack.push(interval_to_value(result));
            return Ok(());
        }
    }
    apply_binary_arithmetic(interp, |a, b| {
        if b.is_zero() {
            Err(AjisaiError::DivisionByZero)
        } else {
            Ok(a.div(b))
        }
    })
}
