use super::{
    require_numeric, CheckedShape, Tensor, TensorData, TensorMemoryBudget, TensorOperatorError,
};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;

pub fn reduce_sum(
    input: &Tensor,
    axes: &[usize],
    keep_dimensions: bool,
    budget: TensorMemoryBudget,
) -> Result<Tensor, TensorOperatorError> {
    reduce(
        input,
        axes,
        keep_dimensions,
        budget,
        Reduction::Sum,
        "REDUCE_SUM",
    )
}

pub fn reduce_max(
    input: &Tensor,
    axes: &[usize],
    keep_dimensions: bool,
    budget: TensorMemoryBudget,
) -> Result<Tensor, TensorOperatorError> {
    reduce(
        input,
        axes,
        keep_dimensions,
        budget,
        Reduction::Maximum,
        "REDUCE_MAX",
    )
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
    operator: &'static str,
) -> Result<Tensor, TensorOperatorError> {
    require_numeric(operator, input.dtype())?;
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
        // The approximate reductions lean on ±infinity as an identity element.
        // The rationals have no such element, so the exact path carries an
        // explicit "nothing seen yet" instead, and an empty REDUCE_MAX slice
        // is reported rather than answered with an invented value.
        (TensorData::Q(values), Reduction::Sum) => TensorData::Q(reduce_exact(
            values,
            input.shape(),
            &output_shape,
            &axes,
            keep_dimensions,
            |accumulated, value| Some(accumulated.map_or(value.clone(), |sum| sum.add(value))),
            Some(Fraction::new(BigInt::from(0), BigInt::from(1))),
            operator,
        )?),
        (TensorData::Q(values), Reduction::Maximum) => TensorData::Q(reduce_exact(
            values,
            input.shape(),
            &output_shape,
            &axes,
            keep_dimensions,
            |accumulated, value| {
                Some(match accumulated {
                    Some(current) if current >= *value => current,
                    _ => value.clone(),
                })
            },
            None,
            operator,
        )?),
        (TensorData::Bool(_), _) => unreachable!("numeric dtype was checked"),
    };
    Tensor::new(output_dimensions, data, budget).map_err(Into::into)
}

/// The exact counterpart of [`reduce_data`], threading an `Option` accumulator
/// so a reduction with no identity element can report an empty slice instead
/// of fabricating one. `empty` supplies the identity where one exists (zero,
/// for a sum) and is `None` where none does (a maximum).
#[allow(clippy::too_many_arguments)]
fn reduce_exact(
    input: &[Fraction],
    input_shape: &CheckedShape,
    output_shape: &CheckedShape,
    axes: &[usize],
    keep_dimensions: bool,
    combine: impl Fn(Option<Fraction>, &Fraction) -> Option<Fraction>,
    empty: Option<Fraction>,
    operator: &'static str,
) -> Result<Vec<Fraction>, TensorOperatorError> {
    let mut output = vec![None; output_shape.element_count()];
    for (linear, value) in input.iter().enumerate() {
        let output_linear = reduced_index(linear, input_shape, output_shape, axes, keep_dimensions);
        output[output_linear] = combine(output[output_linear].take(), value);
    }
    output
        .into_iter()
        .map(|slot| {
            slot.or_else(|| empty.clone())
                .ok_or(TensorOperatorError::EmptyExactReduction { operator })
        })
        .collect()
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
        let output_linear = reduced_index(linear, input_shape, output_shape, axes, keep_dimensions);
        update(&mut output[output_linear], value);
    }
    output
}

/// Map an input element's linear index to the output slot it reduces into.
/// Shared by the approximate and exact paths so the two cannot disagree about
/// which elements belong to the same reduced slice.
fn reduced_index(
    linear: usize,
    input_shape: &CheckedShape,
    output_shape: &CheckedShape,
    axes: &[usize],
    keep_dimensions: bool,
) -> usize {
    unravel(linear, input_shape.dimensions())
        .into_iter()
        .enumerate()
        .filter_map(|(axis, coordinate)| {
            if axes.binary_search(&axis).is_ok() {
                keep_dimensions.then_some(0)
            } else {
                Some(coordinate)
            }
        })
        .zip(output_shape.row_major_strides())
        .map(|(coordinate, stride)| coordinate * stride)
        .sum()
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
