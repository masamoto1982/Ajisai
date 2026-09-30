//! Regression tests for generative-word materialization limits
//! (`crate::interpreter::MAX_MATERIALIZED_ELEMENTS`).
//!
//! `RANGE`, `FILL`, and `RESHAPE` each loop internally to build a vector or
//! tensor, so they count as a single execution step and bypass the
//! step-count backstop. Before these guards, hostile sizes drove the process
//! into an OOM abort (an unrecoverable trap inside the WASM playground) or, for
//! shapes whose element-count product overflows `usize`, into a
//! `multiply with overflow` panic.
//!
//! Phase 3 of the structural-memory-safety roadmap turns the *space-budget*
//! miss of the generative words — a well-formed input whose materialized result
//! exceeds the ceiling — into a diagnosable projected NIL (reason
//! `SpaceExhausted`) that a pipeline can recover with a chosen fallback, rather than
//! a channel error. `RESHAPE` (back since the vocabulary-100 work order's
//! Phase 2) projects the same way; its probe lives in `shape_words_tests`.
//!
//! Two neighbouring probes live here as well: the extreme-index regressions
//! (`i64::MIN` counts and indices past `u32`), and the shared stack rendering
//! every observation surface goes through.

use crate::error::NilReason;
use crate::interpreter::Interpreter;
use crate::test_support::top_nil_reason;
use crate::types::display::render_stack;

#[tokio::test]
async fn range_projects_unbounded_count_onto_a_space_ceiling() {
    let mut interp = Interpreter::new();
    let result = interp.execute("0 9999999999999 RANGE").await;
    assert!(
        result.is_ok(),
        "an over-budget RANGE must project onto NIL, not error: {result:?}"
    );
    assert_eq!(
        top_nil_reason(&interp),
        Some(NilReason::SpaceExhausted),
        "RANGE over the space ceiling must leave a SpaceExhausted NIL"
    );
}

#[tokio::test]
async fn range_space_projection_is_recoverable() {
    // The whole point of a projected NIL over an error: a pipeline can
    // recover it.
    let mut interp = Interpreter::new();
    let result = interp
        .execute("0 9999999999999 RANGE 'S' BIND [ 42 ] S S NIL? SELECT")
        .await;
    assert!(
        result.is_ok(),
        "the space-exhausted NIL must be recoverable: {result:?}"
    );
    assert_eq!(
        top_nil_reason(&interp),
        None,
        "the fallback value, not a NIL, is on top"
    );
}

#[tokio::test]
async fn range_accepts_ordinary_size() {
    let mut interp = Interpreter::new();
    let result = interp.execute("0 5 RANGE").await;
    assert!(result.is_ok(), "small RANGE should still succeed");
    assert_eq!(
        top_nil_reason(&interp),
        None,
        "a small RANGE does not project onto NIL"
    );
}

#[tokio::test]
async fn range_handles_extreme_bounds_without_overflow() {
    // start/end at the i64 extremes: the span arithmetic must not overflow
    // while computing the over-budget element count, and the result
    // projects onto NIL.
    let mut interp = Interpreter::new();
    let program = format!("{} {} RANGE", i64::MIN, i64::MAX);
    let result = interp.execute(&program).await;
    assert!(
        result.is_ok(),
        "full-i64-span RANGE must project onto NIL, not panic: {result:?}"
    );
    assert_eq!(top_nil_reason(&interp), Some(NilReason::SpaceExhausted));
}

#[tokio::test]
async fn range_infinite_direction_is_still_an_error() {
    // A malformed range (a bound that is not an integer) is not a budget
    // miss; it remains an ordinary channel error.
    let mut interp = Interpreter::new();
    let result = interp.execute("0 1/2 RANGE").await;
    assert!(
        result.is_err(),
        "a non-integer RANGE bound stays a malformed-use error"
    );
}

#[tokio::test]
async fn fill_projects_oversized_product_onto_a_space_ceiling() {
    let mut interp = Interpreter::new();
    let result = interp.execute("[ 1000000 1000000 ] 7 FILL").await;
    assert!(
        result.is_ok(),
        "a billion-element FILL must project onto NIL, not error: {result:?}"
    );
    assert_eq!(top_nil_reason(&interp), Some(NilReason::SpaceExhausted));
}

#[tokio::test]
async fn fill_projects_shape_product_overflow_onto_a_space_ceiling() {
    // The product of these dimensions overflows usize; the old
    // `shape.iter().product()` panicked here, then it errored, now it
    // projects onto NIL.
    let mut interp = Interpreter::new();
    let result = interp
        .execute("[ 99999999 99999999 99999999 ] 1 FILL")
        .await;
    assert!(
        result.is_ok(),
        "an overflowing FILL shape must project onto NIL, not panic: {result:?}"
    );
    assert_eq!(top_nil_reason(&interp), Some(NilReason::SpaceExhausted));
}

#[tokio::test]
async fn fill_accepts_ordinary_shape() {
    let mut interp = Interpreter::new();
    let result = interp.execute("[ 2 2 ] 7 FILL").await;
    assert!(result.is_ok(), "small FILL should still succeed");
    assert_eq!(
        top_nil_reason(&interp),
        None,
        "a small FILL does not project onto NIL"
    );
}

// Regression tests for extreme integer indices and counts
// (`compute_take_bounds`, `normalize_index`).
//
// `i64::MIN` has no positive i64 counterpart, so the old `(-count) as usize`
// in `TAKE` panicked the moment a `-9223372036854775808` count reached it.
// Index normalization additionally narrowed positive indices with a bare
// `index as usize`, which truncates out-of-range values on 32-bit wasm. Every
// case below must resolve to a clean error or `NIL` rather than crash or
// silently alias an in-range slot. A well-formed count or index that simply
// names a position past the end is `NIL`, whatever its magnitude: `TAKE`
// projects it the way `GET` always has (LANG.FAILURE.PROJECT).
const I64_MIN: &str = "-9223372036854775808";

#[tokio::test]
async fn take_projects_i64_min_count_without_panicking() {
    let mut interp = Interpreter::new();
    interp
        .execute(&format!("[ 1 2 3 ] {} TAKE", I64_MIN))
        .await
        .expect("i64::MIN TAKE count must project to NIL, not panic or error");
    let stack = interp.get_stack();
    assert_eq!(stack.len(), 1);
    assert!(stack[0].is_nil(), "a count past the end projects to NIL");
}

#[tokio::test]
async fn take_still_handles_ordinary_negative_count() {
    let mut interp = Interpreter::new();
    interp
        .execute("[ 1 2 3 ] -2 TAKE")
        .await
        .expect("negative TAKE within range should succeed");
    // Tail two elements remain.
    assert_eq!(interp.get_stack().len(), 1);
}

#[tokio::test]
async fn get_with_i64_min_index_yields_nil_not_panic() {
    let mut interp = Interpreter::new();
    let result = interp.execute(&format!("[ 1 2 3 ] {} GET", I64_MIN)).await;
    assert!(
        result.is_ok(),
        "out-of-range GET should produce NIL, not error"
    );
}

#[tokio::test]
async fn get_with_huge_positive_index_is_out_of_bounds() {
    // 2^40 is far beyond the vector but also exceeds u32: on 32-bit wasm a
    // truncating `as usize` could have aliased a valid index. It must read
    // as out-of-bounds (NIL) on every target.
    let mut interp = Interpreter::new();
    interp
        .execute("[ 1 2 3 ] 1099511627776 GET")
        .await
        .expect("huge index GET should resolve to NIL without error");
    let stack = interp.get_stack();
    assert!(
        stack.last().map(|v| v.is_nil()).unwrap_or(false),
        "huge out-of-range index must yield NIL"
    );
}

// CS3 (observation): the shared stack rendering.
//
// Every observation surface (CLI stack display, REPL, in-process conformance
// runner, JSON report) renders through one function —
// `crate::types::display::render_stack` — and it renders each slot from its
// value alone (LANG.VALUES.DENOTATION).
async fn render(code: &str) -> Vec<String> {
    let mut interp = Interpreter::new();
    interp
        .execute(code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` unexpectedly errored: {e}"));
    render_stack(interp.get_stack())
}
#[tokio::test]
async fn arithmetic_result_renders_as_a_number() {
    assert_eq!(render("1 2 ADD").await, vec!["3/1".to_string()]);
}

#[tokio::test]
async fn truth_and_absence_render_canonically() {
    assert_eq!(render("TRUE").await, vec!["TRUE".to_string()]);
    assert_eq!(render("FALSE").await, vec!["FALSE".to_string()]);
    assert_eq!(render("NIL").await, vec!["NIL".to_string()]);
}

/// The same value renders the same way however it was produced: a Boolean
/// from a comparison and one from a `FOLD` of `AND`, a NIL from a literal and
/// one that passed through `MUL`.
#[tokio::test]
async fn rendering_does_not_depend_on_the_producing_word() {
    assert_eq!(
        render("3 2 GT").await,
        render("[ 3 4 ] [ 2 GT ] MAP TRUE [ AND ] FOLD").await
    );
    assert_eq!(render("NIL -1 MUL").await, render("NIL").await);
}
