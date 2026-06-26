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
use crate::types::{DenseTensor, Interpretation, SparseTensor, Value, ValueData};
use std::sync::Arc;

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
        ExactArithmeticSchema::Sub => simd_ops::apply_simd_sub(a, b),
        ExactArithmeticSchema::Mul => simd_ops::apply_simd_mul(a, b)
            .or_else(|| simd_ops::apply_simd_scalar_mul(a, b))
            .or_else(|| simd_ops::apply_simd_scalar_mul(b, a)),
        ExactArithmeticSchema::Div => None,
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

enum ScalarFastWrap {
    Scalar,
    Tensor(Vec<usize>),
}

struct ScalarFastOperand {
    fraction: Fraction,
    wrap: ScalarFastWrap,
}

fn scalar_fast_operand(value: &Value) -> Option<ScalarFastOperand> {
    match &value.data {
        ValueData::Scalar(f) => Some(ScalarFastOperand {
            fraction: f.clone(),
            wrap: ScalarFastWrap::Scalar,
        }),
        ValueData::Tensor { data, shape } if data.len() == 1 => Some(ScalarFastOperand {
            fraction: data.get_small_fraction(0)?,
            wrap: ScalarFastWrap::Tensor((**shape).clone()),
        }),
        ValueData::Vector(children)
            if value.hint != Interpretation::Text && children.len() == 1 =>
        {
            let child = scalar_fast_operand(&children[0])?;
            let mut shape = Vec::with_capacity(2);
            shape.push(1);
            match child.wrap {
                ScalarFastWrap::Scalar => {}
                ScalarFastWrap::Tensor(child_shape) => shape.extend(child_shape),
            }
            Some(ScalarFastOperand {
                fraction: child.fraction,
                wrap: ScalarFastWrap::Tensor(shape),
            })
        }
        _ => None,
    }
}

fn same_scalar_fast_wrap(a: &ScalarFastWrap, b: &ScalarFastWrap) -> bool {
    match (a, b) {
        (ScalarFastWrap::Scalar, ScalarFastWrap::Scalar) => true,
        (ScalarFastWrap::Tensor(a_shape), ScalarFastWrap::Tensor(b_shape)) => a_shape == b_shape,
        _ => false,
    }
}

fn build_scalar_fast_result(result: Fraction, wrap: &ScalarFastWrap) -> Value {
    match wrap {
        ScalarFastWrap::Scalar => Value::from_fraction(result),
        ScalarFastWrap::Tensor(shape) => {
            if let Some(data) = DenseTensor::from_fractions(vec![result.clone()], shape.clone()) {
                Value {
                    data: ValueData::Tensor {
                        data: Arc::new(data),
                        shape: Arc::new(shape.clone()),
                    },
                    hint: Interpretation::Unassigned,
                    absence: None,
                }
            } else {
                Value::from_tensor(vec![result], shape.clone())
            }
        }
    }
}

fn push_scalar_fastpath_result(
    interp: &mut Interpreter,
    schema: ExactArithmeticSchema,
) -> Result<bool> {
    if !interp.scalar_fastpath_enabled
        || interp.operation_target_mode != OperationTargetMode::StackTop
        || interp.stack.len() < 2
    {
        return Ok(false);
    }

    let stack_len = interp.stack.len();
    let Some(a) = scalar_fast_operand(&interp.stack[stack_len - 2]) else {
        return Ok(false);
    };
    let Some(b) = scalar_fast_operand(&interp.stack[stack_len - 1]) else {
        return Ok(false);
    };
    if !same_scalar_fast_wrap(&a.wrap, &b.wrap) {
        return Ok(false);
    }

    let result = match schema.fraction(&a.fraction, &b.fraction) {
        Ok(result) => build_scalar_fast_result(result, &a.wrap),
        Err(AjisaiError::DivisionByZero) => division_by_zero_bubble(),
        Err(error) => return Err(error),
    };
    if interp.consumption_mode == ConsumptionMode::Consume {
        interp.stack.pop();
        interp.stack.pop();
    }
    push_result(interp, result);
    interp.runtime_metrics.scalar_fastpath_count = interp
        .runtime_metrics
        .scalar_fastpath_count
        .saturating_add(1);
    Ok(true)
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

    if push_scalar_fastpath_result(interp, schema)? {
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
        // Vectors/structures carrying irrational `ExactScalar` lanes cannot use
        // the rational broadcast (it hard-errors on `FlatTensor::from_value`);
        // route them through the exact-real recursive broadcast instead.
        if push_exact_real_broadcast_result(interp, schema, &a, &b)? {
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

/// `true` when `val` is, or structurally contains, an irrational `ExactScalar`
/// leaf. The all-rational `Fraction`/`FlatTensor` broadcast path cannot carry
/// irrational continued-fraction lanes (`FlatTensor::from_value` rejects
/// `ExactScalar`), so its presence selects the exact-real recursive route
/// below instead. Bare scalar `ExactScalar` operands are already handled by
/// `push_exact_real_schema_result` upstream; this predicate exists to catch the
/// *vector/structural* cases that would otherwise hard-error in broadcast.
fn value_contains_exact_scalar(val: &Value) -> bool {
    match &val.data {
        ValueData::ExactScalar(_) => true,
        ValueData::Vector(items) | ValueData::Record { pairs: items, .. } => {
            items.iter().any(value_contains_exact_scalar)
        }
        // Dense tensors only hold rational `Fraction` lanes; they can never
        // contain an `ExactScalar`.
        _ => false,
    }
}

/// Element-wise binary broadcast over operands that may contain irrational
/// `ExactScalar` leaves, computed lane-by-lane through exact-real arithmetic.
///
/// This mirrors the rational `apply_recursive_broadcast` in `tensor_ops`
/// (same shape rules: scalar broadcasts across a vector; equal-length vectors
/// combine pairwise; unequal lengths raise `VectorLengthMismatch`) but keeps
/// each lane exact. `Value::from_exact_real` renormalizes any lane that lands
/// back on a rational to a plain `Scalar`, so an all-rational result is
/// byte-identical to the rational path. Per-lane division by zero becomes a
/// reasoned `NIL` bubble — the same Bubble Rule the scalar `√x 0 /` path uses
/// (SPEC §11.2) — rather than aborting the whole vector.
fn apply_exact_real_recursive_broadcast(
    a: &Value,
    b: &Value,
    schema: ExactArithmeticSchema,
) -> Result<Value> {
    use crate::interpreter::tensor_ops::broadcast_children;

    match (broadcast_children(a), broadcast_children(b)) {
        (None, None) => {
            let (Some(ea), Some(eb)) = (exact_broadcast_leaf(a), exact_broadcast_leaf(b)) else {
                return Err(AjisaiError::create_structure_error(
                    "number or vector",
                    "non-numeric value",
                ));
            };
            Ok(match schema.exact_real(&ea, &eb) {
                Some(result) => Value::from_exact_real(result),
                None => division_by_zero_bubble(),
            })
        }
        (Some(children), None) => {
            let out = children
                .iter()
                .map(|child| apply_exact_real_recursive_broadcast(child, b, schema))
                .collect::<Result<Vec<Value>>>()?;
            Ok(Value::from_children(out))
        }
        (None, Some(children)) => {
            let out = children
                .iter()
                .map(|child| apply_exact_real_recursive_broadcast(a, child, schema))
                .collect::<Result<Vec<Value>>>()?;
            Ok(Value::from_children(out))
        }
        (Some(a_children), Some(b_children)) => {
            if a_children.len() != b_children.len() {
                return Err(AjisaiError::VectorLengthMismatch {
                    len1: a_children.len(),
                    len2: b_children.len(),
                });
            }
            let out = a_children
                .iter()
                .zip(b_children.iter())
                .map(|(x, y)| apply_exact_real_recursive_broadcast(x, y, schema))
                .collect::<Result<Vec<Value>>>()?;
            Ok(Value::from_children(out))
        }
    }
}

/// Exact-real value of a broadcast leaf (`Scalar`, `ExactScalar`, or `NIL`).
/// `None` for non-numeric leaves, matching `tensor_ops::broadcast_leaf`.
fn exact_broadcast_leaf(value: &Value) -> Option<ExactReal> {
    match &value.data {
        ValueData::Scalar(f) => Some(ExactReal::from_fraction(f.clone())),
        ValueData::ExactScalar(er) => Some(er.clone()),
        ValueData::Nil => Some(ExactReal::from_fraction(Fraction::nil())),
        _ => None,
    }
}

/// Extract aligned exact-real lanes for the *homogeneous flat* broadcast case:
/// both operands are equal-length vectors whose every child is a numeric leaf
/// (no nesting, no ragged shapes). Returns `None` for any shape outside that
/// subset (scalar-broadcast, nesting, length mismatch, empty) so the caller
/// falls back to the sequential recursion, which still handles those exactly.
///
/// This is the only shape eligible for the parallel kernel: it is the
/// 戦局1「均質」case where every lane is an independent, equally-sized,
/// compute-bound Gosper evaluation, so disjoint index ranges fan out cleanly.
fn exact_flat_leaf_lanes(a: &Value, b: &Value) -> Option<(Vec<ExactReal>, Vec<ExactReal>)> {
    use crate::interpreter::tensor_ops::broadcast_children;

    let (a_children, b_children) = (broadcast_children(a)?, broadcast_children(b)?);
    if a_children.is_empty() || a_children.len() != b_children.len() {
        return None;
    }
    let mut a_lanes = Vec::with_capacity(a_children.len());
    let mut b_lanes = Vec::with_capacity(b_children.len());
    for (x, y) in a_children.iter().zip(b_children.iter()) {
        // Any nested child must take the recursive path, not the flat kernel.
        if broadcast_children(x).is_some() || broadcast_children(y).is_some() {
            return None;
        }
        a_lanes.push(exact_broadcast_leaf(x)?);
        b_lanes.push(exact_broadcast_leaf(y)?);
    }
    Some((a_lanes, b_lanes))
}

/// Structural broadcast for operands containing irrational `ExactScalar`
/// lanes. Returns `Ok(false)` (leaving the stack untouched) for the cases the
/// caller still routes elsewhere — Stack target mode and top-level NIL — so the
/// existing NIL-passthrough and reduction paths keep their behavior.
fn push_exact_real_broadcast_result(
    interp: &mut Interpreter,
    schema: ExactArithmeticSchema,
    a: &Value,
    b: &Value,
) -> Result<bool> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Ok(false);
    }
    if a.is_nil() || b.is_nil() {
        return Ok(false);
    }
    if !value_contains_exact_scalar(a) && !value_contains_exact_scalar(b) {
        return Ok(false);
    }
    // Homogeneous flat case (equal-length vectors of numeric leaves): each lane
    // is an independent compute-bound exact-real op, so fan it out across the
    // native pool. `Value` is not `Send`, so the kernel computes `Send`
    // `Option<ExactReal>` lanes (`None` = division-by-zero bubble) and we rebuild
    // `Value`s here. The result is identical to the sequential recursion for
    // this shape, lane for lane.
    let result = if let Some((a_lanes, b_lanes)) = exact_flat_leaf_lanes(a, b) {
        let n = a_lanes.len();
        let lanes: Vec<Option<ExactReal>> =
            crate::interpreter::parallel::compute_bound_map(n, |i| {
                schema.exact_real(&a_lanes[i], &b_lanes[i])
            });
        interp.runtime_metrics.exact_real_parallel_broadcast_count = interp
            .runtime_metrics
            .exact_real_parallel_broadcast_count
            .saturating_add(1);
        let children: Vec<Value> = lanes
            .into_iter()
            .map(|lane| match lane {
                Some(result) => Value::from_exact_real(result),
                None => division_by_zero_bubble(),
            })
            .collect();
        Value::from_children(children)
    } else {
        apply_exact_real_recursive_broadcast(a, b, schema)?
    };
    consume_stacktop_binary(interp);
    push_result(interp, result);
    Ok(true)
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
                interp.stack[stack_len - count..].to_vec()
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
