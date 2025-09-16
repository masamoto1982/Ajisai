// rust/src/interpreter/arithmetic.rs - ビルドエラー修正版

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, Fraction};
use num_traits::Zero;
use web_sys::console;
use wasm_bindgen::JsValue;

fn extract_single_element_value(vector_val: &Value) -> Result<&Value> {
    match &vector_val.val_type {
        ValueType::Vector(v) if v.len() == 1 => Ok(&v[0]),
        ValueType::Vector(_) => Err(AjisaiError::from("Multi-element vector not supported in arithmetic")),
        _ => Err(AjisaiError::type_error("single-element vector", "other type")),
    }
}

fn wrap_result_value(value: Value) -> Value {
    Value {
        val_type: ValueType::Vector(vec![value])
    }
}

fn binary_arithmetic_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> Fraction,
{
    console::log_1(&JsValue::from_str("=== binary_arithmetic_op ==="));
    
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let b_vec = interp.workspace.pop().unwrap();
    let a_vec = interp.workspace.pop().unwrap();
    
    console::log_1(&JsValue::from_str(&format!("a_vec: {:?}, b_vec: {:?}", a_vec, b_vec)));
    
    let a_val = extract_single_element_value(&a_vec)?;
    let b_val = extract_single_element_value(&b_vec)?;
    
    let result = match (&a_val.val_type, &b_val.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            console::log_1(&JsValue::from_str(&format!("Operating on numbers: {}/{} and {}/{}", 
                n1.numerator, n1.denominator, n2.numerator, n2.denominator)));
            Value { val_type: ValueType::Number(op(n1, n2)) }
        },
        _ => return Err(AjisaiError::type_error("number", "other type")),
    };
    
    console::log_1(&JsValue::from_str(&format!("Result: {:?}", result)));
    interp.workspace.push(wrap_result_value(result));
    Ok(())
}

fn binary_comparison_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> bool,
{
    console::log_1(&JsValue::from_str("=== binary_comparison_op ==="));
    
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let b_vec = interp.workspace.pop().unwrap();
    let a_vec = interp.workspace.pop().unwrap();
    
    let a_val = extract_single_element_value(&a_vec)?;
    let b_val = extract_single_element_value(&b_vec)?;
    
    let result = match (&a_val.val_type, &b_val.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            Value { val_type: ValueType::Boolean(op(n1, n2)) }
        },
        _ => return Err(AjisaiError::type_error("number", "other type")),
    };
    
    interp.workspace.push(wrap_result_value(result));
    Ok(())
}

pub fn op_add(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_add ==="));
    binary_arithmetic_op(interp, |a, b| a.add(b))
}

pub fn op_sub(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_sub ==="));
    binary_arithmetic_op(interp, |a, b| a.sub(b))
}

pub fn op_mul(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_mul ==="));
    binary_arithmetic_op(interp, |a, b| a.mul(b))
}

pub fn op_div(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_div ==="));
    
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let b_vec = interp.workspace.pop().unwrap();
    let a_vec = interp.workspace.pop().unwrap();
    
    let a_val = extract_single_element_value(&a_vec)?;
    let b_val = extract_single_element_value(&b_vec)?;
    
    if let ValueType::Number(n) = &b_val.val_type {
        if n.numerator.is_zero() {
            return Err(AjisaiError::DivisionByZero);
        }
    }
    
    let result = match (&a_val.val_type, &b_val.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            Value { val_type: ValueType::Number(n1.div(n2)) }
        },
        _ => return Err(AjisaiError::type_error("number", "other type")),
    };
    
    interp.workspace.push(wrap_result_value(result));
    Ok(())
}

pub fn op_lt(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_lt ==="));
    binary_comparison_op(interp, |a, b| a.lt(b))
}

pub fn op_le(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_le ==="));
    binary_comparison_op(interp, |a, b| a.le(b))
}

pub fn op_gt(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_gt ==="));
    binary_comparison_op(interp, |a, b| a.gt(b))
}

pub fn op_ge(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_ge ==="));
    binary_comparison_op(interp, |a, b| a.ge(b))
}

pub fn op_eq(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_eq ==="));
    
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let b_vec = interp.workspace.pop().unwrap();
    let a_vec = interp.workspace.pop().unwrap();
    
    let result = Value { val_type: ValueType::Boolean(a_vec == b_vec) };
    interp.workspace.push(wrap_result_value(result));
    Ok(())
}

pub fn op_not(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_not ==="));
    
    let val_vec = interp.workspace.pop().ok_or(AjisaiError::WorkspaceUnderflow)?;
    let val = extract_single_element_value(&val_vec)?;
    
    let result = match &val.val_type {
        ValueType::Boolean(b) => Value { val_type: ValueType::Boolean(!b) },
        ValueType::Nil => Value { val_type: ValueType::Nil },
        _ => return Err(AjisaiError::type_error("boolean or nil", "other type")),
    };
    
    interp.workspace.push(wrap_result_value(result));
    Ok(())
}

pub fn op_and(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_and ==="));
    
    if interp.workspace.len() < 2 { 
        return Err(AjisaiError::WorkspaceUnderflow); 
    }
    let b_vec = interp.workspace.pop().unwrap();
    let a_vec = interp.workspace.pop().unwrap();
    let a_val = extract_single_element_value(&a_vec)?;
    let b_val = extract_single_element_value(&b_vec)?;
    
    let result = match (&a_val.val_type, &b_val.val_type) {
        (ValueType::Boolean(a), ValueType::Boolean(b)) => Value { val_type: ValueType::Boolean(*a && *b) },
        (ValueType::Boolean(false), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(false)) => Value { val_type: ValueType::Boolean(false) },
        (ValueType::Boolean(true), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(true)) | (ValueType::Nil, ValueType::Nil) => Value { val_type: ValueType::Nil },
        _ => return Err(AjisaiError::type_error("boolean or nil", "other types")),
    };
    interp.workspace.push(wrap_result_value(result));
    Ok(())
}

pub fn op_or(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_or ==="));
    
    if interp.workspace.len() < 2 { 
        return Err(AjisaiError::WorkspaceUnderflow); 
    }
    let b_vec = interp.workspace.pop().unwrap();
    let a_vec = interp.workspace.pop().unwrap();
    let a_val = extract_single_element_value(&a_vec)?;
    let b_val = extract_single_element_value(&b_vec)?;

    let result = match (&a_val.val_type, &b_val.val_type) {
        (ValueType::Boolean(a), ValueType::Boolean(b)) => Value { val_type: ValueType::Boolean(*a || *b) },
        (ValueType::Boolean(true), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(true)) => Value { val_type: ValueType::Boolean(true) },
        (ValueType::Boolean(false), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(false)) | (ValueType::Nil, ValueType::Nil) => Value { val_type: ValueType::Nil },
        _ => return Err(AjisaiError::type_error("boolean or nil", "other types")),
    };
    interp.workspace.push(wrap_result_value(result));
    Ok(())
}
