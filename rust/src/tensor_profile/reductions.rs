use super::{CheckedShape, Tensor, TensorData, TensorMemoryBudget, TensorOperatorError};

pub fn reduce_sum(
    input: &Tensor,
    axes: &[usize],
    keep_dimensions: bool,
    budget: TensorMemoryBudget,
) -> Result<Tensor, TensorOperatorError> {
    reduce(input, axes, keep_dimensions, budget, Reduction::Sum)
}

pub fn reduce_max(
    input: &Tensor,
    axes: &[usize],
    keep_dimensions: bool,
    budget: TensorMemoryBudget,
) -> Result<Tensor, TensorOperatorError> {
    reduce(input, axes, keep_dimensions, budget, Reduction::Maximum)
}

#[derive(Clone, Copy)]
enum Reduction {
    Sum,
    Maximum,
}

fn reduce(
    input: &Tensor,
    axes: &[usize],
    keep_dimensions: bool,
    budget: TensorMemoryBudget,
    reduction: Reduction,
) -> Result<Tensor, TensorOperatorError> {
    let rank = input.shape().dimensions().len();
    let axes = checked_axes(axes, rank)?;
    let output_dimensions = reduced_shape(input.shape().dimensions(), &axes, keep_dimensions);
    let output_shape = CheckedShape::new(
        output_dimensions.clone(),
        input.dtype().element_bytes(),
        budget,
    )?;
    let data = match (input.data(), reduction) {
        (TensorData::F32(values), Reduction::Sum) => TensorData::F32(reduce_data(
            values,
            input.shape(),
            &output_shape,
            &axes,
            keep_dimensions,
            0.0f32,
            |slot, value| *slot += value,
        )),
        (TensorData::F64(values), Reduction::Sum) => TensorData::F64(reduce_data(
            values,
            input.shape(),
            &output_shape,
            &axes,
            keep_dimensions,
            0.0f64,
            |slot, value| *slot += value,
        )),
        (TensorData::F32(values), Reduction::Maximum) => TensorData::F32(reduce_data(
            values,
            input.shape(),
            &output_shape,
            &axes,
            keep_dimensions,
            f32::NEG_INFINITY,
            |slot, value| {
                if value.is_nan() || value > *slot {
                    *slot = value;
                }
            },
        )),
        (TensorData::F64(values), Reduction::Maximum) => TensorData::F64(reduce_data(
            values,
            input.shape(),
            &output_shape,
            &axes,
            keep_dimensions,
            f64::NEG_INFINITY,
            |slot, value| {
                if value.is_nan() || value > *slot {
                    *slot = value;
                }
            },
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

fn reduced_shape(shape: &[usize], axes: &[usize], keep_dimensions: bool) -> Vec<usize> {
    shape
        .iter()
        .enumerate()
        .filter_map(|(axis, dimension)| {
            if axes.binary_search(&axis).is_ok() {
                keep_dimensions.then_some(1)
            } else {
                Some(*dimension)
            }
        })
        .collect()
}

fn reduce_data<T: Copy>(
    input: &[T],
    input_shape: &CheckedShape,
    output_shape: &CheckedShape,
    axes: &[usize],
    keep_dimensions: bool,
    identity: T,
    update: impl Fn(&mut T, T),
) -> Vec<T> {
    let mut output = vec![identity; output_shape.element_count()];
    for (linear, value) in input.iter().copied().enumerate() {
        let coordinates = unravel(linear, input_shape.dimensions());
        let output_coordinates =
            coordinates
                .into_iter()
                .enumerate()
                .filter_map(|(axis, coordinate)| {
                    if axes.binary_search(&axis).is_ok() {
                        keep_dimensions.then_some(0)
                    } else {
                        Some(coordinate)
                    }
                });
        let output_linear = output_coordinates
            .zip(output_shape.row_major_strides())
            .map(|(coordinate, stride)| coordinate * stride)
            .sum::<usize>();
        update(&mut output[output_linear], value);
    }
    output
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
