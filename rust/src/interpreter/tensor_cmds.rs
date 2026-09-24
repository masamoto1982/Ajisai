use crate::error::{AjisaiError, Result};
use crate::interpreter::record_lift;
use crate::interpreter::value_extraction_helpers::{create_number_value, nil_passthrough_unary};
use crate::interpreter::Interpreter;
use crate::types::exact::ExactReal;
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};

/// Multiply dimension sizes without ever overflowing `usize`. Returns `None`
/// when the running product would wrap, so callers can reject pathological
/// shapes with a structured error instead of panicking (debug) or silently
/// computing a wrong size (release).
pub(super) fn checked_shape_product(shape: &[usize]) -> Option<usize> {
    shape
        .iter()
        .try_fold(1usize, |acc, &dim| acc.checked_mul(dim))
}

use super::tensor_ops::{apply_unary_flat_with_metrics, build_nested_value};

fn apply_unary_math<F, G>(interp: &mut Interpreter, op: F, exact_op: G) -> Result<()>
where
    F: Fn(&Fraction) -> Fraction + Copy,
    G: Fn(&ExactReal) -> ExactReal,
{
    if nil_passthrough_unary(interp) {
        return Ok(());
    }

    let val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    if val.is_nil() {
        let got = val.domain_name();
        interp.stack.push(val);
        return Err(AjisaiError::declared(
            "nonNumeric",
            format!("expected a Scalar or a Vector, got {got}"),
        ));
    }

    if val.is_scalar() {
        if let Some(f) = val.as_scalar() {
            let result: Fraction = op(f);
            interp.stack.push(create_number_value(result));
            return Ok(());
        }
    }

    // ExactScalar path: an algebraic irrational, whose floor and rounding
    // are decidable (LANG.VALUES.EXACT).
    if let ValueData::ExactScalar(er) = &val.data {
        interp.stack.push(Value::from_exact_real(exact_op(er)));
        return Ok(());
    }

    // A NIL lane passes through carrying its reason (LANG.FAILURE.PASSTHROUGH).
    // The flat route below works on bare fractions and would keep the lane
    // absent but drop why, so a vector holding one takes the lane-wise route.
    if val.is_vector() && holds_nil_lane(&val) {
        let scalar_op = |lane: &Value| -> Result<Value> {
            if let Some(f) = lane.as_scalar() {
                return Ok(create_number_value(op(f)));
            }
            if let ValueData::ExactScalar(er) = &lane.data {
                return Ok(Value::from_exact_real(exact_op(er)));
            }
            Err(AjisaiError::declared(
                "nonNumeric",
                format!("expected a Scalar or a Vector, got {}", lane.domain_name()),
            ))
        };
        return match crate::interpreter::math_ops::lift_unary_numeric(&val, &scalar_op) {
            Ok(result) => {
                interp.stack.push(result);
                Ok(())
            }
            Err(e) => {
                interp.stack.push(val);
                Err(e)
            }
        };
    }

    if val.is_vector() {
        match apply_unary_flat_with_metrics(&val, op, Some(&mut interp.runtime_metrics)) {
            Ok(result) => {
                interp.stack.push(result);
                return Ok(());
            }
            Err(_) => {
                interp.stack.push(val);
                return Err(AjisaiError::declared(
                    "nonNumeric",
                    "expected a Scalar or a Vector of Scalars",
                ));
            }
        }
    }

    let got = val.domain_name();
    interp.stack.push(val);
    Err(AjisaiError::declared(
        "nonNumeric",
        format!("expected a Scalar or a Vector, got {got}"),
    ))
}

fn holds_nil_lane(value: &Value) -> bool {
    match value.as_vector_view() {
        Some(items) => items.iter().any(holds_nil_lane),
        None => value.is_nil(),
    }
}

pub fn op_floor(interp: &mut Interpreter) -> Result<()> {
    if record_lift::lift_unary(interp, &op_floor)? {
        return Ok(());
    }
    apply_unary_math(
        interp,
        |f| f.floor(),
        |er| er.floor().expect("a number has a floor"),
    )
}

pub fn op_round(interp: &mut Interpreter) -> Result<()> {
    if record_lift::lift_unary(interp, &op_round)? {
        return Ok(());
    }
    apply_unary_math(
        interp,
        |f| f.round(),
        |er| er.round().expect("a number has a nearest integer"),
    )
}

pub fn op_fill(interp: &mut Interpreter) -> Result<()> {
    let args_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    if args_val.is_nil() {
        interp.stack.push(args_val);
        return Err(AjisaiError::declared(
            "invalidShape",
            "expected a [ shape... value ] vector, got NIL",
        ));
    }

    let n = args_val.len();

    if n < 2 {
        interp.stack.push(args_val);
        return Err(AjisaiError::declared(
            "invalidShape",
            format!(
                "expected a [ shape... value ] Vector of at least 2 elements, got {} element(s)",
                n
            ),
        ));
    }

    let fill_value = match args_val.child(n - 1).and_then(|v| v.as_scalar().cloned()) {
        Some(f) => f,
        None => {
            interp.stack.push(args_val);
            return Err(AjisaiError::declared(
                "invalidShape",
                "expected a Scalar as the last element of [ shape... value ]",
            ));
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
                return Err(AjisaiError::declared(
                    "invalidShape",
                    "expected positive integer dimensions, got invalid dimension",
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
            // budget. The NIL Projection Rule projects it onto a diagnosable NIL
            // (reason `spaceExhausted`), recoverable with a chosen fallback, instead of
            // a channel error.
            interp
                .stack
                .push(crate::interpreter::space_projection::space_exhausted_nil(
                    "FILL",
                    max_materialized,
                    checked_shape_product(&shape).map(|size| size as u128),
                ));
            return Ok(());
        }
    };
    if let Err(e) = crate::interpreter::collection_meter::charge_materialization(interp, total_size)
    {
        interp.stack.push(args_val);
        return Err(e);
    }

    let data: Vec<Fraction> = (0..total_size).map(|_| fill_value.clone()).collect();

    let result = build_nested_value(&data, &shape);

    interp.stack.push(result);
    Ok(())
}
