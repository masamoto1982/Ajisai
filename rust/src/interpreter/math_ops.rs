use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::Zero;

use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::{
    extract_bigint_from_value, extract_operands, nil_passthrough_binary, nil_passthrough_unary,
    push_result,
};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::semantic::{AbsenceOrigin, Recoverability};
use crate::types::continued_fraction::ExactReal;
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Value, ValueData};

/// Runtime safety bound on `POW` exponent magnitude, analogous to the
/// execution step budget (SPEC §5.3). It prevents a single well-formed
/// `POW` from materializing an astronomically large exact rational and
/// exhausting memory; it is not a language-level semantic constraint.
const MAX_POW_EXPONENT: i64 = 1_000_000;

fn require_stack_top(interp: &Interpreter, word: &str) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::from(format!(
            "{}: Stack mode is not supported",
            word
        )));
    }
    Ok(())
}

fn extract_scalar(value: &Value, word: &str) -> Result<Fraction> {
    value
        .as_scalar()
        .cloned()
        .ok_or_else(|| AjisaiError::from(format!("{}: expected a number", word)))
}

/// Exact-real view of a numeric operand: a rational `Scalar` lifts to
/// `ExactReal::Rational`; a lazy `ExactScalar` (an `AlgebraicSqrt` or `Gosper`
/// value) is taken as-is. Non-numeric kinds return `None` — the malformed-use
/// path.
fn exact_real_of(value: &Value) -> Option<ExactReal> {
    match &value.data {
        ValueData::Scalar(f) => Some(ExactReal::from_fraction(f.clone())),
        ValueData::ExactScalar(er) => Some(er.clone()),
        _ => None,
    }
}

/// `NEG` is the additive inverse `-x` over exact numeric values. It computes
/// directly on the exact-real representation, so it accepts the full numeric
/// domain including lazy continued-fraction operands (`2 SQRT NEG` is `-√2`),
/// and is total — no comparison and therefore no `Unknown` is ever involved.
/// NIL-passthrough; a non-numeric operand is malformed use and raises an error.
pub(crate) fn op_neg(interp: &mut Interpreter) -> Result<()> {
    require_stack_top(interp, "NEG")?;
    if nil_passthrough_unary(interp) {
        return Ok(());
    }
    let operands = extract_operands(interp, 1)?;
    match exact_real_of(&operands[0]) {
        Some(er) => {
            push_result(interp, Value::from_exact_real(er.neg()));
            interp
                .semantic_registry
                .push_hint(Interpretation::RawNumber);
            Ok(())
        }
        None => {
            restore_operands(interp, operands);
            Err(AjisaiError::from("NEG: expected a number"))
        }
    }
}

/// `ABS` is the absolute value `|x|`, derived from the sign and exact
/// arithmetic (SPEC §7.4.3): it decides the order of `x` against `0` through
/// the same budgeted comparison as the relations and negates when `x < 0`,
/// otherwise returns `x` unchanged. It therefore accepts the full numeric
/// domain including lazy continued-fraction operands, and over the admitted
/// domain (§4.2.7) is total and exact. When the order against `0` does not
/// decide within the budget, the result is the logical `Unknown` (U) carrying
/// `diagnosis.agreedPrefix`. NIL-passthrough, with NIL taking priority over a
/// U-producing comparison (§4.5.2); a non-numeric operand raises an error.
pub(crate) fn op_abs(interp: &mut Interpreter) -> Result<()> {
    require_stack_top(interp, "ABS")?;
    if nil_passthrough_unary(interp) {
        return Ok(());
    }
    let operands = extract_operands(interp, 1)?;
    let zero = Value::from_fraction(Fraction::from(0));
    match crate::interpreter::comparison::three_way_compare(&operands[0], &zero) {
        Ok(crate::interpreter::comparison::OrderOutcome::Decided(std::cmp::Ordering::Less)) => {
            // |x| = -x for x < 0; a value that compared is numeric.
            let er = exact_real_of(&operands[0]).expect("comparable operand is numeric");
            push_result(interp, Value::from_exact_real(er.neg()));
            interp
                .semantic_registry
                .push_hint(Interpretation::RawNumber);
            Ok(())
        }
        Ok(crate::interpreter::comparison::OrderOutcome::Decided(_)) => {
            // x >= 0: |x| = x, returned unchanged to preserve its exact form.
            push_result(interp, operands[0].clone());
            interp
                .semantic_registry
                .push_hint(Interpretation::RawNumber);
            Ok(())
        }
        Ok(crate::interpreter::comparison::OrderOutcome::Undecided(agreed_prefix)) => {
            crate::interpreter::comparison::push_comparison_unknown(interp, agreed_prefix);
            Ok(())
        }
        Err(e) => {
            restore_operands(interp, operands);
            Err(e)
        }
    }
}

/// `SIGN` extracts the sign of a number as the scalar `-1`, `0`, or `1`
/// (SPEC §7.4.3). Like `MIN`/`MAX`, it decides the order against `0` through
/// the same budgeted comparison as the relations and therefore accepts the
/// full numeric domain, including lazy continued-fraction operands: over the
/// admitted domain (§4.2.7) the sign is total and exact. When the order
/// against `0` does not decide within the budget, the result is the logical
/// `Unknown` (U) carrying `diagnosis.agreedPrefix`, matching the U-honesty of
/// the other comparison-dependent words. NIL-passthrough, with NIL taking
/// priority over a U-producing comparison (§4.5.2). A non-numeric operand is
/// malformed use and raises an error.
pub(crate) fn op_sign(interp: &mut Interpreter) -> Result<()> {
    require_stack_top(interp, "SIGN")?;
    if nil_passthrough_unary(interp) {
        return Ok(());
    }
    let operands = extract_operands(interp, 1)?;
    let zero = Value::from_fraction(Fraction::from(0));
    match crate::interpreter::comparison::three_way_compare(&operands[0], &zero) {
        Ok(crate::interpreter::comparison::OrderOutcome::Decided(ord)) => {
            let sign = match ord {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };
            push_result(interp, Value::from_fraction(Fraction::from(sign)));
            interp
                .semantic_registry
                .push_hint(Interpretation::RawNumber);
            Ok(())
        }
        Ok(crate::interpreter::comparison::OrderOutcome::Undecided(agreed_prefix)) => {
            crate::interpreter::comparison::push_comparison_unknown(interp, agreed_prefix);
            Ok(())
        }
        Err(e) => {
            restore_operands(interp, operands);
            Err(e)
        }
    }
}

/// `MIN` / `MAX` select one of two numeric operands by the order relation
/// (SPEC §7.4.3). They accept the full numeric domain, including lazy
/// continued-fraction operands, and decide the order through the same
/// budgeted comparison as the relations. When the comparison decides, the
/// selected operand is returned unchanged (preserving its exact
/// representation). When it does not decide within the budget, the result is
/// the logical `Unknown` (U) carrying `diagnosis.agreedPrefix` — the program
/// cannot be told which operand is the min/max when their order is unknown.
/// NIL-passthrough, with NIL taking priority over a U-producing comparison.
fn apply_selecting<F>(interp: &mut Interpreter, word: &str, pick_left: F) -> Result<()>
where
    // Given the order of `a` (left) vs `b` (right), return true to keep `a`.
    F: Fn(std::cmp::Ordering) -> bool,
{
    require_stack_top(interp, word)?;
    if nil_passthrough_binary(interp) {
        return Ok(());
    }
    let operands = extract_operands(interp, 2)?;
    match crate::interpreter::comparison::three_way_compare(&operands[0], &operands[1]) {
        Ok(crate::interpreter::comparison::OrderOutcome::Decided(ord)) => {
            let chosen = if pick_left(ord) {
                operands[0].clone()
            } else {
                operands[1].clone()
            };
            push_result(interp, chosen);
            interp
                .semantic_registry
                .push_hint(Interpretation::RawNumber);
            Ok(())
        }
        Ok(crate::interpreter::comparison::OrderOutcome::Undecided(agreed_prefix)) => {
            crate::interpreter::comparison::push_comparison_unknown(interp, agreed_prefix);
            Ok(())
        }
        Err(e) => {
            restore_operands(interp, operands);
            Err(e)
        }
    }
}

pub(crate) fn op_min(interp: &mut Interpreter) -> Result<()> {
    // Keep the left operand when it is less-or-equal to the right.
    apply_selecting(interp, "MIN", |ord| ord != std::cmp::Ordering::Greater)
}

pub(crate) fn op_max(interp: &mut Interpreter) -> Result<()> {
    // Keep the left operand when it is greater-or-equal to the right.
    apply_selecting(interp, "MAX", |ord| ord != std::cmp::Ordering::Less)
}

fn restore_operands(interp: &mut Interpreter, operands: Vec<Value>) {
    if interp.consumption_mode != ConsumptionMode::Keep {
        interp.stack.extend(operands);
    }
}

fn pow_fraction(base: &Fraction, mut exp: u64) -> Fraction {
    let mut result = Fraction::from(1);
    let mut b = base.clone();
    while exp > 0 {
        if exp & 1 == 1 {
            result = result.mul(&b);
        }
        exp >>= 1;
        if exp > 0 {
            b = b.mul(&b);
        }
    }
    result
}

/// `base exp -- result`. Integer-exponent exact power. A non-integer or
/// non-numeric operand is malformed use and raises an error (cf. `CHR`).
/// `0` raised to a negative exponent is a well-formed domain miss and
/// projects to Bubble/NIL with `reason = divisionByZero` (Bubble Rule).
pub(crate) fn op_pow(interp: &mut Interpreter) -> Result<()> {
    require_stack_top(interp, "POW")?;
    if nil_passthrough_binary(interp) {
        return Ok(());
    }
    let operands = extract_operands(interp, 2)?;
    let base = match extract_scalar(&operands[0], "POW") {
        Ok(b) => b,
        Err(e) => {
            restore_operands(interp, operands);
            return Err(e);
        }
    };
    let exp = match extract_bigint_from_value(&operands[1]) {
        Ok(e) => e,
        Err(_) => {
            restore_operands(interp, operands);
            return Err(AjisaiError::from("POW: exponent must be an integer"));
        }
    };
    let exp_i64: i64 = match (&exp).try_into() {
        Ok(n) => n,
        Err(_) => {
            restore_operands(interp, operands);
            return Err(AjisaiError::from(
                "POW: exponent magnitude exceeds the supported bound",
            ));
        }
    };
    if exp_i64.abs() > MAX_POW_EXPONENT {
        restore_operands(interp, operands);
        return Err(AjisaiError::from(
            "POW: exponent magnitude exceeds the supported bound",
        ));
    }

    let result = if exp_i64 == 0 {
        Value::from_fraction(Fraction::from(1))
    } else if exp_i64 > 0 {
        Value::from_fraction(pow_fraction(&base, exp_i64 as u64))
    } else if base.is_zero() {
        Value::bubble_with_reason(
            NilReason::DivisionByZero,
            AbsenceOrigin::ExecutionFailure,
            Recoverability::Recoverable,
        )
    } else {
        let positive = pow_fraction(&base, exp_i64.unsigned_abs());
        Value::from_fraction(Fraction::new(positive.denominator(), positive.numerator()))
    };
    let is_bubble = result.is_nil();
    push_result(interp, result);
    if !is_bubble {
        interp
            .semantic_registry
            .push_hint(Interpretation::RawNumber);
    }
    Ok(())
}

fn apply_integer_binary<F>(interp: &mut Interpreter, word: &str, op: F) -> Result<()>
where
    F: Fn(&BigInt, &BigInt) -> BigInt,
{
    require_stack_top(interp, word)?;
    if nil_passthrough_binary(interp) {
        return Ok(());
    }
    let operands = extract_operands(interp, 2)?;
    let parsed = (
        extract_bigint_from_value(&operands[0]),
        extract_bigint_from_value(&operands[1]),
    );
    let (a, b) = match parsed {
        (Ok(a), Ok(b)) => (a, b),
        _ => {
            restore_operands(interp, operands);
            return Err(AjisaiError::from(format!(
                "{}: expected two integers",
                word
            )));
        }
    };
    push_result(
        interp,
        Value::from_fraction(Fraction::new(op(&a, &b), BigInt::from(1))),
    );
    interp
        .semantic_registry
        .push_hint(Interpretation::RawNumber);
    Ok(())
}

pub(crate) fn op_gcd(interp: &mut Interpreter) -> Result<()> {
    apply_integer_binary(interp, "GCD", |a, b| a.gcd(b))
}

pub(crate) fn op_lcm(interp: &mut Interpreter) -> Result<()> {
    apply_integer_binary(interp, "LCM", |a, b| {
        if a.is_zero() || b.is_zero() {
            BigInt::from(0)
        } else {
            a.lcm(b)
        }
    })
}
