//! Element lifting for the fixed-arity Words whose law is stated lane by lane
//! (LANG.COLLECTIONS.LIFT).
//!
//! The clause is one rule, so it is one function: "A scalar combines with
//! every element of a vector, however that vector is nested … Paired axes
//! combine when their lengths are equal, and when one of them is 1 that
//! operand's single lane is reused across the other's length." Arithmetic
//! reaches that rule through the tensor path; `booleanLogic` and `comparison`
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
use crate::types::Value;

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
            Lanes::Vector(items) if items.len() == 1 => None,
            Lanes::Vector(items) => Some(items.len()),
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

/// Apply `scalar` to the aligned lanes of `operands`.
///
/// `scalar` sees only operands that are not Vectors at this level, so it
/// states the scalar law alone and never repeats the alignment. Where the
/// extents disagree the result is the `shapeMismatch` ERROR the clause
/// requires, naming the first axis on which two operands disagree.
pub(crate) fn lift_lanes<const N: usize>(
    operands: [&Value; N],
    scalar: &dyn Fn([&Value; N]) -> Result<Value>,
) -> Result<Value> {
    let lanes: [Lanes; N] = operands.map(Lanes::of);

    let mut extent: Option<usize> = None;
    for lane in lanes.iter() {
        let Some(len) = lane.extent() else { continue };
        match extent {
            None => extent = Some(len),
            Some(width) if width == len => {}
            Some(width) => {
                return Err(AjisaiError::ShapeMismatch {
                    left: vec![width],
                    right: vec![len],
                    axis: 0,
                })
            }
        }
    }

    let Some(width) = extent else {
        // No operand is a Vector with an extent to pair. A single lane still
        // has to be entered — `[ TRUE ] [ FALSE ] [ TRUE ] SELECT` answers
        // `[ TRUE ]`, because a one-element Vector is a Vector
        // (LANG.VALUES.DISJOINT) and only its contents were reused.
        if lanes.iter().any(Lanes::is_vector) {
            let inner = descend(&lanes, 0, scalar)?;
            return Ok(Value::from_vector(vec![inner]));
        }
        return scalar(operands);
    };

    let mut result = Vec::with_capacity(width);
    for index in 0..width {
        result.push(descend(&lanes, index, scalar)?);
    }
    Ok(Value::from_vector(result))
}

fn descend<const N: usize>(
    lanes: &[Lanes; N],
    index: usize,
    scalar: &dyn Fn([&Value; N]) -> Result<Value>,
) -> Result<Value> {
    let mut next: [&Value; N] = [lanes[0].lane(index); N];
    for (slot, lane) in next.iter_mut().zip(lanes.iter()) {
        *slot = lane.lane(index);
    }
    lift_lanes(next, scalar)
}
