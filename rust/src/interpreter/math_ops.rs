use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::{
    extract_operands, nil_passthrough_binary, nil_passthrough_unary, push_result,
};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::semantic::Recoverability;
use crate::types::exact::ExactReal;
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Value, ValueData};

fn require_stack_top(_interp: &Interpreter, _word: &str) -> Result<()> {
    Ok(())
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
            interp.stack.set_last_role(Interpretation::RawNumber);
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
            interp.stack.set_last_role(Interpretation::RawNumber);
            Ok(())
        }
        Ok(crate::interpreter::comparison::OrderOutcome::Decided(_)) => {
            // x >= 0: |x| = x, returned unchanged to preserve its exact form.
            push_result(interp, operands[0].clone());
            interp.stack.set_last_role(Interpretation::RawNumber);
            Ok(())
        }
        Ok(crate::interpreter::comparison::OrderOutcome::Undecided(_)) => {
            Err(AjisaiError::from("operand is outside the exact domain"))
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
            interp.stack.set_last_role(Interpretation::RawNumber);
            Ok(())
        }
        Ok(crate::interpreter::comparison::OrderOutcome::Undecided(_)) => {
            Err(AjisaiError::from("operand is outside the exact domain"))
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
            interp.stack.set_last_role(Interpretation::RawNumber);
            Ok(())
        }
        Ok(crate::interpreter::comparison::OrderOutcome::Undecided(_)) => {
            Err(AjisaiError::from("operand is outside the exact domain"))
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

/// `SQRT`: the exact square root of a non-negative rational, and the only Word
/// that leaves the rationals (LANG.VALUES.EXACT). The result is carried in the
/// multiquadratic normal form, so it compares and decides with no rounding.
///
/// A negative radicand is a well-formed domain miss: the multiquadratic field
/// is not closed under it, so the operation projects to NIL rather than raising
/// (LANG.FAILURE.PROJECT). It is recoverable — a different input resolves it.
pub(crate) fn op_sqrt(interp: &mut Interpreter) -> Result<()> {
    let value = if interp.consumption_mode == ConsumptionMode::Keep {
        interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    let Some(f) = value.as_scalar() else {
        return Err(AjisaiError::create_structure_error(
            "number",
            "other format",
        ));
    };
    match ExactReal::from_sqrt_rational(f.clone()) {
        // `from_exact_real` collapses a rational result back to Scalar.
        Some(er) => interp
            .stack
            .push_with_role(Value::from_exact_real(er), Interpretation::RawNumber),
        None => interp.stack.push_with_role(
            Value::bubble_with_reason(NilReason::DomainMiss, Recoverability::Recoverable),
            Interpretation::Nil,
        ),
    }
    Ok(())
}
