//! Test suite for `crate::interpreter::vector_ops`.

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
