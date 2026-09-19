//! Search Words: `MEMBER` and `BSEARCH` (LANG.VALUES.VECTOR).
//!
//! Both answer a question `INDEX-OF` already answers one probe at a time, and
//! both earn their slot on cost (docs/dev/vocabulary-100-work-order-2026-09.md
//! §1): `MEMBER` indexes the vector once instead of scanning it once per probe,
//! and `BSEARCH` halves a range until it is empty — a loop whose length depends
//! on the data, which a language with no unbounded loop cannot write at all.

use std::collections::HashSet;

use super::sort::compare_for_sort;
use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::collection_meter::{self, ScanMeter};
use crate::interpreter::comparison_scalar::OrderOutcome;
use crate::interpreter::value_extraction_helpers::extract_operands;
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::semantic::Recoverability;
use crate::types::{Interpretation, Value};

fn restore_operands(interp: &mut Interpreter, operands: Vec<Value>) {
    if interp.consumption_mode != ConsumptionMode::Keep {
        interp.stack.extend(operands);
    }
}

fn non_vector(word: &str) -> AjisaiError {
    AjisaiError::declared(
        "nonVector",
        format!("{word}: expected a Vector, got a non-vector value"),
    )
}

/// `MEMBER ( [ vec ] [ probes ] -> [ truths ] )`: which probes occur in the
/// vector, by the value equality `UNIQUE` and `INDEX-OF` use. A Vector of
/// probes answers a Vector of truths, lane for lane; a single probe answers a
/// single truth.
pub fn op_member(interp: &mut Interpreter) -> Result<()> {
    let operands = extract_operands(interp, 2)?;
    let Some(elements) = operands[0].as_vector_view().map(|view| view.into_owned()) else {
        restore_operands(interp, operands);
        return Err(non_vector("MEMBER"));
    };

    // One hash pass over the vector, priced as UNIQUE's is: the fixed
    // per-element cost of finding a bucket, whatever the vocabulary size.
    let meter = ScanMeter::new(&elements);
    let mut index: HashSet<&Value> = HashSet::with_capacity(elements.len());
    for (completed, item) in elements.iter().enumerate() {
        if let Err(e) = meter.charge_scan_of(interp, completed) {
            drop(index);
            restore_operands(interp, operands);
            return Err(e);
        }
        index.insert(item);
    }

    let answer = match operands[1].as_vector_view() {
        Some(probes) => Value::from_vector(
            probes
                .iter()
                .map(|probe| Value::from_bool(index.contains(probe)))
                .collect(),
        ),
        None => Value::from_bool(index.contains(&operands[1])),
    };
    drop(index);
    interp.stack.push(answer);
    Ok(())
}

/// What one binary search answered.
enum Found {
    At(usize),
    Absent,
    /// A comparison exhausted its refinement budget (LANG.VALUES.EXACT).
    Undecided,
}

/// The first index in ascending `sorted` whose element equals `key`, by
/// halving. `compare_for_sort` decides; a structurally non-comparable key is
/// its `nonComparableElement`.
fn lower_bound(sorted: &[Value], key: &Value) -> Result<Found> {
    let (mut lo, mut hi) = (0usize, sorted.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match compare_for_sort(&sorted[mid], key)? {
            OrderOutcome::Decided(std::cmp::Ordering::Less) => lo = mid + 1,
            OrderOutcome::Decided(_) => hi = mid,
            OrderOutcome::Undecided(_) => return Ok(Found::Undecided),
        }
    }
    if lo == sorted.len() {
        return Ok(Found::Absent);
    }
    match compare_for_sort(&sorted[lo], key)? {
        OrderOutcome::Decided(std::cmp::Ordering::Equal) => Ok(Found::At(lo)),
        OrderOutcome::Decided(_) => Ok(Found::Absent),
        OrderOutcome::Undecided(_) => Ok(Found::Undecided),
    }
}

/// `BSEARCH ( [ sorted ] [ keys ] -> [ indices ] )`: the index of each key in
/// an ascending vector. The order is checked first — one pass — because a
/// binary search over unordered data would answer something rather than
/// nothing, and an unsorted operand is the program being wrong about its own
/// data (`unsortedInput`). A key that is not there is a `missingField` lane; a
/// comparison that exhausts its budget makes the whole answer `undecidable`,
/// as it does for `SORT`.
pub fn op_bsearch(interp: &mut Interpreter) -> Result<()> {
    let operands = extract_operands(interp, 2)?;
    let Some(sorted) = operands[0].as_vector_view().map(|view| view.into_owned()) else {
        restore_operands(interp, operands);
        return Err(non_vector("BSEARCH"));
    };

    // The order check walks the vector once; priced like INDEX-OF's miss.
    let units = collection_meter::element_cost_of_slice(&sorted)
        .probe()
        .saturating_mul(sorted.len() as u64);
    if let Err(e) = collection_meter::charge(interp, units) {
        restore_operands(interp, operands);
        return Err(e);
    }
    let undecidable = || Value::nil_with_reason(NilReason::Undecidable, Recoverability::Retryable);
    for pair in sorted.windows(2) {
        match compare_for_sort(&pair[0], &pair[1]) {
            Ok(OrderOutcome::Decided(std::cmp::Ordering::Greater)) => {
                restore_operands(interp, operands);
                return Err(AjisaiError::declared(
                    "unsortedInput",
                    "BSEARCH: expected an ascending Vector, got one that is not in order",
                ));
            }
            Ok(OrderOutcome::Decided(_)) => {}
            Ok(OrderOutcome::Undecided(_)) => {
                interp.stack.push(undecidable());
                return Ok(());
            }
            Err(e) => {
                restore_operands(interp, operands);
                return Err(e);
            }
        }
    }

    let lane = |found: Found| match found {
        Found::At(index) => Value::from_int(index as i64),
        Found::Absent => {
            Value::nil_with_reason(NilReason::MissingField, Recoverability::Recoverable)
        }
        Found::Undecided => undecidable(),
    };
    let answer = match operands[1].as_vector_view() {
        Some(keys) => {
            let mut lanes = Vec::with_capacity(keys.len());
            for key in keys.iter() {
                match lower_bound(&sorted, key) {
                    Ok(Found::Undecided) => {
                        interp.stack.push(undecidable());
                        return Ok(());
                    }
                    Ok(found) => lanes.push(lane(found)),
                    Err(e) => {
                        restore_operands(interp, operands);
                        return Err(e);
                    }
                }
            }
            Value::from_vector(lanes)
        }
        None => match lower_bound(&sorted, &operands[1]) {
            Ok(found) => {
                let value = lane(found);
                if matches!(value.data, crate::types::ValueData::Scalar(_)) {
                    interp
                        .stack
                        .push_with_role(value, Interpretation::RawNumber);
                } else {
                    interp.stack.push(value);
                }
                return Ok(());
            }
            Err(e) => {
                restore_operands(interp, operands);
                return Err(e);
            }
        },
    };
    interp.stack.push(answer);
    Ok(())
}
