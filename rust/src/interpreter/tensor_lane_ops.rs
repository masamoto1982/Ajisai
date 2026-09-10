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
    ravel_index, rectangular_shape, unravel_index,
};
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};

/// Whether an absent lane sits anywhere inside `value`.
///
/// Two representations record one fact. A NIL that lives as a `Value` carries
/// its `AbsenceMetadata`, and therefore its reason; a NIL that lives as a
/// dense tensor lane is the denominator-0 sentinel `Fraction::nil` stores,
/// which records *that* the lane is absent and nothing about why. Both count
/// here: this predicate exists to steer a broadcast away from the flat
/// `Fraction` kernels, which can only produce the second kind.
///
/// Reached only when a broadcast is about to choose a route, so the cost is
/// one linear scan against a value the flat path would have walked anyway.
pub(crate) fn contains_absent_lane(value: &Value) -> bool {
    match &value.data {
        ValueData::Nil => true,
        ValueData::Vector(items) => items.iter().any(contains_absent_lane),
        ValueData::Tensor { data, .. } => !data.all_lanes_valid(),
        ValueData::Scalar(f) => f.is_nil(),
        ValueData::Boolean(_)
        | ValueData::ExactScalar(_)
        | ValueData::Symbol(_)
        | ValueData::Text(_) => false,
    }
}

/// The per-lane lift of the scalar NIL passthrough law, or `None` when this
/// lane has two present operands and the numeric law decides it.
///
/// `LANG.COLLECTIONS.LIFT` says each lane preserves the scalar law's NIL
/// distinction, and the scalar law is one rule stated once, in
/// `Interpreter::declared_nil_contract`'s `Passthrough` arm: a NIL operand
/// *is* the result, and the leftmost one wins, matching left-to-right
/// evaluation order. This is that rule, per lane — the same
/// `Value::nil_inheriting_absence_from` the scalar helpers use, so a lane
/// cannot answer a NIL the scalar would not.
///
/// It is applied *before* the lane's numeric law, not inside it. A law that
/// takes `&Fraction` operands cannot obey it: `Fraction` records absence as a
/// zero denominator and carries no reason, so every such law could answer was
/// a reasonless NIL — which reads back as `nil:literal`, "a NIL the program
/// wrote rather than computed" (`spec/outcomes.json`). That is how
/// `[ 1 2 ] [ 1 0 ] DIV [ 1 1 ] DIV` reported a written NIL for a computed
/// division by zero.
pub(crate) fn lane_nil_passthrough(a: &Value, b: &Value) -> Option<Value> {
    if a.is_nil() {
        return Some(Value::nil_inheriting_absence_from(a));
    }
    if b.is_nil() {
        return Some(Value::nil_inheriting_absence_from(b));
    }
    None
}

/// The tree-walking half of [`apply_lane_wise_broadcast`], for ragged or
/// nested-mixed operands. Mirrors [`apply_recursive_broadcast`] exactly; only
/// the leaf's return type differs. Every caller (ADD/SUB/MUL/DIV/MOD/
/// QUANTIZE, directly or through DIV/MOD's own division-by-zero fallback)
/// declares `nonNumeric` uniformly, same as `FlatTensor::from_value`.
fn apply_lane_wise_recursive<F>(a: &Value, b: &Value, op: F) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Value> + Copy,
{
    match (broadcast_children(a), broadcast_children(b)) {
        (None, None) => apply_lane_law(a, b, op),
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
/// The leaf law never sees an absent operand: [`apply_lane_law`] settles those
/// first, from the `Value`, so each lane's reason survives the lift. It is not
/// a special case for values that happen to carry one — every lane of every
/// operand takes that route, which is why no caller has to ask whether a
/// reason is at stake before choosing this lift.
pub(crate) fn apply_lane_wise_broadcast<F>(a: &Value, b: &Value, op: F) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Value> + Copy,
{
    if a.is_nil() || b.is_nil() {
        // Defensive: callers pass through a NIL operand before reaching here
        // (LANG.FAILURE.PASSTHROUGH), so this is an invariant guard rather
        // than a condition any Word's contract names.
        return Err(AjisaiError::create_structure_error(
            "two non-NIL operands to broadcast",
            "a NIL operand",
        ));
    }

    // Ragged operands cannot be flattened to a tensor whose shape matches
    // their element count, so they follow the value tree instead.
    let (Some(shape_a), Some(shape_b)) = (rectangular_shape(a), rectangular_shape(b)) else {
        return apply_lane_wise_recursive(a, b, op);
    };

    let lanes_a = flat_leaf_values(a);
    let lanes_b = flat_leaf_values(b);
    let out_shape = broadcast_shape(&shape_a, &shape_b)?;
    let out_size: usize = if out_shape.is_empty() {
        1
    } else {
        out_shape.iter().product()
    };
    let out_strides = compute_strides(&out_shape);
    let strides_a = compute_strides(&shape_a);
    let strides_b = compute_strides(&shape_b);
    let same_shape = shape_a == shape_b;

    let mut out_values: Vec<Value> = Vec::with_capacity(out_size);
    for linear in 0..out_size {
        let (a_offset, b_offset) = if same_shape {
            (linear, linear)
        } else {
            let out_index = unravel_index(linear, &out_shape, &out_strides);
            let a_index = project_broadcast_index(&out_index, &out_shape, &shape_a);
            let b_index = project_broadcast_index(&out_index, &out_shape, &shape_b);
            (
                ravel_index(&a_index, &strides_a),
                ravel_index(&b_index, &strides_b),
            )
        };
        out_values.push(apply_lane_law(&lanes_a[a_offset], &lanes_b[b_offset], op)?);
    }

    Ok(nest_lane_values(out_values, &out_shape))
}

/// One lane: the passthrough law first, then the numeric law.
///
/// The two flat operands and the tree walk share this so a lane cannot be
/// decided one way in a rectangular value and another way in a ragged one.
fn apply_lane_law<F>(a: &Value, b: &Value, op: F) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Value> + Copy,
{
    // Absence first, and from the `Value` rather than the `Fraction`: this is
    // the one point in the lift where the lane's reason is still readable.
    if let Some(nil) = lane_nil_passthrough(a, b) {
        return Ok(nil);
    }
    let (Some(fa), Some(fb)) = (broadcast_leaf(a), broadcast_leaf(b)) else {
        return Err(AjisaiError::declared(
            "nonNumeric",
            "expected a number or vector, got a non-numeric value",
        ));
    };
    op(&fa, &fb)
}

/// The leaves of a rectangular value, in the order `FlatTensor` flattens it.
///
/// This is `tensor_ops::FlatTensor::from_value` with the lane type widened
/// from `Fraction` to `Value` — the whole point of this module. A `Fraction` lane records that
/// it is absent and nothing about why, so flattening through one discards
/// every reason before any lane law could preserve it; a `Value` lane carries
/// its `AbsenceMetadata` intact.
///
/// A dense tensor's absent lane is the one case with nothing to carry: the
/// reason was already gone when the tensor was built, so it materializes as a
/// reasonless NIL, which is what it is.
fn flat_leaf_values(value: &Value) -> Vec<Value> {
    fn collect(value: &Value, out: &mut Vec<Value>) {
        match &value.data {
            ValueData::Vector(items) => {
                for item in items.iter() {
                    collect(item, out);
                }
            }
            ValueData::Tensor { data, .. } => {
                for lane in 0..data.len() {
                    // Through the lane, not through its `Fraction`: the
                    // reason for an absent lane is stored beside it and is
                    // gone by the time a `Fraction` is all that is left.
                    out.push(Value::from_dense_lane(data, lane));
                }
            }
            _ => out.push(value.clone()),
        }
    }
    let mut out = Vec::new();
    collect(value, &mut out);
    out
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
