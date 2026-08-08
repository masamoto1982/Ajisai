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
