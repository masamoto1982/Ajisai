// rust/src/interpreter/comparison.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, BracketType};
use crate::types::fraction::Fraction;

fn binary_comparison_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> bool,
{
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b_vec = interp.stack.pop().unwrap();
    let a_vec = interp.stack.pop().unwrap();
    
    let a_val = extract_single_element_value(&a_vec)?;
    let b_val = extract_single_element_value(&b_vec)?;
    
    let result = match (&a_val.val_type, &b_val.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            Value { val_type: ValueType::Boolean(op(n1, n2)) }
        },
        _ => return Err(AjisaiError::type_error("number", "other type")),
    };
    
    interp.stack.push(wrap_result_value(result));
    Ok(())
}

fn extract_single_element_value(vector_val: &Value) -> Result<&Value> {
    match &vector_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => Ok(&v[0]),
        ValueType::Vector(_, _) => Err(AjisaiError::from("Multi-element vector not supported in comparison/logic")),
        _ => Err(AjisaiError::type_error("single-element vector", "other type")),
    }
}

fn wrap_result_value(value: Value) -> Value {
    Value {
        val_type: ValueType::Vector(vec![value], BracketType::Square)
    }
}

pub fn op_lt(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.lt(b))
}

pub fn op_le(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.le(b))
}

pub fn op_gt(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.gt(b))
}

pub fn op_ge(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.ge(b))
}

pub fn op_eq(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b_vec = interp.stack.pop().unwrap();
    let a_vec = interp.stack.pop().unwrap();
    
    let result = Value { val_type: ValueType::Boolean(a_vec == b_vec) };
    interp.stack.push(wrap_result_value(result));
    Ok(())
}

pub fn op_not(interp: &mut Interpreter) -> Result<()> {
    let val_vec = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let val = extract_single_element_value(&val_vec)?;
    
    let result = match &val.val_type {
        ValueType::Boolean(b) => Value { val_type: ValueType::Boolean(!b) },
        ValueType::Nil => {
            // "No change is an error" principle
            // Re-push the original value before returning error
            interp.stack.push(val_vec);
            return Err(AjisaiError::from("NOT on NIL resulted in no change"));
        },
        _ => return Err(AjisaiError::type_error("boolean or nil", "other type")),
    };
    
    interp.stack.push(wrap_result_value(result));
    Ok(())
}

pub fn op_and(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 { return Err(AjisaiError::StackUnderflow); }
    let b_vec = interp.stack.pop().unwrap();
    let a_vec = interp.stack.pop().unwrap();
    let a_val = extract_single_element_value(&a_vec)?;
    let b_val = extract_single_element_value(&b_vec)?;
    
    let result = match (&a_val.val_type, &b_val.val_type) {
        (ValueType::Boolean(a), ValueType::Boolean(b)) => Value { val_type: ValueType::Boolean(*a && *b) },
        (ValueType::Boolean(false), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(false)) => Value { val_type: ValueType::Boolean(false) },
        (ValueType::Boolean(true), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(true)) | (ValueType::Nil, ValueType::Nil) => Value { val_type: ValueType::Nil },
        _ => return Err(AjisaiError::type_error("boolean or nil", "other types")),
    };
    interp.stack.push(wrap_result_value(result));
    Ok(())
}

pub fn op_or(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 { return Err(AjisaiError::StackUnderflow); }
    let b_vec = interp.stack.pop().unwrap();
    let a_vec = interp.stack.pop().unwrap();
    let a_val = extract_single_element_value(&a_vec)?;
    let b_val = extract_single_element_value(&b_vec)?;

    let result = match (&a_val.val_type, &b_val.val_type) {
        (ValueType::Boolean(a), ValueType::Boolean(b)) => Value { val_type: ValueType::Boolean(*a || *b) },
        (ValueType::Boolean(true), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(true)) => Value { val_type: ValueType::Boolean(true) },
        (ValueType::Boolean(false), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(false)) | (ValueType::Nil, ValueType::Nil) => Value { val_type: ValueType::Nil },
        _ => return Err(AjisaiError::type_error("boolean or nil", "other types")),
    };
    interp.stack.push(wrap_result_value(result));
    Ok(())
}
