



use super::extract_vector_elements;
use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{
    create_number_value, extract_bigint_from_value, extract_integer_from_value,
};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::Value;
use num_traits::ToPrimitive;

fn acquire_stacktop_target(
    interp: &mut Interpreter,
    arg_to_restore: &Value,
    preserve_source: bool,
    missing_target_error: AjisaiError,
) -> Result<Value> {
    if preserve_source {
        return interp.stack.last().cloned().ok_or_else(|| {
            interp.stack.push(arg_to_restore.clone());
            missing_target_error
        });
    }

    interp.stack.pop().ok_or_else(|| {
        interp.stack.push(arg_to_restore.clone());
        missing_target_error
    })
}

fn with_stacktop_vector_target<R, F>(
    interp: &mut Interpreter,
    arg_to_restore: &Value,
    preserve_source: bool,
    missing_target_error: AjisaiError,
    action: F,
) -> Result<R>
where
    F: FnOnce(&Value) -> Result<R>,
{
    let target_val = acquire_stacktop_target(
        interp,
        arg_to_restore,
        preserve_source,
        missing_target_error,
    )?;
    if !target_val.is_vector() {
        if !preserve_source {
            interp.stack.push(target_val);
        }
        interp.stack.push(arg_to_restore.clone());
        return Err(AjisaiError::create_structure_error(
            "vector",
            "other format",
        ));
    }

    match action(&target_val) {
        Ok(result) => Ok(result),
        Err(error) => {
            if !preserve_source {
                interp.stack.push(target_val);
            }
            interp.stack.push(arg_to_restore.clone());
            Err(error)
        }
    }
}

fn compute_take_bounds(len: usize, count: i64, target: &str) -> Result<(usize, usize)> {
    if count < 0 {
        let take = (-count) as usize;
        if take > len {
            return Err(AjisaiError::from(format!(
                "Take count exceeds {} length",
                target
            )));
        }
        return Ok((len - take, len));
    }

    let take = count as usize;
    if take > len {
        return Err(AjisaiError::from(format!(
            "Take count exceeds {} length",
            target
        )));
    }
    Ok((0, take))
}






pub fn op_length(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    let preserve_source = interp.gui_mode || is_keep_mode;

    let len = match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            if preserve_source {
                let target_val = interp.stack.last().ok_or(AjisaiError::StackUnderflow)?;

                if target_val.is_nil() {
                    0
                } else if target_val.is_vector() {
                    extract_vector_elements(target_val).len()
                } else {
                    return Err(AjisaiError::create_structure_error(
                        "vector",
                        "other format",
                    ));
                }
            } else {
                let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

                if target_val.is_nil() {
                    0
                } else if target_val.is_vector() {
                    extract_vector_elements(&target_val).len()
                } else {
                    interp.stack.push(target_val);
                    return Err(AjisaiError::create_structure_error(
                        "vector",
                        "other format",
                    ));
                }
            }
        }
        OperationTargetMode::Stack => {
            if preserve_source {
                interp.stack.len()
            } else {
                let len = interp.stack.len();
                interp.stack.clear();
                len
            }
        }
    };
    let len_frac = Fraction::from(len as i64);
    interp.stack.push(create_number_value(len_frac));
    Ok(())
}






pub fn op_take(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let count = match extract_integer_from_value(&count_val) {
        Ok(v) => v,
        Err(e) => {
            interp.stack.push(count_val);
            return Err(e);
        }
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let result = with_stacktop_vector_target(
                interp,
                &count_val,
                is_keep_mode,
                AjisaiError::StackUnderflow,
                |vector_val| {
                    let elements = extract_vector_elements(vector_val);
                    let (start, end) = compute_take_bounds(elements.len(), count, "vector")?;
                    Ok(elements[start..end].to_vec())
                },
            )?;

            if is_keep_mode {
                interp.stack.push(count_val);
            }
            if result.is_empty() {
                interp.stack.push(Value::nil());
            } else {
                interp.stack.push(Value::from_vector(result));
            }
            Ok(())
        }
        OperationTargetMode::Stack => {
            let len = interp.stack.len();
            let (start, end) = match compute_take_bounds(len, count, "stack") {
                Ok(bounds) => bounds,
                Err(error) => {
                    interp.stack.push(count_val);
                    return Err(error);
                }
            };

            if is_keep_mode {
                let taken: Vec<Value> = interp.stack[start..end].to_vec();
                interp.stack.extend(taken);
            } else if count < 0 {
                interp.stack = interp.stack.split_off(start);
            } else {
                interp.stack.truncate(end);
            }
            Ok(())
        }
    }
}






pub fn op_split(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;


    let args_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;


    let sizes: Vec<usize> = if args_val.is_vector() {
        let n = args_val.len();
        if n == 0 {
            interp.stack.push(args_val);
            return Err(AjisaiError::from("SPLIT requires at least one size"));
        }

        let mut sizes = Vec::with_capacity(n);
        for i in 0..n {
            match extract_bigint_from_value(args_val.get_child(i).unwrap()) {
                Ok(bi) => match bi.to_usize() {
                    Some(s) => sizes.push(s),
                    None => {
                        interp.stack.push(args_val);
                        return Err(AjisaiError::from("Split size is too large"));
                    }
                },
                Err(_) => {
                    interp.stack.push(args_val);
                    return Err(AjisaiError::from("Split sizes must be integers"));
                }
            }
        }
        sizes
    } else {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("SPLIT requires [sizes...] vector"));
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let result_vectors = with_stacktop_vector_target(
                interp,
                &args_val,
                is_keep_mode,
                AjisaiError::from("SPLIT requires a vector to split"),
                |vector_val| {
                    let elements = extract_vector_elements(vector_val);
                    let total_size: usize = sizes.iter().sum();
                    if total_size > elements.len() {
                        return Err(AjisaiError::from("Split sizes sum exceeds vector length"));
                    }

                    let mut current_pos = 0;
                    let mut result_vectors = Vec::new();
                    for &size in &sizes {
                        let chunk = elements[current_pos..current_pos + size].to_vec();
                        result_vectors.push(Value::from_vector(chunk));
                        current_pos += size;
                    }
                    if current_pos < elements.len() {
                        let chunk = elements[current_pos..].to_vec();
                        result_vectors.push(Value::from_vector(chunk));
                    }
                    Ok(result_vectors)
                },
            )?;

            if is_keep_mode {
                interp.stack.push(args_val);
            }
            interp.stack.extend(result_vectors);
            Ok(())
        }
        OperationTargetMode::Stack => {
            let total_size: usize = sizes.iter().sum();
            if total_size > interp.stack.len() {
                interp.stack.push(args_val);
                return Err(AjisaiError::from("Split sizes sum exceeds stack length"));
            }

            if is_keep_mode {

                let original_elements: Vec<Value> = interp.stack.iter().cloned().collect();
                let mut result_stack = Vec::new();
                let mut pos = 0;

                for &size in &sizes {
                    let chunk: Vec<Value> = original_elements[pos..pos + size].to_vec();
                    result_stack.push(Value::from_vector(chunk));
                    pos += size;
                }
                if pos < original_elements.len() {
                    result_stack.push(Value::from_vector(original_elements[pos..].to_vec()));
                }
                interp.stack.extend(result_stack);
            } else {
                let mut remaining_stack = interp.stack.split_off(0);
                let mut result_stack = Vec::new();

                for &size in &sizes {
                    let chunk: Vec<Value> = remaining_stack.drain(..size).collect();
                    result_stack.push(Value::from_vector(chunk));
                }
                if !remaining_stack.is_empty() {
                    result_stack.push(Value::from_vector(remaining_stack));
                }
                interp.stack = result_stack;
            }
            Ok(())
        }
    }
}
