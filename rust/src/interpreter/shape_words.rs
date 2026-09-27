//! Shape Words: `SHAPE`, `RESHAPE`, `FLATTEN` and `DEPTH` (LANG.COLLECTIONS.LIFT).
//!
//! The lifting law already reasons about a Vector's shape — the lengths of its
//! axes, outermost first — and the runtime already carries one for a dense
//! `Tensor`. Until these Words a program could observe none of it: `LENGTH`
//! answers the outermost axis and nothing below it. Two of the four are also
//! the clearest instances of the vocabulary-100 admission test
//! (docs/dev/vocabulary-100-work-order-2026-09.md §1): how deeply a value nests
//! is not known in advance, and a language with no recursion and no unbounded
//! loop cannot walk a structure of unknown depth, so `FLATTEN` and `DEPTH`
//! cannot be written as user definitions at any cost.

use super::ordering_ops::{elements_of, restore, take_operand};
use super::tensor_cmds::checked_shape_product;
use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::collection_meter::charge_materialization;
use crate::interpreter::value_extraction_helpers::is_vector_value;
use crate::interpreter::Interpreter;
use crate::semantic::Recoverability;
use crate::types::{Value, ValueData};

/// The shape of a rectangular nesting, or `None` for a ragged one.
///
/// A leaf — anything that is not a Vector, a Text included — has the empty
/// shape; an empty Vector has the shape `[0]`; a Vector whose children all
/// share one shape prefixes its own length to theirs. A dense `Tensor` is
/// rectangular by construction and already knows its shape.
fn rectangular_shape(value: &Value) -> Option<Vec<usize>> {
    if let ValueData::Tensor { shape, .. } = &value.data {
        return Some(shape.to_vec());
    }
    let Some(children) = value.as_vector_view() else {
        return Some(Vec::new());
    };
    let mut shape = vec![children.len()];
    let Some(first) = children.first() else {
        return Some(shape);
    };
    let inner = rectangular_shape(first)?;
    for child in children.iter().skip(1) {
        if rectangular_shape(child)? != inner {
            return None;
        }
    }
    shape.extend(inner);
    Some(shape)
}

/// How deeply a value nests: a leaf is 0, a Vector one more than its deepest
/// element, so an empty Vector is 1.
fn depth_of(value: &Value) -> usize {
    if let ValueData::Tensor { shape, .. } = &value.data {
        return shape.len();
    }
    match value.as_vector_view() {
        None => 0,
        Some(children) => 1 + children.iter().map(depth_of).max().unwrap_or(0),
    }
}

/// Every leaf under `value`, in index order.
fn flatten_into(value: &Value, out: &mut Vec<Value>) {
    match value.as_vector_view() {
        None => out.push(value.clone()),
        Some(children) => {
            for child in children.iter() {
                flatten_into(child, out);
            }
        }
    }
}

/// `leaves`, regrouped under `shape` in row-major order. The caller has already
/// checked that the product of `shape` is `leaves.len()`. The empty shape is
/// rank 0 — the lone leaf itself, as `5 SHAPE` is `[ ]` — and an axis of
/// length 0 answers empty Vectors below it, as `[ [ ] [ ] ] SHAPE` is `[ 2 0 ]`.
pub(super) fn regroup(leaves: &[Value], shape: &[usize]) -> Value {
    match shape.split_first() {
        None => leaves[0].clone(),
        Some((_, [])) => Value::from_vector_promoted(leaves.to_vec()),
        Some((&outer, inner)) => {
            let stride: usize = inner.iter().product();
            Value::from_vector_promoted(
                (0..outer)
                    .map(|i| regroup(&leaves[i * stride..(i + 1) * stride], inner))
                    .collect(),
            )
        }
    }
}

/// A shape operand: a Vector of non-negative integers — exactly what `SHAPE`
/// answers, the empty shape of a leaf and a zero-length axis included — or
/// `None` for anything else (`invalidShape`).
pub(super) fn parse_shape(shape_val: &Value) -> Option<Vec<usize>> {
    shape_val
        .as_vector_view()?
        .iter()
        .map(|dim| dim.as_usize())
        .collect()
}

/// `SHAPE ( [ vec ] -> [ shape ] )`: the axis lengths of a rectangular Vector,
/// outermost first; a ragged Vector has no shape and projects `domainMiss`.
pub fn op_shape(interp: &mut Interpreter) -> Result<()> {
    let value = take_operand(interp)?;
    // A value that is not a Vector has no axes: its shape is the empty one,
    // rank 0, the shape `DEPTH` already reports as depth 0.
    if !is_vector_value(&value) {
        interp.stack.push(Value::from_vector(Vec::new()));
        return Ok(());
    }
    let answer = match rectangular_shape(&value) {
        Some(shape) => Value::from_vector_promoted(
            shape
                .into_iter()
                .map(|axis| Value::from_int(axis as i64))
                .collect(),
        ),
        // A ragged Vector is well-formed data that the question does not fit:
        // the trichotomy's middle case, not a malformed program.
        None => Value::nil_with_reason(NilReason::DomainMiss, Recoverability::Recoverable),
    };
    interp.stack.push(answer);
    Ok(())
}

/// `DEPTH ( [ x ] -> [ n ] )`: how deeply a value nests. Total over every
/// value, NIL included (a leaf, so 0).
pub fn op_depth(interp: &mut Interpreter) -> Result<()> {
    let value = take_operand(interp)?;
    let depth = depth_of(&value);
    interp.stack.push(Value::from_int(depth as i64));
    Ok(())
}

/// `FLATTEN ( [ vec ] -> [ flat ] )`: every leaf in index order, all axes
/// collapsed into one.
pub fn op_flatten(interp: &mut Interpreter) -> Result<()> {
    let value = take_operand(interp)?;
    let children = match elements_of(&value, "a Vector") {
        Ok(children) => children,
        Err(e) => {
            restore(interp, value);
            return Err(e);
        }
    };
    let mut leaves = Vec::new();
    for child in &children {
        flatten_into(child, &mut leaves);
    }
    if let Err(e) = charge_materialization(interp, leaves.len()) {
        restore(interp, value);
        return Err(e);
    }
    interp.stack.push(Value::from_vector_promoted(leaves));
    Ok(())
}

/// `RESHAPE ( [ vec ] [ shape ] -> [ reshaped ] )`: the Vector's leaves, in
/// order, regrouped under `shape`. The product of the shape must equal the
/// leaf count — nothing is padded or repeated — and a well-formed shape too
/// large to materialize projects `spaceExhausted`, as `FILL` and `RANGE` do.
pub fn op_reshape(interp: &mut Interpreter) -> Result<()> {
    let shape_val = interp.stack.pop().ok_or(AjisaiError::stack_underflow())?;
    let target = match interp.stack.pop() {
        Some(target) => target,
        None => {
            interp.stack.push(shape_val);
            return Err(AjisaiError::stack_underflow());
        }
    };
    let put_back = |interp: &mut Interpreter, target: Value, shape_val: Value| {
        interp.stack.push(target);
        interp.stack.push(shape_val);
    };

    let children = match elements_of(&target, "a Vector") {
        Ok(children) => children,
        Err(e) => {
            put_back(interp, target, shape_val);
            return Err(e);
        }
    };

    let Some(shape) = parse_shape(&shape_val) else {
        put_back(interp, target, shape_val);
        return Err(AjisaiError::declared(
            "invalidShape",
            "expected a shape: a Vector of non-negative integers",
        ));
    };
    // A shape's rank is the nesting of the value it builds, so a rank past the
    // nesting ceiling is declined before the value is built — building it is
    // itself a walk one native frame per axis (`regroup`) — the way a count
    // past the materialization ceiling is.
    let max_nesting = interp.runtime_limits.max_nesting_depth;
    if shape.len() > max_nesting {
        interp
            .stack
            .push(crate::interpreter::space_projection::nesting_exhausted_nil(
                "RESHAPE",
                max_nesting,
                shape.len(),
            ));
        return Ok(());
    }

    let max_materialized = interp.runtime_limits.max_materialized_elements;
    let total = match checked_shape_product(&shape) {
        Some(total) if total <= max_materialized => total,
        _ => {
            // The same projection FILL makes for the same reason: a
            // well-formed request the host declines (LANG.COLLECTIONS.BUDGET).
            interp
                .stack
                .push(crate::interpreter::space_projection::space_exhausted_nil(
                    "RESHAPE",
                    max_materialized,
                    checked_shape_product(&shape).map(|size| size as u128),
                ));
            return Ok(());
        }
    };

    let mut leaves = Vec::new();
    for child in &children {
        flatten_into(child, &mut leaves);
    }
    if leaves.len() != total {
        put_back(interp, target, shape_val);
        return Err(AjisaiError::declared(
            "invalidShape",
            format!(
                "the shape holds {} element(s) but the Vector has {} leaf value(s)",
                total,
                leaves.len()
            ),
        ));
    }
    if let Err(e) = charge_materialization(interp, total) {
        put_back(interp, target, shape_val);
        return Err(e);
    }

    let result = regroup(&leaves, &shape);
    interp.stack.push(result);
    Ok(())
}
