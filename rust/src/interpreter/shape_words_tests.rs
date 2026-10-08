//! Behavioral probes for the shape Words (LANG.COLLECTIONS.LIFT): the two
//! projections `SHAPE` and `RESHAPE` declare.

use crate::interpreter::runtime_limits::RuntimeLimits;
use crate::interpreter::Interpreter;
use crate::test_support::{error_of, reason, top, top_of};

/// A ragged Vector has no shape: the question does not fit the data, which
/// is the trichotomy's middle case, not a malformed program.
#[tokio::test]
async fn a_ragged_vector_projects_domain_miss_from_shape() {
    for code in [
        "[ [ 1 2 ] [ 3 ] ] SHAPE",
        "[ 1 [ 2 ] ] SHAPE",
        "[ [ [ 1 ] ] [ [ 1 2 ] ] ] SHAPE",
    ] {
        assert_eq!(
            reason(code).await.as_deref(),
            Some("domainMiss"),
            "`{code}`"
        );
    }
    for (code, want) in [
        ("[ 1 2 3 ] SHAPE", "[ 3/1 ]"),
        ("[ [ 1 2 ] [ 3 4 ] [ 5 6 ] ] SHAPE", "[ 3/1 2/1 ]"),
        ("[ 'a' 'bc' ] SHAPE", "[ 2/1 ]"),
        ("[ ] SHAPE", "[ 0/1 ]"),
        ("[ [ ] [ ] ] SHAPE", "[ 2/1 0/1 ]"),
    ] {
        assert_eq!(top(code).await, want, "`{code}`");
    }
}

/// A well-formed shape too large to materialize projects the same reason
/// FILL and RANGE project, and never allocates first.
#[tokio::test]
async fn an_over_budget_reshape_projects_space_exhausted() {
    let mut interp = Interpreter::new();
    interp.set_runtime_limits(RuntimeLimits {
        max_materialized_elements: 8,
        ..RuntimeLimits::default()
    });
    interp
        .execute("[ 1 2 3 4 5 6 7 8 9 ] [ 3 3 ] RESHAPE")
        .await
        .expect("an over-budget RESHAPE projects rather than erroring");
    let answer = interp.stack.last().cloned().expect("an answer");
    assert!(answer.is_nil(), "must project NIL, got {answer:?}");
    assert_eq!(
        answer
            .absence_metadata()
            .and_then(|absence| absence.reason.as_ref())
            .map(|reason| reason.as_protocol_str().to_string())
            .as_deref(),
        Some("spaceExhausted")
    );
}

/// A zero-length axis makes the leaf count 0, but the Vectors above it are
/// still built — `[ 9 0 ]` is nine empty Vectors — so they are what the
/// ceiling bounds. Counting leaves alone let `[ 20000000 0 ] 0 FILL` allocate
/// twenty million rows past a one-million ceiling.
#[tokio::test]
async fn a_zero_length_axis_does_not_slip_under_the_ceiling() {
    for (code, over) in [
        ("[ ] [ 9 0 ] RESHAPE", true),
        ("[ 9 0 ] 0 FILL", true),
        ("[ 9 0 ] 'a' FILL", true),
        ("[ 3 3 0 ] 0 FILL", true),
        ("[ 8 0 ] 0 FILL", false),
        ("[ ] [ 2 4 0 ] RESHAPE", false),
        ("[ 0 9 ] 0 FILL", false),
    ] {
        let mut interp = Interpreter::new();
        interp.set_runtime_limits(RuntimeLimits {
            max_materialized_elements: 8,
            ..RuntimeLimits::default()
        });
        interp
            .execute(code)
            .await
            .unwrap_or_else(|e| panic!("`{code}` must not raise, got {e}"));
        let answer = interp.stack.last().cloned().expect("an answer");
        let exhausted = answer
            .absence_metadata()
            .and_then(|absence| absence.reason.as_ref())
            .is_some_and(|reason| reason.as_protocol_str() == "spaceExhausted");
        assert_eq!(exhausted, over, "`{code}` answered {answer:?}");
    }
}

/// A shape whose product is not the leaf count is the program being wrong,
/// so it raises rather than padding or truncating.
#[tokio::test]
async fn a_mismatched_shape_raises_invalid_shape() {
    for code in [
        "[ 1 2 3 4 5 ] [ 2 3 ] RESHAPE",
        "[ 1 2 3 4 ] [ 0 ] RESHAPE",
        "[ 1 2 3 4 ] [ ] RESHAPE",
        "[ 1 2 3 4 ] 'x' RESHAPE",
    ] {
        let mut interp = Interpreter::new();
        let result = interp.execute(code).await;
        let message = result
            .expect_err(&format!("`{code}` must raise"))
            .to_string();
        assert!(
            message.contains("RESHAPE"),
            "`{code}` must name RESHAPE in its diagnosis, got: {message}"
        );
    }
}

/// A shape axis or a `COLLECT` count past `u32::MAX` is well-formed and too
/// large — `spaceExhausted`, `stackUnderflow` — never malformed, and the
/// observed element count is the product of the axes as written. On wasm32,
/// where `usize` is 32 bits, the axis used to be declined as `invalidShape`,
/// the count as `invalidInteger`, and a product past 32 bits reported no
/// observed count at all; the axes are now read as `u64` on every target.
#[tokio::test]
async fn counts_past_32_bits_are_too_large_not_malformed() {
    for code in [
        "[ 4294967296 ] 0 FILL",
        "[ 1 ] [ 4294967297 ] RESHAPE",
        "[ 2 ] 0 FILL [ 4294967296 ] RESHAPE",
        "[ 99999 99999 ] 0 FILL",
    ] {
        assert_eq!(
            reason(code).await.as_deref(),
            Some("spaceExhausted"),
            "`{code}`"
        );
    }
    assert_eq!(error_of("1 2 3 4294967299 COLLECT").await, "stackUnderflow");
    for (shape, observed) in [
        ("[ 4294967296 ]", Some(4_294_967_296u128)),
        ("[ 99999 99999 ]", Some(9_999_800_001)),
        ("[ 2 2147483648 ]", Some(4_294_967_296)),
        ("[ 4294967296 4294967296 4294967296 ]", None),
    ] {
        assert_eq!(
            super::shape_words::shape_observed_size(&top_of(shape).await),
            observed,
            "`{shape}`"
        );
    }
}
