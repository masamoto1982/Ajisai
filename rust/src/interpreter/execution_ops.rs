// rust/src/interpreter/execution_ops.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType, Value};
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};

fn get_integer_from_value(value: &Value) -> Result<i64> {
    match &value.val_type {
        ValueType::Vector(ref v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) if n.denominator == BigInt::one() => {
                    n.numerator.to_i64().ok_or_else(|| AjisaiError::from("Integer is too large"))
                },
                _ => Err(AjisaiError::type_error("integer", "other type")),
            }
        },
        _ => Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    }
}

pub fn op_call(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop().ok_or(AjisaiError::WorkspaceUnderflow)?;
    match val.val_type {
        ValueType::Quotation(tokens) => {
            interp.execute_tokens(&tokens)
        },
        _ => Err(AjisaiError::type_error("quotation", "other type")),
    }
}

pub fn op_repeat(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    let quotation_val = interp.workspace.pop().unwrap();
    let count_val = interp.workspace.pop().unwrap();

    let count = get_integer_from_value(&count_val)?;
    if count < 0 {
        return Err(AjisaiError::from("Repeat count cannot be negative"));
    }

    match quotation_val.val_type {
        ValueType::Quotation(tokens) => {
            for _ in 0..count {
                interp.execute_tokens(&tokens)?;
            }
            Ok(())
        },
        _ => Err(AjisaiError::type_error("quotation", "other type")),
    }
}
