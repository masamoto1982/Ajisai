



use super::extract_vector_elements;
use super::targeting::with_stacktop_vector_target_with_arg;
use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{extract_integer_from_value, normalize_index};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::Value;

fn pop_index_operand(interp: &mut Interpreter) -> Result<(Value, i64)> {
    let index_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index = match extract_integer_from_value(&index_val) {
        Ok(value) => value,
        Err(error) => {
            interp.stack.push(index_val);
            return Err(error);
        }
    };
    Ok((index_val, index))
}

fn parse_index_element_args(word: &str, args_val: &Value) -> Result<(i64, Value)> {
    if !args_val.is_vector() || args_val.len() != 2 {
        return Err(AjisaiError::from(format!(
            "{} requires [index element]",
            word
        )));
    }

    let index = extract_integer_from_value(args_val.get_child(0).unwrap())
        .map_err(|_| AjisaiError::from(format!("{} index must be an integer", word)))?;
    let element = args_val.get_child(1).unwrap().clone();
    Ok((index, element))
}






pub fn op_get(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let (index_val, index) = pop_index_operand(interp)?;


    let preserve_source = interp.gui_mode || is_keep_mode;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let result_elem =
                with_stacktop_vector_target_with_arg(interp, &index_val, preserve_source, |target_val| {
                    let len = target_val.len();
                    if len == 0 {
                        return Err(AjisaiError::IndexOutOfBounds { index, length: 0 });
                    }

                    let actual_index = normalize_index(index, len)
                        .ok_or(AjisaiError::IndexOutOfBounds { index, length: len })?;
                    Ok(target_val.get_child(actual_index).unwrap().clone())
                })?;

            if is_keep_mode {
                interp.stack.push(index_val);
            }
            interp.stack.push(result_elem);
            Ok(())
        }
        OperationTargetMode::Stack => {
            let stack_len = interp.stack.len();
            if stack_len == 0 {
                interp.stack.push(index_val);
                return Err(AjisaiError::IndexOutOfBounds { index, length: 0 });
            }

            let actual_index = match normalize_index(index, stack_len) {
                Some(idx) => idx,
                None => {
                    interp.stack.push(index_val);
                    return Err(AjisaiError::IndexOutOfBounds {
                        index,
                        length: stack_len,
                    });
                }
            };

            let result_elem = interp.stack[actual_index].clone();
            if !preserve_source {

                interp.stack.clear();
            }
            interp.stack.push(result_elem);
            Ok(())
        }
    }
}






pub fn op_insert(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;


    let args_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;


    let (index, element) = match parse_index_element_args("INSERT", &args_val) {
        Ok(parsed) => parsed,
        Err(error) => {
            interp.stack.push(args_val);
            return Err(error);
        }
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let inserted =
                with_stacktop_vector_target_with_arg(interp, &args_val, is_keep_mode, |vector_val| {
                    let mut values = extract_vector_elements(vector_val).to_vec();
                    let len = values.len() as i64;
                    let insert_index = if index < 0 {
                        (len + index).max(0) as usize
                    } else {
                        (index as usize).min(values.len())
                    };

                    values.insert(insert_index, element.clone());
                    Ok(Value::from_vector(values))
                })?;

            if is_keep_mode {
                interp.stack.push(args_val);
            }
            interp.stack.push(inserted);
            Ok(())
        }
        OperationTargetMode::Stack => {
            let len = interp.stack.len() as i64;
            let insert_index = if index < 0 {
                (len + index).max(0) as usize
            } else {
                (index as usize).min(interp.stack.len())
            };
            interp.stack.insert(insert_index, element);
            Ok(())
        }
    }
}






pub fn op_replace(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;


    let args_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let (index, new_element) = match parse_index_element_args("REPLACE", &args_val) {
        Ok(parsed) => parsed,
        Err(error) => {
            interp.stack.push(args_val);
            return Err(error);
        }
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let replaced =
                with_stacktop_vector_target_with_arg(interp, &args_val, is_keep_mode, |vector_val| {
                    let mut values = extract_vector_elements(vector_val).to_vec();
                    let len = values.len();
                    let actual_index = normalize_index(index, len)
                        .ok_or(AjisaiError::IndexOutOfBounds { index, length: len })?;

                    values[actual_index] = new_element.clone();
                    Ok(Value::from_vector(values))
                })?;

            if is_keep_mode {
                interp.stack.push(args_val);
            }
            interp.stack.push(replaced);
            Ok(())
        }
        OperationTargetMode::Stack => {
            let len = interp.stack.len();
            let actual_index = match normalize_index(index, len) {
                Some(idx) => idx,
                None => {
                    interp.stack.push(args_val);
                    return Err(AjisaiError::IndexOutOfBounds { index, length: len });
                }
            };

            if is_keep_mode {
                let original_stack = interp.stack.clone();
                interp.stack[actual_index] = new_element;





                let _ = original_stack;
            } else {
                interp.stack[actual_index] = new_element;
            }
            Ok(())
        }
    }
}






pub fn op_remove(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let (index_val, index) = pop_index_operand(interp)?;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let removed =
                with_stacktop_vector_target_with_arg(interp, &index_val, is_keep_mode, |vector_val| {
                    let mut values = extract_vector_elements(vector_val).to_vec();
                    let len = values.len();
                    let actual_index = normalize_index(index, len)
                        .ok_or(AjisaiError::IndexOutOfBounds { index, length: len })?;

                    values.remove(actual_index);
                    if values.is_empty() {
                        return Ok(Value::nil());
                    }
                    Ok(Value::from_vector(values))
                })?;

            if is_keep_mode {
                interp.stack.push(index_val);
            }
            interp.stack.push(removed);
            Ok(())
        }
        OperationTargetMode::Stack => {
            let len = interp.stack.len();
            let actual_index = match normalize_index(index, len) {
                Some(idx) => idx,
                None => {
                    interp.stack.push(index_val);
                    return Err(AjisaiError::IndexOutOfBounds { index, length: len });
                }
            };

            interp.stack.remove(actual_index);
            Ok(())
        }
    }
}
