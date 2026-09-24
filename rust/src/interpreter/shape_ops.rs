//! Shape Words: `ZIP` and `PUT`.
//!
//! The companion module to `ordering_ops`, on the same reasoning: each of these
//! is expressible in the existing vocabulary, and each one written that way
//! costs asymptotically more.

use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::extract_integer_from_value;
use crate::interpreter::Interpreter;
use crate::semantic::Recoverability;
use crate::types::{Interpretation, Value};

use super::ordering_ops::{elements_of, restore, take_operand};

/// `extract_integer_from_value`, with a non-integer operand reclassified as
/// `nonInteger` — PUT is the only Word that declares it in
/// `spec/words.json`'s `errorWhen`, and the shared helper serves callers
/// (GET, TAKE, COLLECT, ...) that declare no such condition, so it cannot
/// make this remap itself (the same shared-helper lesson as Phase 2's
/// tensor-conversion helpers). Catches every `StructureError` the helper can
/// produce, not just the fraction case: a fix that only caught
/// `expected == "integer" && got == "fraction"` left every other non-integer
/// shape (a string, a Vector, a Boolean, an ExactReal, ...) still falling
/// back to generic `structureError`.
fn require_integer_operand(value: &Value) -> Result<i64> {
    match extract_integer_from_value(value) {
        Err(AjisaiError::StructureError { got, .. }) => Err(AjisaiError::declared(
            "nonInteger",
            format!("expected an integer, got {}", got),
        )),
        other => other,
    }
}

/// `ZIP ( [ [ vec... ] ] -> [ [ tuple... ] ] )`: bundle equal-length vectors
/// position by position.
///
/// Three jobs in one Word. Walking features beside labels is
/// `XS YS 2 COLLECT ZIP { .. } MAP`; transposing a matrix is `M ZIP`; and
/// walking a vector with its own indices is
/// `0 N RANGE XS 2 COLLECT ZIP`. All three previously went through
/// `1 COLLECT 'IV' BIND XS IV GET`, the index-plumbing phrase that made up
/// roughly half the tokens of any program that had to look two things up at
/// the same position.
pub fn op_zip(interp: &mut Interpreter) -> Result<()> {
    let value = take_operand(interp)?;
    if value.is_nil() {
        interp.stack.push(value);
        return Ok(());
    }

    let rows = match elements_of(&value, "vector of vectors") {
        Ok(rows) => rows,
        Err(e) => {
            restore(interp, value);
            return Err(e);
        }
    };

    if rows.is_empty() {
        interp.stack.push(Value::from_vector(Vec::new()));
        return Ok(());
    }

    let mut columns: Vec<Vec<Value>> = Vec::new();
    for row in &rows {
        match row.as_vector_view() {
            Some(view) => columns.push(view.into_owned()),
            None => {
                restore(interp, value);
                return Err(AjisaiError::declared(
                    "nonVector",
                    "ZIP: expected a vector of vectors, got a row that is not itself a Vector",
                ));
            }
        }
    }

    let width = columns[0].len();
    if let Some(other) = columns.iter().find(|column| column.len() != width) {
        restore(interp, value);
        return Err(AjisaiError::VectorLengthMismatch {
            len1: width,
            len2: other.len(),
        });
    }

    // A transpose copies every cell exactly once, so the price is the whole
    // grid: rows x width, measured on the rows rather than on the outer vector
    // (whose "elements" are the rows themselves).
    let cell_units = columns
        .iter()
        .map(|column| {
            crate::interpreter::collection_meter::element_cost_of_slice(column).copies(column.len())
        })
        .fold(0u64, u64::saturating_add);
    if let Err(e) = crate::interpreter::collection_meter::charge(interp, cell_units) {
        restore(interp, value);
        return Err(e);
    }

    let out: Vec<Value> = (0..width)
        .map(|position| {
            Value::from_vector(
                columns
                    .iter()
                    .map(|column| column[position].clone())
                    .collect(),
            )
        })
        .collect();
    interp
        .stack
        .push_with_role(Value::from_vector(out), Interpretation::Unassigned);
    Ok(())
}

/// `PUT ( [ vec ] [ idx ] [ value ] -> [ vec' ] )`: a copy of `vec` with the
/// element at `idx` replaced. A negative index counts from the end, as
/// everywhere else.
///
/// The alternative was rebuilding the whole vector from an index mask and a
/// `SELECT`, or the `TAKE`/`CONCAT` surgery that spells the same thing in
/// three phrases. Updating one position is what a parameter step, a centroid
/// move, a confusion-matrix increment and a histogram bin all do.
pub fn op_put(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 3 {
        return Err(AjisaiError::StackUnderflow);
    }
    let replacement = interp.stack.pop().expect("checked by len()");
    let index_value = interp.stack.pop().expect("checked by len()");
    let target = interp.stack.pop().expect("checked by len()");

    // `PUT` answers with a copy of the vector, one element replaced, so it
    // copies the whole thing however small the edit is.
    if let Err(e) =
        crate::interpreter::collection_meter::charge_copy_of(interp, &target, target.len())
    {
        interp.stack.push(target);
        interp.stack.push(index_value);
        interp.stack.push(replacement);
        return Err(e);
    }

    let mut items = match target.as_vector_view() {
        Some(view) => view.into_owned(),
        None => {
            interp.stack.push(target);
            interp.stack.push(index_value);
            interp.stack.push(replacement);
            return Err(AjisaiError::declared(
                "nonVector",
                "PUT: expected a Vector, got a non-vector value",
            ));
        }
    };

    let raw_index = match require_integer_operand(&index_value) {
        Ok(index) => index,
        Err(e) => {
            interp.stack.push(target);
            interp.stack.push(index_value);
            interp.stack.push(replacement);
            return Err(e);
        }
    };

    let length = items.len();
    let position = if raw_index < 0 {
        length as i64 + raw_index
    } else {
        raw_index
    };
    if position < 0 || position as usize >= length {
        // A well-formed index over a well-formed Vector that names no slot is
        // the question `GET` already projects for, and `TAKE` now projects for
        // too: data that did not work out, not a malformed program
        // (LANG.FAILURE.PROJECT). `PUT` used to raise here on the grounds that
        // it answers with the whole Vector and so has no single slot to empty
        // — but the absence is of the *answer*, not of a slot, and that is
        // what a reasoned NIL says.
        interp.stack.push(Value::nil_with_reason(
            NilReason::IndexOutOfBounds,
            Recoverability::Recoverable,
        ));
        return Ok(());
    }

    items[position as usize] = replacement;
    interp
        .stack
        .push_with_role(Value::from_vector(items), Interpretation::Unassigned);
    Ok(())
}
