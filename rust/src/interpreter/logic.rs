use crate::error::{AjisaiError, Result};
use crate::interpreter::interpreter_core::RuntimeMetrics;
use crate::interpreter::tensor_ops::{
    apply_binary_broadcast_with_metrics, apply_unary_flat_with_metrics,
};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::types::fraction::Fraction;
use crate::types::Value;

/// Whether an operand forces the Boolean path rather than element-wise numeric
/// broadcast: an operational NIL, or a definite Boolean truth value. A definite
/// Boolean must route through the Boolean path so that `AND`/`OR` of truth
/// values yield a Boolean result rather than a 0/1 number. When no operand is
/// truth-valued, `AND`/`OR` keep their element-wise numeric semantics over
/// numeric vectors.
fn forces_boolean_path(value: &Value) -> bool {
    value.is_nil() || value.as_truth().is_some()
}

/// Truthiness of a Boolean-path operand. Only reached for operands that are
/// not NIL, so the numeric fallback is a plain zero test.
fn operand_truth(value: &Value) -> bool {
    if let Some(b) = value.as_truth() {
        return b;
    }
    value.as_scalar().map(|f| !f.is_zero()).unwrap_or(false)
}

/// Binary Boolean combination with NIL passthrough: an input NIL flows to the
/// output unchanged, keeping its reason (LANG.FAILURE.PASSTHROUGH). The left
/// operand's absence wins, matching left-to-right evaluation order.
fn compute_boolean_binary(and: bool, a: &Value, b: &Value) -> Value {
    if a.is_nil() {
        return a.clone();
    }
    if b.is_nil() {
        return b.clone();
    }
    let (x, y) = (operand_truth(a), operand_truth(b));
    Value::from_bool(if and { x && y } else { x || y })
}

fn compute_inverted_fraction(f: &Fraction) -> Fraction {
    if f.is_zero() {
        Fraction::from(1)
    } else {
        Fraction::from(0)
    }
}

fn compute_inverted_value(val: &Value, metrics: Option<&mut RuntimeMetrics>) -> Result<Value> {
    // NIL passthrough: an absent operand flows out unchanged, keeping its
    // reason (LANG.FAILURE.PASSTHROUGH).
    if val.is_nil() {
        return Ok(val.clone());
    }
    // A definite Boolean inverts to the opposite Boolean (¬T=F, ¬F=T), staying
    // a truth value rather than collapsing to a 0/1 number.
    if let Some(b) = val.as_truth() {
        return Ok(Value::from_bool(!b));
    }
    if let Some(f) = val.as_scalar() {
        return Ok(Value::from_fraction(compute_inverted_fraction(f)));
    }
    apply_unary_flat_with_metrics(val, compute_inverted_fraction, metrics)
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

    let result = match compute_inverted_value(&val, Some(&mut interp.runtime_metrics)) {
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

    // Boolean path when either operand is an operational NIL or a definite
    // truth value; otherwise keep element-wise numeric broadcast.
    if forces_boolean_path(&a_val) || forces_boolean_path(&b_val) {
        interp
            .stack
            .push(compute_boolean_binary(true, &a_val, &b_val));
        return Ok(());
    }

    let result = apply_binary_broadcast_with_metrics(
        &a_val,
        &b_val,
        |a, b| {
            let a_truthy = !a.is_zero();
            let b_truthy = !b.is_zero();
            Ok(Fraction::from(if a_truthy && b_truthy { 1 } else { 0 }))
        },
        Some(&mut interp.runtime_metrics),
    )?;
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

    // Boolean path when either operand is an operational NIL or a definite
    // truth value; otherwise keep element-wise numeric broadcast.
    if forces_boolean_path(&a_val) || forces_boolean_path(&b_val) {
        interp
            .stack
            .push(compute_boolean_binary(false, &a_val, &b_val));
        return Ok(());
    }

    let result = apply_binary_broadcast_with_metrics(
        &a_val,
        &b_val,
        |a, b| {
            let a_truthy = !a.is_zero();
            let b_truthy = !b.is_zero();
            Ok(Fraction::from(if a_truthy || b_truthy { 1 } else { 0 }))
        },
        Some(&mut interp.runtime_metrics),
    )?;
    interp.stack.push(result);
    Ok(())
}
