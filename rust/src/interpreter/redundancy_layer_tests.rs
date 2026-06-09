//! Test suite for `crate::interpreter::redundancy_layer`.

#[cfg(test)]
mod tests {
    use crate::interpreter::execute_def::op_def_inner;
    use crate::interpreter::quantized_block::{
        DataMovementClass, VtuBackendCandidate, VtuHint, VtuSuitability,
    };
    use crate::interpreter::redundancy_layer::{
        select_deterministic_route, DeterministicExecutionRoute, RedundancyCheckpoint,
    };
    use crate::interpreter::Interpreter;
    use crate::tokenizer;
    use crate::types::fraction::Fraction;
    use crate::types::Value;

    fn define_word(interp: &mut Interpreter, name: &str, src: &str) {
        let tokens = tokenizer::tokenize(src).expect("tokenize");
        op_def_inner(interp, name, &tokens).expect("define word");
    }

    fn vtu_hint(suitability: VtuSuitability) -> VtuHint {
        VtuHint {
            suitability,
            backend_candidates: vec![VtuBackendCandidate::DenseTensorLoop],
            data_movement: DataMovementClass::Low,
            reason: "test",
        }
    }

    #[test]
    fn resolve_cache_hit_increments_metrics() {
        let mut interp = Interpreter::new();
        define_word(&mut interp, "CACHE-HIT", "[ 42 ]");

        let _ = interp.resolve_word_entry("CACHE-HIT");
        let before = interp.runtime_metrics.resolve_cache_hit_count;
        let _ = interp.resolve_word_entry("CACHE-HIT");

        assert!(interp.runtime_metrics.resolve_cache_hit_count > before);
    }

    #[test]
    fn dictionary_epoch_change_invalidates_resolve_cache() {
        let mut interp = Interpreter::new();
        define_word(&mut interp, "CACHE-INVALIDATE", "[ 1 ]");

        let _ = interp.resolve_word_entry("CACHE-INVALIDATE");
        let before = interp.runtime_metrics.resolve_cache_invalidation_count;
        interp.bump_dictionary_epoch();

        assert!(interp.runtime_metrics.resolve_cache_invalidation_count > before);
    }

    #[tokio::test]
    async fn execution_plan_set_with_compiled_and_quantized_is_stored() {
        let mut interp = Interpreter::new();
        interp
            .execute("{ 1 2 + } 'PLAN-WORD' DEF")
            .await
            .expect("def");
        interp.execute("PLAN-WORD").await.expect("execute");

        let (_, def) = interp
            .resolve_word_entry_readonly("PLAN-WORD")
            .expect("word exists");
        let plan_set = def.execution_plans.as_ref().expect("plan set exists");

        assert!(plan_set.compiled.is_some());
        assert!(plan_set.quantized.is_some());
    }

    #[tokio::test]
    async fn shadow_validation_success_is_measured() {
        let mut interp = Interpreter::new();
        interp.set_elastic_mode(crate::elastic::ElasticMode::HedgedSafe);
        interp
            .execute("{\n1\n2 +\n} 'SHADOW-OK' DEF")
            .await
            .expect("def");

        let before = interp.runtime_metrics.shadow_validation_success_count;
        interp.execute("SHADOW-OK").await.expect("exec");

        assert!(interp.runtime_metrics.shadow_validation_started_count >= 1);
        assert!(interp.runtime_metrics.shadow_validation_success_count >= before);
    }

    #[tokio::test]
    async fn checkpoint_restores_stack_without_failure_history() {
        let mut interp = Interpreter::new();
        interp.execute("[10 20 30]").await.expect("push vector");
        let stack_before = interp.get_stack().clone();

        let checkpoint = RedundancyCheckpoint::capture(&interp);
        interp
            .stack
            .push(Value::from_number(Fraction::from(999i64)));
        assert_ne!(interp.get_stack(), &stack_before, "stack should be dirty");

        checkpoint.restore(&mut interp);

        assert_eq!(interp.get_stack(), &stack_before);
        assert!(interp.runtime_metrics.redundancy_restore_count >= 1);
    }

    #[test]
    fn deterministic_route_uses_simd_soa_for_strong_candidate_only() {
        assert_eq!(
            select_deterministic_route(&vtu_hint(VtuSuitability::StrongCandidate)),
            DeterministicExecutionRoute::SimdSoa
        );
        assert_eq!(
            select_deterministic_route(&vtu_hint(VtuSuitability::WeakCandidate)),
            DeterministicExecutionRoute::Plain
        );
        assert_eq!(
            select_deterministic_route(&vtu_hint(VtuSuitability::NotSuitable)),
            DeterministicExecutionRoute::Plain
        );
    }

    #[tokio::test]
    async fn stack_checkpoint_restores_on_single_path_error() {
        let mut interp = Interpreter::new();
        interp.execute("[1 2]").await.expect("push");
        let stack_before = interp.get_stack().clone();

        let result = interp.with_stack_checkpoint(|interp| {
            interp.stack.clear();
            Err(crate::error::AjisaiError::from("forced failure"))
        });

        assert!(result.is_err());
        assert_eq!(interp.get_stack(), &stack_before);
    }
}
