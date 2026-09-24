//! `FORMAT` — render an exact scalar as decimal text at a stated precision
//! (LANG.VALUES.EXACT, LANG.VALUES.DISJOINT).
//!
//! Arithmetic never rounds and `STR` refuses a number with no exact lexeme,
//! so the language had no place a program could ask for `1/3` to three
//! places without first rounding the *value* (`1000 MUL ROUND 1000 DIV`) and then spelling
//! the rounded number. `FORMAT` is that place, and it is the only one: what
//! leaves it is text, so the rounded quantity never re-enters arithmetic as
//! if it were exact. The rule is fixed — a tie rounds away from zero, the
//! one rule `ROUND` already applies — because a choice of
//! rounding mode is a family of Words, and the language keeps one rule.
//!
//! The decision is exact at every tier. A rational scales and rounds
//! outright. An algebraic irrational compares its scaled fraction part against
//! one half through the field's total order, which never ties. A computable
//! real refines under the default comparison water and, when the last digit
//! does not settle, projects `undecidable` — the outcome its comparisons
//! already reach when refinement runs out — rather than guess a digit.

use num_bigint::BigInt;
use num_traits::{One, Signed};
use std::cmp::Ordering;

use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::extract_operands;
use crate::interpreter::Interpreter;
use crate::semantic::Recoverability;
use crate::types::exact::{ExactCmp, ExactReal, DEFAULT_COMPARISON_WATER};
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};

/// The most digits one `FORMAT` may ask for. Every digit is a decimal place
/// of big-integer work, and the meter charges each one, so the cap only
/// stops a single request from allocating a number the meter would refuse
/// anyway.
const MAX_DIGITS: u64 = 1 << 16;

fn restore_all(interp: &mut Interpreter, operands: Vec<Value>) {
    for operand in operands {
        interp.stack.push(operand);
    }
}

fn exact_real_of(value: &Value) -> Option<ExactReal> {
    match &value.data {
        ValueData::Scalar(f) => Some(ExactReal::from_fraction(f.clone())),
        ValueData::ExactScalar(er) => Some(er.clone()),
        _ => None,
    }
}

fn digit_count(value: &Value) -> Option<u64> {
    let f = value.as_scalar()?;
    if !f.is_integer() || f.numerator().is_negative() {
        return None;
    }
    let n = f.numerator();
    (n <= BigInt::from(MAX_DIGITS)).then(|| n.to_string().parse().ok())?
}

/// What settling the last digit produced.
enum Rounded {
    Integer(BigInt),
    Undecidable,
}

/// `x * 10^digits`, rounded to an integer with a tie away from zero.
fn round_scaled(x: &ExactReal, digits: u64) -> Rounded {
    let scale = Fraction::new(BigInt::from(10).pow(digits as u32), BigInt::one());
    if let Some(f) = x.as_rational() {
        let rounded = f.mul(&scale).round();
        return Rounded::Integer(rounded.numerator());
    }
    let scaled = x.mul(&ExactReal::from_fraction(scale));
    let Some(floor) = scaled.floor() else {
        return Rounded::Undecidable;
    };
    let floor_int = floor
        .as_rational()
        .expect("a floor is an integer")
        .numerator();
    let fraction_part = scaled.sub(&floor);
    let half = ExactReal::from_fraction(Fraction::new(BigInt::one(), BigInt::from(2)));
    match fraction_part.cmp_within(&half, DEFAULT_COMPARISON_WATER) {
        ExactCmp::Decided(Ordering::Less) => Rounded::Integer(floor_int),
        ExactCmp::Decided(Ordering::Greater) => Rounded::Integer(floor_int + 1),
        // A tie cannot occur for an irrational, but the rule is stated all
        // the same: away from zero.
        ExactCmp::Decided(Ordering::Equal) => Rounded::Integer(if floor_int.is_negative() {
            floor_int
        } else {
            floor_int + 1
        }),
        ExactCmp::Starved { .. } | ExactCmp::Absent => Rounded::Undecidable,
    }
}

/// Spell `n / 10^digits` in decimal: a sign only for a non-zero value, at
/// least one digit before the point, exactly `digits` after it, and no
/// point at all when `digits` is zero.
fn spell(n: &BigInt, digits: u64) -> String {
    let magnitude = n.abs().to_string();
    let digits = digits as usize;
    let mut body = if magnitude.len() <= digits {
        let mut padded = "0".repeat(digits + 1 - magnitude.len());
        padded.push_str(&magnitude);
        padded
    } else {
        magnitude
    };
    if digits > 0 {
        body.insert(body.len() - digits, '.');
    }
    if n.is_negative() {
        body.insert(0, '-');
    }
    body
}

/// `FORMAT ( [ x ] [ digits ] -> [ text ] )`.
pub(crate) fn op_format(interp: &mut Interpreter) -> Result<()> {
    let operands = extract_operands(interp, 2)?;
    let Some(x) = exact_real_of(&operands[0]) else {
        restore_all(interp, operands);
        return Err(AjisaiError::declared(
            "nonNumeric",
            "FORMAT: expected an exact scalar as the value, got a non-numeric operand",
        ));
    };
    let Some(digits) = digit_count(&operands[1]) else {
        restore_all(interp, operands);
        return Err(AjisaiError::declared(
            "invalidCount",
            "FORMAT: expected a non-negative integer digit count",
        ));
    };
    // Every digit is a decimal place of big-integer work.
    if let Err(e) = interp.charge_numeric_work(digits + 1) {
        restore_all(interp, operands);
        return Err(e);
    }
    match round_scaled(&x, digits) {
        Rounded::Integer(n) => interp.stack.push(Value::from_string(&spell(&n, digits))),
        Rounded::Undecidable => interp.stack.push(Value::nil_with_reason(
            NilReason::Undecidable,
            Recoverability::Retryable,
        )),
    }
    Ok(())
}
