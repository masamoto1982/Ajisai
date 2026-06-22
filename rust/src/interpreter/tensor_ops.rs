use crate::error::{AjisaiError, Result};
use crate::interpreter::interpreter_core::RuntimeMetrics;
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Value, ValueData};
use std::sync::Arc;

#[inline]
fn record_flatten(metrics: &mut Option<&mut RuntimeMetrics>, elements: usize) {
    if let Some(m) = metrics.as_deref_mut() {
        m.vtu_tensor_flatten_count = m.vtu_tensor_flatten_count.saturating_add(1);
        m.vtu_tensor_flattened_elements = m
            .vtu_tensor_flattened_elements
            .saturating_add(elements as u64);
    }
}

fn record_sparse_candidate_value(metrics: &mut Option<&mut RuntimeMetrics>, value: &Value) {
    let ValueData::Tensor { data, .. } = &value.data else {
        return;
    };
    if !data.is_sparse_candidate() {
        return;
    }

    if let Some(m) = metrics.as_deref_mut() {
        let nonzero = data.nonzero_count() as u64;
        let zero = data.zero_count() as u64;
        m.vtu_sparse_candidate_count = m.vtu_sparse_candidate_count.saturating_add(1);
        m.vtu_sparse_candidate_elements = m
            .vtu_sparse_candidate_elements
            .saturating_add(data.len() as u64);
        m.vtu_sparse_candidate_nonzero_elements = m
            .vtu_sparse_candidate_nonzero_elements
            .saturating_add(nonzero);
        m.vtu_sparse_skippable_zero_elements =
            m.vtu_sparse_skippable_zero_elements.saturating_add(zero);
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
            ValueData::Scalar(f) => Ok(Self {
                data: vec![f.clone()],
                shape: Vec::new(),
                strides: Vec::new(),
            }),
            ValueData::Vector(_) | ValueData::Record { .. } => {
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
            ValueData::Boolean(_)
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => Err(AjisaiError::from(
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
            return Err(AjisaiError::from(format!(
                "Cannot broadcast shapes {:?} and {:?}",
                a, b
            )));
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
        ValueData::Tensor { shape, .. } => Some((**shape).clone()),
        ValueData::Vector(items) | ValueData::Record { pairs: items, .. } => {
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
        ValueData::Boolean(_)
        | ValueData::CodeBlock(_)
        | ValueData::ProcessHandle(_)
        | ValueData::SupervisorHandle(_) => None,
    }
}

/// One level of children for a value, or `None` for a leaf (scalar/NIL) or a
/// non-broadcastable value. Dense tensors are decomposed into their outermost
/// rows so that recursive broadcasting treats them like nested vectors.
fn broadcast_children(value: &Value) -> Option<Vec<Value>> {
    match &value.data {
        ValueData::Vector(items) | ValueData::Record { pairs: items, .. } => {
            Some(items.as_ref().clone())
        }
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
    match (broadcast_children(a), broadcast_children(b)) {
        (None, None) => {
            let (Some(fa), Some(fb)) = (broadcast_leaf(a), broadcast_leaf(b)) else {
                return Err(AjisaiError::create_structure_error(
                    "number or vector",
                    "non-numeric value",
                ));
            };
            Ok(Value::from_fraction(op(&fa, &fb)?))
        }
        (Some(children), None) => {
            let out: Vec<Value> = children
                .iter()
                .map(|child| apply_recursive_broadcast(child, b, op))
                .collect::<Result<Vec<Value>>>()?;
            Ok(Value::from_children(out))
        }
        (None, Some(children)) => {
            let out: Vec<Value> = children
                .iter()
                .map(|child| apply_recursive_broadcast(a, child, op))
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
                .map(|(x, y)| apply_recursive_broadcast(x, y, op))
                .collect::<Result<Vec<Value>>>()?;
            Ok(Value::from_children(out))
        }
    }
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
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
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

    if let Some(m) = metrics.as_deref_mut() {
        m.vtu_broadcast_count = m.vtu_broadcast_count.saturating_add(1);
        m.vtu_allocated_elements = m.vtu_allocated_elements.saturating_add(out_size as u64);
    }

    if tensor_a.shape == tensor_b.shape {
        if let Some(m) = metrics.as_deref_mut() {
            m.vtu_same_shape_elementwise_count =
                m.vtu_same_shape_elementwise_count.saturating_add(1);
        }
        let mut out_data = Vec::with_capacity(out_size);
        for i in 0..out_size {
            out_data.push(op(&tensor_a.data[i], &tensor_b.data[i])?);
        }
        let out_tensor = FlatTensor::from_shape_and_data(out_shape, out_data)?;
        return Ok(out_tensor.to_value());
    }

    if let Some(m) = metrics {
        m.vtu_projected_broadcast_count = m.vtu_projected_broadcast_count.saturating_add(1);
    }

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

    if let Some(m) = metrics {
        m.vtu_unary_flat_count = m.vtu_unary_flat_count.saturating_add(1);
        m.vtu_allocated_elements = m
            .vtu_allocated_elements
            .saturating_add(element_count as u64);
    }

    let result_data: Vec<Fraction> = tensor.data.into_iter().map(|f| op(&f)).collect();
    let result_tensor = FlatTensor::from_shape_and_data(tensor.shape, result_data)?;
    Ok(result_tensor.to_value())
}
