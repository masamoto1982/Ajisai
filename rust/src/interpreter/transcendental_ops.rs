//! `EXP`, `LN`, `SIN`, `COS`, `ATAN` — the transcendental Words
//! (LANG.VALUES.EXACT, Phase 7 of the vocabulary-100 work order).
//!
//! Each is the scalar law of `types::exact::transcendental` lifted the way
//! every unary arithmetic Word is lifted: element-wise over a Vector, in the
//! value direction over a Record, a NIL lane passing through. The answer is
//! a computable real except where the argument makes it exact, and what the
//! kernel refuses is projected under the reason the contract declares.

use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::math_ops::lift_unary_numeric;
use crate::interpreter::record_lift;
use crate::interpreter::value_extraction_helpers::{extract_operands, push_result};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::semantic::Recoverability;
use crate::types::exact::{ExactReal, Transcendental};
use crate::types::{Interpretation, Value, ValueData};

pub(crate) fn exact_real_of(value: &Value) -> Option<ExactReal> {
    match &value.data {
        ValueData::Scalar(f) => Some(ExactReal::from_fraction(f.clone())),
        ValueData::ExactScalar(er) => Some(er.clone()),
        _ => None,
    }
}

/// The Value a transcendental outcome answers with.
pub(crate) fn value_of(outcome: Transcendental) -> Value {
    match outcome {
        Transcendental::Value(er) => Value::from_exact_real(er),
        Transcendental::DomainMiss => {
            Value::nil_with_reason(NilReason::DomainMiss, Recoverability::Recoverable)
        }
        Transcendental::Undecidable => {
            Value::nil_with_reason(NilReason::Undecidable, Recoverability::Retryable)
        }
        Transcendental::SpaceExhausted => {
            Value::nil_with_reason(NilReason::SpaceExhausted, Recoverability::Unknown)
        }
    }
}

fn scalar_law(
    word: &'static str,
    f: fn(&ExactReal) -> Transcendental,
) -> impl Fn(&Value) -> Result<Value> {
    move |value: &Value| match exact_real_of(value) {
        Some(er) => Ok(value_of(f(&er))),
        None => Err(AjisaiError::declared(
            "nonNumeric",
            format!("{word}: expected a number, got a non-numeric value"),
        )),
    }
}

/// Run one unary transcendental Word over the operand on top of the stack.
pub(crate) fn unary(
    interp: &mut Interpreter,
    word: &'static str,
    f: fn(&ExactReal) -> Transcendental,
) -> Result<()> {
    let operands = extract_operands(interp, 1)?;
    match lift_unary_numeric(&operands[0], &scalar_law(word, f)) {
        Ok(result) => {
            let role = if result.is_nil() {
                Interpretation::Nil
            } else {
                Interpretation::RawNumber
            };
            push_result(interp, result);
            interp.stack.set_last_role(role);
            Ok(())
        }
        Err(e) => {
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.extend(operands);
            }
            Err(e)
        }
    }
}

pub(crate) fn op_exp(interp: &mut Interpreter) -> Result<()> {
    if record_lift::lift_unary(interp, &op_exp)? {
        return Ok(());
    }
    unary(interp, "EXP", ExactReal::exp)
}

pub(crate) fn op_ln(interp: &mut Interpreter) -> Result<()> {
    if record_lift::lift_unary(interp, &op_ln)? {
        return Ok(());
    }
    unary(interp, "LN", ExactReal::ln)
}

pub(crate) fn op_sin(interp: &mut Interpreter) -> Result<()> {
    if record_lift::lift_unary(interp, &op_sin)? {
        return Ok(());
    }
    unary(interp, "SIN", ExactReal::sin)
}

pub(crate) fn op_cos(interp: &mut Interpreter) -> Result<()> {
    if record_lift::lift_unary(interp, &op_cos)? {
        return Ok(());
    }
    unary(interp, "COS", ExactReal::cos)
}

pub(crate) fn op_atan(interp: &mut Interpreter) -> Result<()> {
    if record_lift::lift_unary(interp, &op_atan)? {
        return Ok(());
    }
    unary(interp, "ATAN", ExactReal::atan)
}
