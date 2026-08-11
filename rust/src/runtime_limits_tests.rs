//! CS5 attacker-input tests: each internal-cost ceiling must fire at a **low
//! injected limit** — deterministically and synchronously, with a diagnosable
//! `AjisaiError` — without the test having to actually allocate or compute
//! anything huge. A limit failure must also leave the interpreter usable (no
//! corrupted partial stack that poisons the next program).
//!
//! Conformance never depends on a specific limit value (limits are a safety
//! control, not value semantics): the "ordinary program under default limits"
//! cases pin that normal work is untouched.

#[cfg(test)]
mod runtime_limits_tests {
    use crate::interpreter::runtime_limits::RuntimeLimits;
    use crate::interpreter::Interpreter;

    fn with_limits(limits: RuntimeLimits) -> Interpreter {
        let mut interp = Interpreter::new();
        interp.set_runtime_limits(limits);
        interp
    }

    // ── source-byte ceiling ────────────────────────────────────────────────

    #[tokio::test]
    async fn oversized_source_is_rejected_before_tokenizing() {
        let mut interp = with_limits(RuntimeLimits {
            max_source_bytes: 4,
            ..RuntimeLimits::default()
        });
        let err = interp
            .execute("1 2 3 +")
            .await
            .expect_err("source over the byte ceiling must error");
        assert!(
            err.to_string().contains("exceeds the limit"),
            "diagnosable source-size error, got: {err}"
        );
    }

    #[tokio::test]
    async fn source_at_the_byte_ceiling_is_accepted() {
        let mut interp = with_limits(RuntimeLimits {
            max_source_bytes: 3,
            ..RuntimeLimits::default()
        });
        assert!(
            interp.execute("1 2").await.is_ok(),
            "3-byte source is allowed"
        );
    }

    // ── numeric-literal digit ceiling ──────────────────────────────────────

    #[tokio::test]
    async fn oversized_numeric_literal_is_rejected_before_the_bigint_parse() {
        // A modest 6-digit literal fires the guard at an injected 3-digit
        // ceiling — no astronomically large value is ever built.
        let mut interp = with_limits(RuntimeLimits {
            max_numeric_literal_digits: 3,
            ..RuntimeLimits::default()
        });
        let err = interp
            .execute("123456")
            .await
            .expect_err("literal over the digit ceiling must error");
        assert!(
            err.to_string().contains("exceeds the limit"),
            "diagnosable numeric-literal error, got: {err}"
        );
    }

    #[tokio::test]
    async fn numeric_literal_at_the_digit_ceiling_is_accepted() {
        let mut interp = with_limits(RuntimeLimits {
            max_numeric_literal_digits: 3,
            ..RuntimeLimits::default()
        });
        assert!(
            interp.execute("123").await.is_ok(),
            "3-digit literal is allowed"
        );
        assert_eq!(interp.get_stack().len(), 1);
    }

    #[tokio::test]
    async fn digit_ceiling_counts_digits_only_not_sign_or_point() {
        // Injected ceiling of 4 digits: `-1.5` has 2 digits and must pass;
        // `-123.45` has 5 digits and must fail. Confirms sign / radix point are
        // excluded from the count.
        let mut interp = with_limits(RuntimeLimits {
            max_numeric_literal_digits: 4,
            ..RuntimeLimits::default()
        });
        assert!(interp.execute("-1.5").await.is_ok());
        let mut interp2 = with_limits(RuntimeLimits {
            max_numeric_literal_digits: 4,
            ..RuntimeLimits::default()
        });
        assert!(interp2.execute("-123.45").await.is_err());
    }

    // ── materialization ceiling (folded RANGE / FILL guards) ───────────────
    //
    // Phase 3 (structural-memory-safety roadmap): the materialization ceiling is
    // a *space water level*, so a well-formed but over-budget generative call
    // projects onto a diagnosable Bubble/NIL (reason `SpaceExhausted`) instead
    // of a channel error — deterministically and synchronously, without
    // allocating anything huge. The other ceilings below stay ordinary errors.

    fn top_nil_reason(interp: &Interpreter) -> Option<crate::error::NilReason> {
        interp
            .get_stack()
            .last()
            .and_then(|v| v.absence.as_ref())
            .and_then(|a| a.reason)
    }

    #[tokio::test]
    async fn range_bubbles_at_a_low_injected_materialization_limit() {
        let mut interp = with_limits(RuntimeLimits {
            max_materialized_elements: 10,
            ..RuntimeLimits::default()
        });
        interp
            .execute("[ 0 100 ] RANGE")
            .await
            .expect("RANGE over the injected element cap must bubble, not error");
        assert_eq!(
            top_nil_reason(&interp),
            Some(crate::error::NilReason::SpaceExhausted),
            "RANGE over the injected cap must leave a SpaceExhausted NIL"
        );
    }

    #[tokio::test]
    async fn fill_bubbles_at_a_low_injected_materialization_limit() {
        let mut interp = with_limits(RuntimeLimits {
            max_materialized_elements: 10,
            ..RuntimeLimits::default()
        });
        interp
            .execute("[ 100 100 ] FILL")
            .await
            .expect("FILL over the injected element cap must bubble, not error");
        assert_eq!(
            top_nil_reason(&interp),
            Some(crate::error::NilReason::SpaceExhausted),
            "FILL over the injected cap must leave a SpaceExhausted NIL"
        );
    }

    // ── recovery: a space bubble must not corrupt the interpreter ──────────

    #[tokio::test]
    async fn interpreter_stays_usable_after_a_materialization_space_bubble() {
        let mut interp = with_limits(RuntimeLimits {
            max_materialized_elements: 10,
            ..RuntimeLimits::default()
        });
        assert_eq!(
            {
                interp.execute("[ 0 100 ] RANGE").await.ok();
                top_nil_reason(&interp)
            },
            Some(crate::error::NilReason::SpaceExhausted),
        );
        // A subsequent ordinary program must run cleanly on the same
        // interpreter — no poisoned partial stack.
        interp.set_runtime_limits(RuntimeLimits::default());
        assert!(interp.execute("2 3 +").await.is_ok());
        assert_eq!(
            interp.get_stack().last().and_then(|v| v.as_i64()),
            Some(5),
            "2 3 + must evaluate to 5 after a materialization space bubble"
        );
    }

    // ── algebraic term-count ceiling (exact-arithmetic result size) ────────

    #[tokio::test]
    async fn algebraic_term_explosion_is_rejected_at_a_low_injected_limit() {
        // (√2+√3)·(√5+√7) = √10+√14+√15+√21 — a 4-term algebraic value. At an
        // injected 3-term ceiling the multiply's result is rejected; the
        // 2-term intermediate sums pass, so the guard fires on the explosion,
        // not on ordinary exact work.
        let mut interp = with_limits(RuntimeLimits {
            max_algebraic_terms: 3,
            ..RuntimeLimits::default()
        });
        let err = interp
            .execute("2 SQRT 3 SQRT + 5 SQRT 7 SQRT + *")
            .await
            .expect_err("a 4-term product past a 3-term ceiling must error");
        assert!(
            matches!(
                err,
                crate::error::AjisaiError::ResourceLimitExceeded {
                    resource: crate::error::ResourceLimit::AlgebraicTerms,
                    limit: 3,
                    ..
                }
            ),
            "the algebraic-term ceiling must report itself by name, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn same_algebraic_product_succeeds_under_default_limits() {
        let mut interp = Interpreter::new();
        assert!(
            interp
                .execute("2 SQRT 3 SQRT + 5 SQRT 7 SQRT + *")
                .await
                .is_ok(),
            "a 4-term algebraic product is ordinary work under default limits"
        );
    }

    // ── numeric-work meter (per-operation internal cost, cumulative) ────────

    #[tokio::test]
    async fn runaway_numeric_work_is_charged_and_rejected_before_computing() {
        // An injected budget of 1 work unit is spent by the first algebraic
        // operation, so the computation fails deterministically at the meter
        // rather than grinding — without building anything huge.
        let mut interp = with_limits(RuntimeLimits {
            max_numeric_work: 1,
            ..RuntimeLimits::default()
        });
        let err = interp
            .execute("2 SQRT 3 SQRT +")
            .await
            .expect_err("exact work past the meter must error");
        assert!(
            matches!(
                err,
                crate::error::AjisaiError::ResourceLimitExceeded {
                    resource: crate::error::ResourceLimit::NumericWork,
                    limit: 1,
                    ..
                }
            ),
            "the numeric-work meter must report itself by name, got: {err:?}"
        );
    }

    // ── ordinary work is untouched under default limits ────────────────────

    #[tokio::test]
    async fn ordinary_programs_pass_under_default_limits() {
        let mut interp = Interpreter::new();
        assert!(interp.execute("[ 0 5 ] RANGE").await.is_ok());
        let mut interp2 = Interpreter::new();
        assert!(interp2.execute("123456789 2 *").await.is_ok());
        // Ordinary exact arithmetic (√2·√2 = 2, √2+√3) is untouched.
        let mut interp3 = Interpreter::new();
        assert!(interp3.execute("2 SQRT 2 SQRT *").await.is_ok());
        let mut interp4 = Interpreter::new();
        assert!(interp4.execute("2 SQRT 3 SQRT +").await.is_ok());
    }

    // ── the work meter prices operand size, not operation count ────────────

    #[tokio::test]
    async fn tier_zero_multiplication_is_charged() {
        // The scalar fast path returns before the exact-real path that does the
        // charging, so a chain of big-integer multiplications used to be
        // metered at zero and bounded only by the step budget — which counts
        // words, not the size of the numbers in them. Four hundred of these,
        // 0.4% of that budget, spent forty seconds building a multi-megabyte
        // integer with every size ceiling silent.
        let big = "9".repeat(512);
        let mut interp = Interpreter::new();
        interp
            .execute(&format!("1 {big} * {big} *"))
            .await
            .expect("ordinary big-integer work still succeeds");
        assert!(
            interp.numeric_work_used > 0,
            "a wide rational multiply must reach the meter, charged {}",
            interp.numeric_work_used
        );
    }

    #[tokio::test]
    async fn work_is_priced_by_operand_width_not_operation_count() {
        // Two multiplications, identical in count and in every other respect;
        // the meter used to charge them the same. It is the width that makes
        // one of them expensive, so it is the width the meter has to see.
        async fn charge(source: &str) -> u64 {
            let mut interp = Interpreter::new();
            interp.execute(source).await.expect("source computes");
            interp.numeric_work_used
        }
        let narrow = charge("2 3 *").await;
        let wide = charge(&format!("{} {} *", "9".repeat(2048), "9".repeat(2048))).await;
        assert!(
            wide > narrow * 100,
            "a 2048-digit product must cost far more than a one-digit product, got {wide} vs {narrow}"
        );
    }

    #[tokio::test]
    async fn tier_zero_growth_is_bounded_by_the_bigint_ceiling() {
        // `bigintBits` had no path that reached it from ordinary rational
        // arithmetic. It does now, and it reports itself by name.
        let big = "9".repeat(4096);
        let mut interp = with_limits(RuntimeLimits {
            max_bigint_bits: 20_000,
            ..RuntimeLimits::default()
        });
        let err = interp
            .execute(&format!("1 {big} * {big} *"))
            .await
            .expect_err("a product past the bit ceiling must error");
        assert!(
            matches!(
                err,
                crate::error::AjisaiError::ResourceLimitExceeded {
                    resource: crate::error::ResourceLimit::BigintBits,
                    limit: 20_000,
                    ..
                }
            ),
            "the BigInt ceiling must report itself by name, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn ordinary_arithmetic_stays_far_below_the_meter() {
        // The meter only earns its place if it is invisible to real programs.
        for source in [
            "1 3 /",
            "0.1 0.2 +",
            "2 SQRT",
            "[ 1 2 ] 10 +",
            "8 SQRT 2 SQRT 2 SQRT + =",
            "2 SQRT 3 SQRT + 5 SQRT 7 SQRT + *",
        ] {
            let mut interp = Interpreter::new();
            interp
                .execute(source)
                .await
                .expect("ordinary source computes");
            assert!(
                interp.numeric_work_used < 100_000,
                "`{source}` charged {} units, which is not ordinary",
                interp.numeric_work_used
            );
        }
    }
}
