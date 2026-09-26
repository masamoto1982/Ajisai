//! Shape Words: `ZIP` and `PUT`.
//!
//! The companion module to `ordering_ops`, on the same reasoning: each of these
//! is expressible in the existing vocabulary, and each one written that way
//! costs asymptotically more.

use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::extract_integer_from_value;
use crate::interpreter::Interpreter;
use crate::semantic::Recoverability;
use crate::types::Value;

use super::ordering_ops::{elements_of, restore, take_operand};

/// `extract_integer_from_value`, with a non-integer operand raised as the
/// declared `invalidInteger` that every integer-taking Word shares.
fn require_integer_operand(value: &Value) -> Result<i64> {
    extract_integer_from_value(value).map_err(|e| {
        AjisaiError::declared(
            "invalidInteger",
            format!("expected an integer, got {}", e.got),
        )
    })
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
                let got = row.domain_name();
                restore(interp, value);
                return Err(AjisaiError::declared(
                    "nonVector",
                    format!("expected every row to be a Vector, got {got}"),
                ));
            }
        }
    }

    let width = columns[0].len();
    if let Some(other) = columns.iter().find(|column| column.len() != width) {
        restore(interp, value);
        return Err(AjisaiError::length_mismatch(width, other.len()));
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
    interp.stack.push(Value::from_vector(out));
    Ok(())
}

/// `PUT ( [ container ] [ key ] [ value ] -> [ container' ] )`: a copy of a
/// Vector with the element at an index replaced, or of a Record with a key
/// set — one Word for writing a container, as `GET` is one Word for reading
/// one. A negative index counts from the end, as everywhere else. A Record key
/// already present is replaced in place and an absent one is appended: a
/// Record's keys are its own to extend, where a Vector's positions are fixed
/// by its length, so an index past the end projects `indexOutOfBounds`.
///
/// The alternative was rebuilding the whole vector from an index mask and a
/// `SELECT`, or the `TAKE`/`CONCAT` surgery that spells the same thing in
/// three phrases. Updating one position is what a parameter step, a centroid
/// move, a confusion-matrix increment and a histogram bin all do.
pub fn op_put(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 3 {
        return Err(AjisaiError::stack_underflow());
    }
    let replacement = interp.stack.pop().expect("checked by len()");
    let index_value = interp.stack.pop().expect("checked by len()");
    let target = interp.stack.pop().expect("checked by len()");

    // Either container is answered as a copy with one entry changed, so the
    // whole of it is priced however small the edit is.
    if let Some(record) = target.as_record() {
        let entries = record.len();
        if let Err(e) =
            crate::interpreter::collection_meter::charge_copy_of(interp, &target, entries)
        {
            interp.stack.push(target);
            interp.stack.push(index_value);
            interp.stack.push(replacement);
            return Err(e);
        }
        let next = record.with(index_value, replacement);
        interp.stack.push(Value::from_record(next));
        return Ok(());
    }

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
            let got = target.domain_name();
            interp.stack.push(target);
            interp.stack.push(index_value);
            interp.stack.push(replacement);
            return Err(AjisaiError::declared(
                "nonContainer",
                format!("expected a Vector or a Record, got {got}"),
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
    interp.stack.push(Value::from_vector(items));
    Ok(())
}
