#[cfg(test)]
mod tests {
    use crate::interpreter::execute_def::op_def_inner;
    use crate::interpreter::Interpreter;
    use crate::tokenizer;

    fn define_word(interp: &mut Interpreter, name: &str, src: &str) {
        let tokens = tokenizer::tokenize(src).expect("tokenize");
        op_def_inner(interp, name, &tokens, None).expect("define word");
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
            .execute("{ 1 } { 2 + } 'SHADOW-OK' DEF")
            .await
            .expect("def");

        let before = interp.runtime_metrics.shadow_validation_success_count;
        interp.execute("SHADOW-OK").await.expect("exec");

        assert!(interp.runtime_metrics.shadow_validation_started_count >= 1);
        assert!(interp.runtime_metrics.shadow_validation_success_count >= before);
    }
}
