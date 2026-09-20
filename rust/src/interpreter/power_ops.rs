//! `POW`, `GCD`, `RATIO` — the Words that close the number concept
//! (LANG.VALUES.EXACT, Phase 7 of the vocabulary-100 work order).
//!
//! `POW` is the kernel's `ExactReal::pow` lifted like every binary
//! arithmetic Word. `GCD` exposes the reduction the machine already performs
//! on every rational, and `RATIO` reads a rational's two parts back as a
//! Vector, so that arithmetic lifts over the answer. Both refuse what is not
//! a rational integer or rational: an irrational projects `domainMiss`, and
//! a computable real — whose integrality or rationality no budget proves —
//! projects `undecidable`.

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::Signed;

use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::math_ops::{lift_binary_numeric, lift_unary_numeric};
use crate::interpreter::record_lift;
use crate::interpreter::transcendental_ops::exact_real_of;
use crate::interpreter::value_extraction_helpers::{extract_operands, push_result};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::semantic::Recoverability;
use crate::types::exact::{ExactReal, PowOutcome};
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Value};

fn non_numeric(word: &str) -> AjisaiError {
    AjisaiError::declared(
        "nonNumeric",
        format!("{word}: expected a number, got a non-numeric value"),
    )
}

fn nil(reason: NilReason, recoverability: Recoverability) -> Value {
    Value::nil_with_reason(reason, recoverability)
}

fn pow_scalar(x: &Value, y: &Value) -> Result<Value> {
    let (Some(base), Some(exponent)) = (exact_real_of(x), exact_real_of(y)) else {
        return Err(non_numeric("POW"));
    };
    Ok(match base.pow(&exponent) {
        PowOutcome::Value(er) => Value::from_exact_real(er),
        PowOutcome::DivisionByZero => nil(NilReason::DivisionByZero, Recoverability::Recoverable),
        PowOutcome::DomainMiss => nil(NilReason::DomainMiss, Recoverability::Recoverable),
        PowOutcome::Undecidable => nil(NilReason::Undecidable, Recoverability::Retryable),
        PowOutcome::SpaceExhausted => nil(NilReason::SpaceExhausted, Recoverability::Unknown),
    })
}

/// The integer a rational integer scalar holds; the projection otherwise.
fn integer_of(value: &Value) -> std::result::Result<BigInt, Value> {
    match exact_real_of(value) {
        Some(ExactReal::Rational(q)) if q.is_integer() => Ok(q.numerator()),
        Some(ExactReal::Computable(_)) => {
            Err(nil(NilReason::Undecidable, Recoverability::Retryable))
        }
        Some(_) => Err(nil(NilReason::DomainMiss, Recoverability::Recoverable)),
        None => Err(non_numeric_value()),
    }
}

/// A marker for "not a number at all", told apart from a projection by
/// the caller.
fn non_numeric_value() -> Value {
    Value::from_symbol("__nonNumeric")
}

fn gcd_scalar(a: &Value, b: &Value) -> Result<Value> {
    if exact_real_of(a).is_none() || exact_real_of(b).is_none() {
        return Err(non_numeric("GCD"));
    }
    match (integer_of(a), integer_of(b)) {
        (Ok(x), Ok(y)) => Ok(Value::from_fraction(Fraction::new(
            x.gcd(&y).abs(),
            BigInt::from(1),
        ))),
        (Err(projection), _) | (_, Err(projection)) => Ok(projection),
    }
}

fn ratio_scalar(value: &Value) -> Result<Value> {
    Ok(match exact_real_of(value) {
        Some(ExactReal::Rational(q)) => {
            let (n, d) = q.to_bigint_pair();
            let (n, d) = if d.is_negative() { (-n, -d) } else { (n, d) };
            Value::from_vector(vec![
                Value::from_fraction(Fraction::new(n, BigInt::from(1))),
                Value::from_fraction(Fraction::new(d, BigInt::from(1))),
            ])
        }
        Some(ExactReal::Algebraic(_)) => nil(NilReason::DomainMiss, Recoverability::Recoverable),
        Some(ExactReal::Computable(_)) => nil(NilReason::Undecidable, Recoverability::Retryable),
        None => return Err(non_numeric("RATIO")),
    })
}

fn restore(interp: &mut Interpreter, operands: Vec<Value>) {
    if interp.consumption_mode != ConsumptionMode::Keep {
        interp.stack.extend(operands);
    }
}

fn finish(interp: &mut Interpreter, result: Value) {
    let role = if result.is_nil() {
        Interpretation::Nil
    } else {
        Interpretation::RawNumber
    };
    push_result(interp, result);
    interp.stack.set_last_role(role);
}

fn binary(interp: &mut Interpreter, leaf: &dyn Fn(&Value, &Value) -> Result<Value>) -> Result<()> {
    let operands = extract_operands(interp, 2)?;
    match lift_binary_numeric(&operands[0], &operands[1], leaf) {
        Ok(result) => {
            finish(interp, result);
            Ok(())
        }
        Err(e) => {
            restore(interp, operands);
            Err(e)
        }
    }
}

pub(crate) fn op_pow(interp: &mut Interpreter) -> Result<()> {
    if record_lift::lift_binary(interp, &op_pow)? {
        return Ok(());
    }
    binary(interp, &pow_scalar)
}

pub(crate) fn op_gcd(interp: &mut Interpreter) -> Result<()> {
    if record_lift::lift_binary(interp, &op_gcd)? {
        return Ok(());
    }
    binary(interp, &gcd_scalar)
}

pub(crate) fn op_ratio(interp: &mut Interpreter) -> Result<()> {
    if record_lift::lift_unary(interp, &op_ratio)? {
        return Ok(());
    }
    let operands = extract_operands(interp, 1)?;
    match lift_unary_numeric(&operands[0], &ratio_scalar) {
        Ok(result) => {
            let role = if result.is_nil() {
                Interpretation::Nil
            } else {
                Interpretation::Unassigned
            };
            push_result(interp, result);
            interp.stack.set_last_role(role);
            Ok(())
        }
        Err(e) => {
            restore(interp, operands);
            Err(e)
        }
    }
}
