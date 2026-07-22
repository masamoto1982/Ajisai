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

    #[tokio::test]
    async fn range_fires_materialization_guard_at_a_low_injected_limit() {
        let mut interp = with_limits(RuntimeLimits {
            max_materialized_elements: 10,
            ..RuntimeLimits::default()
        });
        let err = interp
            .execute("[ 0 100 ] RANGE")
            .await
            .expect_err("RANGE over the injected element cap must error");
        assert!(
            err.to_string().contains("exceeding the limit"),
            "diagnosable materialization error, got: {err}"
        );
    }

    #[tokio::test]
    async fn fill_fires_materialization_guard_at_a_low_injected_limit() {
        let mut interp = with_limits(RuntimeLimits {
            max_materialized_elements: 10,
            ..RuntimeLimits::default()
        });
        let err = interp
            .execute("[ 100 100 ] FILL")
            .await
            .expect_err("FILL over the injected element cap must error");
        assert!(
            err.to_string().contains("too many elements"),
            "diagnosable materialization error, got: {err}"
        );
    }

    // ── recovery: a limit failure must not corrupt the interpreter ─────────

    #[tokio::test]
    async fn interpreter_stays_usable_after_a_materialization_limit_failure() {
        let mut interp = with_limits(RuntimeLimits {
            max_materialized_elements: 10,
            ..RuntimeLimits::default()
        });
        assert!(interp.execute("[ 0 100 ] RANGE").await.is_err());
        // A subsequent ordinary program must run cleanly on the same
        // interpreter — no poisoned partial stack.
        interp.set_runtime_limits(RuntimeLimits::default());
        assert!(interp.execute("2 3 +").await.is_ok());
        assert_eq!(
            interp.get_stack().last().and_then(|v| v.as_i64()),
            Some(5),
            "2 3 + must evaluate to 5 after recovering from a limit failure"
        );
    }

    // ── ordinary work is untouched under default limits ────────────────────

    #[tokio::test]
    async fn ordinary_programs_pass_under_default_limits() {
        let mut interp = Interpreter::new();
        assert!(interp.execute("[ 0 5 ] RANGE").await.is_ok());
        let mut interp2 = Interpreter::new();
        assert!(interp2.execute("123456789 2 *").await.is_ok());
    }
}
