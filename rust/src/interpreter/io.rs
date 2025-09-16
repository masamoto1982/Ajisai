// rust/src/interpreter/io.rs - ビルドエラー修正版

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::ValueType;
use num_bigint::BigInt;
use num_traits::{Zero, One, ToPrimitive};
use web_sys::console;
use wasm_bindgen::JsValue;

pub fn op_print(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_print ==="));
    
    let val = interp.workspace.pop().ok_or(AjisaiError::WorkspaceUnderflow)?;
    console::log_1(&JsValue::from_str(&format!("Printing value: {:?}", val)));
    
    interp.output_buffer.push_str(&format!("{} ", val));
    Ok(())
}

pub fn op_cr(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_cr ==="));
    interp.output_buffer.push('\n');
    Ok(())
}

pub fn op_space(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_space ==="));
    interp.output_buffer.push(' ');
    Ok(())
}

pub fn op_spaces(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_spaces ==="));
    
    let val = interp.workspace.pop().ok_or(AjisaiError::WorkspaceUnderflow)?;
    match val.val_type {
        ValueType::Vector(v) if v.len() == 1 => match &v[0].val_type {
            ValueType::Number(n) => {
                if n.denominator == BigInt::one() && n.numerator >= BigInt::zero() {
                    if let Some(count) = n.numerator.to_usize() {
                        interp.output_buffer.push_str(&" ".repeat(count));
                        return Ok(());
                    }
                }
                Err(AjisaiError::from("SPACES requires a non-negative integer"))
            },
            _ => Err(AjisaiError::type_error("number", "other type")),
        },
        _ => Err(AjisaiError::type_error("single-element vector with number", "other type")),
    }
}

pub fn op_emit(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_emit ==="));
    
    let val = interp.workspace.pop().ok_or(AjisaiError::WorkspaceUnderflow)?;
    match val.val_type {
        ValueType::Vector(v) if v.len() == 1 => match &v[0].val_type {
            ValueType::Number(n) => {
                if n.denominator == BigInt::one() && n.numerator >= BigInt::zero() && n.numerator <= BigInt::from(255) {
                    if let Some(byte) = n.numerator.to_u8() {
                        interp.output_buffer.push(byte as char);
                        return Ok(());
                    }
                }
                Err(AjisaiError::from("EMIT requires an integer between 0 and 255"))
            },
            _ => Err(AjisaiError::type_error("number", "other type")),
        },
        _ => Err(AjisaiError::type_error("single-element vector with number", "other type")),
    }
}
