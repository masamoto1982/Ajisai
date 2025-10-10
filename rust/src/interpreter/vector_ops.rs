// rust/src/interpreter/vector_ops.rs

use crate::interpreter::{Interpreter, OperationTarget, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, Fraction, BracketType};
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive, Zero};
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
    let index_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index_bigint = get_index_from_value(&index_val)?;
    let index = index_bigint.to_i64().ok_or_else(|| AjisaiError::from("Index is too large"))?;

    match interp.operation_target {
        OperationTarget::StackTop => {
            let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            match target_val.val_type {
                ValueType::Vector(v, bracket_type) => {
                    let len = v.len();
                    if len == 0 { return Err(AjisaiError::IndexOutOfBounds { index, length: 0 }); }
                    let actual_index = if index < 0 { (len as i64 + index) as usize } else { index as usize };

                    if actual_index < len {
                        let result_elem = v[actual_index].clone();
                        interp.stack.push(target_val); // Push back the original vector
                        interp.stack.push(Value { val_type: ValueType::Vector(vec![result_elem], bracket_type) });
                        Ok(())
                    } else {
                        Err(AjisaiError::IndexOutOfBounds { index, length: len })
                    }
                },
                _ => Err(AjisaiError::type_error("vector", "other type")),
            }
        }
        OperationTarget::Stack => {
            let stack_len = interp.stack.len();
            if stack_len == 0 { return Err(AjisaiError::IndexOutOfBounds { index, length: 0 }); }
            let actual_index = if index < 0 { (stack_len as i64 + index) as usize } else { index as usize };

            if actual_index < stack_len {
                let result_elem = interp.stack[actual_index].clone();
                interp.stack.push(result_elem);
                Ok(())
            } else {
                Err(AjisaiError::IndexOutOfBounds { index, length: stack_len })
            }
        }
    }
}

pub fn op_insert(interp: &mut Interpreter) -> Result<()> {
    let element = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index = get_index_from_value(&index_val)?.to_i64().ok_or_else(|| AjisaiError::from("Index is too large"))?;

    match interp.operation_target {
        OperationTarget::StackTop => {
            let vector_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            match vector_val.val_type {
                ValueType::Vector(mut v, bracket_type) => {
                    let len = v.len() as i64;
                    let insert_index = if index < 0 { (len + index + 1).max(0) as usize } else { (index as usize).min(v.len()) };
                    v.insert(insert_index, element);
                    interp.stack.push(Value { val_type: ValueType::Vector(v, bracket_type) });
                    Ok(())
                },
                _ => Err(AjisaiError::type_error("vector", "other type")),
            }
        }
        OperationTarget::Stack => {
            let len = interp.stack.len() as i64;
            let insert_index = if index < 0 { (len + index + 1).max(0) as usize } else { (index as usize).min(interp.stack.len()) };
            interp.stack.insert(insert_index, element);
            Ok(())
        }
    }
}

pub fn op_replace(interp: &mut Interpreter) -> Result<()> {
    let new_element = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index = get_index_from_value(&index_val)?.to_i64().ok_or_else(|| AjisaiError::from("Index too large"))?;

    match interp.operation_target {
        OperationTarget::StackTop => {
            let vector_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            match vector_val.val_type {
                ValueType::Vector(mut v, bracket_type) => {
                    let len = v.len();
                    let actual_index = if index < 0 { (len as i64 + index) as usize } else { index as usize };
                    if actual_index < len {
                        v[actual_index] = new_element;
                        interp.stack.push(Value { val_type: ValueType::Vector(v, bracket_type) });
                        Ok(())
                    } else {
                        Err(AjisaiError::IndexOutOfBounds { index, length: len })
                    }
                },
                _ => Err(AjisaiError::type_error("vector", "other type")),
            }
        }
        OperationTarget::Stack => {
            let len = interp.stack.len();
            let actual_index = if index < 0 { (len as i64 + index) as usize } else { index as usize };
            if actual_index < len {
                interp.stack[actual_index] = new_element;
                Ok(())
            } else {
                Err(AjisaiError::IndexOutOfBounds { index, length: len })
            }
        }
    }
}

pub fn op_remove(interp: &mut Interpreter) -> Result<()> {
    let index_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index = get_index_from_value(&index_val)?.to_i64().ok_or_else(|| AjisaiError::from("Index too large"))?;

    match interp.operation_target {
        OperationTarget::StackTop => {
            let vector_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            match vector_val.val_type {
                ValueType::Vector(mut v, bracket_type) => {
                    let len = v.len();
                    let actual_index = if index < 0 { (len as i64 + index) as usize } else { index as usize };
                    if actual_index < len {
                        v.remove(actual_index);
                        interp.stack.push(Value { val_type: ValueType::Vector(v, bracket_type) });
                        Ok(())
                    } else {
                        Err(AjisaiError::IndexOutOfBounds { index, length: len })
                    }
                },
                _ => Err(AjisaiError::type_error("vector", "other type")),
            }
        }
        OperationTarget::Stack => {
            let len = interp.stack.len();
            let actual_index = if index < 0 { (len as i64 + index) as usize } else { index as usize };
            if actual_index < len {
                interp.stack.remove(actual_index);
                Ok(())
            } else {
                Err(AjisaiError::IndexOutOfBounds { index, length: len })
            }
        }
    }
}

// ========== 量指定操作（1オリジン）==========

pub fn op_length(interp: &mut Interpreter) -> Result<()> {
    let len = match interp.operation_target {
        OperationTarget::StackTop => {
            let target_val = interp.stack.last().ok_or(AjisaiError::StackUnderflow)?;
            match &target_val.val_type {
                ValueType::Vector(v, _) => v.len(),
                _ => return Err(AjisaiError::type_error("vector", "other type")),
            }
        }
        OperationTarget::Stack => interp.stack.len(),
    };
    let len_frac = Fraction::new(BigInt::from(len), BigInt::one());
    let val = Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Number(len_frac) }], BracketType::Square) };
    interp.stack.push(val);
    Ok(())
}

pub fn op_take(interp: &mut Interpreter) -> Result<()> {
    let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let count = get_index_from_value(&count_val)?.to_i64().ok_or_else(|| AjisaiError::from("Count is too large"))?;

    match interp.operation_target {
        OperationTarget::StackTop => {
            let vector_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            match vector_val.val_type {
                ValueType::Vector(v, bracket_type) => {
                    let len = v.len();
                    let result = if count < 0 {
                        let abs_count = (-count) as usize;
                        if abs_count > len { return Err(AjisaiError::from("Take count exceeds vector length")); }
                        v[len - abs_count..].to_vec()
                    } else {
                        let take_count = count as usize;
                        if take_count > len { return Err(AjisaiError::from("Take count exceeds vector length")); }
                        v[..take_count].to_vec()
                    };
                    interp.stack.push(Value { val_type: ValueType::Vector(result, bracket_type) });
                    Ok(())
                },
                _ => Err(AjisaiError::type_error("vector", "other type")),
            }
        }
        OperationTarget::Stack => {
            let len = interp.stack.len();
            let new_stack = if count < 0 {
                let abs_count = (-count) as usize;
                if abs_count > len { return Err(AjisaiError::from("Take count exceeds stack length")); }
                interp.stack.split_off(len - abs_count)
            } else {
                let take_count = count as usize;
                if take_count > len { return Err(AjisaiError::from("Take count exceeds stack length")); }
                let mut rest = interp.stack.split_off(take_count);
                std::mem::replace(&mut interp.stack, rest)
            };
            interp.stack = new_stack;
            Ok(())
        }
    }
}


pub fn op_split(interp: &mut Interpreter) -> Result<()> {
    let mut sizes_values = VecDeque::new();
    while let Some(top) = interp.stack.last() {
        if get_index_from_value(top).is_ok() {
            sizes_values.push_front(interp.stack.pop().unwrap());
        } else {
            break;
        }
    }

    if sizes_values.is_empty() { return Err(AjisaiError::from("SPLIT requires at least one size")); }
    
    let sizes: Vec<usize> = sizes_values.into_iter()
        .map(|v| get_index_from_value(&v).and_then(|bi| bi.to_usize().ok_or_else(|| AjisaiError::from("Split size is too large"))))
        .collect::<Result<Vec<_>>>()?;

    match interp.operation_target {
        OperationTarget::StackTop => {
            let vector_val = interp.stack.pop().ok_or_else(|| AjisaiError::from("SPLIT requires a vector to split"))?;
            match vector_val.val_type {
                ValueType::Vector(v, bracket_type) => {
                    let total_size: usize = sizes.iter().sum();
                    if total_size > v.len() { return Err(AjisaiError::from("Split sizes sum exceeds vector length")); }
                    
                    let mut current_pos = 0;
                    let mut result_vectors = Vec::new();
                    for &size in &sizes {
                        result_vectors.push(Value {
                            val_type: ValueType::Vector(v[current_pos..current_pos + size].to_vec(), bracket_type.clone())
                        });
                        current_pos += size;
                    }
                    if current_pos < v.len() {
                        result_vectors.push(Value { val_type: ValueType::Vector(v[current_pos..].to_vec(), bracket_type) });
                    }
                    interp.stack.extend(result_vectors);
                    Ok(())
                },
                _ => Err(AjisaiError::type_error("vector", "other type")),
            }
        }
        OperationTarget::Stack => {
            let total_size: usize = sizes.iter().sum();
            if total_size > interp.stack.len() { return Err(AjisaiError::from("Split sizes sum exceeds stack length")); }
            
            let mut remaining_stack = interp.stack.split_off(0);
            let mut result_stack = Vec::new();
            
            let mut current_pos = 0;
            for &size in &sizes {
                let chunk = remaining_stack.drain(..size).collect();
                result_stack.push(Value { val_type: ValueType::Vector(chunk, BracketType::Square) });
                current_pos += size;
            }
            if !remaining_stack.is_empty() {
                 result_stack.push(Value { val_type: ValueType::Vector(remaining_stack, BracketType::Square) });
            }
            interp.stack = result_stack;
            Ok(())
        }
    }
}

// ========== Vector構造操作 ==========

pub fn op_concat(interp: &mut Interpreter) -> Result<()> {
    let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let count = get_index_from_value(&count_val)?.to_usize().ok_or_else(|| AjisaiError::from("Count is too large"))?;

    if interp.stack.len() < count { return Err(AjisaiError::StackUnderflow); }

    match interp.operation_target {
        OperationTarget::StackTop => { // This is equivalent to STACK mode for CONCAT
            let mut vecs_to_concat = interp.stack.split_off(interp.stack.len() - count);
            let mut result_vec = Vec::new();
            let mut final_bracket_type = BracketType::Square;

            if !vecs_to_concat.is_empty() {
                if let ValueType::Vector(_, bracket_type) = &vecs_to_concat[0].val_type {
                    final_bracket_type = bracket_type.clone();
                }
            }
            
            for val in vecs_to_concat {
                if let ValueType::Vector(v, _) = val.val_type {
                    result_vec.extend(v);
                } else {
                    return Err(AjisaiError::type_error("vector", "other type"));
                }
            }
            interp.stack.push(Value { val_type: ValueType::Vector(result_vec, final_bracket_type) });
        }
        OperationTarget::Stack => {
            let mut vecs_to_concat = interp.stack.split_off(interp.stack.len() - count);
            let mut result_vec = Vec::new();
            
            for val in vecs_to_concat {
                if let ValueType::Vector(v, _) = val.val_type {
                    result_vec.extend(v);
                } else {
                    result_vec.push(val);
                }
            }
            interp.stack.push(Value { val_type: ValueType::Vector(result_vec, BracketType::Square) });
        }
    }
    Ok(())
}


pub fn op_reverse(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            match val.val_type {
                ValueType::Vector(mut v, bracket_type) => {
                    v.reverse();
                    interp.stack.push(Value { val_type: ValueType::Vector(v, bracket_type) });
                    Ok(())
                },
                _ => Err(AjisaiError::type_error("vector", "other type")),
            }
        }
        OperationTarget::Stack => {
            interp.stack.reverse();
            Ok(())
        }
    }
}

fn flatten_vector_recursive(vec: Vec<Value>, result: &mut Vec<Value>) {
    for val in vec {
        if let ValueType::Vector(inner_vec, _) = val.val_type {
            flatten_vector_recursive(inner_vec, result);
        } else {
            result.push(val);
        }
    }
}

pub fn op_level(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            match val.val_type {
                ValueType::Vector(v, bracket_type) => {
                    let mut flattened = Vec::new();
                    flatten_vector_recursive(v, &mut flattened);
                    interp.stack.push(Value {
                        val_type: ValueType::Vector(flattened, bracket_type),
                    });
                    Ok(())
                },
                _ => Err(AjisaiError::type_error("vector", "other type")),
            }
        }
        OperationTarget::Stack => {
            let mut flattened = Vec::new();
            let current_stack = std::mem::take(&mut interp.stack);
            flatten_vector_recursive(current_stack, &mut flattened);
            interp.stack.push(Value {
                val_type: ValueType::Vector(flattened, BracketType::Square),
            });
            Ok(())
        }
    }
}
