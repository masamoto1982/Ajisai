use super::{CheckedShape, DType, ShapeError, Tensor, TensorData, TensorError, TensorMemoryBudget};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TensorOperatorError {
    Rank {
        operator: &'static str,
        minimum: usize,
        actual: usize,
    },
    DTypeMismatch {
        left: DType,
        right: DType,
    },
    ContractionMismatch {
        left: usize,
        right: usize,
    },
    BatchBroadcastMismatch {
        left: Vec<usize>,
        right: Vec<usize>,
    },
    AxisOutOfRange {
        axis: usize,
        rank: usize,
    },
    DuplicateAxis(usize),
    Shape(ShapeError),
    Tensor(TensorError),
}

impl fmt::Display for TensorOperatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rank {
                operator,
                minimum,
                actual,
            } => {
                write!(
                    f,
                    "{operator} requires rank >= {minimum}, received {actual}"
                )
            }
            Self::DTypeMismatch { left, right } => {
                write!(f, "tensor dtypes differ: {left:?} and {right:?}")
            }
            Self::ContractionMismatch { left, right } => {
                write!(
                    f,
                    "MATMUL contraction dimensions differ: {left} and {right}"
                )
            }
            Self::BatchBroadcastMismatch { left, right } => {
                write!(
                    f,
                    "MATMUL batch shapes {left:?} and {right:?} do not broadcast"
                )
            }
            Self::AxisOutOfRange { axis, rank } => {
                write!(f, "reduction axis {axis} is out of range for rank {rank}")
            }
            Self::DuplicateAxis(axis) => write!(f, "reduction axis {axis} is duplicated"),
            Self::Shape(error) => error.fmt(f),
            Self::Tensor(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for TensorOperatorError {}

impl From<ShapeError> for TensorOperatorError {
    fn from(value: ShapeError) -> Self {
        Self::Shape(value)
    }
}

impl From<TensorError> for TensorOperatorError {
    fn from(value: TensorError) -> Self {
        Self::Tensor(value)
    }
}

/// Deterministic reference-CPU implementation of `tensor.matmul.v1`.
///
/// Reduction follows increasing `K` order. Output allocation is checked against
/// both element and byte budgets before a buffer is created.
pub fn matmul(
    left: &Tensor,
    right: &Tensor,
    budget: TensorMemoryBudget,
) -> Result<Tensor, TensorOperatorError> {
    if left.dtype() != right.dtype() {
        return Err(TensorOperatorError::DTypeMismatch {
            left: left.dtype(),
            right: right.dtype(),
        });
    }
    let left_dimensions = left.shape().dimensions();
    let right_dimensions = right.shape().dimensions();
    check_rank(left_dimensions.len())?;
    check_rank(right_dimensions.len())?;

    let m = left_dimensions[left_dimensions.len() - 2];
    let left_k = left_dimensions[left_dimensions.len() - 1];
    let right_k = right_dimensions[right_dimensions.len() - 2];
    let n = right_dimensions[right_dimensions.len() - 1];
    if left_k != right_k {
        return Err(TensorOperatorError::ContractionMismatch {
            left: left_k,
            right: right_k,
        });
    }

    let left_batch = &left_dimensions[..left_dimensions.len() - 2];
    let right_batch = &right_dimensions[..right_dimensions.len() - 2];
    let batch_shape = broadcast_batch_shape(left_batch, right_batch)?;
    let mut output_dimensions = batch_shape.clone();
    output_dimensions.extend([m, n]);
    let output_shape = CheckedShape::new(
        output_dimensions.clone(),
        left.dtype().element_bytes(),
        budget,
    )?;

    let data = match (left.data(), right.data()) {
        (TensorData::F32(left_data), TensorData::F32(right_data)) => TensorData::F32(matmul_data(
            left_data,
            right_data,
            left.shape(),
            right.shape(),
            &batch_shape,
            m,
            left_k,
            n,
            |sum, left, right| sum + left * right,
            0.0f32,
        )),
        (TensorData::F64(left_data), TensorData::F64(right_data)) => TensorData::F64(matmul_data(
            left_data,
            right_data,
            left.shape(),
            right.shape(),
            &batch_shape,
            m,
            left_k,
            n,
            |sum, left, right| sum + left * right,
            0.0f64,
        )),
        _ => unreachable!("dtype equality was checked"),
    };
    debug_assert_eq!(data_len(&data), output_shape.element_count());
    Tensor::new(output_dimensions, data, budget).map_err(Into::into)
}

/// Reference implementation of `tensor.exp.v1`. IEEE NaN and infinities stay
/// tensor elements; they are never projected to Core NIL.
pub fn tensor_exp(
    input: &Tensor,
    budget: TensorMemoryBudget,
) -> Result<Tensor, TensorOperatorError> {
    map_unary(input, budget, f32::exp, f64::exp)
}

/// Reference implementation of `tensor.log.v1`. IEEE domain results (including
/// NaN and negative infinity) remain ordinary approximate tensor elements.
pub fn tensor_log(
    input: &Tensor,
    budget: TensorMemoryBudget,
) -> Result<Tensor, TensorOperatorError> {
    map_unary(input, budget, f32::ln, f64::ln)
}

/// Reference implementation of `tensor.reduce_sum.v1`.
pub fn reduce_sum(
    input: &Tensor,
    axes: &[usize],
    keep_dimensions: bool,
    budget: TensorMemoryBudget,
) -> Result<Tensor, TensorOperatorError> {
    let rank = input.shape().dimensions().len();
    let axes = checked_axes(axes, rank)?;
    let output_dimensions: Vec<usize> = input
        .shape()
        .dimensions()
        .iter()
        .enumerate()
        .filter_map(|(axis, dimension)| {
            if axes.binary_search(&axis).is_ok() {
                keep_dimensions.then_some(1)
            } else {
                Some(*dimension)
            }
        })
        .collect();
    let output_shape = CheckedShape::new(
        output_dimensions.clone(),
        input.dtype().element_bytes(),
        budget,
    )?;
    let data = match input.data() {
        TensorData::F32(values) => TensorData::F32(reduce_sum_data(
            values,
            input.shape(),
            &output_shape,
            &axes,
            keep_dimensions,
            0.0f32,
        )),
        TensorData::F64(values) => TensorData::F64(reduce_sum_data(
            values,
            input.shape(),
            &output_shape,
            &axes,
            keep_dimensions,
            0.0f64,
        )),
    };
    Tensor::new(output_dimensions, data, budget).map_err(Into::into)
}

fn checked_axes(axes: &[usize], rank: usize) -> Result<Vec<usize>, TensorOperatorError> {
    let mut checked = axes.to_vec();
    checked.sort_unstable();
    for (index, axis) in checked.iter().copied().enumerate() {
        if axis >= rank {
            return Err(TensorOperatorError::AxisOutOfRange { axis, rank });
        }
        if index > 0 && checked[index - 1] == axis {
            return Err(TensorOperatorError::DuplicateAxis(axis));
        }
    }
    Ok(checked)
}

fn reduce_sum_data<T: Copy + std::ops::AddAssign>(
    input: &[T],
    input_shape: &CheckedShape,
    output_shape: &CheckedShape,
    axes: &[usize],
    keep_dimensions: bool,
    zero: T,
) -> Vec<T> {
    let mut output = vec![zero; output_shape.element_count()];
    for (linear, value) in input.iter().copied().enumerate() {
        let input_coordinates = unravel(linear, input_shape.dimensions());
        let output_coordinates: Vec<usize> = input_coordinates
            .into_iter()
            .enumerate()
            .filter_map(|(axis, coordinate)| {
                if axes.binary_search(&axis).is_ok() {
                    keep_dimensions.then_some(0)
                } else {
                    Some(coordinate)
                }
            })
            .collect();
        let output_linear = output_coordinates
            .iter()
            .zip(output_shape.row_major_strides())
            .map(|(coordinate, stride)| coordinate * stride)
            .sum::<usize>();
        output[output_linear] += value;
    }
    output
}

fn map_unary(
    input: &Tensor,
    budget: TensorMemoryBudget,
    f32_operation: impl Fn(f32) -> f32,
    f64_operation: impl Fn(f64) -> f64,
) -> Result<Tensor, TensorOperatorError> {
    CheckedShape::new(
        input.shape().dimensions().to_vec(),
        input.dtype().element_bytes(),
        budget,
    )?;
    let data = match input.data() {
        TensorData::F32(values) => {
            TensorData::F32(values.iter().copied().map(f32_operation).collect())
        }
        TensorData::F64(values) => {
            TensorData::F64(values.iter().copied().map(f64_operation).collect())
        }
    };
    Tensor::new(input.shape().dimensions().to_vec(), data, budget).map_err(Into::into)
}

fn check_rank(rank: usize) -> Result<(), TensorOperatorError> {
    if rank < 2 {
        return Err(TensorOperatorError::Rank {
            operator: "MATMUL",
            minimum: 2,
            actual: rank,
        });
    }
    Ok(())
}

fn broadcast_batch_shape(
    left: &[usize],
    right: &[usize],
) -> Result<Vec<usize>, TensorOperatorError> {
    let rank = left.len().max(right.len());
    let mut output = Vec::with_capacity(rank);
    for offset in (0..rank).rev() {
        let left_dimension = trailing_dimension(left, offset);
        let right_dimension = trailing_dimension(right, offset);
        let dimension = match (left_dimension, right_dimension) {
            (Some(left), Some(right)) if left == right => left,
            (Some(1), Some(right)) => right,
            (Some(left), Some(1)) => left,
            (Some(_), Some(_)) => {
                return Err(TensorOperatorError::BatchBroadcastMismatch {
                    left: left.to_vec(),
                    right: right.to_vec(),
                })
            }
            (Some(dimension), None) | (None, Some(dimension)) => dimension,
            (None, None) => unreachable!("rank bounds the loop"),
        };
        output.push(dimension);
    }
    Ok(output)
}

fn trailing_dimension(shape: &[usize], offset: usize) -> Option<usize> {
    shape
        .len()
        .checked_sub(offset + 1)
        .map(|index| shape[index])
}

#[allow(clippy::too_many_arguments)]
fn matmul_data<T: Copy>(
    left: &[T],
    right: &[T],
    left_shape: &CheckedShape,
    right_shape: &CheckedShape,
    batch_shape: &[usize],
    m: usize,
    k: usize,
    n: usize,
    multiply_add: impl Fn(T, T, T) -> T,
    zero: T,
) -> Vec<T> {
    let batch_count = batch_shape.iter().product::<usize>();
    let mut output = Vec::with_capacity(batch_count.saturating_mul(m).saturating_mul(n));
    for batch_linear in 0..batch_count {
        let coordinates = unravel(batch_linear, batch_shape);
        let left_base = batch_base(&coordinates, batch_shape, left_shape);
        let right_base = batch_base(&coordinates, batch_shape, right_shape);
        let left_row_stride = left_shape.row_major_strides()[left_shape.dimensions().len() - 2];
        let right_row_stride = right_shape.row_major_strides()[right_shape.dimensions().len() - 2];
        for row in 0..m {
            for column in 0..n {
                let mut sum = zero;
                for contraction in 0..k {
                    let left_value = left[left_base + row * left_row_stride + contraction];
                    let right_value = right[right_base + contraction * right_row_stride + column];
                    sum = multiply_add(sum, left_value, right_value);
                }
                output.push(sum);
            }
        }
    }
    output
}

fn unravel(mut linear: usize, shape: &[usize]) -> Vec<usize> {
    let mut coordinates = vec![0; shape.len()];
    for axis in (0..shape.len()).rev() {
        let dimension = shape[axis];
        if dimension != 0 {
            coordinates[axis] = linear % dimension;
            linear /= dimension;
        }
    }
    coordinates
}

fn batch_base(
    output_coordinates: &[usize],
    output_batch_shape: &[usize],
    input_shape: &CheckedShape,
) -> usize {
    let input_batch_rank = input_shape.dimensions().len() - 2;
    let rank_offset = output_batch_shape.len() - input_batch_rank;
    (0..input_batch_rank)
        .map(|axis| {
            let dimension = input_shape.dimensions()[axis];
            let coordinate = if dimension == 1 {
                0
            } else {
                output_coordinates[rank_offset + axis]
            };
            coordinate * input_shape.row_major_strides()[axis]
        })
        .sum()
}

fn data_len(data: &TensorData) -> usize {
    match data {
        TensorData::F32(values) => values.len(),
        TensorData::F64(values) => values.len(),
    }
}
