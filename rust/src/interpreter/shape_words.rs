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
use crate::interpreter::{ConsumptionMode, Interpreter};
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
/// checked that the product of `shape` is `leaves.len()`.
fn regroup(leaves: &[Value], shape: &[usize]) -> Value {
    if shape.len() <= 1 {
        return Value::from_vector_promoted(leaves.to_vec());
    }
    let stride: usize = shape[1..].iter().product();
    Value::from_vector_promoted(
        leaves
            .chunks(stride)
            .map(|chunk| regroup(chunk, &shape[1..]))
            .collect(),
    )
}

/// `SHAPE ( [ vec ] -> [ shape ] )`: the axis lengths of a rectangular Vector,
/// outermost first; a ragged Vector has no shape and projects `domainMiss`.
pub fn op_shape(interp: &mut Interpreter) -> Result<()> {
    let value = take_operand(interp)?;
    if !is_vector_value(&value) {
        restore(interp, value);
        return Err(AjisaiError::declared(
            "nonVector",
            "SHAPE: expected a Vector, got a non-vector value",
        ));
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
    let keep = interp.consumption_mode == ConsumptionMode::Keep;
    let shape_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let target = if keep {
        match interp.stack.last().cloned() {
            Some(target) => target,
            None => {
                interp.stack.push(shape_val);
                return Err(AjisaiError::StackUnderflow);
            }
        }
    } else {
        match interp.stack.pop() {
            Some(target) => target,
            None => {
                interp.stack.push(shape_val);
                return Err(AjisaiError::StackUnderflow);
            }
        }
    };
    let put_back = |interp: &mut Interpreter, target: Value, shape_val: Value| {
        if !keep {
            interp.stack.push(target);
        }
        interp.stack.push(shape_val);
    };

    let children = match elements_of(&target, "a Vector") {
        Ok(children) => children,
        Err(e) => {
            put_back(interp, target, shape_val);
            return Err(e);
        }
    };

    let shape: Vec<usize> = match shape_val.as_vector_view().and_then(|dims| {
        dims.iter()
            .map(|dim| dim.as_usize().filter(|n| *n > 0))
            .collect::<Option<Vec<usize>>>()
    }) {
        Some(shape) if !shape.is_empty() => shape,
        _ => {
            put_back(interp, target, shape_val);
            return Err(AjisaiError::declared(
                "invalidShape",
                "RESHAPE: expected a shape — a Vector of positive integers — got an invalid shape",
            ));
        }
    };

    let max_materialized = interp.runtime_limits.max_materialized_elements;
    let total = match checked_shape_product(&shape) {
        Some(total) if total <= max_materialized => total,
        _ => {
            // The same projection FILL makes for the same reason: a
            // well-formed request the host declines (LANG.COLLECTIONS.BUDGET).
            // Under KEEP the operands stay, as on the success path.
            if keep {
                interp.stack.push(shape_val);
            }
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
                "RESHAPE: the shape holds {} element(s) but the Vector has {} leaf value(s)",
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
    if keep {
        interp.stack.push(shape_val);
    }
    interp.stack.push(result);
    Ok(())
}
