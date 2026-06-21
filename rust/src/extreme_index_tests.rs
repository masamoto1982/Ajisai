//! Regression tests for extreme integer indices and counts
//! (`compute_take_bounds`, `normalize_index`).
//!
//! `i64::MIN` has no positive i64 counterpart, so the old `(-count) as usize`
//! in `TAKE` panicked the moment a `-9223372036854775808` count reached it.
//! Index normalization additionally narrowed positive indices with a bare
//! `index as usize`, which truncates out-of-range values on 32-bit wasm. Every
//! case below must resolve to a clean error or `NIL` rather than crash or
//! silently alias an in-range slot.

#[cfg(test)]
mod extreme_index_tests {
    use crate::interpreter::Interpreter;

    const I64_MIN: &str = "-9223372036854775808";

    #[tokio::test]
    async fn take_rejects_i64_min_count_without_panicking() {
        let mut interp = Interpreter::new();
        let result = interp.execute(&format!("[ 1 2 3 ] {} TAKE", I64_MIN)).await;
        assert!(result.is_err(), "i64::MIN TAKE count must error, not panic");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exceeds vector length"));
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
}
