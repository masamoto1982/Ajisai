use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::interval_ops::{interval_to_value, value_to_interval};
use crate::interpreter::simd_ops;
use crate::interpreter::tensor_ops::apply_binary_broadcast_with_metrics;
use crate::interpreter::value_extraction_helpers::{
    extract_integer_from_value, extract_operands, nil_passthrough_binary, push_result,
};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::semantic::{AbsenceOrigin, Recoverability};
use crate::types::continued_fraction::ExactReal;
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, SparseTensor, Value, ValueData};

fn extract_scalar_from_value(val: &Value) -> Option<Fraction> {
    match &val.data {
        ValueData::Scalar(f) => Some(f.clone()),
        ValueData::ExactScalar(_) => None, // handled by ExactReal path
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

fn extract_exact_real_from_value(val: &Value) -> Option<ExactReal> {
    match &val.data {
        ValueData::Scalar(f) => Some(ExactReal::from_fraction(f.clone())),
        ValueData::ExactScalar(er) => Some(er.clone()),
        _ => None,
    }
}

fn is_scalar_value(val: &Value) -> bool {
    matches!(&val.data, ValueData::Scalar(_) | ValueData::ExactScalar(_))
        || matches!(&val.data, ValueData::Vector(c) if c.len() == 1 && extract_scalar_from_value(&c[0]).is_some())
        || matches!(&val.data, ValueData::Tensor { data, .. } if data.len() == 1)
}

/// Apply an ExactReal binary operation to two stack values where at least
/// one is an `ExactScalar`. Returns `None` if neither value is an ExactScalar
/// (let the Fraction fast path handle it).
fn try_exact_real_binary_op<F>(a_val: &Value, b_val: &Value, op: F) -> Option<Value>
where
    F: Fn(&ExactReal, &ExactReal) -> Option<ExactReal>,
{
    let has_exact = matches!(&a_val.data, ValueData::ExactScalar(_))
        || matches!(&b_val.data, ValueData::ExactScalar(_));
    if !has_exact {
        return None;
    }
    let a = extract_exact_real_from_value(a_val)?;
    let b = extract_exact_real_from_value(b_val)?;
    let result = op(&a, &b)?;
    Some(Value::from_exact_real(result))
}

fn sparse_mul_candidate(a: &Value, b: &Value) -> Option<Value> {
    if let Some(result) = sparse_tensor_scalar_mul(a, b) {
        return Some(result);
    }
    if let Some(result) = sparse_tensor_scalar_mul(b, a) {
        return Some(result);
    }
    sparse_same_shape_tensor_mul(a, b)
}

fn sparse_tensor_scalar_mul(tensor_value: &Value, scalar_value: &Value) -> Option<Value> {
    let (dense, shape) = tensor_value.as_dense_tensor()?;
    if !dense.is_sparse_candidate() {
        return None;
    }
    let scalar = extract_scalar_from_value(scalar_value)?;
    let sparse = SparseTensor::from_dense(dense)?;
    let mut result = vec![Fraction::from(0_i64); sparse.len];
    for (&index, entry) in sparse.indices.iter().zip(0..) {
        result[index] = Fraction::new(
            sparse.numerators[entry].into(),
            sparse.denominators[entry].into(),
        )
        .mul(&scalar);
    }
    Some(Value::from_tensor(result, shape.to_vec()))
}

fn sparse_same_shape_tensor_mul(a: &Value, b: &Value) -> Option<Value> {
    let (a_dense, a_shape) = a.as_dense_tensor()?;
    let (b_dense, b_shape) = b.as_dense_tensor()?;
    if a_shape != b_shape || a_dense.len() != b_dense.len() {
        return None;
    }

    let use_a_sparse = a_dense.is_sparse_candidate();
    let use_b_sparse = b_dense.is_sparse_candidate();
    if !use_a_sparse && !use_b_sparse {
        return None;
    }

    let (sparse, other) = if use_a_sparse {
        (SparseTensor::from_dense(a_dense)?, b_dense)
    } else {
        (SparseTensor::from_dense(b_dense)?, a_dense)
    };

    let mut result = vec![Fraction::from(0_i64); sparse.len];
    for (&index, entry) in sparse.indices.iter().zip(0..) {
        let lhs = Fraction::new(
            sparse.numerators[entry].into(),
            sparse.denominators[entry].into(),
        );
        let rhs = other.get_small_fraction(index)?;
        result[index] = lhs.mul(&rhs);
    }
    Some(Value::from_tensor(result, a_shape.to_vec()))
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
    // ExactScalar path: at least one operand is an exact irrational
    if interp.operation_target_mode == OperationTargetMode::StackTop && interp.stack.len() >= 2 {
        let stack_len = interp.stack.len();
        let a = &interp.stack[stack_len - 2];
        let b = &interp.stack[stack_len - 1];
        if let Some(result) = try_exact_real_binary_op(a, b, |a, b| Some(a.add(b))) {
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
    // ExactScalar path
    if interp.operation_target_mode == OperationTargetMode::StackTop && interp.stack.len() >= 2 {
        let stack_len = interp.stack.len();
        let a = &interp.stack[stack_len - 2];
        let b = &interp.stack[stack_len - 1];
        if let Some(result) = try_exact_real_binary_op(a, b, |a, b| Some(a.sub(b))) {
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

        if let Some(result) = sparse_mul_candidate(a, b) {
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            interp.stack.push(result);
            return Ok(());
        }
    }
    // ExactScalar path
    if interp.operation_target_mode == OperationTargetMode::StackTop && interp.stack.len() >= 2 {
        let stack_len = interp.stack.len();
        let a = &interp.stack[stack_len - 2];
        let b = &interp.stack[stack_len - 1];
        if let Some(result) = try_exact_real_binary_op(a, b, |a, b| Some(a.mul(b))) {
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
            match ai.div(&bi) {
                Ok(result) => interp.stack.push(interval_to_value(result)),
                Err(AjisaiError::DivisionByZero) => interp.stack.push(Value::bubble_with_reason(
                    NilReason::DivisionByZero,
                    AbsenceOrigin::ExecutionFailure,
                    Recoverability::Recoverable,
                )),
                Err(error) => return Err(error),
            }
            return Ok(());
        }
    }
    // ExactScalar path: at least one operand is an exact irrational. Placed
    // before the generic broadcast block (which cannot convert ExactScalar to
    // a FlatTensor and would hard-error first), matching op_add/op_sub/op_mul.
    if interp.operation_target_mode == OperationTargetMode::StackTop && interp.stack.len() >= 2 {
        let stack_len = interp.stack.len();
        let a = &interp.stack[stack_len - 2];
        let b = &interp.stack[stack_len - 1];
        if let Some(result) = try_exact_real_binary_op(a, b, |a, b| a.div(b)) {
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            interp.stack.push(result);
            return Ok(());
        } else if matches!(&a.data, ValueData::ExactScalar(_))
            || matches!(&b.data, ValueData::ExactScalar(_))
        {
            // div returned None — structurally-zero divisor. DivisionByZero is a
            // recoverable Bubble (NilReason::DivisionByZero), not a hard error.
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            interp.stack.push(Value::bubble_with_reason(
                NilReason::DivisionByZero,
                AbsenceOrigin::ExecutionFailure,
                Recoverability::Recoverable,
            ));
            return Ok(());
        }
    }
    if interp.operation_target_mode == OperationTargetMode::StackTop {
        let stack_len = interp.stack.len();
        if stack_len >= 2 {
            let left_hint = interp.semantic_registry.lookup_hint_at(stack_len - 2);
            let right_hint = interp.semantic_registry.lookup_hint_at(stack_len - 1);
            if matches!(left_hint, Interpretation::Text) || matches!(right_hint, Interpretation::Text)
            {
                return Err(AjisaiError::create_structure_error("number", "string"));
            }
        }
        let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
        let operands = extract_operands(interp, 2)?;
        let a_val = &operands[0];
        let b_val = &operands[1];

        match apply_binary_broadcast_with_metrics(
            a_val,
            b_val,
            |a, b| {
                if b.is_zero() {
                    Err(AjisaiError::DivisionByZero)
                } else {
                    Ok(a.div(b))
                }
            },
            Some(&mut interp.runtime_metrics),
        ) {
            Ok(result) => {
                push_result(interp, result);
                return Ok(());
            }
            Err(AjisaiError::DivisionByZero) => {
                interp.stack.push(Value::bubble_with_reason(
                    NilReason::DivisionByZero,
                    AbsenceOrigin::ExecutionFailure,
                    Recoverability::Recoverable,
                ));
                return Ok(());
            }
            Err(error) => {
                if !is_keep_mode {
                    for val in operands {
                        interp.stack.push(val);
                    }
                }
                return Err(error);
            }
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
