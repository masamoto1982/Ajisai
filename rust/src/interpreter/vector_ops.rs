// rust/src/interpreter/vector_ops.rs (ビルドエラー完全修正版)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, Fraction, BracketType};
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};
use std::collections::VecDeque;

fn get_index_from_value(value: &Value) -> Result<BigInt> {
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

// ========== 位置指定操作（0オリジン）==========

pub fn op_get(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 { return Err(AjisaiError::from("GET requires vector and index")); }
    let index_val = interp.workspace.pop().unwrap();
    let target_val = interp.workspace.pop().unwrap();
    
    let index_bigint = get_index_from_value(&index_val)?;
    let index = index_bigint.to_i64().ok_or_else(|| AjisaiError::from("Index is too large to be an i64"))?;

    match target_val.val_type {
        ValueType::Vector(v, bracket_type) => {
            if v.is_empty() { return Err(AjisaiError::IndexOutOfBounds { index, length: 0 }); }
            let len = v.len();
            let actual_index = if index < 0 {
                let pos = len as i64 + index;
                if pos < 0 { return Err(AjisaiError::IndexOutOfBounds { index, length: len }); }
                pos as usize
            } else {
                index as usize
            };

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

pub fn op_insert(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 3 { return Err(AjisaiError::WorkspaceUnderflow); }
    let element = interp.workspace.pop().unwrap();
    let index_val = interp.workspace.pop().unwrap();
    let vector_val = interp.workspace.pop().unwrap();
    
    let index = get_index_from_value(&index_val)?;
    let index_i64 = index.to_i64().ok_or_else(|| AjisaiError::from("Index is too large"))?;
    
    let insert_element = match element.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => v[0].clone(),
        _ => return Err(AjisaiError::type_error("single-element vector", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(mut v, bracket_type) => {
            let len = v.len() as i64;
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
    if interp.workspace.len() < 3 { return Err(AjisaiError::WorkspaceUnderflow); }
    let new_element = interp.workspace.pop().unwrap();
    let index_val = interp.workspace.pop().unwrap();
    let vector_val = interp.workspace.pop().unwrap();
    
    let index = get_index_from_value(&index_val)?;
    let index_i64 = index.to_i64().ok_or_else(|| AjisaiError::from("Index too large"))?;

    let replace_element = match new_element.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => v[0].clone(),
        _ => return Err(AjisaiError::type_error("single-element vector", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(mut v, bracket_type) => {
            let len = v.len();
            let actual_index = if index_i64 < 0 { (len as i64 + index_i64) as usize } else { index_i64 as usize };
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

// ========== 量指定操作（1オリジン）==========

pub fn op_length(interp: &mut Interpreter) -> Result<()> {
    let target_val = interp.workspace.pop().ok_or(AjisaiError::WorkspaceUnderflow)?;
    match target_val.val_type {
        ValueType::Vector(v, _) => {
            let len_frac = Fraction::new(BigInt::from(v.len()), BigInt::one());
            let val = Value { val_type: ValueType::Vector(vec![Value{ val_type: ValueType::Number(len_frac)}], BracketType::Square) };
            interp.workspace.push(val);
            Ok(())
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
            let len = v.len();
            let count_i64 = count.to_i64().ok_or_else(|| AjisaiError::from("Count is too large"))?;
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
            let len = v.len();
            let count_i64 = count.to_i64().ok_or_else(|| AjisaiError::from("Drop count is too large"))?;
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

pub fn op_split(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 { return Err(AjisaiError::WorkspaceUnderflow); }
    
    let mut sizes_values = VecDeque::new();
    while let Some(top) = interp.workspace.last() {
        if let Ok(_) = get_index_from_value(top) {
            sizes_values.push_front(interp.workspace.pop().unwrap());
        } else {
            break;
        }
    }

    if sizes_values.is_empty() { return Err(AjisaiError::from("SPLIT requires at least one size")); }
    let vector_val = interp.workspace.pop().ok_or_else(|| AjisaiError::from("SPLIT requires a vector to split"))?;

    let sizes: Vec<usize> = sizes_values.into_iter()
        .map(|v| get_index_from_value(&v).and_then(|bi| bi.to_usize().ok_or_else(|| AjisaiError::from("Split size is too large"))))
        .collect::<Result<Vec<_>>>()?;

    match vector_val.val_type {
        ValueType::Vector(v, bracket_type) => {
            let total_size: usize = sizes.iter().sum();
            if total_size != v.len() {
                return Err(AjisaiError::from(format!("Split sizes sum to {} but vector has {} elements", total_size, v.len())));
            }
            let mut start = 0;
            for size in sizes {
                let end = start + size;
                let slice = v[start..end].to_vec();
                interp.workspace.push(Value { val_type: ValueType::Vector(slice, bracket_type.clone()) });
                start = end;
            }
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// ========== ワークスペース操作 ==========

pub fn op_dup_workspace(interp: &mut Interpreter) -> Result<()> {
    let top = interp.workspace.last().ok_or(AjisaiError::WorkspaceUnderflow)?;
    interp.workspace.push(top.clone());
    Ok(())
}

pub fn op_swap_workspace(interp: &mut Interpreter) -> Result<()> {
    let len = interp.workspace.len();
    if len < 2 { return Err(AjisaiError::WorkspaceUnderflow); }
    interp.workspace.swap(len - 1, len - 2);
    Ok(())
}

pub fn op_rot_workspace(interp: &mut Interpreter) -> Result<()> {
    let len = interp.workspace.len();
    if len < 3 { return Err(AjisaiError::WorkspaceUnderflow); }
    let third = interp.workspace.remove(len - 3);
    interp.workspace.push(third);
    Ok(())
}

// ========== Vector構造操作 ==========

pub fn op_concat(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 { return Err(AjisaiError::WorkspaceUnderflow); }
    let vec2_val = interp.workspace.pop().unwrap();
    let vec1_val = interp.workspace.pop().unwrap();
    
    match (vec1_val.val_type, vec2_val.val_type) {
        (ValueType::Vector(mut v1, bracket_type1), ValueType::Vector(v2, _)) => {
            v1.extend(v2);
            interp.workspace.push(Value { val_type: ValueType::Vector(v1, bracket_type1) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector vector", "other types")),
    }
}

pub fn op_reverse(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop().ok_or(AjisaiError::WorkspaceUnderflow)?;
    match val.val_type {
        ValueType::Vector(mut v, bracket_type) => {
            v.reverse();
            interp.workspace.push(Value { val_type: ValueType::Vector(v, bracket_type) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}
