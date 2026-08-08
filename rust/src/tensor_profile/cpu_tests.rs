use super::*;

const OPEN: TensorMemoryBudget = TensorMemoryBudget::new(usize::MAX, usize::MAX);

fn f32_tensor(shape: Vec<usize>, values: Vec<f32>) -> Tensor {
    Tensor::new(shape, TensorData::F32(values), OPEN).unwrap()
}

#[test]
fn reference_matmul_multiplies_two_matrices_in_k_order() {
    let left = f32_tensor(vec![2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    let right = f32_tensor(vec![3, 2], vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0]);
    let result = matmul(&left, &right, OPEN).unwrap();
    assert_eq!(result.shape().dimensions(), &[2, 2]);
    assert_eq!(
        result.data(),
        &TensorData::F32(vec![58.0, 64.0, 139.0, 154.0])
    );
}

#[test]
fn reference_matmul_broadcasts_batch_dimensions() {
    let left = f32_tensor(vec![2, 1, 1, 2], vec![1.0, 2.0, 3.0, 4.0]);
    let right = f32_tensor(vec![3, 2, 1], vec![1.0, 10.0, 2.0, 20.0, 3.0, 30.0]);
    let result = matmul(&left, &right, OPEN).unwrap();
    assert_eq!(result.shape().dimensions(), &[2, 3, 1, 1]);
    assert_eq!(
        result.data(),
        &TensorData::F32(vec![21.0, 42.0, 63.0, 43.0, 86.0, 129.0])
    );
}

#[test]
fn reference_matmul_rejects_contraction_mismatch() {
    let left = f32_tensor(vec![2, 3], vec![0.0; 6]);
    let right = f32_tensor(vec![4, 2], vec![0.0; 8]);
    assert_eq!(
        matmul(&left, &right, OPEN),
        Err(TensorOperatorError::ContractionMismatch { left: 3, right: 4 })
    );
}

#[test]
fn reference_matmul_checks_output_bytes_before_allocation() {
    let left = f32_tensor(vec![2, 2], vec![1.0; 4]);
    let right = f32_tensor(vec![2, 2], vec![1.0; 4]);
    let budget = TensorMemoryBudget::new(4, 15);
    assert_eq!(
        matmul(&left, &right, budget),
        Err(TensorOperatorError::Shape(ShapeError::ByteBudgetExceeded {
            requested: 16,
            limit: 15
        }))
    );
}

#[test]
fn reference_exp_and_log_preserve_shape_and_dtype() {
    let input = f32_tensor(vec![2], vec![0.0, 1.0]);
    let exponentials = tensor_exp(&input, OPEN).unwrap();
    assert_eq!(exponentials.shape().dimensions(), &[2]);
    let logarithms = tensor_log(&exponentials, OPEN).unwrap();
    let TensorData::F32(values) = logarithms.data() else {
        panic!("dtype changed")
    };
    assert!((values[0] - 0.0).abs() < 1e-6);
    assert!((values[1] - 1.0).abs() < 1e-6);
}

#[test]
fn reference_log_keeps_ieee_domain_states_inside_the_tensor() {
    let result = tensor_log(&f32_tensor(vec![2], vec![0.0, -1.0]), OPEN).unwrap();
    let TensorData::F32(values) = result.data() else {
        panic!("dtype changed")
    };
    assert_eq!(values[0], f32::NEG_INFINITY);
    assert!(values[1].is_nan());
}

#[test]
fn reference_reduce_sum_supports_multiple_axes_and_keep_dimensions() {
    let input = f32_tensor(vec![2, 2, 2], (1..=8).map(|value| value as f32).collect());
    let reduced = reduce_sum(&input, &[0, 2], false, OPEN).unwrap();
    assert_eq!(reduced.shape().dimensions(), &[2]);
    assert_eq!(reduced.data(), &TensorData::F32(vec![14.0, 22.0]));

    let kept = reduce_sum(&input, &[0, 2], true, OPEN).unwrap();
    assert_eq!(kept.shape().dimensions(), &[1, 2, 1]);
    assert_eq!(kept.data(), &TensorData::F32(vec![14.0, 22.0]));
}

#[test]
fn reference_reduce_sum_rejects_duplicate_axes() {
    let input = f32_tensor(vec![2], vec![1.0, 2.0]);
    assert_eq!(
        reduce_sum(&input, &[0, 0], false, OPEN),
        Err(TensorOperatorError::DuplicateAxis(0))
    );
}

#[test]
fn reference_reduce_max_preserves_nan_and_uses_negative_infinity_identity() {
    let input = f32_tensor(vec![2, 3], vec![1.0, f32::NAN, 3.0, -5.0, -2.0, -7.0]);
    let reduced = reduce_max(&input, &[1], false, OPEN).unwrap();
    let TensorData::F32(values) = reduced.data() else {
        panic!("dtype changed")
    };
    assert!(values[0].is_nan());
    assert_eq!(values[1], -2.0);

    let empty = f32_tensor(vec![2, 0], vec![]);
    assert_eq!(
        reduce_max(&empty, &[1], false, OPEN).unwrap().data(),
        &TensorData::F32(vec![f32::NEG_INFINITY; 2])
    );
}
