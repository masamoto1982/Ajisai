use super::extract_vector_elements;
use super::targeting::with_stacktop_vector_target_with_arg;
use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{
    create_number_value, extract_bigint_from_value, extract_integer_from_value,
};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::types::fraction::Fraction;
use crate::types::Stack;
use crate::types::Value;
use num_traits::ToPrimitive;

fn compute_take_bounds(len: usize, count: i64, target: &str) -> Result<(usize, usize)> {
    // Compare the magnitude in u64 before narrowing to usize. `(-count) as
    // usize` overflowed and panicked on i64::MIN (reachable via
    // `[ .. ] -9223372036854775808 TAKE`), and a bare `count as usize` would
    // silently truncate a huge count on 32-bit wasm. Working in u64 keeps both
    // the over-length rejection and the eventual narrowing exact.
    let magnitude: u64 = count.unsigned_abs();
    if magnitude > len as u64 {
        return Err(AjisaiError::from(format!(
            "Take count exceeds {} length",
            target
        )));
    }
    let take = magnitude as usize;

    if count < 0 {
        Ok((len - take, len))
    } else {
        Ok((0, take))
    }
}

pub fn op_length(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    let len = {
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

    let result =
        with_stacktop_vector_target_with_arg(interp, &count_val, is_keep_mode, |vector_val| {
            let elements = extract_vector_elements(vector_val);
            let (start, end) = compute_take_bounds(elements.len(), count, "vector")?;
            Ok(elements[start..end].to_vec())
        })?;

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
            let child = args_val
                .child(i)
                .expect("SPLIT: child index in 0..len must be valid");
            match extract_bigint_from_value(&child) {
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

    let result_vectors =
        with_stacktop_vector_target_with_arg(interp, &args_val, is_keep_mode, |vector_val| {
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
        })?;

    if is_keep_mode {
        interp.stack.push(args_val);
    }
    interp.stack.extend(result_vectors);
    Ok(())
}
