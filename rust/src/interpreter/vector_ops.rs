// rust/src/interpreter/vector_ops.rs (BigInt対応・エラー修正版)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, Fraction, BracketType};
use num_bigint::BigInt;
use num_traits::{Zero, One, ToPrimitive};

fn get_index_from_value(value: &Value) -> Result<BigInt, AjisaiError> {
    match &value.val_type {
        ValueType::Vector(ref v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) if n.denominator == BigInt::one() => Ok(n.numerator.clone()),
                _ => Err(AjisaiError::type_error("integer index", "other type")),
            }
        },
        _ => Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    }
}

pub fn op_get(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 { return Err(AjisaiError::from("GET requires vector and index")); }
    let index_val = interp.workspace.pop().unwrap();
    let target_val = interp.workspace.pop().unwrap();
    
    let index_bigint = get_index_from_value(&index_val)?;
    let index = index_bigint.to_i64().ok_or_else(|| AjisaiError::from("Index is too large"))?;

    match target_val.val_type {
        ValueType::Vector(v, bracket_type) => {
            if v.is_empty() { return Err(AjisaiError::IndexOutOfBounds { index, length: 0 }); }
            let len = v.len();
            let actual_index = if index < 0 { (len as i64 + index) as usize } else { index as usize };

            if actual_index < len {
                let result = Value { val_type: ValueType::Vector(vec![v[actual_index].clone()], bracket_type) };
                interp.workspace.push(result);
                Ok(())
            } else {
                Err(AjisaiError::IndexOutOfBounds { index, length: len })
            }
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// ... (op_insert, op_replace, op_remove are similarly corrected) ...

pub fn op_length(interp: &mut Interpreter) -> Result<()> {
    let target_val = interp.workspace.pop().ok_or(AjisaiError::WorkspaceUnderflow)?;
    match target_val.val_type {
        ValueType::Vector(v, _) => {
            let length_val = Value {
                val_type: ValueType::Number(Fraction::new(BigInt::from(v.len()), BigInt::one()))
            };
            interp.workspace.push(Value { val_type: ValueType::Vector(vec![length_val], BracketType::Square) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// ... (op_take, op_drop_vector, op_repeat, op_split are similarly corrected) ...
pub fn op_insert(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 3 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let element = interp.workspace.pop().unwrap();
    let index_val = interp.workspace.pop().unwrap();
    let vector_val = interp.workspace.pop().unwrap();
    
    let index = get_index_from_value(&index_val)?;
    
    let insert_element = match element.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => v[0].clone(),
        _ => return Err(AjisaiError::type_error("single-element vector", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(mut v, bracket_type) => {
            let len = v.len() as i64;
            let index_i64 = index.to_i64().unwrap_or(i64::MAX);

            let insert_index = if index_i64 < 0 {
                (len + index_i64 + 1).max(0) as usize
            } else {
                (index_i64 as usize).min(v.len())
            };
            
            v.insert(insert_index, insert_element);
            interp.workspace.push(Value { val_type: ValueType::Vector(v, bracket_type) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

pub fn op_replace(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 3 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let new_element = interp.workspace.pop().unwrap();
    let index_val = interp.workspace.pop().unwrap();
    let vector_val = interp.workspace.pop().unwrap();
    
    let index = get_index_from_value(&index_val)?;
    
    let replace_element = match new_element.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => v[0].clone(),
        _ => return Err(AjisaiError::type_error("single-element vector", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(mut v, bracket_type) => {
            let len = v.len();
            let index_i64 = index.to_i64().ok_or_else(|| AjisaiError::from("Index too large"))?;
            
            let actual_index = if index_i64 < 0 {
                (len as i64 + index_i64) as usize
            } else {
                index_i64 as usize
            };
            
            if actual_index < len {
                v[actual_index] = replace_element;
                interp.workspace.push(Value { val_type: ValueType::Vector(v, bracket_type) });
                Ok(())
            } else {
                Err(AjisaiError::IndexOutOfBounds { index: index_i64, length: len })
            }
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

pub fn op_remove(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 { return Err(AjisaiError::WorkspaceUnderflow); }
    let index_val = interp.workspace.pop().unwrap();
    let vector_val = interp.workspace.pop().unwrap();
    let index = get_index_from_value(&index_val)?;
    let index_i64 = index.to_i64().ok_or_else(|| AjisaiError::from("Index too large"))?;

    match vector_val.val_type {
        ValueType::Vector(mut v, bracket_type) => {
            let len = v.len();
            let actual_index = if index_i64 < 0 { (len as i64 + index_i64) as usize } else { index_i64 as usize };
            if actual_index < len {
                v.remove(actual_index);
                interp.workspace.push(Value { val_type: ValueType::Vector(v, bracket_type) });
                Ok(())
            } else {
                Err(AjisaiError::IndexOutOfBounds { index: index_i64, length: len })
            }
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

pub fn op_take(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 { return Err(AjisaiError::WorkspaceUnderflow); }
    let count_val = interp.workspace.pop().unwrap();
    let vector_val = interp.workspace.pop().unwrap();
    let count = get_index_from_value(&count_val)?;
    
    match vector_val.val_type {
        ValueType::Vector(v, bracket_type) => {
            let count_i64 = count.to_i64().ok_or_else(|| AjisaiError::from("Count is too large"))?;
            let len = v.len();
            let result = if count_i64 < 0 {
                let abs_count = (-count_i64) as usize;
                if abs_count > len { return Err(AjisaiError::from("Take count exceeds vector length")); }
                v[len - abs_count..].to_vec()
            } else {
                let take_count = count_i64 as usize;
                if take_count > len { return Err(AjisaiError::from("Take count exceeds vector length")); }
                v[..take_count].to_vec()
            };
            interp.workspace.push(Value { val_type: ValueType::Vector(result, bracket_type) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

pub fn op_drop_vector(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 { return Err(AjisaiError::WorkspaceUnderflow); }
    let count_val = interp.workspace.pop().unwrap();
    let vector_val = interp.workspace.pop().unwrap();
    let count = get_index_from_value(&count_val)?;

    match vector_val.val_type {
        ValueType::Vector(v, bracket_type) => {
            let count_i64 = count.to_i64().ok_or_else(|| AjisaiError::from("Drop count is too large"))?;
            let len = v.len();
            let result = if count_i64 < 0 {
                let abs_count = (-count_i64) as usize;
                if abs_count > len { return Err(AjisaiError::from("Drop count exceeds vector length")); }
                v[..len - abs_count].to_vec()
            } else {
                let drop_count = count_i64 as usize;
                 if drop_count > len { return Err(AjisaiError::from("Drop count exceeds vector length")); }
                v[drop_count..].to_vec()
            };
            interp.workspace.push(Value { val_type: ValueType::Vector(result, bracket_type) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

pub fn op_repeat(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 { return Err(AjisaiError::WorkspaceUnderflow); }
    let times_val = interp.workspace.pop().unwrap();
    let elem_val = interp.workspace.pop().unwrap();
    let times = get_index_from_value(&times_val)?;
    
    if times < BigInt::zero() { return Err(AjisaiError::from("Repeat times must be non-negative")); }
    let times_usize = times.to_usize().ok_or_else(|| AjisaiError::from("Repeat count is too large"))?;

    match elem_val.val_type {
        ValueType::Vector(v, bracket_type) => {
            let mut result = Vec::new();
            for _ in 0..times_usize {
                result.extend_from_slice(&v);
            }
            interp.workspace.push(Value { val_type: ValueType::Vector(result, bracket_type) });
        },
        _ => return Err(AjisaiError::type_error("vector", "other type")),
    }
    Ok(())
}
