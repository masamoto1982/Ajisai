use crate::error::{AjisaiError, Result};
use crate::interpreter::interpreter_core::RuntimeMetrics;
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Value, ValueData};
use std::sync::Arc;

#[inline]
fn record_flatten(metrics: &mut Option<&mut RuntimeMetrics>, _elements: usize) {
    if let Some(_m) = metrics.as_deref_mut() {}
}

fn record_sparse_candidate_value(metrics: &mut Option<&mut RuntimeMetrics>, value: &Value) {
    let ValueData::Tensor { data, .. } = &value.data else {
        return;
    };
    if !data.is_sparse_candidate() {
        return;
    }

    if let Some(_m) = metrics.as_deref_mut() {
        let _nonzero = data.nonzero_count() as u64;
        let _zero = data.zero_count() as u64;
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FlatTensor {
    pub(crate) data: Vec<Fraction>,
    pub(crate) shape: Vec<usize>,
    pub(crate) strides: Vec<usize>,
}

impl FlatTensor {
    pub(crate) fn from_value(value: &Value) -> Result<Self> {
        match &value.data {
            ValueData::Nil => Err(AjisaiError::from(
                "Tensor conversion requires non-NIL value",
            )),
            ValueData::Text(_) => Err(AjisaiError::create_structure_error("vector", "string")),
            ValueData::Scalar(f) => Ok(Self {
                data: vec![f.clone()],
                shape: Vec::new(),
                strides: Vec::new(),
            }),
            ValueData::Vector(_) => {
                let shape: Vec<usize> = value.shape();
                let total_size: usize = value.count_fractions();
                let mut data: Vec<Fraction> = Vec::with_capacity(total_size);
                value.collect_fractions_flat_into(&mut data);
                let strides: Vec<usize> = compute_strides(&shape);
                Ok(Self {
                    data,
                    shape,
                    strides,
                })
            }
            ValueData::Tensor { data, shape } => {
                let shape_vec: Vec<usize> = (**shape).clone();
                let strides: Vec<usize> = compute_strides(&shape_vec);
                Ok(Self {
                    data: data.to_fractions(),
                    shape: shape_vec,
                    strides,
                })
            }
            ValueData::ExactScalar(_) => Err(AjisaiError::from(
                "Tensor conversion does not support exact irrational values",
            )),
            ValueData::Boolean(_) | ValueData::Symbol(_) => Err(AjisaiError::from(
                "Tensor conversion requires scalar or vector",
            )),
        }
    }

    pub(crate) fn from_shape_and_data(shape: Vec<usize>, data: Vec<Fraction>) -> Result<Self> {
        let expected: usize = if shape.is_empty() {
            1
        } else {
            shape.iter().product()
        };
        if data.len() != expected {
            return Err(AjisaiError::from(format!(
                "Tensor shape/data mismatch: data_len={}, required={}, shape={:?}",
                data.len(),
                expected,
                shape
            )));
        }
        let strides: Vec<usize> = compute_strides(&shape);
        Ok(Self {
            data,
            shape,
            strides,
        })
    }

    pub(crate) fn to_value(&self) -> Value {
        if self.shape.is_empty() {
            return Value::from_fraction(self.data[0].clone());
        }
        Value::from_tensor(self.data.clone(), self.shape.clone())
    }
}

pub(crate) fn compute_strides(shape: &[usize]) -> Vec<usize> {
    if shape.is_empty() {
        return Vec::new();
    }
    let mut strides = vec![1; shape.len()];
    for i in (0..shape.len() - 1).rev() {
        strides[i] = strides[i + 1] * shape[i + 1];
    }
    strides
}

fn unravel_index(mut linear: usize, shape: &[usize], strides: &[usize]) -> Vec<usize> {
    if shape.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0; shape.len()];
    for i in 0..shape.len() {
        out[i] = linear / strides[i];
        linear %= strides[i];
    }
    out
}

fn ravel_index(index: &[usize], strides: &[usize]) -> usize {
    index.iter().zip(strides.iter()).map(|(i, s)| i * s).sum()
}

fn project_broadcast_index(
    output_index: &[usize],
    output_shape: &[usize],
    input_shape: &[usize],
) -> Vec<usize> {
    if input_shape.is_empty() {
        return Vec::new();
    }

    let mut projected = vec![0; input_shape.len()];
    let rank_diff = output_shape.len().saturating_sub(input_shape.len());

    for i in 0..input_shape.len() {
        let out_axis = i + rank_diff;
        let out_val = output_index[out_axis];
        projected[i] = if input_shape[i] == 1 { 0 } else { out_val };
    }

    projected
}

pub(crate) fn broadcast_shape(a: &[usize], b: &[usize]) -> Result<Vec<usize>> {
    let rank: usize = a.len().max(b.len());
    let mut out: Vec<usize> = vec![1; rank];

    for i in 0..rank {
        let a_dim = if i >= rank - a.len() {
            a[i - (rank - a.len())]
        } else {
            1
        };
        let b_dim = if i >= rank - b.len() {
            b[i - (rank - b.len())]
        } else {
            1
        };
        if a_dim == b_dim || a_dim == 1 || b_dim == 1 {
            out[i] = a_dim.max(b_dim);
        } else {
            // Report the axis, not just the two shapes. `i` is an index into
            // the *aligned* rank (shapes are right-aligned, NumPy-style), which
            // is the axis a reader counts when they look at the value.
            return Err(AjisaiError::ShapeMismatch {
                left: a.to_vec(),
                right: b.to_vec(),
                axis: i,
            });
        }
    }

    Ok(out)
}

pub(crate) fn build_nested_value(data: &[Fraction], shape: &[usize]) -> Value {
    if shape.is_empty() {
        if data.len() == 1 {
            return Value {
                data: ValueData::Scalar(data[0].clone()),
                hint: Interpretation::RawNumber,
                absence: None,
            };
        }
        let children: Vec<Value> = data
            .iter()
            .map(|f| Value::from_fraction(f.clone()))
            .collect();
        return Value::from_children(children);
    }

    if shape.len() == 1 {
        let children: Vec<Value> = data
            .iter()
            .map(|f| Value::from_fraction(f.clone()))
            .collect();
        return Value {
            data: ValueData::Vector(Arc::new(children)),
            hint: Interpretation::Unassigned,
            absence: None,
        };
    }

    let outer_size: usize = shape[0];
    let inner_shape = &shape[1..];
    let inner_size: usize = inner_shape.iter().product();

    let children: Vec<Value> = (0..outer_size)
        .map(|i| {
            let start = i * inner_size;
            let end = start + inner_size;
            build_nested_value(&data[start..end], inner_shape)
        })
        .collect();

    Value {
        data: ValueData::Vector(Arc::new(children)),
        hint: Interpretation::Unassigned,
        absence: None,
    }
}

/// The rectangular tensor shape of `value`, or `None` when the value cannot
/// be faithfully represented as a flat tensor.
///
/// A value is rectangular when every leaf is a numeric scalar (or NIL lane)
/// and all sibling sub-vectors share an identical shape. Ragged structures —
/// mixed scalar/vector siblings (e.g. `[ 10 [ 1 2 3 ] 10 ]`) or sub-vectors
/// of differing shape — return `None`. Such values must be broadcast
/// structurally (see [`apply_recursive_broadcast`]) rather than flattened,
/// because `shape()` collapses them to a top-level count that disagrees with
/// the recursively flattened element count.
fn rectangular_shape(value: &Value) -> Option<Vec<usize>> {
    match &value.data {
        ValueData::Scalar(_) | ValueData::ExactScalar(_) | ValueData::Nil => Some(Vec::new()),
        ValueData::Text(_) => None,
        ValueData::Tensor { shape, .. } => Some((**shape).clone()),
        ValueData::Vector(items) => {
            if items.is_empty() {
                return Some(vec![0]);
            }
            let first: Vec<usize> = rectangular_shape(&items[0])?;
            for item in items.iter().skip(1) {
                if rectangular_shape(item)? != first {
                    return None;
                }
            }
            let mut shape = Vec::with_capacity(first.len() + 1);
            shape.push(items.len());
            shape.extend(first);
            Some(shape)
        }
        // The logical Unknown (U — `Nil` carrying the `TruthValue` hint)
        // has no dedicated variant, so it takes the `Nil` arm above too and
        // is a rectangular nil lane, same as an operational NIL.
        ValueData::Boolean(_) | ValueData::Symbol(_) => None,
    }
}

/// One level of children for a value, or `None` for a leaf (scalar/NIL) or a
/// non-broadcastable value. Dense tensors are decomposed into their outermost
/// rows so that recursive broadcasting treats them like nested vectors.
pub(crate) fn broadcast_children(value: &Value) -> Option<Vec<Value>> {
    match &value.data {
        ValueData::Vector(items) => Some(items.as_ref().clone()),
        ValueData::Tensor { data, shape } => {
            let nested = build_nested_value(&data.to_fractions(), shape);
            match nested.data {
                ValueData::Vector(items) => Some(items.as_ref().clone()),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The numeric leaf fraction of a value (scalar or NIL lane), or `None` when
/// the value is not a numeric leaf.
fn broadcast_leaf(value: &Value) -> Option<Fraction> {
    match &value.data {
        ValueData::Scalar(f) => Some(f.clone()),
        ValueData::Nil => Some(Fraction::nil()),
        _ => None,
    }
}

/// Structural element-wise broadcast for ragged or nested values.
///
/// Mirrors NumPy-style scalar broadcasting but follows the actual value tree
/// instead of a flattened tensor, so it stays correct when scalars and
/// vectors are mixed as siblings or sub-vectors have differing shapes. A
/// scalar paired with a vector is broadcast across every element; two vectors
/// of equal length combine element-wise; unequal lengths raise
/// `VectorLengthMismatch`. The leaf operation is the same `op` used by the
/// flat path, so NIL-lane handling is identical.
fn apply_recursive_broadcast<F>(a: &Value, b: &Value, op: F) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    apply_lane_wise_broadcast(a, b, |x, y| op(x, y).map(Value::from_fraction))
}

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

/// Metrics-aware tensor broadcast.
///
/// When `metrics` is `Some`, observational VTU counters are incremented at
/// the points where work actually begins, so NIL-rejection and
/// shape-mismatch errors do not bump them. Pass `None` to skip metrics
/// accounting (e.g. internal helpers without access to an interpreter).
pub(crate) fn apply_binary_broadcast_with_metrics<F>(
    a: &Value,
    b: &Value,
    op: F,
    mut metrics: Option<&mut RuntimeMetrics>,
) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy + Sync,
{
    if a.is_nil() || b.is_nil() {
        return Err(AjisaiError::from("Cannot broadcast NIL values"));
    }

    // Ragged or nested-mixed structures (e.g. `[ 10 [ 1 2 3 ] 10 ]`) cannot be
    // flattened to a single tensor whose shape matches its element count, so
    // they are broadcast structurally by following the value tree.
    if rectangular_shape(a).is_none() || rectangular_shape(b).is_none() {
        return apply_recursive_broadcast(a, b, op);
    }

    let tensor_a = FlatTensor::from_value(a)?;
    record_flatten(&mut metrics, tensor_a.data.len());
    let tensor_b = FlatTensor::from_value(b)?;
    record_flatten(&mut metrics, tensor_b.data.len());

    let out_shape = broadcast_shape(&tensor_a.shape, &tensor_b.shape)?;
    record_sparse_candidate_value(&mut metrics, a);
    record_sparse_candidate_value(&mut metrics, b);
    let out_size: usize = if out_shape.is_empty() {
        1
    } else {
        out_shape.iter().product()
    };

    if let Some(_m) = metrics.as_deref_mut() {}

    if tensor_a.shape == tensor_b.shape {
        if let Some(_m) = metrics.as_deref_mut() {}
        // Compute-bound same-shape element-wise op. The per-lane exact-rational
        // arithmetic (num/den cross-multiply + gcd) is the robust parallel
        // scaling target (手4); fan-out is gated by the compute-bound floor and
        // is transparent — the result `Vec<Fraction>` is assembled through the
        // identical `FlatTensor` constructor the sequential lane uses, so the
        // output is structurally identical regardless of worker count.
        let data_a = &tensor_a.data;
        let data_b = &tensor_b.data;
        let out_data: Vec<_> = (0..out_size)
            .map(|i| op(&data_a[i], &data_b[i]))
            .collect::<Result<Vec<_>>>()?;
        let out_tensor = FlatTensor::from_shape_and_data(out_shape, out_data)?;
        return Ok(out_tensor.to_value());
    }

    if let Some(_m) = metrics {}

    let out_strides = compute_strides(&out_shape);
    let mut out_data = Vec::with_capacity(out_size);

    for linear in 0..out_size {
        let out_index = unravel_index(linear, &out_shape, &out_strides);

        let a_index = project_broadcast_index(&out_index, &out_shape, &tensor_a.shape);
        let b_index = project_broadcast_index(&out_index, &out_shape, &tensor_b.shape);

        let a_offset = ravel_index(&a_index, &tensor_a.strides);
        let b_offset = ravel_index(&b_index, &tensor_b.strides);

        out_data.push(op(&tensor_a.data[a_offset], &tensor_b.data[b_offset])?);
    }

    let out_tensor = FlatTensor::from_shape_and_data(out_shape, out_data)?;
    Ok(out_tensor.to_value())
}

/// Structural element-wise unary map for ragged or nested values, mirroring
/// [`apply_recursive_broadcast`]. Follows the value tree instead of a
/// flattened tensor so it stays correct when scalars and vectors are mixed as
/// siblings.
fn apply_recursive_unary<F>(val: &Value, op: F) -> Result<Value>
where
    F: Fn(&Fraction) -> Fraction + Copy,
{
    match broadcast_children(val) {
        Some(children) => {
            let out: Vec<Value> = children
                .iter()
                .map(|child| apply_recursive_unary(child, op))
                .collect::<Result<Vec<Value>>>()?;
            Ok(Value::from_children(out))
        }
        None => {
            let Some(f) = broadcast_leaf(val) else {
                return Err(AjisaiError::create_structure_error(
                    "number or vector",
                    "non-numeric value",
                ));
            };
            Ok(Value::from_fraction(op(&f)))
        }
    }
}

/// Metrics-aware unary flat tensor operation. See
/// [`apply_binary_broadcast_with_metrics`] for the metrics contract.
pub(crate) fn apply_unary_flat_with_metrics<F>(
    val: &Value,
    op: F,
    mut metrics: Option<&mut RuntimeMetrics>,
) -> Result<Value>
where
    F: Fn(&Fraction) -> Fraction + Copy,
{
    // Ragged or nested-mixed structures cannot be flattened to a tensor whose
    // shape matches its element count, so map over the value tree directly.
    if rectangular_shape(val).is_none() {
        return apply_recursive_unary(val, op);
    }

    let tensor = FlatTensor::from_value(val)?;
    let element_count = tensor.data.len();
    record_flatten(&mut metrics, element_count);
    record_sparse_candidate_value(&mut metrics, val);

    if let Some(_m) = metrics {}

    let result_data: Vec<Fraction> = tensor.data.into_iter().map(|f| op(&f)).collect();
    let result_tensor = FlatTensor::from_shape_and_data(tensor.shape, result_data)?;
    Ok(result_tensor.to_value())
}
