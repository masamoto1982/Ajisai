use super::extract_vector_elements;
use super::targeting::with_stacktop_vector_target_with_arg;
use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{
    create_number_value, extract_integer_from_value,
};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::types::fraction::Fraction;
use crate::types::Value;

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

    // `LENGTH` declares `consumption: eat` with `[ vec ] -> [ count ]`: the
    // measured vector leaves the stack unless `KEEP` is in force.
    let target_val = if is_keep_mode {
        interp.stack.last().cloned().ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    let len = {
        if target_val.is_nil() {
            0
        } else if target_val.is_vector() {
            extract_vector_elements(&target_val).len()
        } else {
            if !is_keep_mode {
                interp.stack.push(target_val);
            }
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
    interp.stack.push(Value::from_vector(result));
    Ok(())
}
