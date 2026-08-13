//! Ordering and grouping Words: `ORDER`, `UNIQUE`, `TALLY`, `GROUP`.
//!
//! Every one of these is expressible in the existing vocabulary — and every
//! one of them, written that way, costs asymptotically more than the same work
//! done here. Ordering by a key is the clearest case: the reference's stable
//! ranking idiom is O(n²) interpreted steps, so `SORT` handled five thousand
//! elements natively while a k-nearest-neighbour classifier stopped at ninety.
//! LANG.AUTHORITY.IDENTITY keeps a Word out of Core when a user definition
//! *expresses* it; these are here because expressibility was never the whole
//! question.

use crate::error::{AjisaiError, Result};
use crate::interpreter::collection_meter::{charge_comparison_sort, ScanMeter};
use crate::interpreter::sort::order_indices;
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::types::{Interpretation, Value};

/// Take the Word's single operand, honouring `KEEP`.
pub(super) fn take_operand(interp: &mut Interpreter) -> Result<Value> {
    if interp.consumption_mode == ConsumptionMode::Keep {
        interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)
    }
}

/// Put an operand back when the Word failed and did consume it.
pub(super) fn restore(interp: &mut Interpreter, value: Value) {
    if interp.consumption_mode != ConsumptionMode::Keep {
        interp.stack.push(value);
    }
}

/// The elements of a vector operand, or a structure error naming what came
/// instead. `expected` is a noun phrase: the diagnosis template supplies
/// "Structure error: expected _, got _" and the locus supplies the Word.
pub(super) fn elements_of(value: &Value, expected: &str) -> Result<Vec<Value>> {
    match value.as_vector_view() {
        Some(view) => Ok(view.into_owned()),
        None => Err(AjisaiError::create_structure_error(
            expected,
            "non-vector value",
        )),
    }
}

/// `ORDER ( [ vec ] -> [ permutation ] )`: the indices of `vec` in the order
/// that would sort it ascending, ties in original index order.
///
/// This is the Word the whole ordering family was missing. `SORT` answers
/// *what* the sorted values are and throws away *where* they came from, so
/// anything that has to carry a payload alongside the key — the k nearest
/// neighbours, the top-k scores, a median's position, a ranking, an argmin —
/// had to rebuild the ordering by counting, at quadratic cost. With the
/// permutation in hand each of those is `ORDER` and a `GET`.
///
/// Stability is part of the contract, not an accident of the sort used: two
/// equal keys keep their original order, which is what makes "the 3 nearest"
/// a well-defined set when two distances tie.
pub fn op_order(interp: &mut Interpreter) -> Result<()> {
    let value = take_operand(interp)?;
    if value.is_nil() {
        interp.stack.push(value);
        return Ok(());
    }

    let items = match elements_of(&value, "vector") {
        Ok(items) => items,
        Err(e) => {
            restore(interp, value);
            return Err(e);
        }
    };

    if let Err(e) = charge_comparison_sort(interp, &items) {
        restore(interp, value);
        return Err(e);
    }

    match order_indices(&items) {
        Ok(perm) => {
            let out: Vec<Value> = perm
                .into_iter()
                .map(|i| Value::from_int(i as i64))
                .collect();
            interp
                .stack
                .push_with_role(Value::from_vector(out), Interpretation::Unassigned);
            Ok(())
        }
        Err(e) => {
            restore(interp, value);
            Err(e)
        }
    }
}

/// The distinct elements of `items` in first-occurrence order, paired with how
/// many times each occurred. One pass shared by `UNIQUE` and `TALLY` so the
/// two can never disagree about either the order or the counts.
///
/// Equality is value equality, so this works over the domains `SORT` does not:
/// class labels are usually text, and a grouping key is whatever the data
/// says it is.
///
/// The scan is `n × distinct`, and `distinct` is the thing the scan is finding
/// out, so the charge is taken per element rather than at the entry — against
/// the number of distinct values in hand, which bounds the probes that element
/// can make. See `collection_meter::ScanMeter`.
fn distinct_with_counts(interp: &mut Interpreter, items: &[Value]) -> Result<Vec<(Value, usize)>> {
    let meter = ScanMeter::new(items);
    let mut out: Vec<(Value, usize)> = Vec::new();
    for (completed, item) in items.iter().enumerate() {
        meter.charge_scan_of(interp, completed, out.len())?;
        match out.iter_mut().find(|(seen, _)| seen == item) {
            Some((_, count)) => *count += 1,
            None => {
                meter.charge_retained(interp, completed)?;
                out.push((item.clone(), 1));
            }
        }
    }
    Ok(out)
}

/// `UNIQUE ( [ vec ] -> [ distinct ] )`: the distinct elements in
/// first-occurrence order.
///
/// First-occurrence rather than sorted order, because it has to work on values
/// that have no order at all — the labels of a classifier, the tokens of a
/// vocabulary, the keys of a grouping.
pub fn op_unique(interp: &mut Interpreter) -> Result<()> {
    let value = take_operand(interp)?;
    if value.is_nil() {
        interp.stack.push(value);
        return Ok(());
    }
    let items = match elements_of(&value, "vector") {
        Ok(items) => items,
        Err(e) => {
            restore(interp, value);
            return Err(e);
        }
    };
    match distinct_with_counts(interp, &items) {
        Ok(distinct) => {
            let out: Vec<Value> = distinct.into_iter().map(|(value, _)| value).collect();
            interp
                .stack
                .push_with_role(Value::from_vector(out), Interpretation::Unassigned);
            Ok(())
        }
        Err(e) => {
            restore(interp, value);
            Err(e)
        }
    }
}

/// `TALLY ( [ vec ] -> [ counts ] )`: how many times each distinct element
/// occurs, in the same order `UNIQUE` reports them.
///
/// Designed as `UNIQUE`'s pair rather than as a map from value to count: two
/// vectors that line up positionally compose with everything else here, and a
/// map would be a new value domain for one Word to own.
pub fn op_tally(interp: &mut Interpreter) -> Result<()> {
    let value = take_operand(interp)?;
    if value.is_nil() {
        interp.stack.push(value);
        return Ok(());
    }
    let items = match elements_of(&value, "vector") {
        Ok(items) => items,
        Err(e) => {
            restore(interp, value);
            return Err(e);
        }
    };
    match distinct_with_counts(interp, &items) {
        Ok(distinct) => {
            let out: Vec<Value> = distinct
                .into_iter()
                .map(|(_, count)| Value::from_int(count as i64))
                .collect();
            interp
                .stack
                .push_with_role(Value::from_vector(out), Interpretation::Unassigned);
            Ok(())
        }
        Err(e) => {
            restore(interp, value);
            Err(e)
        }
    }
}

/// `GROUP ( [ values ] [ keys ] -> [ [ group... ] ] )`: bundle `values` by the
/// key at the same position, groups in the order `UNIQUE keys` reports them.
///
/// The core of a centroid update, a decision-tree split, a per-class tally and
/// a stratified partition. Written out, each of those was a nested scan over
/// the keys with the index plumbing done by hand.
pub fn op_group(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    let keep = interp.consumption_mode == ConsumptionMode::Keep;
    let keys_value = interp.stack.pop().expect("checked by len()");
    let values_value = interp.stack.pop().expect("checked by len()");

    let put_back = |interp: &mut Interpreter, values: &Value, keys: &Value| {
        if !keep {
            interp.stack.push(values.clone());
            interp.stack.push(keys.clone());
        }
    };

    let values = match elements_of(&values_value, "vector as first operand") {
        Ok(items) => items,
        Err(e) => {
            put_back(interp, &values_value, &keys_value);
            return Err(e);
        }
    };
    let keys = match elements_of(&keys_value, "vector as second operand") {
        Ok(items) => items,
        Err(e) => {
            put_back(interp, &values_value, &keys_value);
            return Err(e);
        }
    };

    if values.len() != keys.len() {
        put_back(interp, &values_value, &keys_value);
        return Err(AjisaiError::VectorLengthMismatch {
            len1: values.len(),
            len2: keys.len(),
        });
    }

    // Two prices, because `GROUP` does two things: it scans the keys for a
    // match (`n × distinct`, charged per key against the groups in hand) and it
    // copies every value into a bucket (`n`, charged as each one lands).
    let key_meter = ScanMeter::new(&keys);
    let value_meter = ScanMeter::new(&values);
    let mut groups: Vec<(Value, Vec<Value>)> = Vec::new();
    for (completed, (value, key)) in values.iter().zip(keys.iter()).enumerate() {
        let charged = key_meter
            .charge_scan_of(interp, completed, groups.len())
            .and_then(|()| value_meter.charge_retained(interp, completed));
        if let Err(e) = charged {
            put_back(interp, &values_value, &keys_value);
            return Err(e);
        }
        match groups.iter_mut().find(|(seen, _)| seen == key) {
            Some((_, bucket)) => bucket.push(value.clone()),
            None => {
                if let Err(e) = key_meter.charge_retained(interp, completed) {
                    put_back(interp, &values_value, &keys_value);
                    return Err(e);
                }
                groups.push((key.clone(), vec![value.clone()]));
            }
        }
    }

    if keep {
        interp.stack.push(values_value);
        interp.stack.push(keys_value);
    }
    let out: Vec<Value> = groups
        .into_iter()
        .map(|(_, bucket)| Value::from_vector(bucket))
        .collect();
    interp
        .stack
        .push_with_role(Value::from_vector(out), Interpretation::Unassigned);
    Ok(())
}
