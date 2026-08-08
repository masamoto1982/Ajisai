use super::{CheckedShape, Tensor, TensorData, TensorMemoryBudget, TensorOperatorError};

pub fn tensor_add(
    left: &Tensor,
    right: &Tensor,
    budget: TensorMemoryBudget,
) -> Result<Tensor, TensorOperatorError> {
    binary(left, right, budget, |a, b| a + b, |a, b| a + b)
}

pub fn tensor_sub(
    left: &Tensor,
    right: &Tensor,
    budget: TensorMemoryBudget,
) -> Result<Tensor, TensorOperatorError> {
    binary(left, right, budget, |a, b| a - b, |a, b| a - b)
}

pub fn tensor_mul(
    left: &Tensor,
    right: &Tensor,
    budget: TensorMemoryBudget,
) -> Result<Tensor, TensorOperatorError> {
    binary(left, right, budget, |a, b| a * b, |a, b| a * b)
}

pub fn tensor_div(
    left: &Tensor,
    right: &Tensor,
    budget: TensorMemoryBudget,
) -> Result<Tensor, TensorOperatorError> {
    binary(left, right, budget, |a, b| a / b, |a, b| a / b)
}

fn binary(
    left: &Tensor,
    right: &Tensor,
    budget: TensorMemoryBudget,
    f32_operation: impl Fn(f32, f32) -> f32,
    f64_operation: impl Fn(f64, f64) -> f64,
) -> Result<Tensor, TensorOperatorError> {
    if left.dtype() != right.dtype() {
        return Err(TensorOperatorError::DTypeMismatch {
            left: left.dtype(),
            right: right.dtype(),
        });
    }
    let dimensions = broadcast_shape(left.shape().dimensions(), right.shape().dimensions())?;
    let output_shape = CheckedShape::new(dimensions.clone(), left.dtype().element_bytes(), budget)?;
    let data = match (left.data(), right.data()) {
        (TensorData::F32(left_data), TensorData::F32(right_data)) => TensorData::F32(binary_data(
            left_data,
            right_data,
            left.shape(),
            right.shape(),
            &output_shape,
            f32_operation,
        )),
        (TensorData::F64(left_data), TensorData::F64(right_data)) => TensorData::F64(binary_data(
            left_data,
            right_data,
            left.shape(),
            right.shape(),
            &output_shape,
            f64_operation,
        )),
        _ => unreachable!("dtype equality was checked"),
    };
    Tensor::new(dimensions, data, budget).map_err(Into::into)
}

fn broadcast_shape(left: &[usize], right: &[usize]) -> Result<Vec<usize>, TensorOperatorError> {
    let rank = left.len().max(right.len());
    let mut output = Vec::with_capacity(rank);
    for offset in (0..rank).rev() {
        let left_dimension = trailing_dimension(left, offset);
        let right_dimension = trailing_dimension(right, offset);
        output.push(match (left_dimension, right_dimension) {
            (Some(left), Some(right)) if left == right => left,
            (Some(1), Some(right)) => right,
            (Some(left), Some(1)) => left,
            (Some(_), Some(_)) => {
                return Err(TensorOperatorError::ElementwiseBroadcastMismatch {
                    left: left.to_vec(),
                    right: right.to_vec(),
                })
            }
            (Some(dimension), None) | (None, Some(dimension)) => dimension,
            (None, None) => unreachable!("rank bounds the loop"),
        });
    }
    Ok(output)
}

fn binary_data<T: Copy>(
    left: &[T],
    right: &[T],
    left_shape: &CheckedShape,
    right_shape: &CheckedShape,
    output_shape: &CheckedShape,
    operation: impl Fn(T, T) -> T,
) -> Vec<T> {
    (0..output_shape.element_count())
        .map(|linear| {
            let coordinates = unravel(linear, output_shape.dimensions());
            let left_index = broadcast_index(&coordinates, left_shape);
            let right_index = broadcast_index(&coordinates, right_shape);
            operation(left[left_index], right[right_index])
        })
        .collect()
}

fn broadcast_index(output_coordinates: &[usize], input_shape: &CheckedShape) -> usize {
    let offset = output_coordinates.len() - input_shape.dimensions().len();
    input_shape
        .dimensions()
        .iter()
        .zip(input_shape.row_major_strides())
        .enumerate()
        .map(|(axis, (dimension, stride))| {
            let coordinate = if *dimension == 1 {
                0
            } else {
                output_coordinates[offset + axis]
            };
            coordinate * stride
        })
        .sum()
}

fn trailing_dimension(shape: &[usize], offset: usize) -> Option<usize> {
    shape
        .len()
        .checked_sub(offset + 1)
        .map(|index| shape[index])
}

fn unravel(mut linear: usize, shape: &[usize]) -> Vec<usize> {
    let mut coordinates = vec![0; shape.len()];
    for axis in (0..shape.len()).rev() {
        if shape[axis] != 0 {
            coordinates[axis] = linear % shape[axis];
            linear /= shape[axis];
        }
    }
    coordinates
}
