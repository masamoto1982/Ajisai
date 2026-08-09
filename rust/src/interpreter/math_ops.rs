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

/// Apply a unary numeric Word across the shapes LANG.COLLECTIONS.LIFT allows.
///
/// The clause makes element-wise application the rule for an arithmetic Word
/// given a vector. `NEG`, `ABS` and `SIGN` took a scalar only, so `[ -1 2 ] ABS`
/// was an ERROR while `[ -1 2 ] 1 MUL` lifted happily — the same clause read
/// two ways depending on arity. A NIL lane passes through, as it does for the
/// scalar law.
fn lift_unary_numeric(value: &Value, scalar_op: &dyn Fn(&Value) -> Result<Value>) -> Result<Value> {
    match value.as_vector_view() {
        Some(items) => {
            let lanes = items
                .iter()
                .map(|item| lift_unary_numeric(item, scalar_op))
                .collect::<Result<Vec<_>>>()?;
            Ok(Value::from_vector(lanes))
        }
        None if value.is_nil() => Ok(value.clone()),
        None => scalar_op(value),
    }
}

/// The scalar law of `NEG`, lifted by [`lift_unary_numeric`].
fn neg_scalar(value: &Value) -> Result<Value> {
    match exact_real_of(value) {
        Some(er) => Ok(Value::from_exact_real(er.neg())),
        None => Err(AjisaiError::from("NEG: expected a number")),
    }
}

/// The scalar law of `ABS`, lifted by [`lift_unary_numeric`].
fn abs_scalar(value: &Value) -> Result<Value> {
    let zero = Value::from_fraction(Fraction::from(0));
    match crate::interpreter::comparison::three_way_compare(value, &zero)? {
        crate::interpreter::comparison::OrderOutcome::Decided(std::cmp::Ordering::Less) => {
            let er = exact_real_of(value).expect("comparable operand is numeric");
            Ok(Value::from_exact_real(er.neg()))
        }
        crate::interpreter::comparison::OrderOutcome::Decided(_) => Ok(value.clone()),
        crate::interpreter::comparison::OrderOutcome::Undecided(_) => {
            Err(AjisaiError::from("operand is outside the exact domain"))
        }
    }
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
    match lift_unary_numeric(&operands[0], &neg_scalar) {
        Ok(result) => {
            push_result(interp, result);
            interp.stack.set_last_role(Interpretation::RawNumber);
            Ok(())
        }
        Err(e) => {
            restore_operands(interp, operands);
            Err(e)
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
    match lift_unary_numeric(&operands[0], &abs_scalar) {
        Ok(result) => {
            push_result(interp, result);
            interp.stack.set_last_role(Interpretation::RawNumber);
            Ok(())
        }
        Err(e) => {
            restore_operands(interp, operands);
            Err(e)
        }
    }
}

/// Apply a binary numeric Word across the shapes LANG.COLLECTIONS.LIFT allows.
///
/// The shape rules are the ones the arithmetic broadcast already uses: a
/// scalar pairs with every element of a vector, two vectors of equal length
/// pair element-wise, and unequal lengths are a shape error. `MIN` and `MAX`
/// used to take scalars only, so `[ -1 2 -3 ] 0 MAX` — a rectifier, and the
/// most ordinary thing anyone writes with `MAX` — was an ERROR while
/// `[ -1 2 -3 ] 0 ADD` lifted happily. Same clause, same family, two answers.
/// A NIL lane passes through, as it does for the scalar law.
fn lift_binary_numeric(
    a: &Value,
    b: &Value,
    leaf_op: &dyn Fn(&Value, &Value) -> Result<Value>,
) -> Result<Value> {
    use crate::interpreter::tensor_ops::broadcast_children;

    match (broadcast_children(a), broadcast_children(b)) {
        (None, None) => {
            if a.is_nil() {
                return Ok(a.clone());
            }
            if b.is_nil() {
                return Ok(b.clone());
            }
            leaf_op(a, b)
        }
        (Some(children), None) => Ok(Value::from_children(
            children
                .iter()
                .map(|child| lift_binary_numeric(child, b, leaf_op))
                .collect::<Result<Vec<Value>>>()?,
        )),
        (None, Some(children)) => Ok(Value::from_children(
            children
                .iter()
                .map(|child| lift_binary_numeric(a, child, leaf_op))
                .collect::<Result<Vec<Value>>>()?,
        )),
        // A length-1 axis stretches to meet the other, the same rule the
        // arithmetic broadcast applies, so `[ -1 2 -3 ] [ 0 ] MAX` and
        // `[ -1 2 -3 ] 0 MAX` are the same rectifier written two ways.
        (Some(left), Some(right)) if left.len() == 1 && right.len() != 1 => {
            Ok(Value::from_children(
                right
                    .iter()
                    .map(|y| lift_binary_numeric(&left[0], y, leaf_op))
                    .collect::<Result<Vec<Value>>>()?,
            ))
        }
        (Some(left), Some(right)) if right.len() == 1 && left.len() != 1 => {
            Ok(Value::from_children(
                left.iter()
                    .map(|x| lift_binary_numeric(x, &right[0], leaf_op))
                    .collect::<Result<Vec<Value>>>()?,
            ))
        }
        (Some(left), Some(right)) => {
            if left.len() != right.len() {
                return Err(AjisaiError::VectorLengthMismatch {
                    len1: left.len(),
                    len2: right.len(),
                });
            }
            Ok(Value::from_children(
                left.iter()
                    .zip(right.iter())
                    .map(|(x, y)| lift_binary_numeric(x, y, leaf_op))
                    .collect::<Result<Vec<Value>>>()?,
            ))
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
/// Element-wise over vectors, by [`lift_binary_numeric`].
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
    let select = |a: &Value, b: &Value| -> Result<Value> {
        match crate::interpreter::comparison::three_way_compare(a, b)? {
            crate::interpreter::comparison::OrderOutcome::Decided(ord) => {
                Ok(if pick_left(ord) { a.clone() } else { b.clone() })
            }
            crate::interpreter::comparison::OrderOutcome::Undecided(_) => {
                Err(AjisaiError::from("operand is outside the exact domain"))
            }
        }
    };
    match lift_binary_numeric(&operands[0], &operands[1], &select) {
        Ok(result) => {
            push_result(interp, result);
            interp.stack.set_last_role(Interpretation::RawNumber);
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

/// `SQRT`: the exact square root of a non-negative rational, and the only Word
/// that leaves the rationals (LANG.VALUES.EXACT). The result is carried in the
/// multiquadratic normal form, so it compares and decides with no rounding.
///
/// A negative radicand is a well-formed domain miss: the multiquadratic field
/// is not closed under it, so the operation projects to NIL rather than raising
/// (LANG.FAILURE.PROJECT). It is recoverable — a different input resolves it.
///
/// Element-wise over a vector, by [`lift_unary_numeric`]: a per-element
/// standard deviation is `variances SQRT`, not a `MAP` around a block.
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

    match lift_unary_numeric(&value, &sqrt_scalar) {
        Ok(result) => {
            let role = if result.is_nil() {
                Interpretation::Nil
            } else {
                Interpretation::RawNumber
            };
            interp.stack.push_with_role(result, role);
            Ok(())
        }
        Err(e) => {
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.push(value);
            }
            Err(e)
        }
    }
}

/// The scalar law of `SQRT`, lifted by [`lift_unary_numeric`].
fn sqrt_scalar(value: &Value) -> Result<Value> {
    let Some(f) = value.as_scalar() else {
        return Err(AjisaiError::create_structure_error(
            "number",
            "other format",
        ));
    };
    // `from_exact_real` collapses a rational result back to Scalar.
    Ok(match ExactReal::from_sqrt_rational(f.clone()) {
        Some(er) => Value::from_exact_real(er),
        None => Value::bubble_with_reason(NilReason::DomainMiss, Recoverability::Recoverable),
    })
}
