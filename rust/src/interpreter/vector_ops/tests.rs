

use crate::interpreter::Interpreter;

#[tokio::test]
async fn test_range_basic_stacktop() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 0 5 ] RANGE").await;
    assert!(result.is_ok(), "RANGE should succeed: {:?}", result);

    assert_eq!(interp.stack.len(), 1);
}

#[tokio::test]
async fn test_range_with_step() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 0 10 2 ] RANGE").await;
    assert!(
        result.is_ok(),
        "RANGE with step should succeed: {:?}",
        result
    );

    assert_eq!(interp.stack.len(), 1);
}

#[tokio::test]
async fn test_range_descending() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 0 -2 ] RANGE").await;
    assert!(
        result.is_ok(),
        "RANGE descending should succeed: {:?}",
        result
    );
    assert_eq!(interp.stack.len(), 1);
}

#[tokio::test]
async fn test_range_single_element() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 5 5 ] RANGE").await;
    assert!(
        result.is_ok(),
        "RANGE single element should succeed: {:?}",
        result
    );
    assert_eq!(interp.stack.len(), 1);
}

#[tokio::test]
async fn test_range_stack_mode() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 0 5 ] .. RANGE").await;
    assert!(
        result.is_ok(),
        "RANGE stack mode should succeed: {:?}",
        result
    );
    assert_eq!(interp.stack.len(), 1);
}

#[tokio::test]
async fn test_range_error_step_zero_restores_stack_stacktop() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 0 10 0 ] RANGE").await;
    assert!(result.is_err(), "RANGE with step=0 should fail");

    assert_eq!(
        interp.stack.len(),
        1,
        "Arguments should be restored on error"
    );
}

#[tokio::test]
async fn test_range_error_step_zero_restores_stack_stack_mode() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 0 10 0 ] .. RANGE").await;
    assert!(result.is_err(), "RANGE stack mode with step=0 should fail");

    assert_eq!(
        interp.stack.len(),
        1,
        "Arguments should be restored on error in stack mode"
    );
}

#[tokio::test]
async fn test_range_error_infinite_restores_stack() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 0 10 -1 ] RANGE").await;
    assert!(result.is_err(), "RANGE with infinite sequence should fail");

    assert_eq!(
        interp.stack.len(),
        1,
        "Arguments should be restored on infinite error"
    );
}


#[tokio::test]
async fn test_reorder_basic_stacktop() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 20 30 ] [ 2 0 1 ] REORDER").await;
    assert!(result.is_ok(), "REORDER should succeed: {:?}", result);
    assert_eq!(interp.stack.len(), 1);

    let val = &interp.stack[0];
    assert!(val.is_vector(), "Result should be a vector");
}

#[tokio::test]
async fn test_reorder_duplicate_indices() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 20 30 ] [ 0 0 0 ] REORDER").await;
    assert!(
        result.is_ok(),
        "REORDER with duplicate indices should succeed: {:?}",
        result
    );
    assert_eq!(interp.stack.len(), 1);

    let val = &interp.stack[0];
    assert_eq!(val.shape(), vec![3], "Result should have 3 elements");
}

#[tokio::test]
async fn test_reorder_negative_indices() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 20 30 ] [ -1 -2 -3 ] REORDER").await;
    assert!(
        result.is_ok(),
        "REORDER with negative indices should succeed: {:?}",
        result
    );
    assert_eq!(interp.stack.len(), 1);
}

#[tokio::test]
async fn test_reorder_partial_selection() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 20 30 ] [ 1 ] REORDER").await;
    assert!(
        result.is_ok(),
        "REORDER with partial selection should succeed: {:?}",
        result
    );
    assert_eq!(interp.stack.len(), 1);

    let val = &interp.stack[0];
    assert_eq!(val.shape(), vec![1], "Result should have 1 element");
}

#[tokio::test]
async fn test_reorder_stack_mode_swap() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 ] [ 20 ] [ 1 0 ] .. REORDER").await;
    assert!(
        result.is_ok(),
        "REORDER stack mode SWAP should succeed: {:?}",
        result
    );
    assert_eq!(interp.stack.len(), 2, "Stack should have 2 elements");
}

#[tokio::test]
async fn test_reorder_stack_mode_rot() {
    let mut interp = Interpreter::new();

    let result = interp
        .execute("[ 10 ] [ 20 ] [ 30 ] [ 1 2 0 ] .. REORDER")
        .await;
    assert!(
        result.is_ok(),
        "REORDER stack mode ROT should succeed: {:?}",
        result
    );
    assert_eq!(interp.stack.len(), 3, "Stack should have 3 elements");
}

#[tokio::test]
async fn test_reorder_error_empty_indices() {
    let _interp = Interpreter::new();
}

#[tokio::test]
async fn test_reorder_error_out_of_bounds() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 20 30 ] [ 5 ] REORDER").await;
    assert!(
        result.is_err(),
        "REORDER with out of bounds index should fail"
    );

    assert_eq!(interp.stack.len(), 2, "Stack should be restored on error");
}

#[tokio::test]
async fn test_reorder_error_negative_out_of_bounds() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 20 30 ] [ -5 ] REORDER").await;
    assert!(
        result.is_err(),
        "REORDER with negative out of bounds index should fail"
    );

    assert_eq!(interp.stack.len(), 2, "Stack should be restored on error");
}

#[tokio::test]
async fn test_reorder_error_non_vector() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 ] [ 0 ] REORDER").await;
    assert!(
        result.is_ok(),
        "REORDER on scalar-like value should succeed"
    );
}

#[tokio::test]
async fn test_reorder_stack_mode_error_out_of_bounds() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 ] [ 20 ] [ 5 ] .. REORDER").await;
    assert!(
        result.is_err(),
        "REORDER stack mode with out of bounds should fail"
    );

    assert_eq!(
        interp.stack.len(),
        3,
        "Stack should have indices pushed back on error"
    );
}

#[tokio::test]
async fn test_reorder_single_element_index() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 20 30 ] [ 2 ] REORDER").await;
    assert!(
        result.is_ok(),
        "REORDER with single index should succeed: {:?}",
        result
    );
    assert_eq!(interp.stack.len(), 1);

    let val = &interp.stack[0];
    assert_eq!(val.shape(), vec![1], "Result should have 1 element");
}
