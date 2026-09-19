use super::extract_vector_elements;
use super::targeting::with_stacktop_vector_target_with_arg;
use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::{
    create_number_value, extract_integer_from_value,
};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::semantic::Recoverability;
use crate::types::fraction::Fraction;
use crate::types::Value;

/// The slice `TAKE` answers with, or `None` when the count names a position
/// past the end.
///
/// Asking for more than there is, is an index question — the same question
/// `GET` answers past the end, and it is answered the same way: with a
/// reasoned absence rather than a raise. A well-formed count over a
/// well-formed Vector is data that did not work out, not a malformed program,
/// and LANG.FAILURE.PROJECT is what the trichotomy reserves for that. `TAKE`
/// raising where `GET` projected was the standing example of the two
/// disagreeing (`spec/words.schema.json`'s note on `errorWhen`); they now
/// agree.
///
/// Compare the magnitude in u64 before narrowing to usize. `(-count) as
/// usize` overflowed and panicked on i64::MIN (reachable via
/// `[ .. ] -9223372036854775808 TAKE`), and a bare `count as usize` would
/// silently truncate a huge count on 32-bit wasm. Working in u64 keeps both
/// the past-the-end test and the eventual narrowing exact.
fn compute_take_bounds(len: usize, count: i64) -> Option<(usize, usize)> {
    let magnitude: u64 = count.unsigned_abs();
    if magnitude > len as u64 {
        return None;
    }
    let take = magnitude as usize;

    if count < 0 {
        Some((len - take, len))
    } else {
        Some((0, take))
    }
}

pub fn op_length(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    // `LENGTH` declares `consumption: eat` with `[ vec ] -> [ count ]`: the
    // measured vector leaves the stack unless `KEEP` is in force.
    let target_val = if is_keep_mode {
        interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    let len = {
        if target_val.is_nil() {
            0
        } else if target_val.is_vector() {
            // `Value::len()` reads the count. This used to call
            // `extract_vector_elements`, which deep-copies the whole element
            // vector out of the `Cow` and then asks the copy for its length —
            // 18 ms and 100,000 element clones to answer a question the header
            // already holds. Measured by
            // `examples/collection_word_calibration`; a Word documented O(1)
            // was the third most expensive linear Word in the family.
            target_val.len()
        } else {
            if !is_keep_mode {
                interp.stack.push(target_val);
            }
            return Err(AjisaiError::declared(
                "nonVector",
                "LENGTH: expected a Vector, got a non-vector value",
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
        // `invalidCount`: TAKE's own declared condition for a count operand
        // that isn't a well-formed integer.
        Err(AjisaiError::StructureError { got, .. }) => {
            interp.stack.push(count_val);
            return Err(AjisaiError::declared(
                "invalidCount",
                format!("TAKE: expected an integer count, got {}", got),
            ));
        }
        Err(e) => {
            interp.stack.push(count_val);
            return Err(e);
        }
    };

    // Priced on the prefix `TAKE` will copy, not on the vector it was handed:
    // taking 100 elements out of 100,000 copies 100 of them. The count is
    // clamped here only for pricing; `compute_take_bounds` still decides
    // whether an over-long count is an error.
    let wanted = count.unsigned_abs() as usize;
    if let Err(e) =
        crate::interpreter::collection_meter::charge_stacktop_copy(interp, |len| wanted.min(len))
    {
        interp.stack.push(count_val);
        return Err(e);
    }

    let result =
        with_stacktop_vector_target_with_arg(interp, &count_val, is_keep_mode, |vector_val| {
            let elements = extract_vector_elements(vector_val);
            Ok(compute_take_bounds(elements.len(), count)
                .map(|(start, end)| Value::from_vector(elements[start..end].to_vec()))
                .unwrap_or_else(|| {
                    Value::nil_with_reason(NilReason::IndexOutOfBounds, Recoverability::Recoverable)
                }))
        })?;

    if is_keep_mode {
        interp.stack.push(count_val);
    }
    interp.stack.push(result);
    Ok(())
}
