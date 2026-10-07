//! The shape half of a recursive broadcast over ragged or nested-mixed
//! operands, shared by every lift that walks a value tree rather than a
//! flattened tensor.

use crate::error::{AjisaiError, Result};
use crate::interpreter::tensor_ops::broadcast_children;
use crate::types::Value;

/// How a tree broadcast treats two axes of unequal length.
#[derive(Clone, Copy)]
pub(crate) enum UnequalAxes {
    /// `VectorLengthMismatch`, as the arithmetic broadcast raises for two
    /// Vectors whose lengths differ.
    Refuse,
    /// A length-1 axis stretches to meet the other, the same rule the
    /// arithmetic broadcast applies, so `[ -1 2 -3 ] [ 0 ] MAX` and
    /// `[ -1 2 -3 ] 0 MAX` are the same rectifier written two ways. Any
    /// other difference is `shapeMismatch`: the same category the arithmetic
    /// broadcast raises for the same operands, because this is the same
    /// clause — both are element-wise Words of the same family, and
    /// LANG.COLLECTIONS.LIFT gives element-wise pairing one failure
    /// condition, not one per implementation route. This route used to
    /// answer `VectorLengthMismatch` — the category for a Word that
    /// *requires* two lengths equal, like `GROUP` pairing values with keys —
    /// so `[ 1 2 ] [ 1 2 3 ] MIN` and `[ 1 2 ] [ 1 2 3 ] ADD` reported
    /// different categories for one situation, and MIN/MAX contradicted
    /// their own contract, which has always named `shapeMismatch`.
    ///
    /// The axes that parted are reported as the one-axis shapes they are at
    /// this level of the walk: unlike the tensor route, a ragged tree has no
    /// whole-value shape to quote, which is why the walk exists. The category
    /// is the machine-readable surface (LANG.OBSERVATION.DIAGNOSIS); the
    /// message is correspondingly less detailed than the tensor route's.
    StretchSingleton,
}

/// The shape half of a recursive broadcast, for ragged or nested-mixed
/// operands that cannot be flattened to a tensor: a leaf paired with a
/// Vector is broadcast across every element, two Vectors of equal length
/// combine element-wise, and `axes` says what two unequal lengths mean. The
/// leaf law is the caller's, so the walk is the same whether a lane answers
/// a number, a whole `Value`, or a NIL with its reason.
///
/// One walk for the lane-wise lift (`tensor_lane_ops`), the exact-real lift
/// (`arithmetic`) and the numeric lift of `MIN`/`MAX` (`math_ops`); each
/// used to carry its own copy of this `match`, kept in step by hand.
pub(crate) fn broadcast_tree(
    a: &Value,
    b: &Value,
    axes: UnequalAxes,
    leaf: &dyn Fn(&Value, &Value) -> Result<Value>,
) -> Result<Value> {
    let walk = |x: &Value, y: &Value| broadcast_tree(x, y, axes, leaf);
    let children_of = |values: Vec<Result<Value>>| -> Result<Value> {
        Ok(Value::from_children(
            values.into_iter().collect::<Result<Vec<Value>>>()?,
        ))
    };
    match (broadcast_children(a), broadcast_children(b)) {
        (None, None) => leaf(a, b),
        (Some(children), None) => {
            children_of(children.iter().map(|child| walk(child, b)).collect())
        }
        (None, Some(children)) => {
            children_of(children.iter().map(|child| walk(a, child)).collect())
        }
        (Some(left), Some(right)) => match axes {
            UnequalAxes::StretchSingleton if left.len() == 1 && right.len() != 1 => {
                children_of(right.iter().map(|y| walk(&left[0], y)).collect())
            }
            UnequalAxes::StretchSingleton if right.len() == 1 && left.len() != 1 => {
                children_of(left.iter().map(|x| walk(x, &right[0])).collect())
            }
            _ if left.len() != right.len() => Err(match axes {
                UnequalAxes::Refuse => AjisaiError::length_mismatch(left.len(), right.len()),
                UnequalAxes::StretchSingleton => {
                    AjisaiError::shape_mismatch(&[left.len()], &[right.len()], 0)
                }
            }),
            _ => children_of(
                left.iter()
                    .zip(right.iter())
                    .map(|(x, y)| walk(x, y))
                    .collect(),
            ),
        },
    }
}
