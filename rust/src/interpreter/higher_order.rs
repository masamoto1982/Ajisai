// rust/src/interpreter/higher_order.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}, OperationTarget};
use crate::types::{Value, ValueType, BracketType};

fn get_word_name_from_stack(interp: &mut Interpreter) -> Result<String> {
    let name_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    match name_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => match &v[0].val_type {
            ValueType::String(s) => Ok(s.clone()),
            _ => Err(AjisaiError::type_error("string for word name", "other type")),
        },
        _ => Err(AjisaiError::type_error("single-element vector with string", "other type")),
    }
}

fn is_true(value: &Value) -> Result<bool> {
    match &value.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => match &v[0].val_type {
            ValueType::Boolean(b) => Ok(*b),
            _ => Ok(false),
        },
        _ => Ok(false),
    }
}

pub fn op_map(interp: &mut Interpreter) -> Result<()> {
    let word_name = get_word_name_from_stack(interp)?;
    let upper_name = word_name.to_uppercase();
    if !interp.dictionary.contains_key(&upper_name) {
        return Err(AjisaiError::UnknownWord(word_name));
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let mut target_vec = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            if let ValueType::Vector(v, bt) = &mut target_vec.val_type {
                let mut results = Vec::new();
                for elem in v.drain(..) {
                    interp.stack.push(Value { val_type: ValueType::Vector(vec![elem], BracketType::Square) });
                    interp.execute_word_sync(&upper_name)?;
                    let result_item = interp.stack.pop().ok_or("MAP word must return a value")?;
                    results.push(result_item);
                }
                interp.stack.push(Value { val_type: ValueType::Vector(results, bt.clone()) });
            } else {
                return Err(AjisaiError::type_error("vector", "other type"));
            }
        }
        OperationTarget::Stack => {
            let count = interp.stack.pop()
                .and_then(|v| if let ValueType::Vector(vec, _) = v.val_type { vec.into_iter().next() } else { None })
                .and_then(|v| if let ValueType::Number(n) = v.val_type { n.to_i64() } else { None })
                .ok_or("STACK MAP requires a count")? as usize;
            if interp.stack.len() < count { return Err(AjisaiError::StackUnderflow); }
            let mut items_to_map = interp.stack.drain(interp.stack.len() - count..).collect::<Vec<_>>();
            for elem in items_to_map.drain(..) {
                interp.stack.push(elem);
                interp.execute_word_sync(&upper_name)?;
            }
        }
    }
    Ok(())
}

pub fn op_filter(interp: &mut Interpreter) -> Result<()> {
    let word_name = get_word_name_from_stack(interp)?;
    let upper_name = word_name.to_uppercase();
    if !interp.dictionary.contains_key(&upper_name) {
        return Err(AjisaiError::UnknownWord(word_name));
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let mut target_vec = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            if let ValueType::Vector(v, bt) = &mut target_vec.val_type {
                let mut results = Vec::new();
                for elem in v.drain(..) {
                    interp.stack.push(Value { val_type: ValueType::Vector(vec![elem.clone()], BracketType::Square) });
                    interp.execute_word_sync(&upper_name)?;
                    let condition = interp.stack.pop().ok_or("FILTER word must return a boolean")?;
                    if is_true(&condition)? {
                        results.push(elem);
                    }
                }
                interp.stack.push(Value { val_type: ValueType::Vector(results, bt.clone()) });
            } else {
                 return Err(AjisaiError::type_error("vector", "other type"));
            }
        }
        OperationTarget::Stack => {
            let count = interp.stack.pop()
                .and_then(|v| if let ValueType::Vector(vec, _) = v.val_type { vec.into_iter().next() } else { None })
                .and_then(|v| if let ValueType::Number(n) = v.val_type { n.to_i64() } else { None })
                .ok_or("STACK FILTER requires a count")? as usize;

            if interp.stack.len() < count { return Err(AjisaiError::StackUnderflow); }
            
            let items_to_filter = interp.stack.drain(interp.stack.len() - count..).collect::<Vec<_>>();
            for item in items_to_filter {
                interp.stack.push(item.clone());
                interp.execute_word_sync(&upper_name)?;
                let condition = interp.stack.pop().ok_or("FILTER word must return a boolean")?;
                if is_true(&condition)? {
                    interp.stack.push(item);
                }
            }
        }
    }
    Ok(())
}

pub fn op_reduce(interp: &mut Interpreter) -> Result<()> {
    let word_name = get_word_name_from_stack(interp)?;
    let mut accumulator = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let upper_name = word_name.to_uppercase();
    if !interp.dictionary.contains_key(&upper_name) {
        return Err(AjisaiError::UnknownWord(word_name));
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let target_vec = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            if let ValueType::Vector(v, _) = target_vec.val_type {
                for elem in v {
                    interp.stack.push(accumulator);
                    interp.stack.push(Value { val_type: ValueType::Vector(vec![elem], BracketType::Square) });
                    interp.execute_word_sync(&upper_name)?;
                    accumulator = interp.stack.pop().ok_or("REDUCE word must return a value")?;
                }
            } else {
                 return Err(AjisaiError::type_error("vector", "other type"));
            }
        }
        OperationTarget::Stack => {
            let count = interp.stack.pop()
                .and_then(|v| if let ValueType::Vector(vec, _) = v.val_type { vec.into_iter().next() } else { None })
                .and_then(|v| if let ValueType::Number(n) = v.val_type { n.to_i64() } else { None })
                .ok_or("STACK REDUCE requires a count")? as usize;
            if interp.stack.len() < count { return Err(AjisaiError::StackUnderflow); }
            let items_to_reduce = interp.stack.drain(interp.stack.len() - count..).collect::<Vec<_>>();
            for item in items_to_reduce {
                 interp.stack.push(accumulator);
                 interp.stack.push(item);
                 interp.execute_word_sync(&upper_name)?;
                 accumulator = interp.stack.pop().ok_or("REDUCE word must return a value")?;
            }
        }
    }
    interp.stack.push(accumulator);
    Ok(())
}

pub fn op_each(interp: &mut Interpreter) -> Result<()> {
    let word_name = get_word_name_from_stack(interp)?;
    let upper_name = word_name.to_uppercase();
    if !interp.dictionary.contains_key(&upper_name) {
        return Err(AjisaiError::UnknownWord(word_name));
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
             let target_vec = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            if let ValueType::Vector(v, _) = target_vec.val_type {
                for elem in v {
                    interp.stack.push(Value { val_type: ValueType::Vector(vec![elem], BracketType::Square) });
                    interp.execute_word_sync(&upper_name)?;
                }
            } else {
                 return Err(AjisaiError::type_error("vector", "other type"));
            }
        }
        OperationTarget::Stack => {
            let count = interp.stack.pop()
                .and_then(|v| if let ValueType::Vector(vec, _) = v.val_type { vec.into_iter().next() } else { None })
                .and_then(|v| if let ValueType::Number(n) = v.val_type { n.to_i64() } else { None })
                .ok_or("STACK EACH requires a count")? as usize;
            if interp.stack.len() < count { return Err(AjisaiError::StackUnderflow); }
            let items_to_process = interp.stack.drain(interp.stack.len() - count..).collect::<Vec<_>>();
            for item in items_to_process {
                interp.stack.push(item);
                interp.execute_word_sync(&upper_name)?;
            }
        }
    }
    Ok(())
}
