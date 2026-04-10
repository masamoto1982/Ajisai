



use super::extract_vector_elements;
use super::targeting::{with_stacktop_vector_target_no_arg, with_stacktop_vector_target_with_arg};
use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{
    extract_bigint_from_value, extract_integer_from_value, normalize_index,
};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::Value;
use num_traits::ToPrimitive;

fn parse_concat_count(
    interp: &mut Interpreter,
    default_count: i64,
) -> Result<(i64, Option<Value>)> {
    let Some(top) = interp.stack.last() else {
        return Err(AjisaiError::StackUnderflow);
    };

    let Ok(count_bigint) = extract_bigint_from_value(top) else {
        return Ok((default_count, None));
    };

    let count_value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let count = match count_bigint.to_i64() {
        Some(value) => value,
        None => {
            interp.stack.push(count_value);
            return Err(AjisaiError::from("Count is too large"));
        }
    };

    Ok((count, Some(count_value)))
}

fn concat_values(values: Vec<Value>, is_reversed: bool) -> Value {
    let mut ordered = values;
    if is_reversed {
        ordered.reverse();
    }

    let mut result_vec = Vec::new();
    for value in ordered {
        if value.is_vector() {
            result_vec.extend_from_slice(extract_vector_elements(&value));
        } else {
            result_vec.push(value);
        }
    }

    Value::from_vector(result_vec)
}

fn parse_range_bound(args_val: &Value, index: usize, label: &str) -> Result<i64> {
    let bigint = extract_bigint_from_value(args_val.get_child(index).unwrap())
        .map_err(|_| AjisaiError::from(format!("RANGE {} must be an integer", label)))?;
    bigint
        .to_i64()
        .ok_or_else(|| AjisaiError::from(format!("RANGE {} is too large", label)))
}

fn parse_range_args(args_val: &Value) -> Result<(i64, i64, i64)> {
    if !args_val.is_vector() {
        return Err(AjisaiError::from(
            "RANGE requires [start end] or [start end step]",
        ));
    }

    let n = args_val.len();
    if n < 2 || n > 3 {
        return Err(AjisaiError::from(
            "RANGE requires [start end] or [start end step]",
        ));
    }

    let start = parse_range_bound(args_val, 0, "start")?;
    let end = parse_range_bound(args_val, 1, "end")?;
    let step = if n == 3 {
        parse_range_bound(args_val, 2, "step")?
    } else if start <= end {
        1
    } else {
        -1
    };

    Ok((start, end, step))
}

fn parse_reorder_indices(indices_val: &Value) -> Result<Vec<i64>> {
    if indices_val.is_vector() {
        let n = indices_val.len();
        if n == 0 {
            return Err(AjisaiError::from("REORDER requires non-empty index list"));
        }

        let mut indices = Vec::with_capacity(n);
        for i in 0..n {
            let idx = extract_integer_from_value(indices_val.get_child(i).unwrap())
                .map_err(|_| AjisaiError::from("REORDER indices must be integers"))?;
            indices.push(idx);
        }
        return Ok(indices);
    }

    let single = extract_integer_from_value(indices_val)
        .map_err(|_| AjisaiError::from("REORDER requires index list"))?;
    Ok(vec![single])
}


pub fn op_concat(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    let default_count = match interp.operation_target_mode {
        OperationTargetMode::StackTop => 2,
        OperationTargetMode::Stack => interp.stack.len() as i64,
    };
    let (count_i64, count_value_opt) = parse_concat_count(interp, default_count)?;

    let abs_count = count_i64.unsigned_abs() as usize;
    let is_reversed = count_i64 < 0;

    if interp.stack.len() < abs_count {
        if let Some(count_val) = count_value_opt {
            interp.stack.push(count_val);
        }
        return Err(AjisaiError::StackUnderflow);
    }

    let values_to_concat: Vec<Value> = if is_keep_mode {
        let stack_len = interp.stack.len();
        interp.stack[stack_len - abs_count..].to_vec()
    } else {
        interp.stack.split_off(interp.stack.len() - abs_count)
    };

    if is_keep_mode {
        if let Some(count_val) = count_value_opt {
            interp.stack.push(count_val);
        }
    }
    interp
        .stack
        .push(concat_values(values_to_concat, is_reversed));
    Ok(())
}






pub fn op_reverse(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let reversed =
                with_stacktop_vector_target_no_arg(interp, is_keep_mode, |vector_val| {
                    let mut v = extract_vector_elements(vector_val).to_vec();
                    v.reverse();
                    Ok(Value::from_vector(v))
                })?;
            interp.stack.push(reversed);
            Ok(())
        }
        OperationTargetMode::Stack => {
            if is_keep_mode {
                let mut reversed = interp.stack.clone();
                reversed.reverse();
                interp.stack.extend(reversed);
            } else {
                interp.stack.reverse();
            }
            Ok(())
        }
    }
}


pub fn op_range(interp: &mut Interpreter) -> Result<()> {

    let args_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;


    let (start, end, step) = match parse_range_args(&args_val) {
        Ok(values) => values,
        Err(error) => {
            interp.stack.push(args_val);
            return Err(error);
        }
    };

    if step == 0 {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("RANGE step cannot be 0"));
    }

    if (start < end && step < 0) || (start > end && step > 0) {
        interp.stack.push(args_val);
        return Err(AjisaiError::from(
            "RANGE would create an infinite sequence (check start, end, and step values)",
        ));
    }

    let mut range_vec = Vec::new();
    let mut current = start;

    if step > 0 {
        while current <= end {
            range_vec.push(Value::from_fraction(Fraction::from(current)));
            current += step;
        }
    } else {
        while current >= end {
            range_vec.push(Value::from_fraction(Fraction::from(current)));
            current += step;
        }
    }

    interp.stack.push(Value::from_vector(range_vec));

    Ok(())
}






pub fn op_reorder(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;


    let indices_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;


    let indices = match parse_reorder_indices(&indices_val) {
        Ok(values) => values,
        Err(error) => {
            interp.stack.push(indices_val);
            return Err(error);
        }
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let reordered =
                with_stacktop_vector_target_with_arg(interp, &indices_val, is_keep_mode, |target_val| {
                    let len = target_val.len();
                    if len == 0 {
                        return Err(AjisaiError::from("REORDER: target vector is empty"));
                    }

                    let mut result = Vec::with_capacity(indices.len());
                    for &idx in &indices {
                        let actual =
                            normalize_index(idx, len).ok_or(AjisaiError::IndexOutOfBounds {
                                index: idx,
                                length: len,
                            })?;
                        result.push(target_val.get_child(actual).unwrap().clone());
                    }

                    if result.is_empty() {
                        Ok(Value::nil())
                    } else {
                        Ok(Value::from_vector(result))
                    }
                })?;

            if is_keep_mode {
                interp.stack.push(indices_val);
            }
            interp.stack.push(reordered);
            Ok(())
        }
        OperationTargetMode::Stack => {
            let len = interp.stack.len();

            if len == 0 {
                interp.stack.push(indices_val);
                return Err(AjisaiError::from("REORDER: stack is empty"));
            }

            let mut result = Vec::with_capacity(indices.len());
            for &idx in &indices {
                let actual = match normalize_index(idx, len) {
                    Some(i) => i,
                    None => {
                        interp.stack.push(indices_val);
                        return Err(AjisaiError::IndexOutOfBounds {
                            index: idx,
                            length: len,
                        });
                    }
                };
                result.push(interp.stack[actual].clone());
            }

            if !is_keep_mode {
                interp.stack.clear();
            }
            interp.stack.extend(result);
            Ok(())
        }
    }
}


pub fn op_collect(interp: &mut Interpreter) -> Result<()> {
    let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let count_bigint = match extract_bigint_from_value(&count_val) {
        Ok(bi) => bi,
        Err(_) => {
            interp.stack.push(count_val);
            return Err(AjisaiError::create_structure_error(
                "integer",
                "other format",
            ));
        }
    };

    let count: usize = match count_bigint.to_usize() {
        Some(c) if c > 0 => c,
        _ => {
            interp.stack.push(count_val);
            return Err(AjisaiError::from(
                "COLLECT count must be a positive integer",
            ));
        }
    };

    if interp.stack.len() < count {
        interp.stack.push(count_val);
        return Err(AjisaiError::StackUnderflow);
    }

    let collected: Vec<Value> = interp.stack.split_off(interp.stack.len() - count);

    interp.stack.push(Value::from_vector(collected));
    Ok(())
}
