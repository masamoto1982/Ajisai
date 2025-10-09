// rust/src/interpreter/vector_ops.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}, OperationTarget};
use crate::types::{Value, ValueType, Fraction, BracketType};
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};

fn get_optional_integer_arg(interp: &mut Interpreter) -> Result<Option<i64>> {
    if let Some(top) = interp.stack.last() {
        if let ValueType::Vector(v, _) = &top.val_type {
            if v.len() == 1 {
                if let ValueType::Number(n) = &v[0].val_type {
                    if n.denominator == BigInt::one() {
                        let index = n.numerator.to_i64().ok_or("Index too large")?;
                        interp.stack.pop();
                        return Ok(Some(index));
                    }
                }
            }
        }
    }
    Ok(None)
}

fn resolve_index(index: i64, len: usize) -> Result<usize> {
    let ulen = len as i64;
    let actual_index = if index < 0 { ulen + index } else { index };
    if actual_index < 0 || (len > 0 && actual_index >= ulen) {
        Err(AjisaiError::IndexOutOfBounds { index, length: len })
    } else {
        Ok(actual_index as usize)
    }
}

pub fn op_get(interp: &mut Interpreter) -> Result<()> {
    let index = get_optional_integer_arg(interp)?.ok_or("GET requires an index")?;
    
    match interp.operation_target {
        OperationTarget::StackTop => {
            let mut target_vec = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            if let ValueType::Vector(v, _) = &mut target_vec.val_type {
                let len = v.len();
                let actual_index = resolve_index(index, len)?;
                let element = v.get(actual_index).cloned().ok_or(AjisaiError::IndexOutOfBounds { index, length: len })?;
                interp.stack.push(Value { val_type: ValueType::Vector(vec![element], BracketType::Square) });
            } else {
                return Err(AjisaiError::type_error("vector", "other type"));
            }
        }
        OperationTarget::Stack => {
            let len = interp.stack.len();
            let actual_index = resolve_index(index, len)?;
            let element = interp.stack.get(actual_index).cloned().ok_or(AjisaiError::IndexOutOfBounds { index, length: len })?;
            interp.stack.push(element);
        }
    }
    Ok(())
}

pub fn op_insert(interp: &mut Interpreter) -> Result<()> {
    let element_to_insert = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index = get_optional_integer_arg(interp)?.ok_or("INSERT requires an index")?;

    match interp.operation_target {
        OperationTarget::StackTop => {
            let mut target_vec = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            if let ValueType::Vector(v, bt) = &mut target_vec.val_type {
                 let element = match element_to_insert.val_type {
                    ValueType::Vector(v_el, _) if v_el.len() == 1 => v_el.into_iter().next().unwrap(),
                    _ => return Err(AjisaiError::type_error("single-element vector", "other type")),
                };
                let len = v.len();
                let actual_index = resolve_index(index, len + 1)?;
                v.insert(actual_index, element);
                interp.stack.push(Value { val_type: ValueType::Vector(v.clone(), bt.clone())});
            } else {
                return Err(AjisaiError::type_error("vector", "other type"));
            }
        }
        OperationTarget::Stack => {
            let len = interp.stack.len();
            let actual_index = resolve_index(index, len + 1)?;
            interp.stack.insert(actual_index, element_to_insert);
        }
    }
    Ok(())
}

pub fn op_replace(interp: &mut Interpreter) -> Result<()> {
    let new_element = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index = get_optional_integer_arg(interp)?.ok_or("REPLACE requires an index")?;

    match interp.operation_target {
        OperationTarget::StackTop => {
            let mut target_vec = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
             if let ValueType::Vector(v, bt) = &mut target_vec.val_type {
                let element = match new_element.val_type {
                    ValueType::Vector(v_el, _) if v_el.len() == 1 => v_el.into_iter().next().unwrap(),
                    _ => return Err(AjisaiError::type_error("single-element vector", "other type")),
                };
                let len = v.len();
                let actual_index = resolve_index(index, len)?;
                v[actual_index] = element;
                interp.stack.push(Value { val_type: ValueType::Vector(v.clone(), bt.clone())});
            } else {
                return Err(AjisaiError::type_error("vector", "other type"));
            }
        }
        OperationTarget::Stack => {
            let len = interp.stack.len();
            let actual_index = resolve_index(index, len)?;
            interp.stack[actual_index] = new_element;
        }
    }
    Ok(())
}

pub fn op_remove(interp: &mut Interpreter) -> Result<()> {
    let index = get_optional_integer_arg(interp)?.ok_or("REMOVE requires an index")?;
    
    match interp.operation_target {
        OperationTarget::StackTop => {
            let mut target_vec = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            if let ValueType::Vector(v, bt) = &mut target_vec.val_type {
                let len = v.len();
                let actual_index = resolve_index(index, len)?;
                v.remove(actual_index);
                interp.stack.push(Value { val_type: ValueType::Vector(v.clone(), bt.clone())});
            } else {
                return Err(AjisaiError::type_error("vector", "other type"));
            }
        }
        OperationTarget::Stack => {
            let len = interp.stack.len();
            let actual_index = resolve_index(index, len)?;
            interp.stack.remove(actual_index);
        }
    }
    Ok(())
}

pub fn op_length(interp: &mut Interpreter) -> Result<()> {
    let len = match interp.operation_target {
        OperationTarget::StackTop => {
            let target_vec = interp.stack.last().ok_or(AjisaiError::StackUnderflow)?;
            if let ValueType::Vector(v, _) = &target_vec.val_type {
                v.len()
            } else {
                return Err(AjisaiError::type_error("vector", "other type"));
            }
        }
        OperationTarget::Stack => interp.stack.len(),
    };
    let len_val = Value { val_type: ValueType::Number(Fraction::new(BigInt::from(len), BigInt::one())) };
    interp.stack.push(Value { val_type: ValueType::Vector(vec![len_val], BracketType::Square)});
    Ok(())
}

pub fn op_take(interp: &mut Interpreter) -> Result<()> {
    let count_val = get_optional_integer_arg(interp)?.ok_or("TAKE requires a count")?;
    let count = count_val as usize;

    match interp.operation_target {
        OperationTarget::StackTop => {
            let mut target_vec = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            if let ValueType::Vector(v, bt) = &mut target_vec.val_type {
                let new_v = if count_val >= 0 {
                    v.drain(..count.min(v.len())).collect()
                } else {
                    let len = v.len();
                    v.drain(len.saturating_sub(count)..len).collect()
                };
                interp.stack.push(Value { val_type: ValueType::Vector(new_v, bt.clone()) });
            } else {
                 return Err(AjisaiError::type_error("vector", "other type"));
            }
        }
        OperationTarget::Stack => {
            let len = interp.stack.len();
            let new_v = if count_val >= 0 {
                interp.stack.drain(..count.min(len)).collect()
            } else {
                interp.stack.drain(len.saturating_sub(count)..len).collect()
            };
            let new_vec = Value { val_type: ValueType::Vector(new_v, BracketType::Square) };
            interp.stack.push(new_vec);
        }
    }
    Ok(())
}

pub fn op_concat(interp: &mut Interpreter) -> Result<()> {
    let n = get_optional_integer_arg(interp)?.unwrap_or(2) as usize;
    if interp.stack.len() < n { return Err(AjisaiError::StackUnderflow); }

    let mut items_to_concat = interp.stack.drain(interp.stack.len() - n ..).collect::<Vec<_>>();
    
    match interp.operation_target {
        OperationTarget::StackTop => {
             let mut result_vec = Vec::new();
             let mut bracket_type = BracketType::Square;
             for (i, item) in items_to_concat.iter_mut().enumerate() {
                 if let ValueType::Vector(v, bt) = &mut item.val_type {
                     if i == 0 { bracket_type = bt.clone(); }
                     result_vec.append(v);
                 } else {
                     return Err(AjisaiError::type_error("vector", "other type"));
                 }
             }
             interp.stack.push(Value { val_type: ValueType::Vector(result_vec, bracket_type) });
        }
        OperationTarget::Stack => {
            // No-op for STACK, already done by draining.
            interp.stack.extend(items_to_concat);
        }
    }
    Ok(())
}

pub fn op_reverse(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let mut target_vec = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            if let ValueType::Vector(v, _) = &mut target_vec.val_type {
                v.reverse();
                interp.stack.push(target_vec);
            } else {
                return Err(AjisaiError::type_error("vector", "other type"));
            }
        }
        OperationTarget::Stack => {
            interp.stack.reverse();
        }
    }
    Ok(())
}

pub fn op_split(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let target_vec = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            if let ValueType::Vector(v, _) = target_vec.val_type {
                for item in v {
                    interp.stack.push(Value { val_type: ValueType::Vector(vec![item], BracketType::Square) });
                }
            } else {
                return Err(AjisaiError::type_error("vector", "other type"));
            }
        }
        OperationTarget::Stack => {
            // STACK SPLIT is a no-op conceptually
        }
    }
    Ok(())
}

pub fn op_level(interp: &mut Interpreter) -> Result<()> {
     match interp.operation_target {
        OperationTarget::StackTop => {
            let target_vec = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            if let ValueType::Vector(v, bt) = target_vec.val_type {
                let mut flattened = Vec::new();
                flatten_vector_recursive(v, &mut flattened);
                interp.stack.push(Value { val_type: ValueType::Vector(flattened, bt) });
            } else {
                return Err(AjisaiError::type_error("vector", "other type"));
            }
        }
        OperationTarget::Stack => {
            let current_stack = std::mem::take(&mut interp.stack);
            for val in current_stack {
                if let ValueType::Vector(v, _) = val.val_type {
                    for item in v {
                        interp.stack.push(Value { val_type: ValueType::Vector(vec![item], BracketType::Square) });
                    }
                } else {
                    interp.stack.push(val);
                }
            }
        }
    }
    Ok(())
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
