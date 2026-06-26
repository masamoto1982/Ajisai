//! Behavioral coverage for element-wise arithmetic over vectors/structures
//! that carry irrational `ExactScalar` lanes (SPEC §7.1 vector ops + §4.2
//! exact-real scalars).
//!
//! Before this path existed, any vector containing an irrational continued
//! fraction fell through to the rational `FlatTensor` broadcast, which
//! hard-errors on `ExactScalar` (`FlatTensor::from_value`). These tests pin
//! that such vectors now compute lane-by-lane as exact reals, that the
//! all-rational route is unchanged, and that the broadcast shape rules and the
//! per-lane division-by-zero Bubble Rule are preserved.

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

fn vector_children(value: &Value) -> &[Value] {
    match &value.data {
        ValueData::Vector(items) => items.as_slice(),
        other => panic!("expected a vector, got {other:?}"),
    }
}

fn is_exact_real_lane(value: &Value) -> bool {
    matches!(value.data, ValueData::ExactScalar(_) | ValueData::Scalar(_))
}

/// Two equal-length vectors of irrationals add lane-by-lane, staying exact and
/// never collapsing to a tensor-conversion error.
#[tokio::test]
async fn irrational_vector_plus_irrational_vector_is_exact() {
    let stack = run_ok("'math' IMPORT [ 2 3 ] { SQRT } MAP [ 2 3 ] { SQRT } MAP +").await;
    assert_eq!(stack.len(), 1);
    let children = vector_children(&stack[0]);
    assert_eq!(children.len(), 2, "result must keep both lanes");
    assert!(
        children.iter().all(|c| !c.is_nil() && is_exact_real_lane(c)),
        "each lane of √n + √n must stay an exact real, got {children:?}"
    );
}

/// A bare irrational scalar broadcasts across a rational vector, producing one
/// exact lane per element instead of erroring.
#[tokio::test]
async fn irrational_scalar_broadcasts_across_rational_vector() {
    let stack = run_ok("'math' IMPORT [ 1 2 3 ] 2 SQRT *").await;
    assert_eq!(stack.len(), 1);
    let children = vector_children(&stack[0]);
    assert_eq!(children.len(), 3, "√2 must broadcast across all three lanes");
    assert!(
        children.iter().all(|c| !c.is_nil() && is_exact_real_lane(c)),
        "each lane of [1 2 3] * √2 must stay an exact real, got {children:?}"
    );
}

/// Unequal-length vectors raise the same `VectorLengthMismatch` the rational
/// broadcast does — exactness does not relax the shape contract.
#[tokio::test]
async fn irrational_vector_length_mismatch_errors() {
    let mut interp = Interpreter::new();
    let result = interp
        .execute("'math' IMPORT [ 2 3 ] { SQRT } MAP [ 2 3 4 ] { SQRT } MAP +")
        .await;
    assert!(
        result.is_err(),
        "mismatched irrational vector lengths must error, got {:?}",
        interp.get_stack()
    );
}

/// A per-lane division by zero becomes a recoverable DivisionByZero Bubble,
/// matching the scalar `√x 0 /` Bubble Rule rather than aborting the vector.
#[tokio::test]
async fn irrational_vector_div_by_zero_lane_is_bubble() {
    let stack = run_ok("'math' IMPORT [ 2 3 ] { SQRT } MAP [ 1 0 ] /").await;
    assert_eq!(stack.len(), 1);
    let children = vector_children(&stack[0]);
    assert_eq!(children.len(), 2);
    assert!(
        is_exact_real_lane(&children[0]) && !children[0].is_nil(),
        "first lane √2 / 1 must stay exact, got {:?}",
        children[0]
    );
    assert!(children[1].is_nil(), "second lane √3 / 0 must be a Bubble");
    assert_eq!(
        children[1].nil_reason().cloned(),
        Some(NilReason::DivisionByZero),
        "div-by-zero lane must carry NilReason::DivisionByZero"
    );
}

/// Regression: an all-rational vector op never touches the exact path and
/// keeps its plain rational result (still the dense-tensor representation, so
/// compare against the literal value rather than assuming a `Vector`).
#[tokio::test]
async fn rational_vector_addition_unchanged() {
    let stack = run_ok("[ 1 2 3 ] [ 10 20 30 ] +").await;
    assert_eq!(stack.len(), 1);
    let expected = run_ok("[ 11 22 33 ]").await;
    assert_eq!(stack[0], expected[0], "rational vector add must be unchanged");
}
