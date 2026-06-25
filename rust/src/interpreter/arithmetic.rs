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

#[derive(Clone, Copy)]
enum ExactArithmeticSchema {
    Add,
    Sub,
    Mul,
    Div,
}

impl ExactArithmeticSchema {
    fn fraction(self, a: &Fraction, b: &Fraction) -> Result<Fraction> {
        match self {
            ExactArithmeticSchema::Add => Ok(a.add(b)),
            ExactArithmeticSchema::Sub => Ok(a.sub(b)),
            ExactArithmeticSchema::Mul => Ok(a.mul(b)),
            ExactArithmeticSchema::Div => {
                if b.is_zero() {
                    Err(AjisaiError::DivisionByZero)
                } else {
                    Ok(a.div(b))
                }
            }
        }
    }

    fn exact_real(self, a: &ExactReal, b: &ExactReal) -> Option<ExactReal> {
        match self {
            ExactArithmeticSchema::Add => Some(a.add(b)),
            ExactArithmeticSchema::Sub => Some(a.sub(b)),
            ExactArithmeticSchema::Mul => Some(a.mul(b)),
            ExactArithmeticSchema::Div => a.div(b),
        }
    }
}

fn consume_stacktop_binary(interp: &mut Interpreter) {
    if interp.consumption_mode != ConsumptionMode::Keep {
        interp.stack.pop();
        interp.stack.pop();
    }
}

fn division_by_zero_bubble() -> Value {
    Value::bubble_with_reason(
        NilReason::DivisionByZero,
        AbsenceOrigin::ExecutionFailure,
        Recoverability::Recoverable,
    )
}

fn push_interval_schema_result(
    interp: &mut Interpreter,
    schema: ExactArithmeticSchema,
    a: &Value,
    b: &Value,
) -> Result<bool> {
    let (Some(ai), Some(bi)) = (value_to_interval(a), value_to_interval(b)) else {
        return Ok(false);
    };
    consume_stacktop_binary(interp);
    match schema {
        ExactArithmeticSchema::Add => interp.stack.push(interval_to_value(ai.add(&bi))),
        ExactArithmeticSchema::Sub => interp.stack.push(interval_to_value(ai.sub(&bi))),
        ExactArithmeticSchema::Mul => interp.stack.push(interval_to_value(ai.mul(&bi))),
        ExactArithmeticSchema::Div => match ai.div(&bi) {
            Ok(result) => interp.stack.push(interval_to_value(result)),
            Err(AjisaiError::DivisionByZero) => interp.stack.push(division_by_zero_bubble()),
            Err(error) => return Err(error),
        },
    }
    Ok(true)
}

/// Returns `(result, parallel_used)` where `parallel_used` is `true` only when
/// the native multi-core kernel actually fired for this operation.
fn simd_schema_candidate(
    schema: ExactArithmeticSchema,
    a: &Value,
    b: &Value,
) -> Option<(Value, bool)> {
    match schema {
        ExactArithmeticSchema::Add => simd_ops::apply_simd_add(a, b)
            .or_else(|| simd_ops::apply_simd_scalar_add(a, b))
            .or_else(|| simd_ops::apply_simd_scalar_add(b, a)),
        ExactArithmeticSchema::Sub => simd_ops::apply_simd_sub(a, b)
            .or_else(|| simd_ops::apply_simd_scalar_sub(a, b))
            .or_else(|| simd_ops::apply_simd_scalar_rsub(a, b)),
        ExactArithmeticSchema::Mul => simd_ops::apply_simd_mul(a, b)
            .or_else(|| simd_ops::apply_simd_scalar_mul(a, b))
            .or_else(|| simd_ops::apply_simd_scalar_mul(b, a)),
        ExactArithmeticSchema::Div => {
            simd_ops::apply_simd_scalar_div(a, b).or_else(|| simd_ops::apply_simd_scalar_rdiv(a, b))
        }
    }
}

fn push_simd_schema_result(
    interp: &mut Interpreter,
    schema: ExactArithmeticSchema,
    a: &Value,
    b: &Value,
) -> bool {
    let Some((result, parallel_used)) = simd_schema_candidate(schema, a, b) else {
        return false;
    };
    interp.runtime_metrics.vtu_simd_kernel_use_count = interp
        .runtime_metrics
        .vtu_simd_kernel_use_count
        .saturating_add(1);
    if parallel_used {
        interp.runtime_metrics.vtu_parallel_kernel_use_count = interp
            .runtime_metrics
            .vtu_parallel_kernel_use_count
            .saturating_add(1);
    }
    consume_stacktop_binary(interp);
    interp.stack.push(result);
    true
}

fn push_exact_real_schema_result(
    interp: &mut Interpreter,
    schema: ExactArithmeticSchema,
    a: &Value,
    b: &Value,
) -> bool {
    let has_exact = matches!(&a.data, ValueData::ExactScalar(_))
        || matches!(&b.data, ValueData::ExactScalar(_));
    if !has_exact {
        return false;
    }
    let Some(a_exact) = extract_exact_real_from_value(a) else {
        return false;
    };
    let Some(b_exact) = extract_exact_real_from_value(b) else {
        return false;
    };
    consume_stacktop_binary(interp);
    if let Some(result) = schema.exact_real(&a_exact, &b_exact) {
        interp.stack.push(Value::from_exact_real(result));
    } else {
        interp.stack.push(division_by_zero_bubble());
    }
    true
}

fn stacktop_pair(interp: &Interpreter) -> Option<(Value, Value)> {
    if interp.operation_target_mode != OperationTargetMode::StackTop || interp.stack.len() < 2 {
        return None;
    }
    let stack_len = interp.stack.len();
    Some((
        interp.stack[stack_len - 2].clone(),
        interp.stack[stack_len - 1].clone(),
    ))
}

fn apply_exact_arithmetic_schema(
    interp: &mut Interpreter,
    schema: ExactArithmeticSchema,
) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::StackTop
        && nil_passthrough_binary(interp)
    {
        return Ok(());
    }

    if let Some((a, b)) = stacktop_pair(interp) {
        if push_interval_schema_result(interp, schema, &a, &b)? {
            return Ok(());
        }
        if push_simd_schema_result(interp, schema, &a, &b) {
            return Ok(());
        }
        if matches!(schema, ExactArithmeticSchema::Mul) {
            if let Some(result) = sparse_mul_candidate(&a, &b) {
                consume_stacktop_binary(interp);
                interp.stack.push(result);
                return Ok(());
            }
        }
        if push_exact_real_schema_result(interp, schema, &a, &b) {
            return Ok(());
        }
    }

    if matches!(schema, ExactArithmeticSchema::Div)
        && interp.operation_target_mode == OperationTargetMode::StackTop
    {
        let stack_len = interp.stack.len();
        if stack_len >= 2 {
            let left_hint = interp.semantic_registry.lookup_hint_at(stack_len - 2);
            let right_hint = interp.semantic_registry.lookup_hint_at(stack_len - 1);
            if matches!(left_hint, Interpretation::Text)
                || matches!(right_hint, Interpretation::Text)
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
            |a, b| schema.fraction(a, b),
            Some(&mut interp.runtime_metrics),
        ) {
            Ok(result) => {
                push_result(interp, result);
                return Ok(());
            }
            Err(AjisaiError::DivisionByZero) => {
                interp.stack.push(division_by_zero_bubble());
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

    apply_binary_arithmetic(interp, |a, b| schema.fraction(a, b))
}

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
        ValueData::Boolean(_)
        | ValueData::CodeBlock(_)
        | ValueData::ProcessHandle(_)
        | ValueData::SupervisorHandle(_) => None,
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
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy + Sync,
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
    apply_exact_arithmetic_schema(interp, ExactArithmeticSchema::Add)
}

pub fn op_sub(interp: &mut Interpreter) -> Result<()> {
    apply_exact_arithmetic_schema(interp, ExactArithmeticSchema::Sub)
}

pub fn op_mul(interp: &mut Interpreter) -> Result<()> {
    apply_exact_arithmetic_schema(interp, ExactArithmeticSchema::Mul)
}

pub fn op_div(interp: &mut Interpreter) -> Result<()> {
    apply_exact_arithmetic_schema(interp, ExactArithmeticSchema::Div)
}
