//! Behavioral coverage for the ExactScalar path of `op_div` (SPEC §7.4.1).
//!
//! Regression guard for the ordering bug where the generic broadcast block
//! (`apply_binary_broadcast_with_metrics`) ran *before* the ExactScalar
//! block and unconditionally `return`ed on the `FlatTensor::from_value`
//! error for exact irrationals, making the ExactScalar `DIV` path dead code.
//! `op_add`/`op_sub`/`op_mul` place the ExactScalar block before broadcast;
//! these tests pin `op_div` to the same, correct ordering.

use crate::error::NilReason;
use crate::interpreter::Interpreter;
use crate::types::{Value, ValueData};

async fn run_ok(code: &str) -> Vec<Value> {
    let mut interp = Interpreter::new();
    interp
        .execute(code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` unexpectedly errored: {e}"));
    interp.get_stack().to_vec()
}

/// `√2 2 /` is evaluated as an exact value rather than hard-erroring on the
/// (impossible) ExactScalar -> tensor conversion. This is the core P0-a
/// guarantee: the ExactScalar block is now reachable before broadcast.
#[tokio::test]
async fn sqrt2_div_rational_is_exact_not_error() {
    let stack = run_ok("'math' IMPORT 2 MATH@SQRT 2 /").await;
    assert_eq!(stack.len(), 1);
    let top = &stack[0];
    assert!(
        !top.is_nil(),
        "√2 2 / must not collapse to NIL/error, got {top:?}"
    );
    assert!(
        matches!(top.data, ValueData::ExactScalar(_) | ValueData::Scalar(_)),
        "√2 2 / must stay an exact real, got {top:?}"
    );
}

/// `√2 √2 /` is evaluated exactly (a recoverable exact real), not a hard
/// error. NOTE: the current ExactReal engine represents this as a *lazy*
/// `Gosper` (x/y) and does not structurally reduce it to the rational `1/1`;
/// structural simplification is a CF-engine concern outside the P0-a
/// ordering fix. The reachable-path guarantee (exact, not error) is what
/// this test pins.
#[tokio::test]
async fn sqrt2_div_sqrt2_is_exact() {
    let stack = run_ok("'math' IMPORT 2 MATH@SQRT 2 MATH@SQRT /").await;
    assert_eq!(stack.len(), 1);
    let top = &stack[0];
    assert!(
        !top.is_nil(),
        "√2 √2 / must not collapse to NIL/error, got {top:?}"
    );
    assert!(
        matches!(top.data, ValueData::ExactScalar(_) | ValueData::Scalar(_)),
        "√2 √2 / must stay an exact real, got {top:?}"
    );
}

/// `√2 0 /` is a recoverable DivisionByZero Bubble, not a hard error.
#[tokio::test]
async fn sqrt2_div_zero_is_division_by_zero_bubble() {
    let stack = run_ok("'math' IMPORT 2 MATH@SQRT 0 /").await;
    assert_eq!(stack.len(), 1);
    let top = &stack[0];
    assert!(top.is_nil(), "√2 0 / must be a Bubble/NIL, got {top:?}");
    assert_eq!(
        top.nil_reason().cloned(),
        Some(NilReason::DivisionByZero),
        "√2 0 / must carry NilReason::DivisionByZero, got {top:?}"
    );
}

/// Ordinary rational division is unchanged (no regression): `6 3 /` -> `2`.
#[tokio::test]
async fn rational_division_unchanged() {
    let stack = run_ok("6 3 /").await;
    assert_eq!(stack.len(), 1);
    let frac = stack[0]
        .as_scalar()
        .cloned()
        .unwrap_or_else(|| panic!("6 3 / must be a rational, got {:?}", stack[0]));
    assert_eq!(frac.to_i64(), Some(2), "6 3 / must equal 2");
}
