


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
async fn test_collect_for_formant_synthesis() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp
        .execute("[ 800 1200 ] MUSIC@CHORD [ 300 2500 ] MUSIC@CHORD 2 COLLECT")
        .await;
    assert!(
        result.is_ok(),
        "COLLECT for formant should succeed: {:?}",
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
async fn test_collect_error_zero_count() {
    let mut interp = Interpreter::new();

    let result = interp.execute("1 2 3 0 COLLECT").await;
    assert!(result.is_err(), "COLLECT with zero count should fail");
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

    let result = interp.execute("[ 10 20 30 ] [ 0 ] GET").await;
    assert!(result.is_ok(), "GET should succeed: {:?}", result);
    assert_eq!(
        interp.stack.len(),
        1,
        "GET in consume mode should leave only result"
    );
}

#[tokio::test]
async fn test_get_keep_mode() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 20 30 ] [ 0 ] ,, GET").await;
    assert!(
        result.is_ok(),
        "GET with keep mode should succeed: {:?}",
        result
    );
    assert_eq!(
        interp.stack.len(),
        3,
        "GET in keep mode should preserve target, index, and add result"
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
        "LENGTH in consume mode should leave only result"
    );
}

#[tokio::test]
async fn test_length_keep_mode() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 1 2 3 4 5 ] ,, LENGTH").await;
    assert!(
        result.is_ok(),
        "LENGTH with keep mode should succeed: {:?}",
        result
    );
    assert_eq!(
        interp.stack.len(),
        2,
        "LENGTH in keep mode should preserve target and add result"
    );
}

#[tokio::test]
async fn test_reverse_keep_mode() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 3 1 2 ] ,, REVERSE").await;
    assert!(
        result.is_ok(),
        "REVERSE with keep mode should succeed: {:?}",
        result
    );
    assert_eq!(
        interp.stack.len(),
        2,
        "REVERSE in keep mode should preserve original and add result"
    );
}

#[tokio::test]
async fn test_take_keep_mode() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 1 2 3 4 5 ] [ 3 ] ,, TAKE").await;
    assert!(
        result.is_ok(),
        "TAKE with keep mode should succeed: {:?}",
        result
    );
    assert_eq!(
        interp.stack.len(),
        3,
        "TAKE in keep mode should preserve target, args, and add result"
    );
}





#[tokio::test]
async fn test_get_keep_mode_preserves_all_operands() {
    let mut interp = Interpreter::new();
    let result = interp.execute("[ 10 20 30 ] [ 0 ] ,, GET").await;
    assert!(result.is_ok(), "GET ,, should succeed: {:?}", result);
    assert_eq!(interp.stack.len(), 3, "target + index + result");

    assert!(interp.stack[0].is_vector());
    assert!(interp.stack[1].is_vector());
    let result_scalar = interp.stack[2]
        .as_scalar()
        .expect("result should be scalar");
    assert_eq!(result_scalar.to_i64(), Some(10));
}

#[tokio::test]
async fn test_get_stack_consume_drains_stack() {
    let mut interp = Interpreter::new();
    let result = interp.execute("[10] [20] [30] [1] .. GET").await;
    assert!(result.is_ok(), "Stack GET should succeed: {:?}", result);
    assert_eq!(
        interp.stack.len(),
        1,
        "Stack+Consume GET should leave only the result"
    );
    assert!(
        interp.stack[0].is_vector(),
        "result should be a vector [20]"
    );
}

#[tokio::test]
async fn test_get_stack_keep_preserves_stack() {
    let mut interp = Interpreter::new();
    let result = interp.execute("[10] [20] [30] [1] ,, .. GET").await;
    assert!(
        result.is_ok(),
        "Stack Keep GET should succeed: {:?}",
        result
    );
    assert_eq!(
        interp.stack.len(),
        4,
        "Stack+Keep GET should preserve stack and add result"
    );
    assert!(
        interp.stack[3].is_vector(),
        "result should be a vector [20]"
    );
}

#[tokio::test]
async fn test_print_keep_mode() {
    let mut interp = Interpreter::new();
    let result = interp.execute("[ 42 ] ,, PRINT").await;
    assert!(result.is_ok(), "PRINT ,, should succeed: {:?}", result);
    assert_eq!(
        interp.stack.len(),
        1,
        "PRINT in keep mode should preserve value on stack"
    );
    assert!(
        interp.output_buffer.contains("*"),
        "PRINT should output the value, got: {}",
        interp.output_buffer
    );
}

#[tokio::test]
async fn test_floor_keep_mode() {
    let mut interp = Interpreter::new();
    let result = interp.execute("[ 3.7 ] ,, FLOOR").await;
    assert!(result.is_ok(), "FLOOR ,, should succeed: {:?}", result);
    assert_eq!(
        interp.stack.len(),
        2,
        "FLOOR in keep mode should preserve original and add result"
    );
}

#[tokio::test]
async fn test_mod_keep_mode() {
    let mut interp = Interpreter::new();
    let result = interp.execute("[ 10 ] [ 3 ] ,, MOD").await;
    assert!(result.is_ok(), "MOD ,, should succeed: {:?}", result);
    assert_eq!(
        interp.stack.len(),
        3,
        "MOD in keep mode should preserve both operands and add result"
    );
}

#[tokio::test]
async fn test_modifier_order_independence() {
    let mut interp1 = Interpreter::new();
    let result1 = interp1.execute("[1] [2] [3] [3] .. ,, +").await;
    assert!(result1.is_ok());

    let mut interp2 = Interpreter::new();
    let result2 = interp2.execute("[1] [2] [3] [3] ,, .. +").await;
    assert!(result2.is_ok());

    assert_eq!(
        interp1.stack.len(),
        interp2.stack.len(),
        ".. ,, and ,, .. should produce same result"
    );
}

#[tokio::test]
async fn test_modes_auto_reset_after_execution() {
    let mut interp = Interpreter::new();

    let result1 = interp.execute("[ 1 ] [ 2 ] ,, +").await;
    assert!(result1.is_ok());
    assert_eq!(interp.stack.len(), 3);

    let result2 = interp.execute("+").await;
    assert!(result2.is_ok());
    assert_eq!(
        interp.stack.len(),
        2,
        "After auto-reset, + should consume operands"
    );
}
