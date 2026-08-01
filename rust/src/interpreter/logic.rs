use crate::error::{AjisaiError, Result};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::types::Value;

/// The truth value of a `booleanLogic` operand.
///
/// The Boolean domain is the *whole* input domain of `AND`, `OR`, and `NOT`:
/// `spec/semantic-families.json` gives the family `truth: twoValued`, each
/// contract registers `nonTruthValue` as its error condition, and the family
/// carries no `lifting` key — element-wise broadcast belongs to
/// `exactArithmetic` and `comparison` (LANG.COLLECTIONS.LIFT), not here.
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

/// Binary Boolean combination with NIL passthrough: an input NIL flows to the
/// output unchanged, keeping its reason (LANG.FAILURE.PASSTHROUGH). The left
/// operand's absence wins, matching left-to-right evaluation order.
fn compute_boolean_binary(and: bool, a: &Value, b: &Value) -> Result<Value> {
    if a.is_nil() {
        return Ok(a.clone());
    }
    if b.is_nil() {
        return Ok(b.clone());
    }
    let (x, y) = (operand_truth(a)?, operand_truth(b)?);
    Ok(Value::from_bool(if and { x && y } else { x || y }))
}

fn compute_inverted_value(val: &Value) -> Result<Value> {
    // NIL passthrough: an absent operand flows out unchanged, keeping its
    // reason (LANG.FAILURE.PASSTHROUGH).
    if val.is_nil() {
        return Ok(val.clone());
    }
    Ok(Value::from_bool(!operand_truth(val)?))
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

    interp.stack.push(result);
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
    interp.stack.push(result);
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
    interp.stack.push(result);
    Ok(())
}
