use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::{
    create_number_value, nil_passthrough_binary, nil_passthrough_unary,
};
use crate::interpreter::{
    ConsumptionMode, Interpreter, OperationTargetMode, MAX_MATERIALIZED_ELEMENTS,
};
use crate::types::exact::ExactReal;
use crate::types::fraction::{Fraction, RoundingMode};
use crate::types::{Interpretation, Value, ValueData};

/// Multiply dimension sizes without ever overflowing `usize`. Returns `None`
/// when the running product would wrap, so callers can reject pathological
/// shapes with a structured error instead of panicking (debug) or silently
/// computing a wrong size (release).
fn checked_shape_product(shape: &[usize]) -> Option<usize> {
    shape
        .iter()
        .try_fold(1usize, |acc, &dim| acc.checked_mul(dim))
}

/// Push a SPEC §7.4.1 Undecidable NIL. Used when an exact-real (CF)
/// arithmetic word cannot resolve its result within the partial-quotient
/// budget; the Bubble Rule (SPEC §11.2) places NIL on the stack instead
/// of raising an error, matching the comparison-budget exhaustion path.
fn push_undecidable_nil(interp: &mut Interpreter) {
    interp
        .stack
        .push(Value::nil_with_reason(NilReason::Undecidable));
    let stack_len = interp.stack.len();
    interp.semantic_registry.normalize_to_stack_len(stack_len);
    interp
        .semantic_registry
        .update_hint_at(stack_len - 1, Interpretation::Nil);
}

use super::tensor_ops::{
    apply_binary_broadcast_with_metrics, apply_unary_flat_with_metrics, build_nested_value,
    FlatTensor,
};

fn apply_tensor_metadata(
    interp: &mut Interpreter,
    word: &str,
    mapper: fn(&Value) -> Value,
) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: word.into(),
            mode: "Stack".into(),
        });
    }

    let is_keep_mode: bool = interp.consumption_mode == ConsumptionMode::Keep;
    let value: Value = if is_keep_mode {
        interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    let result: Value = mapper(&value);
    interp.stack.push(result);
    Ok(())
}

fn compute_shape_of_value(value: &Value) -> Value {
    if value.is_nil() {
        return Value::nil();
    }

    if !value.is_vector() {
        return Value::from_vector(vec![]);
    }

    let shape_values: Vec<Value> = value
        .shape()
        .iter()
        .map(|&n| Value::from_number(Fraction::from(n as i64)))
        .collect();
    Value::from_vector(shape_values)
}

fn compute_rank_of_value(value: &Value) -> Value {
    if value.is_nil() {
        return Value::nil();
    }

    let rank: i64 = if value.is_vector() {
        value.shape().len() as i64
    } else {
        0
    };
    create_number_value(Fraction::from(rank))
}

pub fn op_shape(interp: &mut Interpreter) -> Result<()> {
    apply_tensor_metadata(interp, "SHAPE", compute_shape_of_value)
}

pub fn op_rank(interp: &mut Interpreter) -> Result<()> {
    apply_tensor_metadata(interp, "RANK", compute_rank_of_value)
}

pub fn op_reshape(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: "RESHAPE".into(),
            mode: "Stack".into(),
        });
    }

    let shape_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let data_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    if !shape_val.is_vector() && !shape_val.is_nil() {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
        return Err(AjisaiError::from("RESHAPE requires shape as vector"));
    }

    let dim_count: usize = shape_val.len();

    let mut new_shape: Vec<usize> = Vec::with_capacity(dim_count);
    for i in 0..dim_count {
        let dim_child = shape_val
            .child(i)
            .expect("RESHAPE: child index in 0..len must be valid");
        let dim = match dim_child.as_scalar().and_then(|f| f.as_usize()) {
            Some(d) => d,
            None => {
                interp.stack.push(data_val);
                interp.stack.push(shape_val);
                return Err(AjisaiError::from(
                    "Shape dimensions must be positive integers",
                ));
            }
        };
        new_shape.push(dim);
    }

    if data_val.is_nil() {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
        return Err(AjisaiError::from("RESHAPE requires data as vector"));
    }

    let input_tensor: FlatTensor = match FlatTensor::from_value(&data_val) {
        Ok(t) => t,
        Err(err) => {
            interp.stack.push(data_val);
            interp.stack.push(shape_val);
            return Err(err);
        }
    };

    // A pathological shape such as `[ 99999999 99999999 99999999 ]` overflows
    // the size product and panics under `iter().product()`. Compute it with
    // overflow protection: an overflowing product can never equal the (bounded)
    // input length, so fall through to the existing mismatch error.
    let required_size: usize = if new_shape.is_empty() {
        1
    } else {
        match checked_shape_product(&new_shape) {
            Some(size) => size,
            None => {
                interp.stack.push(data_val);
                interp.stack.push(shape_val);
                return Err(AjisaiError::from(format!(
                    "RESHAPE failed: shape {:?} is too large to materialize",
                    new_shape
                )));
            }
        }
    };
    if input_tensor.data.len() != required_size {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
        return Err(AjisaiError::from(format!(
            "RESHAPE failed: data length {} doesn't match shape {:?} (requires {})",
            input_tensor.data.len(),
            new_shape,
            required_size
        )));
    }

    let result_tensor: FlatTensor = FlatTensor::from_shape_and_data(new_shape, input_tensor.data)?;

    if interp.consumption_mode == ConsumptionMode::Keep {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
    }

    interp.stack.push(result_tensor.to_value());
    Ok(())
}

pub fn op_transpose(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: "TRANSPOSE".into(),
            mode: "Stack".into(),
        });
    }

    let val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    if val.is_nil() {
        interp.stack.push(Value::nil());
        return Ok(());
    }

    let tensor: FlatTensor = match FlatTensor::from_value(&val) {
        Ok(t) => t,
        Err(err) => {
            interp.stack.push(val);
            return Err(err);
        }
    };

    if tensor.shape.len() != 2 {
        interp.stack.push(val);
        return Err(AjisaiError::from("TRANSPOSE requires 2D vector"));
    }

    let rows: usize = tensor.shape[0];
    let cols: usize = tensor.shape[1];

    let mut transposed: Vec<Fraction> = Vec::with_capacity(tensor.data.len());
    for j in 0..cols {
        for i in 0..rows {
            transposed.push(tensor.data[i * cols + j].clone());
        }
    }

    let result_tensor: FlatTensor = FlatTensor::from_shape_and_data(vec![cols, rows], transposed)?;

    if interp.consumption_mode == ConsumptionMode::Keep {
        interp.stack.push(val);
    }

    interp.stack.push(result_tensor.to_value());
    Ok(())
}

fn apply_unary_math<F, G>(interp: &mut Interpreter, op: F, exact_op: G, op_name: &str) -> Result<()>
where
    F: Fn(&Fraction) -> Fraction + Copy,
    G: Fn(&ExactReal) -> Option<ExactReal>,
{
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: op_name.to_string(),
            mode: "Stack".into(),
        });
    }

    if nil_passthrough_unary(interp) {
        return Ok(());
    }

    let is_keep_mode: bool = interp.consumption_mode == ConsumptionMode::Keep;

    let val: Value = if is_keep_mode {
        interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    if val.is_nil() {
        if !is_keep_mode {
            interp.stack.push(val);
        }
        return Err(AjisaiError::from(format!(
            "{} requires number or vector",
            op_name
        )));
    }

    if val.is_scalar() {
        if let Some(f) = val.as_scalar() {
            let result: Fraction = op(f);
            interp.stack.push(create_number_value(result));
            return Ok(());
        }
    }

    // ExactScalar path: exact irrational via CF (SPEC §4.2.2). When the
    // CF stream exhausts its partial-quotient budget the result is
    // undecidable, so project to a Bubble NIL (SPEC §7.4.1, §11.2)
    // instead of raising an error — matching the comparison-budget path.
    if let ValueData::ExactScalar(er) = &val.data {
        match exact_op(er) {
            Some(result) => interp.stack.push(Value::from_exact_real(result)),
            None => push_undecidable_nil(interp),
        }
        return Ok(());
    }

    if val.is_vector() {
        match apply_unary_flat_with_metrics(&val, op, Some(&mut interp.runtime_metrics)) {
            Ok(result) => {
                interp.stack.push(result);
                return Ok(());
            }
            Err(_) => {
                if !is_keep_mode {
                    interp.stack.push(val);
                }
                return Err(AjisaiError::from(format!(
                    "{} requires number or vector",
                    op_name
                )));
            }
        }
    }

    if !is_keep_mode {
        interp.stack.push(val);
    }
    Err(AjisaiError::from(format!(
        "{} requires number or vector",
        op_name
    )))
}

pub fn op_floor(interp: &mut Interpreter) -> Result<()> {
    apply_unary_math(interp, |f| f.floor(), |er| er.floor(), "FLOOR")
}

pub fn op_ceil(interp: &mut Interpreter) -> Result<()> {
    apply_unary_math(interp, |f| f.ceil(), |er| er.ceil(), "CEIL")
}

pub fn op_round(interp: &mut Interpreter) -> Result<()> {
    apply_unary_math(interp, |f| f.round(), |er| er.round(), "ROUND")
}

/// The exact rational carried by a scalar value, whether it is stored as a
/// plain `Scalar` or as an `ExactScalar` that happens to reduce to a rational.
/// Genuinely irrational exact-reals return `None`.
fn scalar_as_rational(value: &Value) -> Option<Fraction> {
    match &value.data {
        ValueData::Scalar(f) => Some(f.clone()),
        ValueData::ExactScalar(er) => er.to_fraction(),
        _ => None,
    }
}

/// Shared engine for the `QUANTIZE` family (SPEC §7.13; see
/// docs/dev/fintech-value-integrity-design.md). Stack effect
/// `[ x ] [ step ] -> [ q ] [ r ]` where `q` is the multiple of `step` chosen
/// by `mode` and `r = x - q`, so `q + r == x` exactly. `word` names the calling
/// word for diagnostics. The rounding rule is the only thing that varies across
/// the family; NIL/error/decidability handling is identical.
fn quantize_with_mode(interp: &mut Interpreter, mode: RoundingMode, word: &str) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: word.into(),
            mode: "Stack".into(),
        });
    }

    let stack_len = interp.stack.len();
    if stack_len < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    // Operand order is `[ x ] [ step ] <word>`, so `x` is second from top.
    let x_val = interp.stack[stack_len - 2].clone();
    let step_val = interp.stack[stack_len - 1].clone();
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;

    // The step is mandatory and must be a strictly positive rational. A NIL or
    // otherwise malformed step is a channel error (RejectsNil), matching the
    // deliberate asymmetry of MOD-by-zero rather than producing a bubble.
    let step = match scalar_as_rational(&step_val) {
        Some(s) if s.is_positive() => s,
        _ => {
            return Err(AjisaiError::from(format!(
                "{} requires a strictly positive rational step",
                word
            )));
        }
    };

    let pop_operands = |interp: &mut Interpreter| {
        if !is_keep {
            interp.stack.pop();
            interp.stack.pop();
        }
    };

    // NIL-passthrough on `x`: a bubble flows through to both outputs, carrying
    // its reason (SPEC §7.12). The step was already validated above, so a NIL
    // step never reaches this point.
    if x_val.is_operational_nil() {
        pop_operands(interp);
        interp
            .stack
            .push(Value::nil_inheriting_absence_from(&x_val));
        interp
            .stack
            .push(Value::nil_inheriting_absence_from(&x_val));
        return Ok(());
    }

    match scalar_as_rational(&x_val) {
        Some(x) => {
            let (q, r) = x.quantize(&step, mode);
            pop_operands(interp);
            interp.stack.push(create_number_value(q));
            interp.stack.push(create_number_value(r));
            Ok(())
        }
        None => {
            // A genuinely irrational `x` cannot pick a multiple within the
            // comparison budget for the round-to-nearest modes, so project both
            // outputs to an Undecidable bubble rather than guess (SPEC §7.4.1,
            // §11.2). The directed modes take the same conservative path.
            pop_operands(interp);
            interp
                .stack
                .push(Value::nil_with_reason(NilReason::Undecidable));
            interp
                .stack
                .push(Value::nil_with_reason(NilReason::Undecidable));
            Ok(())
        }
    }
}

/// `QUANTIZE` — banker's rounding (ties to even), the currency-safe default.
pub fn op_quantize(interp: &mut Interpreter) -> Result<()> {
    quantize_with_mode(interp, RoundingMode::HalfEven, "QUANTIZE")
}

/// `QUANTIZE-HALF-AWAY` — nearest, ties away from zero (the `ROUND` rule).
pub fn op_quantize_half_away(interp: &mut Interpreter) -> Result<()> {
    quantize_with_mode(interp, RoundingMode::HalfAway, "QUANTIZE-HALF-AWAY")
}

/// `QUANTIZE-FLOOR` — toward negative infinity (the `FLOOR` rule).
pub fn op_quantize_floor(interp: &mut Interpreter) -> Result<()> {
    quantize_with_mode(interp, RoundingMode::Floor, "QUANTIZE-FLOOR")
}

/// `QUANTIZE-CEIL` — toward positive infinity (the `CEIL` rule).
pub fn op_quantize_ceil(interp: &mut Interpreter) -> Result<()> {
    quantize_with_mode(interp, RoundingMode::Ceil, "QUANTIZE-CEIL")
}

/// `QUANTIZE-TRUNC` — toward zero (truncation).
pub fn op_quantize_trunc(interp: &mut Interpreter) -> Result<()> {
    quantize_with_mode(interp, RoundingMode::Trunc, "QUANTIZE-TRUNC")
}

/// The exact-real value carried by a scalar, whether stored as a rational
/// `Scalar` or an `ExactScalar`. Non-scalar and NIL values return `None`.
fn value_as_exact_real(value: &Value) -> Option<ExactReal> {
    match &value.data {
        ValueData::Scalar(f) => Some(ExactReal::from_fraction(f.clone())),
        ValueData::ExactScalar(er) => Some(er.clone()),
        _ => None,
    }
}

/// `CONSERVE` — value-conservation guard (SPEC §13.3 draft; see
/// docs/dev/fintech-value-integrity-design.md). Stack effect
/// `[ total ] [ parts ] -> [ parts ]`: asserts that the exact sum of the
/// scalar vector `parts` equals `total`. On a proven match `parts` passes
/// through unchanged; otherwise evaluation halts with a channel error (fail
/// loudly). This is the value-mass complement to the static flow-mass
/// conservation of §13.1.
pub fn op_conserve(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: "CONSERVE".into(),
            mode: "Stack".into(),
        });
    }

    let stack_len = interp.stack.len();
    if stack_len < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    // Operand order is `[ total ] [ parts ] CONSERVE`, so `total` is second.
    let total_val = interp.stack[stack_len - 2].clone();
    let parts_val = interp.stack[stack_len - 1].clone();
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;

    // RejectsNil: an absent total or an absent part cannot certify
    // conservation ("no lost data"), so a NIL operand is a channel error
    // rather than a bubble that could flow downstream.
    if total_val.is_nil() || parts_val.is_nil() {
        return Err(AjisaiError::from(
            "CONSERVE cannot certify conservation with a NIL operand",
        ));
    }

    let total = value_as_exact_real(&total_val)
        .ok_or_else(|| AjisaiError::from("CONSERVE requires a scalar total"))?;
    let parts = parts_val
        .as_vector_view()
        .ok_or_else(|| AjisaiError::from("CONSERVE requires a vector of scalar parts"))?;

    // Sum the parts exactly. Seed the accumulator with the first element (not
    // a synthetic zero) so a single-part sum is that element unchanged, which
    // keeps rational sums exact and avoids wrapping an exact-real part in an
    // extra Gosper transform that a budgeted comparison might not see through.
    // A NIL or non-scalar element is malformed use.
    let scalar_at = |element: &Value| -> Result<ExactReal> {
        if element.is_nil() {
            return Err(AjisaiError::from(
                "CONSERVE parts must not contain a NIL element",
            ));
        }
        value_as_exact_real(element)
            .ok_or_else(|| AjisaiError::from("CONSERVE parts must be scalars"))
    };
    let mut elements = parts.iter();
    let mut sum = match elements.next() {
        Some(first) => scalar_at(first)?,
        None => ExactReal::from_fraction(Fraction::from(0i64)),
    };
    for element in elements {
        sum = sum.add(&scalar_at(element)?);
    }

    // Only a *proven* equality lets flow pass. Over Tier ≤ 1 the equality
    // is decidable, so the guard always confirms or refutes; the `None`
    // arm remains for an operand outside the decidable domain (a guard
    // that cannot confirm its safe condition must not pass — it fails
    // loudly rather than returning UNKNOWN, SPEC §13.3 draft).
    match sum.cmp_exact(&total) {
        crate::types::exact::ExactCmp::Decided(std::cmp::Ordering::Equal) => {
            if !is_keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            interp.stack.push(parts_val);
            Ok(())
        }
        crate::types::exact::ExactCmp::Decided(_) => Err(AjisaiError::from(
            "Conservation violated: parts do not sum to the total",
        )),
        crate::types::exact::ExactCmp::Starved { .. } | crate::types::exact::ExactCmp::Absent => {
            Err(AjisaiError::from(
                "Conservation undecidable within the comparison budget",
            ))
        }
    }
}

pub fn op_mod(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: "MOD".into(),
            mode: "Stack".into(),
        });
    }

    if nil_passthrough_binary(interp) {
        return Ok(());
    }

    // ExactScalar path: a mod b = a - b * floor(a/b), exact over Tier 1
    if interp.operation_target_mode == OperationTargetMode::StackTop && interp.stack.len() >= 2 {
        let stack_len = interp.stack.len();
        let a_ref = &interp.stack[stack_len - 2];
        let b_ref = &interp.stack[stack_len - 1];
        let has_exact = matches!(&a_ref.data, ValueData::ExactScalar(_))
            || matches!(&b_ref.data, ValueData::ExactScalar(_));
        if has_exact {
            let a_er = match &a_ref.data {
                ValueData::Scalar(f) => Some(ExactReal::from_fraction(f.clone())),
                ValueData::ExactScalar(er) => Some(er.clone()),
                _ => None,
            };
            let b_er = match &b_ref.data {
                ValueData::Scalar(f) => Some(ExactReal::from_fraction(f.clone())),
                ValueData::ExactScalar(er) => Some(er.clone()),
                _ => None,
            };
            if let (Some(a), Some(b)) = (a_er, b_er) {
                // Zero-ness of the divisor is decidable on the normal
                // form: a Tier 1 algebraic is never zero, and a rational
                // shows it structurally.
                if b.is_structurally_zero() {
                    return Err(AjisaiError::from("Modulo by zero"));
                }
                // a mod b = a - b * floor(a/b). A `None` here (after the
                // zero check) means an absent operand slipped through:
                // project to a Bubble NIL rather than erroring.
                let modulo = a
                    .div(&b)
                    .and_then(|q| q.floor())
                    .map(|fl| a.sub(&b.mul(&fl)));
                if interp.consumption_mode != ConsumptionMode::Keep {
                    interp.stack.pop();
                    interp.stack.pop();
                }
                match modulo {
                    Some(result) => interp.stack.push(Value::from_exact_real(result)),
                    None => push_undecidable_nil(interp),
                }
                return Ok(());
            }
        }
    }

    let is_keep_mode: bool = interp.consumption_mode == ConsumptionMode::Keep;

    let b_val: Value = if is_keep_mode {
        interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    let a_val = if is_keep_mode {
        let stack_len = interp.stack.len();
        if stack_len < 2 {
            return Err(AjisaiError::StackUnderflow);
        }
        interp.stack[stack_len - 2].clone()
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    let result = apply_binary_broadcast_with_metrics(
        &a_val,
        &b_val,
        |x, y| {
            if y.is_zero() {
                Err(AjisaiError::from("Modulo by zero"))
            } else {
                Ok(x.modulo(y))
            }
        },
        Some(&mut interp.runtime_metrics),
    );

    match result {
        Ok(r) => {
            interp.stack.push(r);
            Ok(())
        }
        Err(e) => {
            if !is_keep_mode {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
            }
            Err(e)
        }
    }
}

pub fn op_fill(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: "FILL".into(),
            mode: "Stack".into(),
        });
    }

    let args_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    if args_val.is_nil() {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("FILL requires [shape... value] vector"));
    }

    let n = args_val.len();

    if n < 2 {
        interp.stack.push(args_val);
        return Err(AjisaiError::from(
            "FILL requires [shape... value] (at least 2 elements)",
        ));
    }

    let fill_value = match args_val.child(n - 1).and_then(|v| v.as_scalar().cloned()) {
        Some(f) => f,
        None => {
            interp.stack.push(args_val);
            return Err(AjisaiError::from("FILL value must be a scalar"));
        }
    };

    let shape_len = n - 1;

    let mut shape = Vec::with_capacity(shape_len);
    for i in 0..shape_len {
        let dim_child = args_val
            .child(i)
            .expect("FILL: child index in 0..len must be valid");
        let dim = match dim_child.as_scalar().and_then(|f| f.as_usize()) {
            Some(d) if d > 0 => d,
            Some(_) | None => {
                interp.stack.push(args_val);
                return Err(AjisaiError::from(
                    "RESHAPE: expected positive integer dimensions, got invalid dimension",
                ));
            }
        };
        shape.push(dim);
    }

    // Compute the element count with overflow protection and reject anything
    // beyond the materialization cap before allocating. `shape.iter().product()`
    // would otherwise panic on a usize overflow (e.g. three ~1e8 dimensions) or
    // drive an OOM abort for a merely large product — neither is recoverable in
    // the WASM playground.
    let total_size = match checked_shape_product(&shape) {
        Some(size) if size <= MAX_MATERIALIZED_ELEMENTS => size,
        _ => {
            interp.stack.push(args_val);
            return Err(AjisaiError::from(format!(
                "FILL shape {:?} would generate too many elements (limit {})",
                shape, MAX_MATERIALIZED_ELEMENTS
            )));
        }
    };
    let data: Vec<Fraction> = (0..total_size).map(|_| fill_value.clone()).collect();

    let result = build_nested_value(&data, &shape);

    if interp.consumption_mode == ConsumptionMode::Keep {
        interp.stack.push(args_val);
    }

    interp.stack.push(result);
    Ok(())
}
