// Integration tests mirroring js/gui/gui-interpreter-test-cases.ts
// These tests verify the interpreter produces the same results as expected by the GUI tests.

use ajisai_core::interpreter::Interpreter;
use ajisai_core::types::fraction::Fraction;
use ajisai_core::types::Value;
use num_bigint::BigInt;

// Helper to run code and return the stack (gui_mode = true to match WASM API behavior)
async fn run(code: &str) -> Result<Vec<Value>, String> {
    let mut interp = Interpreter::new();
    interp.gui_mode = true;
    interp.execute(code).await.map_err(|e| e.to_string())?;
    Ok(interp.get_stack().clone())
}

// Helper to run code expecting an error
async fn run_expect_error(code: &str) -> bool {
    let mut interp = Interpreter::new();
    interp.gui_mode = true;
    interp.execute(code).await.is_err()
}

// Helper: check a scalar value on the stack
fn assert_number(val: &Value, num: i64, denom: i64) {
    let frac = val
        .as_scalar()
        .unwrap_or_else(|| panic!("Expected scalar, got {:?}", val));
    let expected = Fraction::new(BigInt::from(num), BigInt::from(denom));
    assert_eq!(
        frac, &expected,
        "Expected {}/{}, got {:?}",
        num, denom, frac
    );
}

fn assert_bool_val(val: &Value, expected: bool) {
    // TODO: DisplayHint check will use SemanticRegistry
    let frac = val.as_scalar().unwrap();
    if expected {
        assert!(!frac.is_zero(), "Expected TRUE but got FALSE");
    } else {
        assert!(frac.is_zero(), "Expected FALSE but got TRUE");
    }
}

fn assert_string_val(val: &Value, expected: &str) {
    // TODO: DisplayHint check will use SemanticRegistry
    // String is stored as vector of char codes
    if expected.is_empty() {
        assert!(val.is_nil(), "Expected NIL for empty string");
        return;
    }
    let vec = val
        .as_vector()
        .unwrap_or_else(|| panic!("Expected vector for string, got {:?}", val));
    let chars: String = vec
        .iter()
        .map(|v| {
            let code = v.as_i64().unwrap() as u32;
            char::from_u32(code).unwrap()
        })
        .collect();
    assert_eq!(chars, expected, "String mismatch");
}

fn assert_nil(val: &Value) {
    assert!(val.is_nil(), "Expected NIL, got {:?}", val);
}

// Check a vector element by element
fn assert_vector_numbers(val: &Value, nums: &[(i64, i64)]) {
    assert!(val.is_vector(), "Expected vector, got {:?}", val);
    let vec = val.as_vector().unwrap();
    assert_eq!(
        vec.len(),
        nums.len(),
        "Vector length mismatch: expected {}, got {}",
        nums.len(),
        vec.len()
    );
    for (i, (num, denom)) in nums.iter().enumerate() {
        assert_number(&vec[i], *num, *denom);
    }
}

// ============================================
// Basic Types
// ============================================

#[tokio::test]
async fn test_number_integer() {
    let stack = run("[ 42 ]").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(42, 1)]);
}

#[tokio::test]
async fn test_number_negative() {
    let stack = run("[ -17 ]").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(-17, 1)]);
}

#[tokio::test]
async fn test_number_fraction() {
    let stack = run("[ 3/4 ]").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_number(&vec[0], 3, 4);
}

#[tokio::test]
async fn test_number_decimal_converts_to_fraction() {
    let stack = run("[ 0.5 ]").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_number(&vec[0], 1, 2);
}

#[tokio::test]
async fn test_string_simple() {
    let stack = run("[ 'hello' ]").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_eq!(vec.len(), 1);
    assert_string_val(&vec[0], "hello");
}

#[tokio::test]
async fn test_string_with_spaces() {
    let stack = run("[ 'hello world' ]").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_eq!(vec.len(), 1);
    assert_string_val(&vec[0], "hello world");
}

#[tokio::test]
async fn test_boolean_true() {
    let stack = run("[ TRUE ]").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_eq!(vec.len(), 1);
    assert_bool_val(&vec[0], true);
}

#[tokio::test]
async fn test_boolean_false() {
    let stack = run("[ FALSE ]").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_eq!(vec.len(), 1);
    assert_bool_val(&vec[0], false);
}

#[tokio::test]
async fn test_nil() {
    let stack = run("[ NIL ]").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_eq!(vec.len(), 1);
    assert_nil(&vec[0]);
}

// ============================================
// Arithmetic
// ============================================

#[tokio::test]
async fn test_addition_integers() {
    let stack = run("[ 2 ] [ 3 ] +").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(5, 1)]);
}

#[tokio::test]
async fn test_addition_fractions() {
    let stack = run("[ 1/2 ] [ 1/3 ] +").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_number(&vec[0], 5, 6);
}

#[tokio::test]
async fn test_subtraction() {
    let stack = run("[ 10 ] [ 3 ] -").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(7, 1)]);
}

#[tokio::test]
async fn test_multiplication() {
    let stack = run("[ 4 ] [ 5 ] *").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(20, 1)]);
}

#[tokio::test]
async fn test_division() {
    let stack = run("[ 10 ] [ 4 ] /").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_number(&vec[0], 5, 2);
}

#[tokio::test]
async fn test_division_by_zero_error() {
    assert!(run_expect_error("[ 1 ] [ 0 ] /").await);
}

#[tokio::test]
async fn test_modulo() {
    let stack = run("[ 7 ] [ 3 ] MOD").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(1, 1)]);
}

#[tokio::test]
async fn test_floor() {
    let stack = run("[ 7/3 ] FLOOR").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(2, 1)]);
}

#[tokio::test]
async fn test_ceil() {
    let stack = run("[ 7/3 ] CEIL").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(3, 1)]);
}

#[tokio::test]
async fn test_round() {
    let stack = run("[ 5/2 ] ROUND").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(3, 1)]);
}

// ============================================
// Comparison
// ============================================

#[tokio::test]
async fn test_less_than_true() {
    let stack = run("[ 3 ] [ 5 ] <").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_bool_val(&vec[0], true);
}

#[tokio::test]
async fn test_less_than_false() {
    let stack = run("[ 5 ] [ 3 ] <").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_bool_val(&vec[0], false);
}

#[tokio::test]
async fn test_greater_than_via_le_not() {
    let stack = run("[ 5 ] [ 3 ] <= NOT").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_bool_val(&vec[0], true);
}

#[tokio::test]
async fn test_less_than_or_equal() {
    let stack = run("[ 3 ] [ 3 ] <=").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_bool_val(&vec[0], true);
}

#[tokio::test]
async fn test_greater_than_or_equal_via_lt_not() {
    let stack = run("[ 3 ] [ 3 ] < NOT").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_bool_val(&vec[0], true);
}

#[tokio::test]
async fn test_equal_numbers() {
    let stack = run("[ 5 ] [ 5 ] =").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_bool_val(&vec[0], true);
}

#[tokio::test]
async fn test_equal_fraction_auto_reduction() {
    let stack = run("[ 1/2 ] [ 2/4 ] =").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_bool_val(&vec[0], true);
}

// ============================================
// Logic
// ============================================

#[tokio::test]
async fn test_and_true_true() {
    let stack = run("[ TRUE ] [ TRUE ] AND").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_bool_val(&vec[0], true);
}

#[tokio::test]
async fn test_and_true_false() {
    let stack = run("[ TRUE ] [ FALSE ] AND").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_bool_val(&vec[0], false);
}

#[tokio::test]
async fn test_or_false_true() {
    let stack = run("[ FALSE ] [ TRUE ] OR").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_bool_val(&vec[0], true);
}

#[tokio::test]
async fn test_not_true() {
    let stack = run("[ TRUE ] NOT").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_bool_val(&vec[0], false);
}

#[tokio::test]
async fn test_not_false() {
    let stack = run("[ FALSE ] NOT").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_bool_val(&vec[0], true);
}

// ============================================
// Vector Operations
// ============================================

#[tokio::test]
async fn test_length() {
    let stack = run("[ 1 2 3 4 5 ] LENGTH").await.unwrap();
    assert_eq!(stack.len(), 2);
    assert_vector_numbers(&stack[0], &[(1, 1), (2, 1), (3, 1), (4, 1), (5, 1)]);
    // LENGTH returns a scalar number
    assert_number(&stack[1], 5, 1);
}

#[tokio::test]
async fn test_get_first_element() {
    let stack = run("[ 10 20 30 ] [ 0 ] GET").await.unwrap();
    assert_eq!(stack.len(), 2);
    assert_vector_numbers(&stack[0], &[(10, 1), (20, 1), (30, 1)]);
    assert_number(&stack[1], 10, 1);
}

#[tokio::test]
async fn test_get_negative_index() {
    let stack = run("[ 10 20 30 ] [ -1 ] GET").await.unwrap();
    assert_eq!(stack.len(), 2);
    assert_vector_numbers(&stack[0], &[(10, 1), (20, 1), (30, 1)]);
    assert_number(&stack[1], 30, 1);
}

#[tokio::test]
async fn test_take_positive() {
    let stack = run("[ 1 2 3 4 5 ] [ 3 ] TAKE").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(1, 1), (2, 1), (3, 1)]);
}

#[tokio::test]
async fn test_take_negative() {
    let stack = run("[ 1 2 3 4 5 ] [ -2 ] TAKE").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(4, 1), (5, 1)]);
}

#[tokio::test]
async fn test_reverse() {
    let stack = run("[ 1 2 3 ] REVERSE").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(3, 1), (2, 1), (1, 1)]);
}

#[tokio::test]
async fn test_concat() {
    let stack = run("[ 1 2 ] [ 3 4 ] CONCAT").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(1, 1), (2, 1), (3, 1), (4, 1)]);
}

#[tokio::test]
async fn test_insert() {
    let stack = run("[ 1 3 ] [ 1 2 ] INSERT").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(1, 1), (2, 1), (3, 1)]);
}

#[tokio::test]
async fn test_replace() {
    let stack = run("[ 1 2 3 ] [ 1 9 ] REPLACE").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(1, 1), (9, 1), (3, 1)]);
}

#[tokio::test]
async fn test_remove() {
    let stack = run("[ 1 2 3 ] [ 1 ] REMOVE").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(1, 1), (3, 1)]);
}

// ============================================
// Tensor Operations
// ============================================

#[tokio::test]
async fn test_shape_1d() {
    let stack = run("[ 1 2 3 ] SHAPE").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(3, 1)]);
}

#[tokio::test]
async fn test_shape_2d() {
    let stack = run("[ [ 1 2 3 ] [ 4 5 6 ] ] SHAPE").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(2, 1), (3, 1)]);
}

#[tokio::test]
async fn test_rank_1d() {
    let stack = run("[ 1 2 3 ] RANK").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_number(&stack[0], 1, 1);
}

#[tokio::test]
async fn test_rank_2d() {
    let stack = run("[ [ 1 2 ] [ 3 4 ] ] RANK").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_number(&stack[0], 2, 1);
}

#[tokio::test]
async fn test_transpose() {
    let stack = run("[ [ 1 2 3 ] [ 4 5 6 ] ] TRANSPOSE").await.unwrap();
    assert_eq!(stack.len(), 1);
    let outer = stack[0].as_vector().unwrap();
    assert_eq!(outer.len(), 3);
    assert_vector_numbers(&outer[0], &[(1, 1), (4, 1)]);
    assert_vector_numbers(&outer[1], &[(2, 1), (5, 1)]);
    assert_vector_numbers(&outer[2], &[(3, 1), (6, 1)]);
}

#[tokio::test]
async fn test_reshape() {
    let stack = run("[ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE").await.unwrap();
    assert_eq!(stack.len(), 1);
    let outer = stack[0].as_vector().unwrap();
    assert_eq!(outer.len(), 2);
    assert_vector_numbers(&outer[0], &[(1, 1), (2, 1), (3, 1)]);
    assert_vector_numbers(&outer[1], &[(4, 1), (5, 1), (6, 1)]);
}

// ============================================
// Broadcasting
// ============================================

#[tokio::test]
async fn test_broadcast_scalar_plus_vector() {
    let stack = run("[ 10 ] [ 1 2 3 ] +").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(11, 1), (12, 1), (13, 1)]);
}

#[tokio::test]
async fn test_broadcast_vector_times_scalar() {
    let stack = run("[ 1 2 3 ] [ 2 ] *").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(2, 1), (4, 1), (6, 1)]);
}

#[tokio::test]
async fn test_broadcast_vector_plus_vector() {
    let stack = run("[ 1 2 3 ] [ 10 20 30 ] +").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(11, 1), (22, 1), (33, 1)]);
}

#[tokio::test]
async fn test_broadcast_matrix_plus_row_vector() {
    let stack = run("[ [ 1 2 3 ] [ 4 5 6 ] ] [ 10 20 30 ] +").await.unwrap();
    assert_eq!(stack.len(), 1);
    let outer = stack[0].as_vector().unwrap();
    assert_eq!(outer.len(), 2);
    assert_vector_numbers(&outer[0], &[(11, 1), (22, 1), (33, 1)]);
    assert_vector_numbers(&outer[1], &[(14, 1), (25, 1), (36, 1)]);
}

// ============================================
// Higher-Order Functions
// ============================================

#[tokio::test]
async fn test_map_double() {
    let stack = run("{ [ 2 ] * } 'DBL' DEF\n[ 1 2 3 ] 'DBL' MAP")
        .await
        .unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(2, 1), (4, 1), (6, 1)]);
}

#[tokio::test]
async fn test_filter_positive() {
    let stack = run("{ [ 0 ] <= NOT } 'POS' DEF\n[ -2 -1 0 1 2 ] 'POS' FILTER")
        .await
        .unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(1, 1), (2, 1)]);
}

#[tokio::test]
async fn test_fold_sum() {
    let stack = run("[ 1 2 3 4 ] [ 0 ] '+' FOLD").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(10, 1)]);
}

// ============================================
// Type Conversion
// ============================================

#[tokio::test]
async fn test_str_number_to_string() {
    // Use scalar 42 (not vector [42], which is now heuristically a string '*')
    let stack = run("42 STR").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_string_val(&stack[0], "42");
}

#[tokio::test]
async fn test_str_fraction_to_string() {
    let stack = run("[ 3/4 ] STR").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_string_val(&stack[0], "3/4");
}

#[tokio::test]
async fn test_num_string_to_number() {
    let stack = run("'42' NUM").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_number(&stack[0], 42, 1);
}

#[tokio::test]
async fn test_bool_1_to_true() {
    let stack = run("1 BOOL").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_bool_val(&stack[0], true);
}

#[tokio::test]
async fn test_bool_0_to_false() {
    let stack = run("0 BOOL").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_bool_val(&stack[0], false);
}

// ============================================
// String Operations
// ============================================

#[tokio::test]
async fn test_chars_split_string() {
    let stack = run("'hello' CHARS").await.unwrap();
    assert_eq!(stack.len(), 1);
    let vec = stack[0].as_vector().unwrap();
    assert_eq!(vec.len(), 5);
    assert_string_val(&vec[0], "h");
    assert_string_val(&vec[1], "e");
    assert_string_val(&vec[2], "l");
    assert_string_val(&vec[3], "l");
    assert_string_val(&vec[4], "o");
}

#[tokio::test]
async fn test_join_strings() {
    let stack = run("[ 'h' 'e' 'l' 'l' 'o' ] JOIN").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_string_val(&stack[0], "hello");
}

// ============================================
// Stack Mode (..)
// ============================================

#[tokio::test]
async fn test_stack_mode_length() {
    let stack = run("[ 1 ] [ 2 ] [ 3 ] .. LENGTH").await.unwrap();
    assert_eq!(stack.len(), 4);
    assert_vector_numbers(&stack[0], &[(1, 1)]);
    assert_vector_numbers(&stack[1], &[(2, 1)]);
    assert_vector_numbers(&stack[2], &[(3, 1)]);
    assert_number(&stack[3], 3, 1);
}

#[tokio::test]
async fn test_stack_mode_get() {
    let stack = run("[ 'a' ] [ 'b' ] [ 'c' ] [ 1 ] .. GET").await.unwrap();
    assert_eq!(stack.len(), 4);
    // First 3 are original vectors
    let vec0 = stack[0].as_vector().unwrap();
    assert_string_val(&vec0[0], "a");
    let vec1 = stack[1].as_vector().unwrap();
    assert_string_val(&vec1[0], "b");
    let vec2 = stack[2].as_vector().unwrap();
    assert_string_val(&vec2[0], "c");
    // Last is the GET result: stack[1] which is [ 'b' ]
    let result = stack[3].as_vector().unwrap();
    assert_string_val(&result[0], "b");
}

#[tokio::test]
async fn test_stack_mode_reverse() {
    let stack = run("[ 1 ] [ 2 ] [ 3 ] .. REVERSE").await.unwrap();
    assert_eq!(stack.len(), 3);
    assert_vector_numbers(&stack[0], &[(3, 1)]);
    assert_vector_numbers(&stack[1], &[(2, 1)]);
    assert_vector_numbers(&stack[2], &[(1, 1)]);
}

// ============================================
// User Word Definition
// ============================================

#[tokio::test]
async fn test_def_and_call() {
    let stack = run("{ [ 2 ] * } 'DOUBLE' DEF\n[ 5 ] DOUBLE").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(10, 1)]);
}

#[tokio::test]
async fn test_def_with_branch_guard() {
    // [ 3 ] < 10 → TRUE, so first branch runs: 3 * 2 = 6
    // Migrated from legacy `$` branch guard syntax to COND
    let stack = run("{ { ,, [ 10 ] < } { [ 2 ] * } { IDLE } { [ 3 ] * } COND } 'GUARD' DEF\n[ 3 ] GUARD")
        .await
        .unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(6, 1)]);
}

#[tokio::test]
async fn test_del_delete_user_word() {
    assert!(run_expect_error("{ [ 2 ] * } 'TEMP' DEF\n'TEMP' DEL\nTEMP").await);
}

// ============================================
// Control Flow
// ============================================

#[tokio::test]
async fn test_cond_multi_branch() {
    // Test multi-branch COND: [ 10 ] matches second guard (> 5 via reversed <)
    // Migrated from legacy `&` loop guard syntax; loop construct removed from spec.
    // Tests COND with multiple guard/body pairs instead.
    let stack = run("[ 10 ] { ,, [ 0 ] < } { 'negative' } { ,, [ 5 ] < } { 'small' } { IDLE } { 'big' } COND")
        .await
        .unwrap();
    assert_eq!(stack.len(), 1);
    assert_string_val(&stack[0], "big");
}

// ============================================
// Tensor Generation
// ============================================

#[tokio::test]
async fn test_fill() {
    let stack = run("[ 3 7 ] FILL").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(7, 1), (7, 1), (7, 1)]);
}

// ============================================
// NIL Safety
// ============================================

#[tokio::test]
async fn test_nil_coalescing_nil_case() {
    let stack = run("NIL => [ 0 ]").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(0, 1)]);
}

#[tokio::test]
async fn test_nil_coalescing_non_nil_case() {
    let stack = run("[ 42 ] => [ 0 ]").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(42, 1)]);
}

// ============================================
// Error Cases
// ============================================

#[tokio::test]
async fn test_error_stack_underflow() {
    assert!(run_expect_error("+").await);
}

#[tokio::test]
async fn test_error_unknown_word() {
    assert!(run_expect_error("UNKNOWNWORD").await);
}

#[tokio::test]
async fn test_error_index_out_of_bounds() {
    assert!(run_expect_error("[ 1 2 3 ] [ 10 ] GET").await);
}

#[tokio::test]
async fn test_error_incompatible_shapes() {
    assert!(run_expect_error("[ 1 2 3 ] [ 1 2 ] +").await);
}

#[tokio::test]
async fn test_error_empty_vector() {
    assert!(run_expect_error("[ ]").await);
}

#[tokio::test]
async fn test_sort_already_sorted_succeeds() {
    let stack = run("[ 1 2 3 ] SORT").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_vector_numbers(&stack[0], &[(1, 1), (2, 1), (3, 1)]);
}
