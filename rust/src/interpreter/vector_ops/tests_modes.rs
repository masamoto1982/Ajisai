//! Test suite for `crate::interpreter::vector_ops` operand consumption.

use crate::interpreter::Interpreter;

#[tokio::test]
async fn test_collect_basic() {
    let mut interp = Interpreter::new();

    let result = interp.execute("1 2 3 3 COLLECT").await;
    assert!(result.is_ok(), "COLLECT should succeed: {:?}", result);
    assert_eq!(interp.stack.len(), 1);

    let val = &interp.stack[0];
    assert_eq!(val.shape(), vec![3], "Result should have 3 elements");
}

#[tokio::test]
async fn test_collect_vectors_without_flattening() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 1 2 ] [ 3 4 ] 2 COLLECT").await;
    assert!(
        result.is_ok(),
        "COLLECT vectors should succeed: {:?}",
        result
    );
    assert_eq!(interp.stack.len(), 1);

    let val = &interp.stack[0];
    assert!(val.is_vector(), "Result should be a vector");
}
#[tokio::test]
async fn test_collect_error_underflow() {
    let mut interp = Interpreter::new();

    let result = interp.execute("1 2 5 COLLECT").await;
    assert!(
        result.is_err(),
        "COLLECT with insufficient stack should fail"
    );

    assert_eq!(interp.stack.len(), 3, "Stack should have count pushed back");
}

#[tokio::test]
async fn test_collect_zero_count_is_the_empty_vector() {
    // N is a non-negative integer, so zero is a count like any other: it
    // takes nothing and answers `[ ]`, leaving the stack below untouched.
    let mut interp = Interpreter::new();

    let result = interp.execute("1 2 3 0 COLLECT").await;
    assert!(
        result.is_ok(),
        "0 COLLECT answers the empty Vector: {:?}",
        result
    );
    assert_eq!(interp.stack.len(), 4);
    assert_eq!(
        crate::types::display::render_stack(interp.get_stack())
            .last()
            .map(String::as_str),
        Some("[ ]")
    );
}

#[tokio::test]
async fn test_collect_error_negative_count() {
    let mut interp = Interpreter::new();

    let result = interp.execute("1 2 3 -2 COLLECT").await;
    assert!(result.is_err(), "COLLECT with negative count should fail");
}

#[tokio::test]
async fn test_get_consume_mode() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 20 30 ] 0 GET").await;
    assert!(result.is_ok(), "GET should succeed: {:?}", result);
    assert_eq!(
        interp.stack.len(),
        1,
        "GET consumes what it reads: both operands leave, the element stays"
    );
}

#[tokio::test]
async fn test_length_consume_mode() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 1 2 3 4 5 ] LENGTH").await;
    assert!(result.is_ok(), "LENGTH should succeed: {:?}", result);
    assert_eq!(
        interp.stack.len(),
        1,
        "LENGTH consumes what it reads: the measured vector leaves the stack"
    );
}

#[tokio::test]
async fn test_get_returns_the_element() {
    let mut interp = Interpreter::new();
    let result = interp.execute("[ 10 20 30 ] 0 GET").await;
    assert!(result.is_ok(), "GET should succeed: {:?}", result);
    assert_eq!(interp.stack.len(), 1, "only the element remains");
    let result_scalar = interp.stack[0]
        .as_scalar()
        .expect("result should be scalar");
    assert_eq!(result_scalar.to_i64(), Some(10));
}

#[tokio::test]
async fn test_print_outputs_and_consumes() {
    let mut interp = Interpreter::new();
    let result = interp.execute("[ 42 ] PRINT").await;
    assert!(result.is_ok(), "PRINT should succeed: {:?}", result);
    assert!(interp.stack.is_empty(), "PRINT consumes its operand");
    assert!(
        interp.output_buffer.contains("42/1"),
        "PRINT should output the value, got: {}",
        interp.output_buffer
    );
}

#[tokio::test]
async fn test_floor_consumes_operand() {
    let mut interp = Interpreter::new();
    let result = interp.execute("[ 3.7 ] FLOOR").await;
    assert!(result.is_ok(), "FLOOR should succeed: {:?}", result);
    assert_eq!(interp.stack.len(), 1, "FLOOR leaves only its result");
}
