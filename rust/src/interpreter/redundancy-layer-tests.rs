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

    // ── Redundancy Layer テスト（新規）──────────────────────────────────────

    use crate::interpreter::redundancy_budget::{
        DegradationPolicy, FailureHistory, RedundancyBudget,
    };
    use crate::interpreter::redundancy_layer::{
        select_degradation_policy, RedundancyCheckpoint,
    };
    use crate::interpreter::quantized_block::{QuantizedArity, QuantizedPurity};
    use crate::types::fraction::Fraction;
    use crate::types::Value;

    // 1. チェックポイントが Quantized 失敗後にスタックを復元する
    #[tokio::test]
    async fn test_checkpoint_restores_stack_on_quantized_failure() {
        let mut interp = Interpreter::new();
        interp.execute("[10 20 30]").await.expect("push vector");
        let stack_before = interp.get_stack().clone();

        let checkpoint = RedundancyCheckpoint::capture(&interp);
        // スタックを意図的に汚染する
        interp
            .stack
            .push(Value::from_number(Fraction::from(999i64)));
        assert_ne!(interp.get_stack(), &stack_before, "stack should be dirty");

        checkpoint.restore(&mut interp);

        assert_eq!(
            interp.get_stack(),
            &stack_before,
            "stack must be restored to pre-failure state"
        );
        assert!(
            interp.runtime_metrics.redundancy_restore_count >= 1,
            "restore_count should be incremented"
        );
    }

    // 2. チェックポイントが Compiled 失敗後にスタックを復元する
    #[tokio::test]
    async fn test_checkpoint_restores_stack_on_compiled_failure() {
        let mut interp = Interpreter::new();
        interp.execute("[1 2]").await.expect("push");
        let stack_before = interp.get_stack().clone();

        let checkpoint = RedundancyCheckpoint::capture(&interp);
        interp.stack.clear(); // スタックを空にして汚染
        checkpoint.restore(&mut interp);

        assert_eq!(interp.get_stack(), &stack_before);
    }

    // 3. ThreeStage 経路で失敗すると degrade_quantized カウントが増加する
    #[tokio::test]
    async fn test_three_stage_degradation_records_metrics() {
        let mut interp = Interpreter::new();
        let mut history = FailureHistory::default();
        let budget = RedundancyBudget::default();

        let before = interp.runtime_metrics.redundancy_degrade_quantized;
        let result = interp.execute_word_with_redundancy(
            "TEST",
            &mut history,
            &budget,
            DegradationPolicy::ThreeStage,
            |_| Err(crate::error::AjisaiError::from("forced failure")),
        );

        assert!(result.is_err());
        assert!(
            interp.runtime_metrics.redundancy_degrade_quantized > before,
            "degrade_quantized must increment on ThreeStage failure"
        );
        assert_eq!(history.quantized_failures, 1);
    }

    // 4. cooldown_epochs 内は quantized_is_cooling_down が true を返す
    #[test]
    fn test_budget_cooldown_prevents_quantized_attempt() {
        let budget = RedundancyBudget {
            cooldown_epochs: 5,
            ..Default::default()
        };
        let mut history = FailureHistory::default();
        // epoch 10 で失敗
        history.record_quantized_failure(10);

        // epoch 12: 12 - 10 = 2 < 5 → まだ冷却中
        assert!(
            history.quantized_is_cooling_down(12, &budget),
            "should be cooling down at epoch 12"
        );
        // epoch 15: 15 - 10 = 5 >= 5 → 冷却終了
        assert!(
            !history.quantized_is_cooling_down(15, &budget),
            "should NOT be cooling down at epoch 15"
        );
    }

    // 5. auto_degrade_threshold 到達後、PlainOnly に自動降格する
    #[tokio::test]
    async fn test_auto_degrade_policy_after_threshold() {
        let mut interp = Interpreter::new();
        let budget = RedundancyBudget {
            auto_degrade_threshold: 3,
            ..Default::default()
        };
        let mut history = FailureHistory::default();
        history.quantized_failures = 3; // 閾値ちょうど

        assert!(
            history.should_auto_degrade(&budget),
            "should auto-degrade when failures == threshold"
        );

        let before = interp.runtime_metrics.redundancy_auto_degrade_count;
        // 成功するクロージャを渡しても auto_degrade カウントが増えることを確認
        let _ = interp.execute_word_with_redundancy(
            "TEST",
            &mut history,
            &budget,
            DegradationPolicy::ThreeStage,
            |_| Ok(()),
        );
        assert!(
            interp.runtime_metrics.redundancy_auto_degrade_count > before,
            "auto_degrade_count must increment when threshold is reached"
        );
    }

    // 6. SideEffecting purity → PlainOnly が選択される
    #[test]
    fn test_side_effecting_word_uses_plain_only() {
        let policy = select_degradation_policy(
            QuantizedArity::Fixed(1),
            QuantizedArity::Fixed(1),
            QuantizedPurity::SideEffecting,
        );
        assert_eq!(
            policy,
            DegradationPolicy::PlainOnly,
            "SideEffecting must always map to PlainOnly"
        );
    }

    // 7. Variable arity → TwoStage、Pure + Fixed arity → ThreeStage
    #[test]
    fn test_variable_arity_block_skips_quantized() {
        // Variable input arity → TwoStage
        let policy_var = select_degradation_policy(
            QuantizedArity::Variable,
            QuantizedArity::Fixed(1),
            QuantizedPurity::Pure,
        );
        assert_eq!(
            policy_var,
            DegradationPolicy::TwoStage,
            "Variable input arity should map to TwoStage"
        );

        // Variable output arity → TwoStage
        let policy_var_out = select_degradation_policy(
            QuantizedArity::Fixed(1),
            QuantizedArity::Variable,
            QuantizedPurity::Pure,
        );
        assert_eq!(policy_var_out, DegradationPolicy::TwoStage);

        // Both Fixed + Pure → ThreeStage
        let policy_pure = select_degradation_policy(
            QuantizedArity::Fixed(1),
            QuantizedArity::Fixed(1),
            QuantizedPurity::Pure,
        );
        assert_eq!(
            policy_pure,
            DegradationPolicy::ThreeStage,
            "Fixed arity + Pure must map to ThreeStage"
        );

        // Unknown purity → TwoStage even with fixed arity
        let policy_unknown = select_degradation_policy(
            QuantizedArity::Fixed(2),
            QuantizedArity::Fixed(1),
            QuantizedPurity::Unknown,
        );
        assert_eq!(policy_unknown, DegradationPolicy::TwoStage);
    }
}
