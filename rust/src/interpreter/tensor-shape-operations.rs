use crate::error::{AjisaiError, Result};
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};
use std::rc::Rc;

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
            ValueData::CodeBlock(_) => Err(AjisaiError::from(
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
        build_nested_value(&self.data, &self.shape)
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
    }
}

pub(crate) fn apply_binary_broadcast<F>(a: &Value, b: &Value, op: F) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    if a.is_nil() || b.is_nil() {
        return Err(AjisaiError::from("Cannot broadcast NIL values"));
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

pub(crate) fn apply_unary_flat<F>(val: &Value, op: F) -> Result<Value>
where
    F: Fn(&Fraction) -> Fraction,
{
    let tensor = FlatTensor::from_value(val)?;
    let result_data: Vec<Fraction> = tensor.data.iter().map(&op).collect();
    let result_tensor = FlatTensor::from_shape_and_data(tensor.shape, result_data)?;
    Ok(result_tensor.to_value())
}

/// In-place variant of `apply_unary_flat`.
///
/// When the linear-consumption optimization hook (`FlowToken::can_update_in_place`)
/// indicates the value is uniquely owned and fully available, this function
/// reuses the data buffer instead of allocating a new Vec<Fraction>.
/// Falls back to `apply_unary_flat` when in-place update is not safe.
pub(crate) fn apply_unary_flat_inplace<F>(val: &Value, op: F, in_place_hint: bool) -> Result<Value>
where
    F: Fn(&Fraction) -> Fraction,
{
    let mut tensor = FlatTensor::from_value(val)?;
    if in_place_hint {
        for f in tensor.data.iter_mut() {
            *f = op(f);
        }
        Ok(tensor.to_value())
    } else {
        let result_data: Vec<Fraction> = tensor.data.iter().map(&op).collect();
        let result_tensor = FlatTensor::from_shape_and_data(tensor.shape, result_data)?;
        Ok(result_tensor.to_value())
    }
}

/// In-place variant of `apply_binary_broadcast`.
///
/// When the linear-consumption optimization hook indicates the left operand
/// is uniquely owned and shapes match without broadcasting, this function
/// reuses the left operand's data buffer instead of allocating a new one.
/// Falls back to the standard allocation path otherwise.
pub(crate) fn apply_binary_broadcast_inplace<F>(
    a: &Value,
    b: &Value,
    op: F,
    in_place_hint: bool,
) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    if a.is_nil() || b.is_nil() {
        return Err(AjisaiError::from("Cannot broadcast NIL values"));
    }

    let mut tensor_a = FlatTensor::from_value(a)?;
    let tensor_b = FlatTensor::from_value(b)?;

    let out_shape = broadcast_shape(&tensor_a.shape, &tensor_b.shape)?;

    // In-place path: reuse tensor_a's data buffer when shapes match exactly
    // (no broadcasting needed) and the hint allows it
    if in_place_hint && tensor_a.shape == out_shape && tensor_b.shape == out_shape {
        for i in 0..tensor_a.data.len() {
            tensor_a.data[i] = op(&tensor_a.data[i], &tensor_b.data[i])?;
        }
        return Ok(tensor_a.to_value());
    }

    // Standard allocation path
    let out_size: usize = if out_shape.is_empty() {
        1
    } else {
        out_shape.iter().product()
    };
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
