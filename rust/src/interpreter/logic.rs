use crate::error::{AjisaiError, Result};
use crate::interpreter::lane_lift::lift_lanes;
use crate::interpreter::Interpreter;
use crate::types::Value;

/// The truth value of a `booleanLogic` operand.
///
/// The Boolean domain is the *whole* definite input domain of `AND`,
/// `NOT`, and `SELECT`'s truth operand: `spec/semantic-families.json` gives
/// the family `truth: threeValued` and each contract registers
/// `nonTruthValue` as its error condition. NIL is handled separately by
/// [`truth_or_unknown`], not by this accessor, because NIL is not itself a
/// definite truth value — it is UNKNOWN (LANG.VALUES.TRUTH).
///
/// The family lifts (`lifting: elementwise`), so this accessor sees one lane
/// at a time: [`lift_lanes`] has already aligned the operands, and a Vector
/// reaching here is a Vector standing where a truth value belongs, which is
/// the `nonTruthValue` it reports. Masks are built by the comparison Words,
/// which lift the same way, so `[ 1 2 3 ] [ 2 ] GT [ 1 2 3 ] [ 2 ] LT AND` is
/// an ordinary phrase rather than a shape error.
///
/// So a scalar is not an operand. These Words used to select between a Boolean
/// path and an element-wise numeric path based on operand shape, which made
/// `0` and `1` behave as truth values and contradicted LANG.VALUES.DISJOINT
/// ("FALSE is not scalar zero, TRUE is not scalar one"). The numeric path also
/// returned a Scalar that the display rendered as `TRUE`, so `1 1 AND` printed
/// `TRUE` while `1 1 AND TRUE EQ` decided FALSE. A caller who means a numeric
/// test writes it: `0 EQ NOT`.
fn operand_truth(value: &Value) -> Result<bool> {
    value.as_truth().ok_or_else(|| {
        AjisaiError::declared(
            "nonTruthValue",
            "expected a truth value, got a non-truth value",
        )
    })
}

/// The definite truth of a `booleanLogic` operand, or `None` for UNKNOWN.
///
/// UNKNOWN has no dedicated data representation (LANG.VALUES.TRUTH): any NIL
/// standing in truth position reads as UNKNOWN, whatever its reason. A
/// non-NIL, non-Boolean operand is still the `nonTruthValue` ERROR that
/// [`operand_truth`] raises.
fn truth_or_unknown(value: &Value) -> Result<Option<bool>> {
    if value.is_nil() {
        return Ok(None);
    }
    operand_truth(value).map(Some)
}

/// Conjunction under the strong Kleene table (LANG.VALUES.TRUTH): FALSE
/// absorbs into `AND` even against an UNKNOWN operand, because the absorbing
/// value is decided by the definite operand alone. Only where neither operand
/// is FALSE does an UNKNOWN operand surface in the result — the left
/// operand's, when both are UNKNOWN, matching left-to-right evaluation order.
fn compute_conjunction(a: &Value, b: &Value) -> Result<Value> {
    match (truth_or_unknown(a)?, truth_or_unknown(b)?) {
        (Some(x), Some(y)) => Ok(Value::from_bool(x && y)),
        (Some(false), None) | (None, Some(false)) => Ok(Value::from_bool(false)),
        (Some(true), None) => Ok(b.clone()),
        (None, Some(true)) | (None, None) => Ok(a.clone()),
    }
}

/// `AND` over whole operands: the scalar law above, applied lane by lane
/// (LANG.COLLECTIONS.LIFT).
fn lifted_conjunction(a: &Value, b: &Value) -> Result<Value> {
    lift_lanes([a, b], &|[x, y]| compute_conjunction(x, y))
}

/// `SELECT`'s scalar law: a definite truth chooses one of the two values it
/// was handed, and UNKNOWN chooses neither.
///
/// Nothing is evaluated here. Both candidates are values the program already
/// built, so the work that produced them happened before `SELECT` ran, once
/// each and in the order they were written — which is why `SELECT` is `pure`
/// and `const` on the step axis where `COND` was `unbounded` on all three.
fn compute_selection(when_true: &Value, when_false: &Value, mask: &Value) -> Result<Value> {
    match truth_or_unknown(mask)? {
        Some(true) => Ok(when_true.clone()),
        Some(false) => Ok(when_false.clone()),
        // UNKNOWN chooses neither: the answer is the absence the truth
        // operand carried, reason intact, so `NIL-REASON` can still say why.
        None => Ok(mask.clone()),
    }
}

fn compute_inverted_value(val: &Value) -> Result<Value> {
    // NOT has no second operand to absorb into, so UNKNOWN simply inverts to
    // UNKNOWN: an absent operand flows out unchanged, keeping its reason
    // (LANG.VALUES.TRUTH's NOT row).
    if val.is_nil() {
        return Ok(val.clone());
    }
    Ok(Value::from_bool(!operand_truth(val)?))
}

pub fn op_not(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let result = match lift_lanes([&val], &|[x]| compute_inverted_value(x)) {
        Ok(v) => v,
        Err(e) => {
            interp.stack.push(val);
            return Err(e);
        }
    };

    interp.stack.push(result);
    Ok(())
}

pub fn op_and(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let result = match lifted_conjunction(&a_val, &b_val) {
        Ok(v) => v,
        Err(e) => {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
            return Err(e);
        }
    };
    interp.stack.push(result);
    Ok(())
}

/// `SELECT` — the conditional.
///
/// `[ whenTrue ] [ whenFalse ] [ mask ] SELECT` answers one of the two
/// candidates per lane. The truth operand comes last because that is where
/// every Word puts the operand that decides what it does, and because
/// `NIL?` leaves its answer exactly there: `[ 0 ] X X NIL? SELECT` reads as
/// "0 if X is absent, else X" with nothing moved on the stack.
///
/// Unlike the `COND` this replaces, `SELECT` evaluates nothing and holds no
/// frame: its operands are ordinary values, aligned by the one lifting rule
/// (LANG.COLLECTIONS.LIFT), so branching is no longer a construct with laws
/// of its own. There is no else-clause to reach and no clause set to exhaust
/// — two candidates and a truth are total by construction.
pub fn op_select(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 3 {
        return Err(AjisaiError::StackUnderflow);
    }

    let mask = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let when_false = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let when_true = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let result = match lift_lanes([&when_true, &when_false, &mask], &|[t, f, m]| {
        compute_selection(t, f, m)
    }) {
        Ok(v) => v,
        Err(e) => {
            interp.stack.push(when_true);
            interp.stack.push(when_false);
            interp.stack.push(mask);
            return Err(e);
        }
    };

    interp.stack.push(result);
    Ok(())
}
