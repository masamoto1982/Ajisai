//! Phase 5 regression tests: cross-reset compiled-artifact reuse.
//!
//! These pin the behaviours the design memo (§12.6) requires:
//! - an unchanged word's compiled plan survives a *session* reset unrebuilt,
//! - reuse is content-identity keyed (name-independent, dependency-sensitive),
//! - a redefinition never reuses the stale plan,
//! - compile flags are part of the artifact key,
//! - the store is bounded and evicts,
//! - disabling reuse is observationally transparent.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    /// Rendered stack contents, used to assert result equality regardless of how
    /// the plan was sourced (rebuilt vs reused).
    fn rendered_stack(interp: &Interpreter) -> Vec<String> {
        interp.get_stack().iter().map(|v| v.to_string()).collect()
    }

    #[tokio::test]
    async fn compiled_plan_survives_session_reset() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 2 ] * } 'DBL' DEF").await.unwrap();
        interp.execute("[ 5 ] DBL").await.unwrap();

        let after_first = interp.runtime_metrics();
        assert!(
            after_first.compiled_plan_build_count >= 1,
            "first run must build the compiled plan"
        );
        // The artifact-store counters are the durable cross-reset signal; a
        // session reset zeroes the per-session `runtime_metrics` but keeps the
        // store (and its cumulative build/hit/miss/eviction totals) alive.
        let hits_before = after_first.artifact_cache_hit_count;
        assert_eq!(hits_before, 0, "nothing to reuse on the first run");
        let result_before = rendered_stack(&interp);

        // Session reset keeps the artifact cache alive; redefine the identical
        // word and run it again, exactly as the GUI worker does per execution.
        interp.execute_session_reset().unwrap();
        interp.execute("{ [ 2 ] * } 'DBL' DEF").await.unwrap();
        interp.execute("[ 5 ] DBL").await.unwrap();

        let after_second = interp.runtime_metrics();
        assert_eq!(
            after_second.compiled_plan_build_count, 0,
            "the unchanged word's plan must be reused, so this session builds nothing"
        );
        assert!(
            after_second.artifact_cache_hit_count > hits_before,
            "the compiled plan must be served from the cross-reset store"
        );
        assert_eq!(
            rendered_stack(&interp),
            result_before,
            "reuse must not change the result"
        );
    }

    #[tokio::test]
    async fn full_reset_drops_artifact_cache() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 2 ] * } 'DBL' DEF").await.unwrap();
        interp.execute("[ 5 ] DBL").await.unwrap();
        let hits_before = interp.runtime_metrics().artifact_cache_hit_count;

        // A full reset clears the store, so the same word must rebuild.
        interp.execute_reset().unwrap();
        assert_eq!(
            interp.artifact_store_len(),
            0,
            "full reset empties the store"
        );
        interp.execute("{ [ 2 ] * } 'DBL' DEF").await.unwrap();
        interp.execute("[ 5 ] DBL").await.unwrap();

        let m = interp.runtime_metrics();
        assert!(
            m.compiled_plan_build_count >= 1,
            "full reset must force a recompile"
        );
        assert_eq!(
            m.artifact_cache_hit_count, hits_before,
            "a cleared store yields no reuse"
        );
        assert_eq!(
            interp.artifact_store_len(),
            1,
            "the rebuilt plan is re-inserted into the store"
        );
    }

    #[tokio::test]
    async fn same_content_different_name_reuses() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 2 ] * } 'DBL' DEF").await.unwrap();
        interp.execute("[ 5 ] DBL").await.unwrap();
        let builds_before = interp.runtime_metrics().compiled_plan_build_count;
        let hits_before = interp.runtime_metrics().artifact_cache_hit_count;

        // A different name with byte-identical content shares one §8.6 identity,
        // so its plan is reused without a rebuild.
        interp.execute("{ [ 2 ] * } 'TWICE' DEF").await.unwrap();
        interp.execute("[ 5 ] TWICE").await.unwrap();

        let m = interp.runtime_metrics();
        assert_eq!(
            m.compiled_plan_build_count, builds_before,
            "identical content must not rebuild under a new name"
        );
        assert!(
            m.artifact_cache_hit_count > hits_before,
            "the identically-bodied word must hit the store"
        );
    }

    #[tokio::test]
    async fn changed_dependency_does_not_reuse() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 1 ] + } 'H' DEF").await.unwrap();
        interp.execute("{ [ 5 ] H } 'W' DEF").await.unwrap();
        interp.execute("W").await.unwrap();
        let builds_before = interp.runtime_metrics().compiled_plan_build_count;
        let result_first = rendered_stack(&interp);
        interp.stack.clear();

        // Redefine the dependency H (forced, since W references it). W's body
        // text is unchanged, but its content identity folds in H's identity, so
        // the artifact key changes and the stale plan is not reused.
        interp.execute("! { [ 100 ] + } 'H' DEF").await.unwrap();
        interp.execute("W").await.unwrap();

        let m = interp.runtime_metrics();
        assert!(
            m.compiled_plan_build_count > builds_before,
            "a changed dependency must invalidate reuse and rebuild"
        );
        assert_ne!(
            rendered_stack(&interp),
            result_first,
            "the rebuilt plan must reflect the new dependency"
        );
    }

    #[tokio::test]
    async fn redefinition_does_not_reuse_stale_plan() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 1 ] + } 'INC' DEF").await.unwrap();
        interp.execute("[ 10 ] INC").await.unwrap();
        let builds_before = interp.runtime_metrics().compiled_plan_build_count;
        interp.stack.clear();

        interp.execute("{ [ 2 ] + } 'INC' DEF").await.unwrap();
        interp.execute("[ 10 ] INC").await.unwrap();

        assert!(
            interp.runtime_metrics().compiled_plan_build_count > builds_before,
            "a redefinition with new content must rebuild"
        );
        assert_eq!(
            rendered_stack(&interp),
            vec!["[ 12/1 ]".to_string()],
            "the new definition's result, not the stale plan's"
        );
    }

    #[tokio::test]
    async fn compile_flags_are_part_of_the_key() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 2 ] * } 'DBL' DEF").await.unwrap();
        interp.execute("[ 5 ] DBL").await.unwrap();
        let hits_before = interp.runtime_metrics().artifact_cache_hit_count;

        // Flip a compile flag, then re-derive the plan after a session reset. The
        // key now differs, so the previously stored plan is not reused.
        interp.set_compiled_clause_enabled(false);
        interp.execute_session_reset().unwrap();
        interp.execute("{ [ 2 ] * } 'DBL' DEF").await.unwrap();
        interp.execute("[ 5 ] DBL").await.unwrap();

        let m = interp.runtime_metrics();
        assert!(
            m.compiled_plan_build_count >= 1,
            "a different compile flag must rebuild rather than reuse the old-flag plan"
        );
        assert_eq!(
            m.artifact_cache_hit_count, hits_before,
            "the plan lowered under the old flag must not be reused"
        );
    }

    #[tokio::test]
    async fn store_is_bounded_and_evicts() {
        let mut interp = Interpreter::new();
        interp.set_artifact_store_capacity(2);

        for (idx, body) in ["[ 1 ] +", "[ 2 ] +", "[ 3 ] +"].iter().enumerate() {
            let name = format!("WORD{}", idx);
            interp
                .execute(&format!("{{ {} }} '{}' DEF", body, name))
                .await
                .unwrap();
            interp.execute(&format!("[ 0 ] {}", name)).await.unwrap();
            interp.stack.clear();
        }

        assert!(
            interp.artifact_store_len() <= 2,
            "the store must stay within its capacity, got {}",
            interp.artifact_store_len()
        );
        assert!(
            interp.runtime_metrics().artifact_cache_eviction_count >= 1,
            "exceeding capacity must evict"
        );
    }

    #[tokio::test]
    async fn disabling_reuse_is_transparent() {
        let program = &["{ [ 2 ] * } 'DBL' DEF", "[ 5 ] DBL"];

        async fn run(reuse: bool, program: &[&str]) -> Vec<String> {
            let mut interp = Interpreter::new();
            interp.set_artifact_reuse_enabled(reuse);
            for line in program {
                interp.execute(line).await.unwrap();
            }
            // A session reset + rerun exercises the reuse path (or its absence).
            interp.execute_session_reset().unwrap();
            for line in program {
                interp.execute(line).await.unwrap();
            }
            interp.get_stack().iter().map(|v| v.to_string()).collect()
        }

        let with_reuse = run(true, program).await;
        let without_reuse = run(false, program).await;
        assert_eq!(
            with_reuse, without_reuse,
            "artifact reuse must not change observable results"
        );

        // And with reuse off, nothing is ever served from the store.
        let mut interp = Interpreter::new();
        interp.set_artifact_reuse_enabled(false);
        for line in program {
            interp.execute(line).await.unwrap();
        }
        interp.execute_session_reset().unwrap();
        for line in program {
            interp.execute(line).await.unwrap();
        }
        assert_eq!(
            interp.runtime_metrics().artifact_cache_hit_count,
            0,
            "disabled reuse must never hit the store"
        );
    }
}
