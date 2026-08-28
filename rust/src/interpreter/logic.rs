use crate::error::{AjisaiError, Result};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::types::{Interpretation, Value};

/// The truth value of a `booleanLogic` operand.
///
/// The Boolean domain is the *whole* definite input domain of `AND`, `OR`,
/// and `NOT`: `spec/semantic-families.json` gives the family
/// `truth: threeValued`, each contract registers `nonTruthValue` as its error
/// condition, and the family carries no `lifting` key — element-wise
/// broadcast belongs to `exactArithmetic` and `comparison`
/// (LANG.COLLECTIONS.LIFT), not here. NIL is handled separately by
/// [`truth_or_unknown`], not by this accessor, because NIL is not itself a
/// definite truth value — it is UNKNOWN (LANG.VALUES.TRUTH).
///
/// So a scalar is not an operand. These Words used to select between a Boolean
/// path and an element-wise numeric path based on operand shape, which made
/// `0` and `1` behave as truth values and contradicted LANG.VALUES.DISJOINT
/// ("FALSE is not scalar zero, TRUE is not scalar one"). The numeric path also
/// returned a Scalar that the display rendered as `TRUE`, so `1 1 AND` printed
/// `TRUE` while `1 1 AND TRUE EQ` decided FALSE. A caller who means a numeric
/// test writes it: `0 NEQ`.
fn operand_truth(value: &Value) -> Result<bool> {
    value
        .as_truth()
        .ok_or_else(|| AjisaiError::create_structure_error("truth value", "non-truth value"))
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

/// Binary Boolean combination under the strong Kleene tables
/// (LANG.VALUES.TRUTH): FALSE absorbs into `AND` and TRUE absorbs into `OR`
/// even against an UNKNOWN operand, because the absorbing value is decided by
/// the definite operand alone. Only where neither operand is the absorbing
/// value does an UNKNOWN operand surface in the result — the left operand's,
/// when both are UNKNOWN, matching left-to-right evaluation order.
fn compute_boolean_binary(and: bool, a: &Value, b: &Value) -> Result<Value> {
    let absorbing = !and; // AND absorbs on FALSE, OR absorbs on TRUE.
    match (truth_or_unknown(a)?, truth_or_unknown(b)?) {
        (Some(x), Some(y)) => Ok(Value::from_bool(if and { x && y } else { x || y })),
        (Some(x), None) => {
            if x == absorbing {
                Ok(Value::from_bool(absorbing))
            } else {
                Ok(as_unknown(b))
            }
        }
        (None, Some(y)) => {
            if y == absorbing {
                Ok(Value::from_bool(absorbing))
            } else {
                Ok(as_unknown(a))
            }
        }
        (None, None) => Ok(as_unknown(a)),
    }
}

fn compute_inverted_value(val: &Value) -> Result<Value> {
    // NOT has no second operand to absorb into, so UNKNOWN simply inverts to
    // UNKNOWN: an absent operand flows out unchanged, keeping its reason
    // (LANG.VALUES.TRUTH's NOT row).
    if val.is_nil() {
        return Ok(as_unknown(val));
    }
    Ok(Value::from_bool(!operand_truth(val)?))
}

/// A NIL operand read in truth position, marked as the logical UNKNOWN (U):
/// `ValueData::Nil` carrying the `TruthValue` hint (LANG.VALUES.TRUTH). No
/// vocabulary Word could construct U directly before this — the exact
/// comparison domain always decides (Tier ≤ 1) — so `AND`/`OR`/`NOT` are the
/// first to make U a value a program can actually observe, not just a value
/// the type system reserves room for.
fn as_unknown(value: &Value) -> Value {
    let mut unknown = value.clone();
    unknown.hint = Interpretation::TruthValue;
    unknown
}

/// Push a `booleanLogic` result and mark it as truth-valued on the semantic
/// plane too (SPEC observation axis `truthValue`), mirroring
/// `comparison::push_boolean_result`. [`as_unknown`] already gives a
/// propagated UNKNOWN the right `hint` for `Value::truth_value()`; this also
/// sets the stack-level role the protocol boundary reads, so observation
/// agrees whichever path a consumer reads it through.
fn push_truth_result(interp: &mut Interpreter, result: Value) {
    interp.stack.push(result);
    let stack_len = interp.stack.len();
    interp
        .stack
        .set_role_at(stack_len - 1, Interpretation::TruthValue);
}

pub fn op_not(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    let val = if is_keep_mode {
        interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    let result = match compute_inverted_value(&val) {
        Ok(v) => v,
        Err(e) => {
            if !is_keep_mode {
                interp.stack.push(val);
            }
            return Err(e);
        }
    };

    push_truth_result(interp, result);
    Ok(())
}

pub fn op_and(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let (a_val, b_val) = if is_keep_mode {
        let stack_len = interp.stack.len();
        (
            interp.stack[stack_len - 2].clone(),
            interp.stack[stack_len - 1].clone(),
        )
    } else {
        let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        (a_val, b_val)
    };

    let result = match compute_boolean_binary(true, &a_val, &b_val) {
        Ok(v) => v,
        Err(e) => {
            if !is_keep_mode {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
            }
            return Err(e);
        }
    };
    push_truth_result(interp, result);
    Ok(())
}

pub fn op_or(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let (a_val, b_val) = if is_keep_mode {
        let stack_len = interp.stack.len();
        (
            interp.stack[stack_len - 2].clone(),
            interp.stack[stack_len - 1].clone(),
        )
    } else {
        let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        (a_val, b_val)
    };

    let result = match compute_boolean_binary(false, &a_val, &b_val) {
        Ok(v) => v,
        Err(e) => {
            if !is_keep_mode {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
            }
            return Err(e);
        }
    };
    push_truth_result(interp, result);
    Ok(())
}
