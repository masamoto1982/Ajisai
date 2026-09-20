//! `QUANTIZE`: the scaled-round-unscale derivation, run on the stack so that
//! broadcasting, exact-real handling and the NIL contract are the ones `MUL`,
//! `ROUND` and `DIV` already define. Split out of `tensor_cmds` to keep that
//! file within its line budget when the Record lift hooks joined it.

use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::record_lift;
use crate::interpreter::tensor_cmds::op_round;
use crate::interpreter::value_extraction_helpers::{create_number_value, nil_passthrough_binary};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Value, ValueData};

/// The single number an operand denotes, for a Word whose second operand is a
/// scalar parameter rather than a broadcast operand.
///
/// `Err` is malformed use (text, a code block, a multi-element vector); `None`
/// is a well-formed number that is not a rational — an exact irrational, which
/// is in the numeric domain but not in *this* Word's domain, so it projects
/// rather than raising (LANG.FAILURE.PROJECT).
fn single_rational_operand(value: &Value) -> Result<Option<Fraction>> {
    match &value.data {
        ValueData::Scalar(f) => Ok(Some(f.clone())),
        ValueData::ExactScalar(er) => Ok(er.to_fraction()),
        ValueData::Vector(children) if children.len() == 1 => single_rational_operand(&children[0]),
        ValueData::Tensor { data, .. } if data.len() == 1 => Ok(data.get_small_fraction(0)),
        ValueData::Vector(_) | ValueData::Tensor { .. } => Err(AjisaiError::declared(
            "nonNumeric",
            "QUANTIZE: expected a single-element number, got a multi-element vector",
        )),
        ValueData::Text(_) => Err(AjisaiError::declared(
            "nonNumeric",
            "QUANTIZE: expected a number, got a string",
        )),
        ValueData::Boolean(_) => Err(AjisaiError::declared(
            "nonNumeric",
            "QUANTIZE: expected a number, got a boolean",
        )),
        ValueData::Record(_) => Err(AjisaiError::declared(
            "nonNumeric",
            "QUANTIZE: expected a number, got a record",
        )),
        ValueData::Symbol(_) => Err(AjisaiError::declared(
            "nonNumeric",
            "QUANTIZE: expected a number, got a symbol",
        )),
        ValueData::Nil => Err(AjisaiError::declared(
            "nonNumeric",
            "QUANTIZE: expected a number, got NIL",
        )),
    }
}

/// `QUANTIZE ( x [ d ] -> x' )`: the nearest multiple of `1/d` to `x`, with
/// ties away from zero — `ROUND` scaled by `d`, and exactly `ROUND` at `d = 1`.
///
/// Exactness has a cost the step count does not price. An exact number's
/// denominator records the whole history of how it was computed, so every
/// iterative method — gradient descent, k-means, EM, the power method, Newton —
/// multiplies denominators step over step. Five points and a learning rate of
/// `1/50` reach 18-digit numerators by iteration 12 and 43-digit ones by 30,
/// still short of an answer whose exact form is `[ 23/10 -1/2 ]`. At that point
/// the representation is not the value; it is the derivation, kept in full.
///
/// Quantizing each step throws the derivation away and keeps the value: the
/// denominator is bounded by `d` forever, so the cost per step stops growing
/// and the loop runs at a fixed price. The trade is stated in the source, by
/// the caller, at the resolution the caller chose — which is the difference
/// between an approximation and a leak.
///
/// The implementation is the derivation it is named for (`x d MUL ROUND d DIV`)
/// run on the stack, so broadcasting, exact-real handling, and the NIL contract
/// are the ones those Words already define, and cannot drift from them.
pub fn op_quantize(interp: &mut Interpreter) -> Result<()> {
    if nil_passthrough_binary(interp) {
        return Ok(());
    }
    if record_lift::lift_binary(interp, &op_quantize)? {
        return Ok(());
    }
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let keep: bool = interp.consumption_mode == ConsumptionMode::Keep;
    let top: usize = interp.stack.len() - 1;
    let denominator_operand: Value = interp.stack[top].clone();

    // A denominator that is not a positive integer is a well-formed operand
    // outside the Word's domain, so it projects — the `SQRT` of a negative
    // rule, not the "you passed a code block" rule.
    let denominator: Fraction = match single_rational_operand(&denominator_operand)? {
        Some(d) if d.is_integer() && d.is_positive() => d,
        _ => {
            if !keep {
                interp.stack.truncate(top - 1);
            }
            interp
                .stack
                .push(Value::nil_with_reason_unknown(NilReason::DomainMiss));
            let len = interp.stack.len();
            interp.stack.set_role_at(len - 1, Interpretation::Nil);
            return Ok(());
        }
    };

    let subject: Value = interp.stack[top - 1].clone();
    let subject_role: Interpretation = interp.stack.role_at(top - 1);
    let restore: crate::types::Stack = interp.stack.clone();

    if !keep {
        interp.stack.truncate(top - 1);
    }
    interp.stack.push_with_role(subject, subject_role);
    interp.stack.push(create_number_value(denominator.clone()));

    let saved_mode = std::mem::replace(&mut interp.consumption_mode, ConsumptionMode::Consume);
    let outcome = quantize_on_stack(interp, &denominator);
    interp.consumption_mode = saved_mode;

    match outcome {
        Ok(()) => Ok(()),
        Err(error) => {
            // A half-finished derivation is not an observation: put the stack
            // back the way the caller left it and report the failure.
            interp.stack = restore;
            Err(error)
        }
    }
}

/// The scaled-round-unscale derivation, applied to `[ .. x d ]` on top of the
/// stack. Split out so the caller can restore the stack on any step's failure.
fn quantize_on_stack(interp: &mut Interpreter, denominator: &Fraction) -> Result<()> {
    super::arithmetic::op_mul(interp)?;
    op_round(interp)?;
    interp.stack.push(create_number_value(denominator.clone()));
    super::arithmetic::op_div(interp)
}
