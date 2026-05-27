use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{
    extract_operands, nil_passthrough_binary, nil_passthrough_unary, push_result,
};
use crate::interpreter::{Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Value};

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

fn apply_unary<F>(interp: &mut Interpreter, word: &str, op: F) -> Result<()>
where
    F: Fn(Fraction) -> Fraction,
{
    require_stack_top(interp, word)?;
    if nil_passthrough_unary(interp) {
        return Ok(());
    }
    let operands = extract_operands(interp, 1)?;
    let scalar = match extract_scalar(&operands[0], word) {
        Ok(s) => s,
        Err(e) => {
            if interp.consumption_mode != crate::interpreter::ConsumptionMode::Keep {
                interp.stack.extend(operands);
            }
            return Err(e);
        }
    };
    push_result(interp, Value::from_fraction(op(scalar)));
    interp.semantic_registry.push_hint(Interpretation::RawNumber);
    Ok(())
}

fn apply_binary<F>(interp: &mut Interpreter, word: &str, op: F) -> Result<()>
where
    F: Fn(Fraction, Fraction) -> Fraction,
{
    require_stack_top(interp, word)?;
    if nil_passthrough_binary(interp) {
        return Ok(());
    }
    let operands = extract_operands(interp, 2)?;
    let parsed = (
        extract_scalar(&operands[0], word),
        extract_scalar(&operands[1], word),
    );
    let (a, b) = match parsed {
        (Ok(a), Ok(b)) => (a, b),
        _ => {
            if interp.consumption_mode != crate::interpreter::ConsumptionMode::Keep {
                interp.stack.extend(operands);
            }
            return Err(AjisaiError::from(format!("{}: expected two numbers", word)));
        }
    };
    push_result(interp, Value::from_fraction(op(a, b)));
    interp.semantic_registry.push_hint(Interpretation::RawNumber);
    Ok(())
}

pub(crate) fn op_abs(interp: &mut Interpreter) -> Result<()> {
    apply_unary(interp, "ABS", |f| f.abs())
}

pub(crate) fn op_neg(interp: &mut Interpreter) -> Result<()> {
    apply_unary(interp, "NEG", |f| Fraction::from(0).sub(&f))
}

pub(crate) fn op_sign(interp: &mut Interpreter) -> Result<()> {
    apply_unary(interp, "SIGN", |f| {
        if f.is_zero() {
            Fraction::from(0)
        } else if f.lt(&Fraction::from(0)) {
            Fraction::from(-1)
        } else {
            Fraction::from(1)
        }
    })
}

pub(crate) fn op_min(interp: &mut Interpreter) -> Result<()> {
    apply_binary(interp, "MIN", |a, b| if a.le(&b) { a } else { b })
}

pub(crate) fn op_max(interp: &mut Interpreter) -> Result<()> {
    apply_binary(interp, "MAX", |a, b| if a.ge(&b) { a } else { b })
}
