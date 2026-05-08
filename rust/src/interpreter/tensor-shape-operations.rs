use crate::error::{AjisaiError, Result};
use crate::interpreter::interpreter_core::RuntimeMetrics;
use crate::types::fraction::Fraction;
use crate::types::{DisplayHint, Value, ValueData};
use std::rc::Rc;

#[inline]
fn record_flatten(metrics: &mut Option<&mut RuntimeMetrics>, elements: usize) {
    if let Some(m) = metrics.as_deref_mut() {
        m.vtu_tensor_flatten_count = m.vtu_tensor_flatten_count.saturating_add(1);
        m.vtu_tensor_flattened_elements = m
            .vtu_tensor_flattened_elements
            .saturating_add(elements as u64);
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
            ValueData::CodeBlock(_)
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
                hint: DisplayHint::Number,
                nil_reason: None,
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
            data: ValueData::Vector(Rc::new(children)),
            hint: DisplayHint::Auto,
            nil_reason: None,
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
        data: ValueData::Vector(Rc::new(children)),
        hint: DisplayHint::Auto,
        nil_reason: None,
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

    let tensor_a = FlatTensor::from_value(a)?;
    record_flatten(&mut metrics, tensor_a.data.len());
    let tensor_b = FlatTensor::from_value(b)?;
    record_flatten(&mut metrics, tensor_b.data.len());

    let out_shape = broadcast_shape(&tensor_a.shape, &tensor_b.shape)?;
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

/// Metrics-aware unary flat tensor operation. See
/// [`apply_binary_broadcast_with_metrics`] for the metrics contract.
pub(crate) fn apply_unary_flat_with_metrics<F>(
    val: &Value,
    op: F,
    mut metrics: Option<&mut RuntimeMetrics>,
) -> Result<Value>
where
    F: Fn(&Fraction) -> Fraction,
{
    let tensor = FlatTensor::from_value(val)?;
    let element_count = tensor.data.len();
    record_flatten(&mut metrics, element_count);

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
