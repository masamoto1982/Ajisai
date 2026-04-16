/// Elastic Engine — MVP test suite
///
/// Covers:
/// - M1  purity_table: correct purity / cost / order_sensitive for every builtin
/// - M2  evaluation_unit: struct fields, eligibility predicates
/// - M4  cache_manager: store → fetch round-trip, pure gate, hit rate, prefix invalidation
/// - M5  fallback_bridge: should_fallback rules; execution_mode: from_str / as_str / is_elastic
/// - Semantics: greedy and elastic-safe produce identical stack output for a set of programs

#[cfg(test)]
mod tests {
    // ────────────────────────────────────────────────────────────────────────
    // M1 — Purity table
    // ────────────────────────────────────────────────────────────────────────

    use crate::elastic::purity_table::{infer_purity, purity_by_name, EvalCost, Purity};

    #[test]
    fn purity_table_arithmetic_is_pure_trivial() {
        for word in &["+", "-", "*", "/", "=", "<", "<="] {
            let info = purity_by_name(word)
                .unwrap_or_else(|| panic!("missing purity entry for '{}'", word));
            assert_eq!(info.purity, Purity::Pure, "{}: expected Pure", word);
            assert_eq!(
                info.cost,
                EvalCost::Trivial,
                "{}: expected Trivial cost",
                word
            );
            assert!(
                !info.order_sensitive,
                "{}: should not be order_sensitive",
                word
            );
        }
    }

    #[test]
    fn purity_table_io_is_impure() {
        for word in &["PRINT", "NOW", "CSPRNG"] {
            let info = purity_by_name(word)
                .unwrap_or_else(|| panic!("missing purity entry for '{}'", word));
            assert_eq!(info.purity, Purity::Impure, "{}: expected Impure", word);
            assert!(info.order_sensitive, "{}: should be order_sensitive", word);
        }
    }

    #[test]
    fn purity_table_higher_order_is_unknown() {
        for word in &["MAP", "FILTER", "FOLD", "COND"] {
            let info = purity_by_name(word)
                .unwrap_or_else(|| panic!("missing purity entry for '{}'", word));
            assert_eq!(info.purity, Purity::Unknown, "{}: expected Unknown", word);
        }
    }

    #[test]
    fn purity_table_vector_ops_pure_light() {
        for word in &["LENGTH", "CONCAT", "REVERSE", "SORT"] {
            let info = purity_by_name(word)
                .unwrap_or_else(|| panic!("missing purity entry for '{}'", word));
            assert_eq!(info.purity, Purity::Pure, "{}: expected Pure", word);
            assert_eq!(info.cost, EvalCost::Light, "{}: expected Light cost", word);
        }
    }

    #[test]
    fn purity_table_unknown_word_is_none() {
        assert!(purity_by_name("TOTALLY_UNKNOWN_WORD_XYZ").is_none());
    }

    #[test]
    fn infer_purity_all_pure() {
        assert_eq!(infer_purity(&[Purity::Pure, Purity::Pure]), Purity::Pure);
    }

    #[test]
    fn infer_purity_one_unknown() {
        assert_eq!(
            infer_purity(&[Purity::Pure, Purity::Unknown, Purity::Pure]),
            Purity::Unknown
        );
    }

    #[test]
    fn infer_purity_one_impure_dominates() {
        assert_eq!(
            infer_purity(&[Purity::Pure, Purity::Unknown, Purity::Impure]),
            Purity::Impure
        );
    }

    #[test]
    fn infer_purity_empty_slice() {
        // No components → conservatively pure (vacuous truth)
        assert_eq!(infer_purity(&[]), Purity::Pure);
    }

    // ────────────────────────────────────────────────────────────────────────
    // M2 — EvaluationUnit
    // ────────────────────────────────────────────────────────────────────────

    use crate::elastic::evaluation_unit::{EvaluationUnit, UnitState};
    use crate::elastic::purity_table::purity_by_name as pbn;
    use std::collections::HashSet;

    #[test]
    fn evaluation_unit_defaults() {
        let u = EvaluationUnit::new("+");
        assert_eq!(u.word_name, "+");
        assert_eq!(u.state, UnitState::Pending);
        assert!(!u.pure, "default pure should be false");
        assert!(u.order_sensitive, "default order_sensitive should be true");
        assert!(!u.eager_required);
        assert!(u.depends_on.is_empty());
        assert!(u.cache_key.is_none());
    }

    #[test]
    fn evaluation_unit_from_purity_pure() {
        let info = pbn("+").unwrap();
        let u = EvaluationUnit::from_purity("+", &info);
        assert!(u.pure);
        assert!(!u.order_sensitive);
        assert!(!u.eager_required);
        assert!(u.elastic_eligible());
    }

    #[test]
    fn evaluation_unit_from_purity_impure() {
        let info = pbn("PRINT").unwrap();
        let u = EvaluationUnit::from_purity("PRINT", &info);
        assert!(!u.pure);
        assert!(u.order_sensitive);
        assert!(u.eager_required);
        assert!(!u.elastic_eligible());
    }

    #[test]
    fn evaluation_unit_promotable() {
        let mut u = EvaluationUnit::new("X");
        u.depends_on = vec![1, 2, 3];

        let mut done: HashSet<u32> = HashSet::new();
        assert!(!u.promotable(&done));
        done.insert(1);
        assert!(!u.promotable(&done));
        done.insert(2);
        done.insert(3);
        assert!(u.promotable(&done));
    }

    #[test]
    fn evaluation_unit_priority_score() {
        let mut u = EvaluationUnit::new("FOO");
        u.estimated_cost = 4.0;
        u.pruning_bonus = 1.5;
        // priority_score = cost - bonus = 2.5
        let score = u.priority_score();
        assert!((score - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn evaluation_unit_ids_are_unique() {
        let a = EvaluationUnit::new("A");
        let b = EvaluationUnit::new("B");
        assert_ne!(a.id, b.id);
    }

    // ────────────────────────────────────────────────────────────────────────
    // M4 — CacheManager
    // ────────────────────────────────────────────────────────────────────────

    use crate::elastic::cache_manager::CacheManager;
    use crate::types::fraction::Fraction;
    use crate::types::{DisplayHint, Value, ValueData};

    fn scalar_value(n: i64) -> Value {
        Value {
            data: ValueData::Scalar(Fraction::from(n)),
            hint: DisplayHint::Number,
        }
    }

    #[test]
    fn cache_store_then_fetch_hit() {
        let mut cm = CacheManager::new();
        let key = CacheManager::build_key("+", "[1, 2]", "elastic-safe");
        cm.store(key.clone(), scalar_value(3), true);

        let (val, hit) = cm.fetch(&key);
        assert!(hit, "expected cache hit");
        assert!(val.is_some());
        assert_eq!(cm.hit_count(), 1);
        assert_eq!(cm.miss_count(), 0);
    }

    #[test]
    fn cache_miss_on_absent_key() {
        let mut cm = CacheManager::new();
        let missing_key = CacheManager::build_key("nonexistent", "[]", "greedy");
        let (val, hit) = cm.fetch(&missing_key);
        assert!(!hit, "expected cache miss");
        assert!(val.is_none());
        assert_eq!(cm.miss_count(), 1);
    }

    #[test]
    fn cache_impure_gate_prevents_store() {
        let mut cm = CacheManager::new();
        let key = CacheManager::build_key("PRINT", "[]", "elastic-safe");
        cm.store(key.clone(), scalar_value(0), false); // pure=false → no-op

        let (_, hit) = cm.fetch(&key);
        assert!(!hit, "impure result must not be cached");
    }

    #[test]
    fn cache_hit_rate_computation() {
        let mut cm = CacheManager::new();
        let key = CacheManager::build_key("+", "[1,2]", "elastic-safe");
        cm.store(key.clone(), scalar_value(3), true);

        cm.fetch(&key); // hit
        cm.fetch(&key); // hit
        cm.fetch("other"); // miss

        let rate = cm.hit_rate();
        // 2 hits / 3 total = 0.666…
        assert!((rate - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn cache_invalidate_prefix() {
        let mut cm = CacheManager::new();
        cm.store(
            CacheManager::build_key("+", "[1,2]", "es"),
            scalar_value(3),
            true,
        );
        cm.store(
            CacheManager::build_key("-", "[5,2]", "es"),
            scalar_value(3),
            true,
        );
        cm.store(
            CacheManager::build_key("MAP", "[]", "es"),
            scalar_value(0),
            true,
        );

        assert_eq!(cm.cached_key_count(), 3);
        cm.invalidate_prefix("+");
        assert_eq!(cm.cached_key_count(), 2);
    }

    // ────────────────────────────────────────────────────────────────────────
    // M5 — ElasticMode
    // ────────────────────────────────────────────────────────────────────────

    use crate::elastic::execution_mode::ElasticMode;

    #[test]
    fn elastic_mode_from_str_known() {
        assert_eq!(ElasticMode::from_str("greedy"), ElasticMode::Greedy);
        assert_eq!(
            ElasticMode::from_str("elastic-safe"),
            ElasticMode::ElasticSafe
        );
        assert_eq!(
            ElasticMode::from_str("elastic-force"),
            ElasticMode::ElasticForce
        );
        assert_eq!(
            ElasticMode::from_str("hedged-safe"),
            ElasticMode::HedgedSafe
        );
        assert_eq!(
            ElasticMode::from_str("hedged-trace"),
            ElasticMode::HedgedTrace
        );
        assert_eq!(
            ElasticMode::from_str("fast-guarded"),
            ElasticMode::FastGuarded
        );
        assert_eq!(
            ElasticMode::from_str("elastic_safe"),
            ElasticMode::ElasticSafe
        );
        assert_eq!(
            ElasticMode::from_str("elastic_force"),
            ElasticMode::ElasticForce
        );
        assert_eq!(
            ElasticMode::from_str(" Elastic-Safe "),
            ElasticMode::ElasticSafe
        );
    }

    #[test]
    fn elastic_mode_from_str_unknown_falls_back_to_greedy() {
        assert_eq!(ElasticMode::from_str("unknown-xyz"), ElasticMode::Greedy);
    }

    #[test]
    fn elastic_mode_is_elastic() {
        assert!(!ElasticMode::Greedy.is_elastic());
        assert!(ElasticMode::ElasticSafe.is_elastic());
        assert!(ElasticMode::HedgedSafe.is_elastic());
        assert!(ElasticMode::HedgedTrace.is_elastic());
        assert!(ElasticMode::FastGuarded.is_elastic());
        assert!(ElasticMode::ElasticForce.is_elastic());
    }

    #[test]
    fn elastic_mode_as_str_round_trip() {
        for mode in &[
            ElasticMode::Greedy,
            ElasticMode::ElasticSafe,
            ElasticMode::HedgedSafe,
            ElasticMode::HedgedTrace,
            ElasticMode::FastGuarded,
            ElasticMode::ElasticForce,
        ] {
            assert_eq!(ElasticMode::from_str(mode.as_str()), *mode);
        }
    }

    // ────────────────────────────────────────────────────────────────────────
    // M5 — FallbackBridge
    // ────────────────────────────────────────────────────────────────────────

    use crate::elastic::fallback_bridge::{FallbackBridge, FallbackReason};

    fn make_unit(pure: bool, order_sensitive: bool, eager: bool) -> EvaluationUnit {
        let mut u = EvaluationUnit::new("TEST");
        u.pure = pure;
        u.order_sensitive = order_sensitive;
        u.eager_required = eager;
        u
    }

    #[test]
    fn fallback_impure_word_returns_unknown_purity() {
        let mut bridge = FallbackBridge::new();
        let u = make_unit(false, false, false);
        let reason = bridge.should_fallback(&u, ElasticMode::ElasticSafe);
        assert_eq!(reason, Some(FallbackReason::UnknownPurity));
        assert_eq!(bridge.fallback_log.len(), 1);
    }

    #[test]
    fn fallback_order_sensitive_returns_order_sensitive() {
        let mut bridge = FallbackBridge::new();
        let u = make_unit(true, true, false);
        let reason = bridge.should_fallback(&u, ElasticMode::ElasticSafe);
        assert_eq!(reason, Some(FallbackReason::OrderSensitive));
    }

    #[test]
    fn fallback_eager_required_returns_order_sensitive() {
        let mut bridge = FallbackBridge::new();
        let u = make_unit(true, false, true);
        let reason = bridge.should_fallback(&u, ElasticMode::ElasticSafe);
        assert_eq!(reason, Some(FallbackReason::OrderSensitive));
    }

    #[test]
    fn fallback_pure_non_sensitive_returns_none() {
        let mut bridge = FallbackBridge::new();
        let u = make_unit(true, false, false);
        let reason = bridge.should_fallback(&u, ElasticMode::ElasticSafe);
        assert_eq!(reason, None);
        assert!(bridge.fallback_log.is_empty());
    }

    #[test]
    fn fallback_elastic_force_never_falls_back() {
        let mut bridge = FallbackBridge::new();
        // Even fully impure + order-sensitive → no fallback in ElasticForce
        let u = make_unit(false, true, true);
        let reason = bridge.should_fallback(&u, ElasticMode::ElasticForce);
        assert_eq!(reason, None);
        assert!(
            bridge.fallback_log.is_empty(),
            "ElasticForce must not log fallbacks"
        );
    }

    // ────────────────────────────────────────────────────────────────────────
    // Semantics — greedy vs elastic-safe output equivalence
    // ────────────────────────────────────────────────────────────────────────

    use crate::interpreter::Interpreter;

    /// Run `code` in greedy mode and return the stack snapshot.
    async fn run_greedy(code: &str) -> Vec<String> {
        let mut interp = Interpreter::new();
        interp.set_elastic_mode(ElasticMode::Greedy);
        interp.execute(code).await.expect("greedy execution failed");
        interp.stack.iter().map(|v| format!("{:?}", v)).collect()
    }

    /// Run `code` in elastic-safe mode and return the stack snapshot.
    async fn run_elastic(code: &str) -> Vec<String> {
        let mut interp = Interpreter::new();
        interp.set_elastic_mode(ElasticMode::ElasticSafe);
        interp
            .execute(code)
            .await
            .expect("elastic-safe execution failed");
        interp.stack.iter().map(|v| format!("{:?}", v)).collect()
    }

    async fn run_hedged_safe(code: &str) -> Vec<String> {
        let mut interp = Interpreter::new();
        interp.set_elastic_mode(ElasticMode::HedgedSafe);
        interp
            .execute(code)
            .await
            .expect("hedged-safe execution failed");
        interp.stack.iter().map(|v| format!("{:?}", v)).collect()
    }

    async fn run_fast_guarded(code: &str) -> Vec<String> {
        let mut interp = Interpreter::new();
        interp.set_elastic_mode(ElasticMode::FastGuarded);
        interp
            .execute(code)
            .await
            .expect("fast-guarded execution failed");
        interp.stack.iter().map(|v| format!("{:?}", v)).collect()
    }

    macro_rules! assert_semantics_match {
        ($code:expr) => {{
            let greedy = run_greedy($code).await;
            let elastic = run_elastic($code).await;
            assert_eq!(
                greedy, elastic,
                "Semantic divergence for `{}`:\n  greedy  = {:?}\n  elastic = {:?}",
                $code, greedy, elastic
            );
        }};
    }

    #[tokio::test]
    async fn semantics_basic_arithmetic() {
        assert_semantics_match!("[1] [2] +");
    }

    #[tokio::test]
    async fn semantics_vector_ops() {
        assert_semantics_match!("[1 2 3] [4 5 6] CONCAT");
    }

    #[tokio::test]
    async fn semantics_logic() {
        assert_semantics_match!("[1] [2] < NOT");
    }

    #[tokio::test]
    async fn semantics_map() {
        assert_semantics_match!("[1 2 3] { [2] * } MAP");
    }

    #[tokio::test]
    async fn semantics_fold() {
        assert_semantics_match!("[1 2 3 4] [0] { + } FOLD");
    }

    #[tokio::test]
    async fn semantics_filter() {
        assert_semantics_match!("[1 2 3 4 5 6] { [2] MOD [0] = } FILTER");
    }

    macro_rules! assert_greedy_hedged_match {
        ($code:expr) => {{
            let greedy = run_greedy($code).await;
            let hedged = run_hedged_safe($code).await;
            assert_eq!(
                greedy, hedged,
                "Semantic divergence for `{}`:\n  greedy = {:?}\n  hedged = {:?}",
                $code, greedy, hedged
            );
        }};
    }

    #[tokio::test]
    async fn semantics_hedged_hof_kernels() {
        assert_greedy_hedged_match!("[1 2 3] { [2] * } MAP");
        assert_greedy_hedged_match!("[1 2 3 4 5 6] { [2] MOD [0] = } FILTER");
        assert_greedy_hedged_match!("[1 2 3 4] [0] { + } FOLD");
        assert_greedy_hedged_match!("[1 2 3 4] [0] { + } SCAN");
    }

    #[tokio::test]
    async fn semantics_fast_guarded_hof_kernels() {
        let code = "[1 2 3 4] { [1] + } MAP [1 2 3 4] { [2] MOD [0] = } FILTER [1 2 3 4] [0] { + } FOLD";
        let greedy = run_greedy(code).await;
        let fast_guarded = run_fast_guarded(code).await;
        assert_eq!(
            greedy, fast_guarded,
            "Semantic divergence for fast-guarded mode:\n  greedy = {:?}\n  fast_guarded = {:?}",
            greedy, fast_guarded
        );
    }

    #[tokio::test]
    async fn fast_guarded_avoids_hedged_race_start() {
        let mut interp = Interpreter::new();
        interp.set_elastic_mode(ElasticMode::FastGuarded);
        interp
            .execute("[1 2 3 4] { [1] + } MAP [1 2 3 4] [0] { + } FOLD")
            .await
            .expect("fast-guarded execution failed");
        let m = interp.runtime_metrics();
        assert_eq!(m.hedged_race_started_count, 0);
        assert!(
            m.quantized_block_use_count >= 1,
            "fast-guarded should still use quantized kernels when guards pass"
        );
    }

    #[tokio::test]
    async fn hedged_metrics_increment_for_hof_kernels() {
        let mut interp = Interpreter::new();
        interp.set_elastic_mode(ElasticMode::HedgedSafe);
        interp
            .execute(
                "[1 2 3 4] { [1] + } MAP [1 2 3 4] { [2] MOD [0] = } FILTER [1 2 3 4] [0] { + } FOLD",
            )
            .await
            .expect("hedged-safe execution failed");
        let m = interp.runtime_metrics();
        assert!(m.hedged_race_started_count >= 1);
        assert!(m.hedged_race_winner_quantized_count >= 1 || m.hedged_race_winner_plain_count >= 1);
    }

    #[tokio::test]
    async fn compiled_plan_plain_race_runs_in_hedged_mode() {
        let mut interp = Interpreter::new();
        interp.set_elastic_mode(ElasticMode::HedgedSafe);
        interp
            .execute("{ [2] * } 'DOUBLE' DEF [5] DOUBLE [7] DOUBLE")
            .await
            .expect("hedged compiled/plain race should succeed");
        let m = interp.runtime_metrics();
        assert!(
            m.hedged_race_started_count >= 1,
            "compiled/plain race should increment started count"
        );
    }

    #[tokio::test]
    async fn hedged_trace_collects_events() {
        let mut interp = Interpreter::new();
        interp.set_elastic_mode(ElasticMode::HedgedTrace);
        interp
            .execute("[1 2 3] { [2] * } MAP")
            .await
            .expect("hedged trace execution should succeed");
        let trace = interp.drain_hedged_trace();
        assert!(!trace.is_empty(), "hedged trace should record events");
    }

    #[tokio::test]
    async fn semantics_cond_first_clause() {
        // Separate guard/body pairs: { guard } { body } ... COND
        assert_semantics_match!("[1] { [1] = } { [42] } { IDLE } { [99] } COND");
    }

    #[tokio::test]
    async fn semantics_cond_else_clause() {
        assert_semantics_match!("[9] { [1] = } { [42] } { IDLE } { [99] } COND");
    }

    #[tokio::test]
    async fn semantics_range_and_length() {
        assert_semantics_match!("[0 9] RANGE LENGTH");
    }

    #[tokio::test]
    async fn semantics_nested_arithmetic() {
        assert_semantics_match!("[3] [4] * [2] + [10] -");
    }

    #[tokio::test]
    async fn semantics_user_defined_word() {
        let code = "{ [2] * } 'DOUBLE' DEF [5] DOUBLE";
        assert_semantics_match!(code);
    }

    // ────────────────────────────────────────────────────────────────────────
    // Tracer — basic record / reset
    // ────────────────────────────────────────────────────────────────────────

    use crate::elastic::tracer;

    #[test]
    fn tracer_disabled_by_default() {
        // Reset so prior tests don't bleed state
        tracer::reset();
        // Tracer is off unless env var is set; in tests we don't set it.
        // At minimum, call_count must be consistent after reset.
        tracer::reset();
        assert_eq!(tracer::call_count("ANYTHING"), 0);
    }

    #[test]
    fn tracer_record_increments_count() {
        tracer::reset();
        tracer::set_enabled(true);
        tracer::record("TEST_WORD", 1_000_000);
        tracer::record("TEST_WORD", 2_000_000);
        assert_eq!(tracer::call_count("TEST_WORD"), 2);
        assert_eq!(tracer::total_nanos("TEST_WORD"), 3_000_000);
        // Restore default
        tracer::set_enabled(false);
        tracer::reset();
    }

    #[test]
    fn tracer_disabled_no_record() {
        tracer::reset();
        tracer::set_enabled(false);
        tracer::record("SILENT", 999);
        assert_eq!(tracer::call_count("SILENT"), 0);
    }

    // ────────────────────────────────────────────────────────────────────────
    // Interpreter integration — elastic fields accessible
    // ────────────────────────────────────────────────────────────────────────

    #[test]
    fn interpreter_default_mode_is_greedy() {
        let interp = Interpreter::new();
        assert_eq!(interp.elastic_mode(), ElasticMode::Greedy);
    }

    #[test]
    fn interpreter_set_elastic_mode() {
        let mut interp = Interpreter::new();
        interp.set_elastic_mode(ElasticMode::ElasticSafe);
        assert_eq!(interp.elastic_mode(), ElasticMode::ElasticSafe);
    }

    #[tokio::test]
    async fn interpreter_elastic_safe_runs_without_error() {
        let mut interp = Interpreter::new();
        interp.set_elastic_mode(ElasticMode::ElasticSafe);
        interp
            .execute("[10] [20] + [5] - LENGTH")
            .await
            .expect("elastic-safe mode failed");
    }
}
