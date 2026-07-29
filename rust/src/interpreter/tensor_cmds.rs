use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::{
    create_number_value, nil_passthrough_binary, nil_passthrough_unary,
};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
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
    interp.stack.set_role_at(stack_len - 1, Interpretation::Nil);
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

/// The exact-real value carried by a scalar, whether stored as a rational
/// `Scalar` or an `ExactScalar`. Non-scalar and NIL values return `None`.
fn value_as_exact_real(value: &Value) -> Option<ExactReal> {
    match &value.data {
        ValueData::Scalar(f) => Some(ExactReal::from_fraction(f.clone())),
        ValueData::ExactScalar(er) => Some(er.clone()),
        _ => None,
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
    // CS5: cap sourced from the injectable per-interpreter `RuntimeLimits`
    // (folded), so tests can fire this guard with a tiny limit; same behavior
    // and message as before.
    let max_materialized = interp.runtime_limits.max_materialized_elements;
    let total_size = match checked_shape_product(&shape) {
        Some(size) if size <= max_materialized => size,
        _ => {
            // Phase 3 (structural-memory-safety roadmap): a well-formed shape
            // whose element product exceeds the space water level (or overflows
            // `usize`) is a well-formed operation that cannot materialize within
            // budget. The Bubble Rule projects it onto a diagnosable NIL (reason
            // `spaceExhausted`), recoverable with `^` (VENT), instead of a
            // channel error. Under KEEP the operands are retained as on the
            // success path.
            if interp.consumption_mode == ConsumptionMode::Keep {
                interp.stack.push(args_val);
            }
            interp
                .stack
                .push(Value::nil_with_reason(NilReason::SpaceExhausted));
            return Ok(());
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
