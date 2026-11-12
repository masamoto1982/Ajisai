//! Element lifting for the fixed-arity Words whose law is stated lane by lane
//! (LANG.COLLECTIONS.LIFT).
//!
//! The clause is one rule, so it is one function: "A scalar combines with
//! every element of a vector, however that vector is nested … Paired axes
//! combine when their lengths are equal, and when one of them is 1 that
//! operand's single lane is reused across the other's length." Arithmetic
//! reaches that rule through the tensor path; the truth-order connectives and comparisons
//! reach it through here, so the one-lane broadcast means the same thing in
//! all three families instead of being spelled once per family — the
//! comparison Words used to accept `[ 1 2 3 ] [ 2 2 2 ] GT` and refuse
//! `[ 1 2 3 ] [ 2 ] GT`, which is the clause's own example with the numbers
//! changed.
//!
//! The lift is over a fixed number of operands rather than exactly two,
//! because `SELECT` is the same law with three: its truth operand chooses
//! between the other two lane by lane, and nothing about the alignment
//! changes for having one more operand to align.

use std::borrow::Cow;

use crate::error::{AjisaiError, Result};
use crate::types::{RecordData, Value, ValueData};

/// How one operand supplies its lanes at the current axis.
///
/// The Vector arm holds the `Cow` `as_vector_view` hands back rather than a
/// slice: a Tensor materializes its nested view on demand, so the lanes of a
/// Tensor operand are owned here and would dangle behind a borrow.
enum Lanes<'a> {
    /// Not a Vector here: the same value combines with every lane.
    Scalar(&'a Value),
    /// A Vector. One of length 1 imposes no extent — its single lane is
    /// reused across whatever length the other operands set.
    Vector(Cow<'a, [Value]>),
}

impl<'a> Lanes<'a> {
    fn of(value: &'a Value) -> Lanes<'a> {
        match value.as_vector_view() {
            None => Lanes::Scalar(value),
            Some(items) => Lanes::Vector(items),
        }
    }

    /// The extent this operand imposes on the axis, or `None` when it imposes
    /// none (a scalar, or a reused single lane).
    fn extent(&self) -> Option<usize> {
        match self {
            Lanes::Scalar(_) => None,
            Lanes::Vector(items) => vector_extent(items.len()),
        }
    }

    fn is_vector(&self) -> bool {
        matches!(self, Lanes::Vector(_))
    }

    fn lane(&self, index: usize) -> &Value {
        match self {
            Lanes::Scalar(value) => value,
            Lanes::Vector(items) if items.len() == 1 => &items[0],
            Lanes::Vector(items) => &items[index],
        }
    }
}

/// The extent a Vector of `len` lanes sets on the lifted axis: none when it
/// holds one lane, which is reused across whatever length the others set.
pub(crate) fn vector_extent(len: usize) -> Option<usize> {
    (len != 1).then_some(len)
}

/// The extent the operands set together: `None` when none sets one, or the
/// first width set and the first that disagrees with it. `truth_ops` aligns
/// its columns by the same rule.
pub(crate) fn common_extent(
    extents: impl IntoIterator<Item = Option<usize>>,
) -> std::result::Result<Option<usize>, (usize, usize)> {
    let mut width = None;
    for extent in extents.into_iter().flatten() {
        match width {
            None => width = Some(extent),
            Some(w) if w == extent => {}
            Some(w) => return Err((w, extent)),
        }
    }
    Ok(width)
}

/// Apply `scalar` to the aligned lanes of `operands`, every operand lifted.
///
/// `scalar` sees only operands that are not Vectors or Records at this
/// level, so it states the scalar law alone and never repeats the alignment.
/// Where the extents disagree the result is the `shapeMismatch` ERROR the
/// clause requires, naming the first axis on which two operands disagree.
pub(crate) fn lift_lanes<const N: usize>(
    operands: [&Value; N],
    scalar: impl Fn([&Value; N]) -> Result<Value>,
) -> Result<Value> {
    if let Some(flat) = lift_flat(operands, &scalar) {
        return flat;
    }
    let mut adapter = |lanes: &[&Value]| -> Result<Value> {
        let lanes: [&Value; N] = lanes
            .try_into()
            .expect("lift_lanes_dyn hands back as many operands as it was given");
        scalar(lanes)
    };
    lift_lanes_dyn(&operands, &[true; N], &mut adapter)
}

/// [`lift_lanes`] where every operand is a leaf or a Vector of leaves: one
/// monomorphized loop over the lanes, the scalar law inlined. `None` when the
/// operands need the general walk — a Record, a Tensor, a nested lane, or
/// extents that disagree (whose `shapeMismatch` the walk words).
#[inline]
fn lift_flat<const N: usize>(
    operands: [&Value; N],
    scalar: &impl Fn([&Value; N]) -> Result<Value>,
) -> Option<Result<Value>> {
    // The extent the Vectors set: `None` before any, a length-1 Vector
    // setting none of its own. An empty one is left to the general walk.
    let mut seen = false;
    let mut width = 1;
    for value in operands {
        match &value.data {
            ValueData::Vector(items) => match items.len() {
                0 => return None,
                1 => seen = true,
                len if !seen || width == 1 => (seen, width) = (true, len),
                len if len == width => {}
                _ => return None,
            },
            ValueData::Tensor { .. } | ValueData::Record(_) | ValueData::Truths(_) => return None,
            _ => {}
        }
    }
    if !seen {
        return None;
    }
    fn lane(value: &Value, index: usize) -> Option<&Value> {
        let lane = match &value.data {
            ValueData::Vector(items) if items.len() == 1 => &items[0],
            ValueData::Vector(items) => &items[index],
            _ => return Some(value),
        };
        (!is_container(lane)).then_some(lane)
    }
    let mut result = Vec::with_capacity(width);
    for index in 0..width {
        let mut lanes = operands;
        for slot in lanes.iter_mut() {
            *slot = lane(slot, index)?;
        }
        match scalar(lanes) {
            Ok(value) => result.push(value),
            Err(e) => return Some(Err(e)),
        }
    }
    Some(Ok(Value::from_vector(result)))
}

/// The lifting rule over any number of operands, of which only those marked
/// in `lifted` are aligned; the rest are handed to every lane whole.
///
/// A Record lifts first, in the value direction (LANG.RECORDS.STRUCTURE):
/// lifted Records must share their key sequence, pair value by value, and
/// every other operand — a Vector included — combines with each value. Then
/// Vectors align lane by lane, a one-element Vector's lane reused across the
/// others' length. What remains goes to `scalar`.
pub(crate) fn lift_lanes_dyn(
    operands: &[&Value],
    lifted: &[bool],
    scalar: &mut dyn FnMut(&[&Value]) -> Result<Value>,
) -> Result<Value> {
    let records: Vec<(usize, &RecordData)> = operands
        .iter()
        .zip(lifted)
        .enumerate()
        .filter_map(|(i, (v, l))| {
            if *l {
                v.as_record().map(|r| (i, r))
            } else {
                None
            }
        })
        .collect();
    if let Some((_, first)) = records.first() {
        if let Some((_, other)) = records.iter().find(|(_, r)| !r.same_keys(first)) {
            return Err(AjisaiError::declared(
                "shapeMismatch",
                format!(
                    "expected Records with the same keys, got {} keys against {} keys that do not line up",
                    first.len(),
                    other.len()
                ),
            ));
        }
        let mut values = Vec::with_capacity(first.len());
        for k in 0..first.len() {
            let mut next: Vec<&Value> = operands.to_vec();
            for (i, record) in &records {
                next[*i] = &record.values()[k];
            }
            values.push(lift_lanes_dyn(&next, lifted, scalar)?);
        }
        return Ok(Value::from_record(
            RecordData::new(first.keys().to_vec(), values)
                .expect("the keys are one Record's own, so distinct and aligned"),
        ));
    }

    // Two or more Vectors pair by shape, aligned at the innermost axis; a
    // single Vector meets only leaves, which combine with every element
    // however it nests, a ragged one included.
    let vectors = operands
        .iter()
        .zip(lifted)
        .filter(|(v, l)| **l && v.as_vector_view().is_some())
        .count();
    if vectors >= 2 {
        let ranks = aligned_ranks(operands, lifted)?;
        return walk_aligned(operands, &ranks, scalar);
    }
    walk_lanes(operands, lifted, scalar)
}

/// The rank of each operand that pairs by shape (0 for a leaf or an operand
/// not lifted), once every Vector among them is known to be rectangular and
/// their shapes to broadcast: aligned at the innermost axis, an axis of
/// length 1 reused across the other's length (LANG.COLLECTIONS.LIFT).
fn aligned_ranks(operands: &[&Value], lifted: &[bool]) -> Result<Vec<usize>> {
    let mut joined: Vec<usize> = Vec::new();
    let mut ranks = Vec::with_capacity(operands.len());
    for (value, l) in operands.iter().zip(lifted) {
        if !*l || value.as_vector_view().is_none() {
            ranks.push(0);
            continue;
        }
        let Some(shape) = lift_shape(value) else {
            return Err(AjisaiError::declared(
                "shapeMismatch",
                "expected two Vectors with a shape, got a ragged one",
            ));
        };
        joined = crate::interpreter::tensor_ops::broadcast_shape(&joined, &shape)?;
        ranks.push(shape.len());
    }
    Ok(ranks)
}

/// The shape of a rectangular nesting, every non-Vector a leaf, or `None`
/// for a ragged one.
fn lift_shape(value: &Value) -> Option<Vec<usize>> {
    let Some(items) = value.as_vector_view() else {
        return Some(Vec::new());
    };
    let Some(first) = items.first() else {
        return Some(vec![0]);
    };
    let inner = lift_shape(first)?;
    for item in items.iter().skip(1) {
        if lift_shape(item)? != inner {
            return None;
        }
    }
    let mut shape = Vec::with_capacity(inner.len() + 1);
    shape.push(items.len());
    shape.extend(inner);
    Some(shape)
}

/// The walk for operands whose shapes broadcast: the operands of the highest
/// rank are entered axis by axis while the others are held whole, which is
/// what aligning at the innermost axis means; once the ranks meet, the axes
/// pair as [`walk_lanes`] pairs them.
fn walk_aligned(
    operands: &[&Value],
    ranks: &[usize],
    scalar: &mut dyn FnMut(&[&Value]) -> Result<Value>,
) -> Result<Value> {
    let top = ranks.iter().copied().max().unwrap_or(0);
    if top == 0 {
        return scalar(operands);
    }
    let lanes: Vec<Lanes> = operands
        .iter()
        .zip(ranks)
        .map(|(v, r)| {
            if *r == top {
                Lanes::of(v)
            } else {
                Lanes::Scalar(v)
            }
        })
        .collect();
    let next_ranks: Vec<usize> = ranks
        .iter()
        .map(|r| if *r == top { r - 1 } else { *r })
        .collect();
    let extent = common_extent(lanes.iter().map(Lanes::extent))
        .map_err(|(width, len)| AjisaiError::shape_mismatch(&[width], &[len], 0))?;
    let width = extent.unwrap_or(1);
    let mut result = Vec::with_capacity(width);
    let mut next: Vec<&Value> = Vec::with_capacity(lanes.len());
    for index in 0..width {
        next.clear();
        next.extend(lanes.iter().map(|lane| lane.lane(index)));
        result.push(walk_aligned(&next, &next_ranks, scalar)?);
    }
    Ok(Value::from_vector(result))
}

/// The walk where at most one operand is a Vector: it is entered lane by
/// lane, every other operand held whole.
fn walk_lanes(
    operands: &[&Value],
    lifted: &[bool],
    scalar: &mut dyn FnMut(&[&Value]) -> Result<Value>,
) -> Result<Value> {
    let lanes: Vec<Lanes> = operands
        .iter()
        .zip(lifted)
        .map(|(v, l)| if *l { Lanes::of(v) } else { Lanes::Scalar(v) })
        .collect();

    let extent = common_extent(lanes.iter().map(Lanes::extent))
        .map_err(|(width, len)| AjisaiError::shape_mismatch(&[width], &[len], 0))?;

    let Some(width) = extent else {
        // No operand is a Vector with an extent to pair. A single lane still
        // has to be entered — `[ TRUE ] [ FALSE ] [ TRUE ] SELECT` answers
        // `[ TRUE ]`, because a one-element Vector is a Vector
        // (LANG.VALUES.DISJOINT) and only its contents were reused.
        if lanes.iter().any(Lanes::is_vector) {
            let first: Vec<&Value> = lanes.iter().map(|lane| lane.lane(0)).collect();
            let inner = lift_lanes_dyn(&first, lifted, scalar)?;
            return Ok(Value::from_vector(vec![inner]));
        }
        return scalar(operands);
    };

    // One buffer serves every lane. Where no lifted lane is a container the
    // recursion would only hand the lane straight to `scalar`, so that step is
    // taken here — the common flat case then allocates nothing per lane.
    let mut result = Vec::with_capacity(width);
    let mut next: Vec<&Value> = Vec::with_capacity(lanes.len());
    for index in 0..width {
        next.clear();
        next.extend(lanes.iter().map(|lane| lane.lane(index)));
        let leaf = next
            .iter()
            .zip(lifted)
            .all(|(v, l)| !*l || !is_container(v));
        result.push(if leaf {
            scalar(&next)?
        } else {
            lift_lanes_dyn(&next, lifted, scalar)?
        });
    }
    Ok(Value::from_vector(result))
}

/// Whether the lifting rule would enter `value` rather than hand it to the
/// scalar law: a Vector, a Tensor or a Record.
fn is_container(value: &Value) -> bool {
    matches!(
        value.data,
        ValueData::Vector(_)
            | ValueData::Tensor { .. }
            | ValueData::Record(_)
            | ValueData::Truths(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn shown(code: &str) -> std::result::Result<String, String> {
        let mut interp = crate::interpreter::Interpreter::new();
        interp.execute(code).await.map_err(|e| format!("{e:?}"))?;
        Ok(interp
            .stack
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" "))
    }

    /// The comparisons, the connectives and SELECT pair two Vectors at the
    /// innermost axis, as arithmetic does (LANG.COLLECTIONS.LIFT): they used
    /// to pair them at the outermost, so `[ [ 1 2 ] [ 3 4 ] ] [ 2 3 ] LT`
    /// compared 3 with 2 where `SUB` subtracted 3 from 3.
    #[tokio::test]
    async fn every_word_aligns_two_vectors_at_the_innermost_axis() {
        for (code, expected) in [
            (
                "[ [ 1 2 ] [ 3 4 ] ] [ 2 3 ] SUB",
                "[ [ -1/1 -1/1 ] [ 1/1 1/1 ] ]",
            ),
            (
                "[ [ 1 2 ] [ 3 4 ] ] [ 2 3 ] LT",
                "[ [ TRUE TRUE ] [ FALSE FALSE ] ]",
            ),
            (
                "[ [ 1 2 ] [ 3 4 ] ] [ 2 3 ] GT",
                "[ [ FALSE FALSE ] [ TRUE TRUE ] ]",
            ),
            (
                "[ [ TRUE TRUE ] [ FALSE TRUE ] ] [ TRUE FALSE ] AND",
                "[ [ TRUE FALSE ] [ FALSE FALSE ] ]",
            ),
            // An empty inner axis keeps its rows, with or without a NIL lane:
            // the shapes [0] and [1, 1] broadcast to [1, 0].
            ("[ ] [ [ 1 ] ] ADD", "[ [ ] ]"),
            ("[ ] [ [ NIL ] ] ADD", "[ [ ] ]"),
            ("[ [ NIL ] [ 1 ] ] [ ] MUL", "[ [ ] [ ] ]"),
            (
                "[ [ 1 2 ] [ 3 4 ] ] [ 8 9 ] FALSE SELECT",
                "[ [ 8/1 9/1 ] [ 8/1 9/1 ] ]",
            ),
            (
                "[ [ 1 2 ] [ 3 4 ] ] 0 [ TRUE FALSE ] SELECT",
                "[ [ 1/1 0/1 ] [ 3/1 0/1 ] ]",
            ),
            ("[ 1 2 ] 0 NIL SELECT", "[ 0/1 0/1 ]"),
            ("[ 1 2 3 ] [ 2 ] GT", "[ FALSE FALSE TRUE ]"),
            ("[ [ 1 ] ] [ [ 2 ] [ 3 ] ] LT", "[ [ TRUE ] [ TRUE ] ]"),
            ("[ [ 1 2 ] [ 3 ] ] 0 TRUE SELECT", "[ [ 1/1 2/1 ] [ 3/1 ] ]"),
            ("[ TRUE [ FALSE ] ] NOT", "[ FALSE [ TRUE ] ]"),
        ] {
            assert_eq!(shown(code).await.as_deref(), Ok(expected), "`{code}`");
        }
    }

    /// A ragged Vector meets a leaf, never another Vector, in every Word.
    #[tokio::test]
    async fn a_ragged_vector_pairs_with_no_vector() {
        for code in [
            "[ [ 1 2 ] [ 3 ] ] [ 7 8 ] TRUE SELECT",
            "[ [ 1 2 ] [ 3 ] ] [ 0 ] TRUE SELECT",
            "[ [ 1 2 ] [ 3 ] ] [ 1 ] LT",
            "[ [ TRUE ] FALSE ] [ TRUE ] AND",
        ] {
            let error = shown(code).await.expect_err(code);
            assert!(error.contains("shapeMismatch"), "`{code}` raised {error}");
        }
    }

    /// The flat loop answers exactly what the general walk answers, error
    /// included, wherever it answers at all.
    #[test]
    fn the_flat_loop_agrees_with_the_general_walk() {
        let t = Value::from_bool(true);
        let f = Value::from_bool(false);
        let leaves = [t.clone(), f.clone(), Value::nil(), Value::from_int(7)];
        let vector = |items: &[Value]| Value::from_vector(items.to_vec());
        let mut operands = leaves.to_vec();
        operands.extend([
            vector(&[]),
            vector(std::slice::from_ref(&t)),
            vector(&[t.clone(), f.clone(), Value::nil()]),
            vector(&[Value::nil(), t.clone(), f.clone()]),
            vector(&[t.clone(), f.clone()]),
            vector(&[vector(std::slice::from_ref(&t)), f.clone(), t.clone()]),
            Value::from_int_tensor(vec![1, 2, 3]),
        ]);
        let law = |[a, b]: [&Value; 2]| -> Result<Value> {
            match (&a.data, &b.data) {
                (ValueData::Scalar(_), _) | (_, ValueData::Scalar(_)) => Err(
                    AjisaiError::declared("nonTruthValue", "a number".to_string()),
                ),
                _ => Ok(Value::from_vector(vec![a.clone(), b.clone()])),
            }
        };
        let mut checked = 0;
        for a in &operands {
            for b in &operands {
                let general = lift_lanes_dyn(&[a, b], &[true; 2], &mut |lanes: &[&Value]| {
                    law([lanes[0], lanes[1]])
                });
                if let Some(flat) = lift_flat([a, b], &law) {
                    checked += 1;
                    assert_eq!(
                        format!("{flat:?}"),
                        format!("{general:?}"),
                        "{a:?} and {b:?}"
                    );
                }
            }
        }
        assert!(checked > 20, "the flat loop answered only {checked} pairs");
    }
}

#[cfg(kani)]
#[path = "lane_lift_proofs.rs"]
mod proofs;
