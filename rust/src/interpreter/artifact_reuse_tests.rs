//! Phase 5 session/artifact lifetime tests.
//!
//! These guard the first split between per-session runtime state and reusable
//! artifacts. The public full reset still clears dictionaries and artifacts;
//! `execute_session_reset` intentionally keeps definition-derived artifacts so
//! a same-worker GUI replay can reuse them safely.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn session_reset_preserves_compiled_plan_for_unchanged_word() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 2 ] * } 'DBL' DEF").await.unwrap();

        interp.execute("[ 3 ] DBL").await.unwrap();
        assert_eq!(interp.runtime_metrics().compiled_plan_build_count, 1);
        assert_eq!(interp.runtime_metrics().compiled_plan_cache_miss_count, 1);

        interp.execute_session_reset().unwrap();
        assert!(interp.get_stack().is_empty());

        interp.execute("[ 4 ] DBL").await.unwrap();
        let metrics = interp.runtime_metrics();
        assert_eq!(
            metrics.compiled_plan_cache_miss_count, 0,
            "session reset must not discard the cached plan for unchanged definitions"
        );
        assert_eq!(
            metrics.compiled_plan_cache_hit_count, 1,
            "the first post-reset call should reuse the pre-reset plan"
        );
    }

    #[tokio::test]
    async fn full_reset_still_clears_definitions_and_artifacts() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 2 ] * } 'DBL' DEF").await.unwrap();
        interp.execute("[ 3 ] DBL").await.unwrap();

        interp.execute_reset().unwrap();
        let err = interp.execute("[ 4 ] DBL").await.unwrap_err();
        assert!(
            err.to_string().contains("Unknown word: DBL"),
            "full reset must preserve the legacy reinitialize semantics"
        );
    }
}
