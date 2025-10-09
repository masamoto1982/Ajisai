// rust/src/interpreter/arithmetic.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}, OperationTarget};
use crate::types::{Value, ValueType, Fraction, BracketType};
use num_traits::{Zero, One, ToPrimitive};

fn binary_arithmetic_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> Fraction,
{
    match interp.operation_target {
        OperationTarget::StackTop => {
            let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            if let (ValueType::Vector(v_a, bt), ValueType::Vector(v_b, _)) = (a_val.val_type, b_val.val_type) {
                if v_a.len() != v_b.len() { return Err(AjisaiError::from("Vectors must have same length for element-wise operation")); }
                
                let mut result_v = Vec::new();
                for (item_a, item_b) in v_a.iter().zip(v_b.iter()) {
                     if let (ValueType::Number(n_a), ValueType::Number(n_b)) = (&item_a.val_type, &item_b.val_type) {
                         result_v.push(Value { val_type: ValueType::Number(op(n_a, n_b)) });
                     } else {
                         return Err(AjisaiError::type_error("number", "other type"));
                     }
                }
                interp.stack.push(Value { val_type: ValueType::Vector(result_v, bt) });
            } else {
                 return Err(AjisaiError::type_error("vector", "other type"));
            }
        }
        OperationTarget::Stack => {
            let count = interp.stack.pop()
                .and_then(|v| if let ValueType::Vector(vec, _) = v.val_type { vec.into_iter().next() } else { None })
                .and_then(|v| if let ValueType::Number(n) = v.val_type { n.to_i64() } else { None })
                .ok_or("STACK operation requires a count")? as usize;

            if interp.stack.len() < count { return Err(AjisaiError::StackUnderflow); }
            
            let mut items: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();
            let mut result_vec = Vec::new();

            if let Some(first_item) = items.first() {
                if let ValueType::Vector(v, _) = &first_item.val_type {
                    for i in 0..v.len() {
                        let mut acc = if let ValueType::Number(n) = &v[i].val_type { n.clone() } else { return Err(AjisaiError::type_error("number", "other type")) };
                        for j in 1..count {
                            if let ValueType::Vector(v_j, _) = &items[j].val_type {
                                 if let ValueType::Number(n_j) = &v_j[i].val_type {
                                     acc = op(&acc, n_j);
                                 } else { return Err(AjisaiError::type_error("number", "other type")) }
                            }
                        }
                        result_vec.push(Value { val_type: ValueType::Number(acc) });
                    }
                }
                 interp.stack.push(Value { val_type: ValueType::Vector(result_vec, BracketType::Square) });
            }
        }
    }
    Ok(())
}


pub fn op_add(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| a.add(b))
}

pub fn op_sub(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| a.sub(b))
}

pub fn op_mul(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| a.mul(b))
}

pub fn op_div(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| {
        if b.numerator.is_zero() {
            panic!("Division by zero");
        }
        a.div(b)
    })
}

fn extract_single_element_value(vector_val: &Value) -> Result<&Value> {
    match &vector_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => Ok(&v[0]),
        ValueType::Vector(_, _) => Err(AjisaiError::from("Multi-element vector not supported in this context")),
        _ => Err(AjisaiError::type_error("single-element vector", "other type")),
    }
}

fn wrap_result_value(value: Value) -> Value {
    Value {
        val_type: ValueType::Vector(vec![value], BracketType::Square)
    }
}

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
        ValueType::Nil => Value { val_type: ValueType::Nil },
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
