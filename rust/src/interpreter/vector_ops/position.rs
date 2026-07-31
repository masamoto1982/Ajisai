use super::extract_vector_elements;
use super::targeting::with_stacktop_vector_target_with_arg;
use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::{extract_integer_from_value, normalize_index};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::semantic::Recoverability;
use crate::types::{Interpretation, Value};

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

    let index_child = args_val
        .child(0)
        .ok_or_else(|| AjisaiError::from(format!("{} missing index", word)))?;
    let index = extract_integer_from_value(&index_child)
        .map_err(|_| AjisaiError::from(format!("{} index must be an integer", word)))?;
    let element = args_val
        .child(1)
        .ok_or_else(|| AjisaiError::from(format!("{} missing element", word)))?;
    Ok((index, element))
}

pub fn op_get(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    if matches!(interp.stack.last_role(), Interpretation::Text) {
        return Err(AjisaiError::create_structure_error("numeric index", "text"));
    }
    let (index_val, index) = pop_index_operand(interp)?;

    let target_val = match interp.stack.last().cloned() {
        Some(value) => value,
        None => {
            interp.stack.push(index_val);
            return Err(AjisaiError::StackUnderflow);
        }
    };

    if !target_val.is_vector() {
        interp.stack.push(index_val);
        return Err(AjisaiError::create_structure_error(
            "vector",
            "other format",
        ));
    }

    let result_elem = {
        let len = target_val.len();
        let actual_index = if len == 0 {
            None
        } else {
            normalize_index(index, len)
        };

        actual_index
            .and_then(|idx| target_val.child(idx))
            .unwrap_or_else(|| {
                Value::bubble_with_reason(NilReason::IndexOutOfBounds, Recoverability::Recoverable)
            })
    };

    if is_keep_mode {
        interp.stack.push(index_val);
    }
    interp.stack.push(result_elem);
    Ok(())
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

pub fn op_remove(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let (index_val, index) = pop_index_operand(interp)?;

    let removed =
        with_stacktop_vector_target_with_arg(interp, &index_val, is_keep_mode, |vector_val| {
            let mut values = extract_vector_elements(vector_val).to_vec();
            let len = values.len();
            let actual_index = normalize_index(index, len)
                .ok_or(AjisaiError::IndexOutOfBounds { index, length: len })?;

            values.remove(actual_index);
            if values.is_empty() {
                return Ok(Value::nil_with_reason(NilReason::EmptySequence));
            }
            Ok(Value::from_vector(values))
        })?;

    if is_keep_mode {
        interp.stack.push(index_val);
    }
    interp.stack.push(removed);
    Ok(())
}
