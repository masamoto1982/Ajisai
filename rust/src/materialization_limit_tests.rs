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
//! exceeds the water level — into a diagnosable Bubble/NIL (reason
//! `SpaceExhausted`) that a pipeline can recover with `^` (VENT), rather than a
//! channel error. `RESHAPE`'s over-limit case is a shape *mismatch* (malformed),
//! so it remains an ordinary error.

#[cfg(test)]
mod materialization_limit_tests {
    use crate::error::NilReason;
    use crate::interpreter::Interpreter;

    fn top_nil_reason(interp: &Interpreter) -> Option<NilReason> {
        interp
            .get_stack()
            .last()
            .and_then(|v| v.absence.as_ref())
            .and_then(|a| a.reason.clone())
    }

    #[tokio::test]
    async fn range_projects_unbounded_count_onto_a_space_bubble() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 0 9999999999999 ] RANGE").await;
        assert!(
            result.is_ok(),
            "an over-budget RANGE must bubble, not error: {result:?}"
        );
        assert_eq!(
            top_nil_reason(&interp),
            Some(NilReason::SpaceExhausted),
            "RANGE over the space water level must leave a SpaceExhausted NIL"
        );
    }

    #[tokio::test]
    async fn range_space_bubble_is_vent_recoverable() {
        // The whole point of a bubble over an error: a pipeline can recover it.
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 0 9999999999999 ] RANGE ^ [ 42 ]").await;
        assert!(
            result.is_ok(),
            "VENT must recover the space bubble: {result:?}"
        );
        assert_eq!(
            top_nil_reason(&interp),
            None,
            "after VENT the fallback value, not a NIL, is on top"
        );
    }

    #[tokio::test]
    async fn range_accepts_ordinary_size() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 0 5 ] RANGE").await;
        assert!(result.is_ok(), "small RANGE should still succeed");
        assert_eq!(
            top_nil_reason(&interp),
            None,
            "a small RANGE is not a bubble"
        );
    }

    #[tokio::test]
    async fn range_handles_extreme_bounds_without_overflow() {
        // start/end at the i64 extremes: the span arithmetic must not overflow
        // while computing the over-budget element count, and the result bubbles.
        let mut interp = Interpreter::new();
        let program = format!("[ {} {} ] RANGE", i64::MIN, i64::MAX);
        let result = interp.execute(&program).await;
        assert!(
            result.is_ok(),
            "full-i64-span RANGE must bubble, not panic: {result:?}"
        );
        assert_eq!(top_nil_reason(&interp), Some(NilReason::SpaceExhausted));
    }

    #[tokio::test]
    async fn range_infinite_direction_is_still_an_error() {
        // A malformed range (wrong direction / would never terminate) is not a
        // budget miss; it remains an ordinary channel error.
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 5 0 1 ] RANGE").await;
        assert!(
            result.is_err(),
            "an infinite-direction RANGE stays a malformed-use error"
        );
    }

    #[tokio::test]
    async fn fill_projects_oversized_product_onto_a_space_bubble() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1000000 1000000 7 ] FILL").await;
        assert!(
            result.is_ok(),
            "a billion-element FILL must bubble, not error: {result:?}"
        );
        assert_eq!(top_nil_reason(&interp), Some(NilReason::SpaceExhausted));
    }

    #[tokio::test]
    async fn fill_projects_shape_product_overflow_onto_a_space_bubble() {
        // The product of these dimensions overflows usize; the old
        // `shape.iter().product()` panicked here, then it errored, now it bubbles.
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 99999999 99999999 99999999 1 ] FILL")
            .await;
        assert!(
            result.is_ok(),
            "an overflowing FILL shape must bubble, not panic: {result:?}"
        );
        assert_eq!(top_nil_reason(&interp), Some(NilReason::SpaceExhausted));
    }

    #[tokio::test]
    async fn fill_accepts_ordinary_shape() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 2 2 7 ] FILL").await;
        assert!(result.is_ok(), "small FILL should still succeed");
        assert_eq!(
            top_nil_reason(&interp),
            None,
            "a small FILL is not a bubble"
        );
    }

    #[tokio::test]
    async fn reshape_rejects_shape_product_overflow_without_panicking() {
        // RESHAPE's over-limit case is a shape mismatch (malformed), not a
        // space-budget miss, so it stays an ordinary error.
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 1 2 3 ] [ 99999999 99999999 99999999 ] RESHAPE")
            .await;
        assert!(
            result.is_err(),
            "overflowing RESHAPE shape must error, not panic"
        );
    }

    #[tokio::test]
    async fn reshape_accepts_matching_shape() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE").await;
        assert!(result.is_ok(), "matching RESHAPE should still succeed");
    }
}
