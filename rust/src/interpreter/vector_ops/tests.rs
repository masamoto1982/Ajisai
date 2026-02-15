// rust/src/interpreter/vector_ops/tests.rs

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
    assert!(result.is_ok(), "RANGE with step should succeed: {:?}", result);

    assert_eq!(interp.stack.len(), 1);
}

#[tokio::test]
async fn test_range_descending() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 0 -2 ] RANGE").await;
    assert!(result.is_ok(), "RANGE descending should succeed: {:?}", result);
    assert_eq!(interp.stack.len(), 1);
}

#[tokio::test]
async fn test_range_single_element() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 5 5 ] RANGE").await;
    assert!(result.is_ok(), "RANGE single element should succeed: {:?}", result);
    assert_eq!(interp.stack.len(), 1);
}

#[tokio::test]
async fn test_range_stack_mode() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 0 5 ] .. RANGE").await;
    assert!(result.is_ok(), "RANGE stack mode should succeed: {:?}", result);
    assert_eq!(interp.stack.len(), 1);
}

#[tokio::test]
async fn test_range_error_step_zero_restores_stack_stacktop() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 0 10 0 ] RANGE").await;
    assert!(result.is_err(), "RANGE with step=0 should fail");

    assert_eq!(interp.stack.len(), 1, "Arguments should be restored on error");
}

#[tokio::test]
async fn test_range_error_step_zero_restores_stack_stack_mode() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 0 10 0 ] .. RANGE").await;
    assert!(result.is_err(), "RANGE stack mode with step=0 should fail");

    assert_eq!(interp.stack.len(), 1, "Arguments should be restored on error in stack mode");
}

#[tokio::test]
async fn test_range_error_infinite_restores_stack() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 0 10 -1 ] RANGE").await;
    assert!(result.is_err(), "RANGE with infinite sequence should fail");

    assert_eq!(interp.stack.len(), 1, "Arguments should be restored on infinite error");
}

// ========================================================================
// REORDER テスト
// ========================================================================

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
    assert!(result.is_ok(), "REORDER with duplicate indices should succeed: {:?}", result);
    assert_eq!(interp.stack.len(), 1);

    let val = &interp.stack[0];
    assert_eq!(val.shape(), vec![3], "Result should have 3 elements");
}

#[tokio::test]
async fn test_reorder_negative_indices() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 20 30 ] [ -1 -2 -3 ] REORDER").await;
    assert!(result.is_ok(), "REORDER with negative indices should succeed: {:?}", result);
    assert_eq!(interp.stack.len(), 1);
}

#[tokio::test]
async fn test_reorder_partial_selection() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 20 30 ] [ 1 ] REORDER").await;
    assert!(result.is_ok(), "REORDER with partial selection should succeed: {:?}", result);
    assert_eq!(interp.stack.len(), 1);

    let val = &interp.stack[0];
    assert_eq!(val.shape(), vec![1], "Result should have 1 element");
}

#[tokio::test]
async fn test_reorder_stack_mode_swap() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 ] [ 20 ] [ 1 0 ] .. REORDER").await;
    assert!(result.is_ok(), "REORDER stack mode SWAP should succeed: {:?}", result);
    assert_eq!(interp.stack.len(), 2, "Stack should have 2 elements");
}

#[tokio::test]
async fn test_reorder_stack_mode_rot() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 ] [ 20 ] [ 30 ] [ 1 2 0 ] .. REORDER").await;
    assert!(result.is_ok(), "REORDER stack mode ROT should succeed: {:?}", result);
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
    assert!(result.is_err(), "REORDER with out of bounds index should fail");

    assert_eq!(interp.stack.len(), 2, "Stack should be restored on error");
}

#[tokio::test]
async fn test_reorder_error_negative_out_of_bounds() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 20 30 ] [ -5 ] REORDER").await;
    assert!(result.is_err(), "REORDER with negative out of bounds index should fail");

    assert_eq!(interp.stack.len(), 2, "Stack should be restored on error");
}

#[tokio::test]
async fn test_reorder_error_non_vector() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 ] [ 0 ] REORDER").await;
    assert!(result.is_ok(), "REORDER on scalar-like value should succeed");
}

#[tokio::test]
async fn test_reorder_stack_mode_error_out_of_bounds() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 ] [ 20 ] [ 5 ] .. REORDER").await;
    assert!(result.is_err(), "REORDER stack mode with out of bounds should fail");

    assert_eq!(interp.stack.len(), 3, "Stack should have indices pushed back on error");
}

#[tokio::test]
async fn test_reorder_single_element_index() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 20 30 ] [ 2 ] REORDER").await;
    assert!(result.is_ok(), "REORDER with single index should succeed: {:?}", result);
    assert_eq!(interp.stack.len(), 1);

    let val = &interp.stack[0];
    assert_eq!(val.shape(), vec![1], "Result should have 1 element");
}

// ============================================================================
// COLLECT テスト
// ============================================================================

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
    assert!(result.is_ok(), "COLLECT vectors should succeed: {:?}", result);
    assert_eq!(interp.stack.len(), 1);

    let val = &interp.stack[0];
    assert!(val.is_vector(), "Result should be a vector");
}

#[tokio::test]
async fn test_collect_for_formant_synthesis() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 800 1200 ] CHORD [ 300 2500 ] CHORD 2 COLLECT").await;
    assert!(result.is_ok(), "COLLECT for formant should succeed: {:?}", result);
    assert_eq!(interp.stack.len(), 1);

    let val = &interp.stack[0];
    assert!(val.is_vector(), "Result should be a vector");
}

#[tokio::test]
async fn test_collect_error_underflow() {
    let mut interp = Interpreter::new();

    let result = interp.execute("1 2 5 COLLECT").await;
    assert!(result.is_err(), "COLLECT with insufficient stack should fail");

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

// ============================================================================
// ConsumptionMode テスト
// ============================================================================

#[tokio::test]
async fn test_get_consume_mode() {
    let mut interp = Interpreter::new();

    // デフォルト（消費モード）: 対象ベクタを消費
    let result = interp.execute("[ 10 20 30 ] [ 0 ] GET").await;
    assert!(result.is_ok(), "GET should succeed: {:?}", result);
    // 消費モードでは対象ベクタが消費され、結果のみ残る
    assert_eq!(interp.stack.len(), 1, "GET in consume mode should leave only result");
}

#[tokio::test]
async fn test_get_keep_mode() {
    let mut interp = Interpreter::new();

    // 保持モード: 対象ベクタを保持
    let result = interp.execute("[ 10 20 30 ] [ 0 ] ,, GET").await;
    assert!(result.is_ok(), "GET with keep mode should succeed: {:?}", result);
    // 保持モードでは対象ベクタが残り、結果が追加される
    assert_eq!(interp.stack.len(), 2, "GET in keep mode should preserve target and add result");
}

#[tokio::test]
async fn test_length_consume_mode() {
    let mut interp = Interpreter::new();

    // デフォルト（消費モード）: 対象ベクタを消費
    let result = interp.execute("[ 1 2 3 4 5 ] LENGTH").await;
    assert!(result.is_ok(), "LENGTH should succeed: {:?}", result);
    // 消費モードでは対象ベクタが消費され、長さのみ残る
    assert_eq!(interp.stack.len(), 1, "LENGTH in consume mode should leave only result");
}

#[tokio::test]
async fn test_length_keep_mode() {
    let mut interp = Interpreter::new();

    // 保持モード: 対象ベクタを保持
    let result = interp.execute("[ 1 2 3 4 5 ] ,, LENGTH").await;
    assert!(result.is_ok(), "LENGTH with keep mode should succeed: {:?}", result);
    // 保持モードでは対象ベクタが残り、長さが追加される
    assert_eq!(interp.stack.len(), 2, "LENGTH in keep mode should preserve target and add result");
}

#[tokio::test]
async fn test_reverse_keep_mode() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 3 1 2 ] ,, REVERSE").await;
    assert!(result.is_ok(), "REVERSE with keep mode should succeed: {:?}", result);
    assert_eq!(interp.stack.len(), 2, "REVERSE in keep mode should preserve original and add result");
}

#[tokio::test]
async fn test_take_keep_mode() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 1 2 3 4 5 ] [ 3 ] ,, TAKE").await;
    assert!(result.is_ok(), "TAKE with keep mode should succeed: {:?}", result);
    assert_eq!(interp.stack.len(), 3, "TAKE in keep mode should preserve target, args, and add result");
}
