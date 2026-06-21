//! Regression tests for generative-word materialization limits
//! (`crate::interpreter::MAX_MATERIALIZED_ELEMENTS`).
//!
//! `RANGE`, `FILL`, and `RESHAPE` each loop internally to build a vector or
//! tensor, so they count as a single execution step and bypass the
//! step-count backstop. Before these guards, hostile sizes drove the process
//! into an OOM abort (an unrecoverable trap inside the WASM playground) or, for
//! shapes whose element-count product overflows `usize`, into a
//! `multiply with overflow` panic. Every case below must surface a recoverable
//! `AjisaiError` instead, while ordinary sizes keep working.

#[cfg(test)]
mod materialization_limit_tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn range_rejects_unbounded_element_count() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 0 9999999999999 ] RANGE").await;
        assert!(result.is_err(), "an astronomically large RANGE must error");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("exceeding the limit"),
            "RANGE should report a materialization-limit error"
        );
    }

    #[tokio::test]
    async fn range_accepts_ordinary_size() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 0 5 ] RANGE").await;
        assert!(result.is_ok(), "small RANGE should still succeed");
    }

    #[tokio::test]
    async fn range_handles_extreme_bounds_without_overflow() {
        // start/end at the i64 extremes: the span arithmetic must not overflow
        // while computing the rejected element count.
        let mut interp = Interpreter::new();
        let program = format!("[ {} {} ] RANGE", i64::MIN, i64::MAX);
        let result = interp.execute(&program).await;
        assert!(result.is_err(), "full-i64-span RANGE must error, not panic");
    }

    #[tokio::test]
    async fn fill_rejects_oversized_product() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1000000 1000000 7 ] FILL").await;
        assert!(result.is_err(), "a billion-element FILL must error");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("too many elements"));
    }

    #[tokio::test]
    async fn fill_rejects_shape_product_overflow_without_panicking() {
        // The product of these dimensions overflows usize; the old
        // `shape.iter().product()` panicked here.
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 99999999 99999999 99999999 1 ] FILL")
            .await;
        assert!(
            result.is_err(),
            "overflowing FILL shape must error, not panic"
        );
    }

    #[tokio::test]
    async fn fill_accepts_ordinary_shape() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 2 2 7 ] FILL").await;
        assert!(result.is_ok(), "small FILL should still succeed");
    }

    #[tokio::test]
    async fn reshape_rejects_shape_product_overflow_without_panicking() {
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
