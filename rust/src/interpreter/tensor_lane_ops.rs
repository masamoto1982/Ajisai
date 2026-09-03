//! Element-wise broadcast for Words whose scalar law can project.
//!
//! The `Fraction`-level broadcasts in [`tensor_ops`] answer each lane with a
//! number, which is all a dense tensor lane can hold: a lane records presence,
//! not why something is absent. A Word whose scalar law may *project* — answer
//! NIL for a well-formed operand (`LANG.FAILURE.TRICHOTOMY`) — therefore
//! cannot lift through them without flattening every projection into an
//! anonymous absence, or worse, into one failure for the whole operation.
//!
//! This module is the lift for those Words. It reuses `tensor_ops`' shape
//! rules exactly, so the result shape never depends on whether the Word
//! projected, and differs only in what a lane may answer with.
//!
//! [`tensor_ops`]: crate::interpreter::tensor_ops

use crate::error::{AjisaiError, Result};
use crate::interpreter::tensor_ops::{
    broadcast_children, broadcast_leaf, broadcast_shape, compute_strides, project_broadcast_index,
    ravel_index, rectangular_shape, unravel_index, FlatTensor,
};
use crate::types::fraction::Fraction;
use crate::types::Value;

/// The tree-walking half of [`apply_lane_wise_broadcast`], for ragged or
/// nested-mixed operands. Mirrors [`apply_recursive_broadcast`] exactly; only
/// the leaf's return type differs.
fn apply_lane_wise_recursive<F>(a: &Value, b: &Value, op: F) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Value> + Copy,
{
    match (broadcast_children(a), broadcast_children(b)) {
        (None, None) => {
            let (Some(fa), Some(fb)) = (broadcast_leaf(a), broadcast_leaf(b)) else {
                return Err(AjisaiError::create_structure_error(
                    "number or vector",
                    "non-numeric value",
                ));
            };
            op(&fa, &fb)
        }
        (Some(children), None) => {
            let out: Vec<Value> = children
                .iter()
                .map(|child| apply_lane_wise_recursive(child, b, op))
                .collect::<Result<Vec<Value>>>()?;
            Ok(Value::from_children(out))
        }
        (None, Some(children)) => {
            let out: Vec<Value> = children
                .iter()
                .map(|child| apply_lane_wise_recursive(a, child, op))
                .collect::<Result<Vec<Value>>>()?;
            Ok(Value::from_children(out))
        }
        (Some(a_children), Some(b_children)) => {
            if a_children.len() != b_children.len() {
                return Err(AjisaiError::VectorLengthMismatch {
                    len1: a_children.len(),
                    len2: b_children.len(),
                });
            }
            let out: Vec<Value> = a_children
                .iter()
                .zip(b_children.iter())
                .map(|(x, y)| apply_lane_wise_recursive(x, y, op))
                .collect::<Result<Vec<Value>>>()?;
            Ok(Value::from_children(out))
        }
    }
}

/// Element-wise broadcast whose leaf law answers with a whole `Value`.
///
/// The `Fraction`-level broadcasts can only hand a lane a number, so a Word
/// whose scalar law may *project* — answer NIL for a well-formed operand —
/// cannot use them without flattening every projection into an anonymous
/// absence: a dense lane records presence, not a reason. `SQRT` already lifts
/// this way through `lift_unary_numeric`, which is why a negative lane comes
/// back as `NIL(domainMiss)` beside its neighbours. This is the binary
/// counterpart, and `DIV` uses it so a zero divisor empties its own lane and
/// says why, rather than emptying the vector.
///
/// Shape handling is the flat path's, lane for lane — the same
/// [`broadcast_shape`] and the same index projection — so a Word cannot mean
/// one thing when it projects and another when it does not: `[ 6 ] [ 1 2 0 ] /`
/// broadcasts its single dividend across three divisors here exactly as
/// `[ 6 ] [ 1 2 3 ] /` does there.
///
/// The leaf sees `Fraction::nil()` for an absent operand (see
/// [`broadcast_leaf`]), so the law can tell a NIL operand apart from a zero.
pub(crate) fn apply_lane_wise_broadcast<F>(a: &Value, b: &Value, op: F) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Value> + Copy,
{
    if a.is_nil() || b.is_nil() {
        return Err(AjisaiError::from("Cannot broadcast NIL values"));
    }

    if rectangular_shape(a).is_none() || rectangular_shape(b).is_none() {
        return apply_lane_wise_recursive(a, b, op);
    }

    let tensor_a = FlatTensor::from_value(a)?;
    let tensor_b = FlatTensor::from_value(b)?;
    let out_shape = broadcast_shape(&tensor_a.shape, &tensor_b.shape)?;
    let out_size: usize = if out_shape.is_empty() {
        1
    } else {
        out_shape.iter().product()
    };
    let out_strides = compute_strides(&out_shape);
    let same_shape = tensor_a.shape == tensor_b.shape;

    let mut out_values: Vec<Value> = Vec::with_capacity(out_size);
    for linear in 0..out_size {
        let (a_offset, b_offset) = if same_shape {
            (linear, linear)
        } else {
            let out_index = unravel_index(linear, &out_shape, &out_strides);
            let a_index = project_broadcast_index(&out_index, &out_shape, &tensor_a.shape);
            let b_index = project_broadcast_index(&out_index, &out_shape, &tensor_b.shape);
            (
                ravel_index(&a_index, &tensor_a.strides),
                ravel_index(&b_index, &tensor_b.strides),
            )
        };
        out_values.push(op(&tensor_a.data[a_offset], &tensor_b.data[b_offset])?);
    }

    Ok(nest_lane_values(out_values, &out_shape))
}

/// Fold flat lane values back into `out_shape`.
///
/// The nested `Vector` form is kept deliberately: promoting back to a dense
/// tensor is what would discard the per-lane reason this path exists to carry.
/// An empty shape is the scalar case — one lane, and it *is* the result.
fn nest_lane_values(mut values: Vec<Value>, shape: &[usize]) -> Value {
    if shape.is_empty() {
        return values.pop().unwrap_or_else(Value::nil);
    }
    if shape.len() == 1 {
        return Value::from_children(values);
    }
    let inner_shape = &shape[1..];
    let inner_size: usize = inner_shape.iter().product();
    if inner_size == 0 {
        return Value::from_children(Vec::new());
    }
    let mut rest = values.split_off(0);
    let mut outer: Vec<Value> = Vec::with_capacity(shape[0]);
    for _ in 0..shape[0] {
        let tail = rest.split_off(inner_size.min(rest.len()));
        outer.push(nest_lane_values(rest, inner_shape));
        rest = tail;
    }
    Value::from_children(outer)
}
