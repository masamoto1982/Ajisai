use super::elementwise::{broadcast_index, broadcast_shape, unravel};
use super::{CheckedShape, Tensor, TensorData, TensorMemoryBudget, TensorOperatorError};

/// Reference implementation of `tensor.where.v1`.
///
/// The predicate is a `bool` tensor, never a numeric tensor read as truthy:
/// the profile forbids implicit casts, so "nonzero means true" would be an
/// unrecorded conversion. All three operands broadcast against one another and
/// the two value operands share the output dtype.
///
/// Selection is `bitwise` deterministic. It copies the chosen element rather
/// than computing with it, so a masked-out NaN or infinity can never leak into
/// the result — which is what makes `WHERE` usable for attention masks, where
/// the discarded branch is routinely negative infinity.
pub fn tensor_where(
    predicate: &Tensor,
    on_true: &Tensor,
    on_false: &Tensor,
    budget: TensorMemoryBudget,
) -> Result<Tensor, TensorOperatorError> {
    let TensorData::Bool(mask) = predicate.data() else {
        return Err(TensorOperatorError::PredicateDTypeMismatch {
            operator: "WHERE",
            dtype: predicate.dtype(),
        });
    };
    if on_true.dtype() != on_false.dtype() {
        return Err(TensorOperatorError::DTypeMismatch {
            left: on_true.dtype(),
            right: on_false.dtype(),
        });
    }

    let values_shape =
        broadcast_shape(on_true.shape().dimensions(), on_false.shape().dimensions())?;
    let dimensions = broadcast_shape(predicate.shape().dimensions(), &values_shape)?;
    let output_shape =
        CheckedShape::new(dimensions.clone(), on_true.dtype().element_bytes(), budget)?;

    let data = match (on_true.data(), on_false.data()) {
        (TensorData::F32(left), TensorData::F32(right)) => TensorData::F32(select(
            mask,
            left,
            right,
            predicate.shape(),
            on_true.shape(),
            on_false.shape(),
            &output_shape,
        )),
        (TensorData::F64(left), TensorData::F64(right)) => TensorData::F64(select(
            mask,
            left,
            right,
            predicate.shape(),
            on_true.shape(),
            on_false.shape(),
            &output_shape,
        )),
        (TensorData::Bool(left), TensorData::Bool(right)) => TensorData::Bool(select(
            mask,
            left,
            right,
            predicate.shape(),
            on_true.shape(),
            on_false.shape(),
            &output_shape,
        )),
        _ => unreachable!("dtype equality was checked"),
    };
    Tensor::new(dimensions, data, budget).map_err(Into::into)
}

#[allow(clippy::too_many_arguments)]
fn select<T: Clone>(
    mask: &[bool],
    on_true: &[T],
    on_false: &[T],
    predicate_shape: &CheckedShape,
    true_shape: &CheckedShape,
    false_shape: &CheckedShape,
    output_shape: &CheckedShape,
) -> Vec<T> {
    (0..output_shape.element_count())
        .map(|linear| {
            let coordinates = unravel(linear, output_shape.dimensions());
            if mask[broadcast_index(&coordinates, predicate_shape)] {
                on_true[broadcast_index(&coordinates, true_shape)].clone()
            } else {
                on_false[broadcast_index(&coordinates, false_shape)].clone()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor_profile::DType;

    const OPEN: TensorMemoryBudget = TensorMemoryBudget::new(usize::MAX, usize::MAX);

    fn f32_tensor(shape: Vec<usize>, values: Vec<f32>) -> Tensor {
        Tensor::new(shape, TensorData::F32(values), OPEN).unwrap()
    }

    fn mask(shape: Vec<usize>, values: Vec<bool>) -> Tensor {
        Tensor::new(shape, TensorData::Bool(values), OPEN).unwrap()
    }

    #[test]
    fn selects_elementwise_from_both_branches() {
        let result = tensor_where(
            &mask(vec![4], vec![true, false, true, false]),
            &f32_tensor(vec![4], vec![1.0, 2.0, 3.0, 4.0]),
            &f32_tensor(vec![4], vec![10.0, 20.0, 30.0, 40.0]),
            OPEN,
        )
        .unwrap();
        assert_eq!(result.data(), &TensorData::F32(vec![1.0, 20.0, 3.0, 40.0]));
    }

    #[test]
    fn broadcasts_predicate_and_both_value_operands() {
        // A causal-style mask: one column of predicates chooses between a
        // full matrix and a scalar fill value.
        let result = tensor_where(
            &mask(vec![2, 1], vec![true, false]),
            &f32_tensor(vec![2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
            &f32_tensor(vec![], vec![f32::NEG_INFINITY]),
            OPEN,
        )
        .unwrap();
        assert_eq!(result.shape().dimensions(), &[2, 3]);
        let TensorData::F32(values) = result.data() else {
            panic!("dtype changed")
        };
        assert_eq!(&values[..3], &[1.0, 2.0, 3.0]);
        assert!(values[3..].iter().all(|value| *value == f32::NEG_INFINITY));
    }

    #[test]
    fn masked_out_nan_never_reaches_the_result() {
        let result = tensor_where(
            &mask(vec![2], vec![true, true]),
            &f32_tensor(vec![2], vec![1.0, 2.0]),
            &f32_tensor(vec![2], vec![f32::NAN, f32::NAN]),
            OPEN,
        )
        .unwrap();
        assert_eq!(result.data(), &TensorData::F32(vec![1.0, 2.0]));
    }

    #[test]
    fn rejects_a_numeric_predicate_instead_of_reading_it_as_truthy() {
        assert_eq!(
            tensor_where(
                &f32_tensor(vec![2], vec![1.0, 0.0]),
                &f32_tensor(vec![2], vec![1.0, 2.0]),
                &f32_tensor(vec![2], vec![3.0, 4.0]),
                OPEN,
            ),
            Err(TensorOperatorError::PredicateDTypeMismatch {
                operator: "WHERE",
                dtype: DType::F32,
            })
        );
    }

    #[test]
    fn rejects_branches_that_disagree_on_dtype() {
        let on_false = Tensor::new(vec![2], TensorData::F64(vec![0.0, 0.0]), OPEN).unwrap();
        assert_eq!(
            tensor_where(
                &mask(vec![2], vec![true, false]),
                &f32_tensor(vec![2], vec![1.0, 2.0]),
                &on_false,
                OPEN,
            ),
            Err(TensorOperatorError::DTypeMismatch {
                left: DType::F32,
                right: DType::F64,
            })
        );
    }

    #[test]
    fn rejects_shapes_that_do_not_broadcast() {
        assert!(matches!(
            tensor_where(
                &mask(vec![2], vec![true, false]),
                &f32_tensor(vec![3], vec![1.0, 2.0, 3.0]),
                &f32_tensor(vec![3], vec![4.0, 5.0, 6.0]),
                OPEN,
            ),
            Err(TensorOperatorError::ElementwiseBroadcastMismatch { .. })
        ));
    }

    #[test]
    fn checks_the_output_budget_before_allocating() {
        assert_eq!(
            tensor_where(
                &mask(vec![4], vec![true; 4]),
                &f32_tensor(vec![4], vec![1.0; 4]),
                &f32_tensor(vec![4], vec![0.0; 4]),
                TensorMemoryBudget::new(4, 15),
            ),
            Err(TensorOperatorError::Shape(
                super::super::ShapeError::ByteBudgetExceeded {
                    requested: 16,
                    limit: 15,
                }
            ))
        );
    }
}
