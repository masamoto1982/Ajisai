//! `DIV`'s projection law: what a zero divisor does to the value around it.
//!
//! Split out of `arithmetic.rs` because `DIV` is the one exact-arithmetic
//! schema whose scalar law can *project* — answer NIL for a well-formed
//! operand (`LANG.FAILURE.TRICHOTOMY`) — and lifting a projecting law over a
//! collection is a different problem from lifting a total one. Every other
//! schema either answers with a number in every lane or raises.

use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::arithmetic::{ExactArithmeticSchema, ScalarFastWrap};
use crate::interpreter::arithmetic_meter::check_result_size;
use crate::interpreter::tensor_lane_ops::apply_lane_wise_broadcast;
use crate::interpreter::tensor_ops::apply_binary_broadcast_with_metrics;
use crate::interpreter::value_extraction_helpers::{extract_operands, push_result};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::semantic::Recoverability;
use crate::types::fraction::Fraction;
use crate::types::Value;

pub(crate) fn division_by_zero_projection() -> Value {
    Value::nil_with_reason(NilReason::DivisionByZero, Recoverability::Recoverable)
}

/// The scalar law of `DIV` as a whole `Value`, for the lane-wise lift.
///
/// A zero divisor is a projection, not a failure (`LANG.FAILURE.TRICHOTOMY`),
/// so it answers with the reasoned NIL the scalar `6 0 /` answers with. A NIL
/// operand is ordinary passthrough and carries no reason of its own — the
/// operand was already absent before `DIV` saw it. The NIL test comes first
/// because `Fraction::nil` has numerator 0, so an absent divisor answers
/// `is_zero` as well.
fn divide_lane(a: &Fraction, b: &Fraction) -> Result<Value> {
    if a.is_nil() || b.is_nil() {
        return Ok(Value::nil());
    }
    if b.is_zero() {
        return Ok(division_by_zero_projection());
    }
    Ok(Value::from_fraction(a.div(b)))
}

/// A zero divisor on the one-lane fast path projects *inside* the operand's
/// wrap, for the same reason it projects per lane in the broadcast: the shape
/// of `[ 6 ] [ 0 ] /` is the shape of `[ 6 ] [ 2 ] /`. Answering with a bare
/// NIL here made `DIV` the one Word whose result shape depended on whether it
/// projected — `[ 6 ] [ 2 ] /` gave `[ 3/1 ]` while `[ 6 ] [ 0 ] /` gave a
/// scalar `NIL`.
///
/// The projection is a reasoned NIL, so the wrap is rebuilt as a nested
/// `Vector`: a dense lane could hold the absence but not the reason for it.
pub(crate) fn build_scalar_fast_projection(wrap: &ScalarFastWrap) -> Value {
    match wrap {
        ScalarFastWrap::Scalar => division_by_zero_projection(),
        ScalarFastWrap::Tensor(shape) => {
            let mut value = division_by_zero_projection();
            for _ in shape {
                value = Value::from_children(vec![value]);
            }
            value
        }
    }
}

/// The `DIV` arm of [`apply_exact_arithmetic_schema`], after the fast paths
/// declined it.
///
/// [`apply_exact_arithmetic_schema`]: crate::interpreter::arithmetic
pub(crate) fn apply_division_schema(
    interp: &mut Interpreter,
    schema: ExactArithmeticSchema,
) -> Result<()> {
    let stack_len = interp.stack.len();
    if stack_len >= 2 {
        let slots = interp.stack.as_slice();
        let left_is_text = slots[stack_len - 2].is_text();
        let right_is_text = slots[stack_len - 1].is_text();
        if left_is_text || right_is_text {
            return Err(AjisaiError::create_structure_error("number", "string"));
        }
    }
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let operands = extract_operands(interp, 2)?;
    let a_val = &operands[0];
    let b_val = &operands[1];

    let computed = apply_binary_broadcast_with_metrics(
        a_val,
        b_val,
        |a, b| schema.fraction(a, b),
        Some(&mut interp.runtime_metrics),
    )
    // Bound accumulation before the result is pushed; on a refusal the
    // operands go back, exactly as for any other failure of this arm.
    .and_then(|result| {
        check_result_size(interp, &result)?;
        Ok(result)
    });

    // `LANG.COLLECTIONS.LIFT`: "Each lane preserves the exactness, truth, NIL,
    // and ERROR distinctions of the scalar law." A zero divisor empties its own
    // lane; it does not empty the vector.
    //
    // The flat rational broadcast above cannot say that. Its leaf law answers
    // with a `Fraction`, so a projection can only surface as one error for the
    // whole operation, and the lanes that had already divided were discarded
    // with it: `[ 6 6 6 ] [ 1 2 0 ] /` answered `NIL` where the same division
    // through `MAP` answered `[ 6/1 3/1 NIL ]`, so one `DIV` meant two
    // different things depending on the route it took.
    //
    // Re-run it lane-wise, where the leaf law answers with a value and each
    // projection carries its own reason — the shape `SQRT` already produces for
    // a negative lane. The re-run costs a second pass only when a zero divisor
    // was actually met; a division that projects nothing keeps the flat path.
    let computed = match computed {
        Err(AjisaiError::DivisionByZero) => apply_lane_wise_broadcast(a_val, b_val, divide_lane)
            .and_then(|result| {
                check_result_size(interp, &result)?;
                Ok(result)
            }),
        other => other,
    };

    match computed {
        Ok(result) => {
            push_result(interp, result);
            Ok(())
        }
        Err(error) => {
            if !is_keep_mode {
                for val in operands {
                    interp.stack.push(val);
                }
            }
            Err(error)
        }
    }
}
