//! Test suite for `crate::interpreter::arithmetic`.

#[cfg(test)]
mod ceil_tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_ceil_positive_remainder() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 7/3 ] CEIL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ 3/1 ]", "CEIL(7/3) should be 3");
    }

    #[tokio::test]
    async fn test_ceil_negative_remainder() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ -7/3 ] CEIL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ -2/1 ]", "CEIL(-7/3) should be -2");
    }

    #[tokio::test]
    async fn test_ceil_positive_integer() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 6/3 ] CEIL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ 2/1 ]", "CEIL(6/3) should be 2");
    }

    #[tokio::test]
    async fn test_ceil_negative_integer() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ -6/3 ] CEIL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ -2/1 ]", "CEIL(-6/3) should be -2");
    }

    #[tokio::test]
    async fn test_ceil_operation_target_stack_error() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        let result = interp.execute("[ 1 2 3 ] .. CEIL").await;
        assert!(result.is_err(), "CEIL should not support Stack mode (..)");
    }

    #[tokio::test]
    async fn test_ceil_of_nil_passes_nil_through() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("NIL").await.unwrap();
        interp
            .execute("CEIL")
            .await
            .expect("CEIL of NIL should succeed and produce NIL");
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_nil(), "CEIL of NIL should yield NIL");
    }
}

#[cfg(test)]
mod round_tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_round_positive_below_half() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 7/3 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ 2/1 ]", "ROUND(7/3) should be 2");
    }

    #[tokio::test]
    async fn test_round_positive_half() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 5/2 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ 3/1 ]", "ROUND(5/2) should be 3");
    }

    #[tokio::test]
    async fn test_round_positive_above_half() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 8/3 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ 3/1 ]", "ROUND(8/3) should be 3");
    }

    #[tokio::test]
    async fn test_round_negative_below_half() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ -7/3 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ -2/1 ]", "ROUND(-7/3) should be -2");
    }

    #[tokio::test]
    async fn test_round_negative_half() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ -5/2 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ -3/1 ]", "ROUND(-5/2) should be -3");
    }

    #[tokio::test]
    async fn test_round_negative_above_half() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ -8/3 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ -3/1 ]", "ROUND(-8/3) should be -3");
    }

    #[tokio::test]
    async fn test_round_positive_integer() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 6/3 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ 2/1 ]", "ROUND(6/3) should be 2");
    }

    #[tokio::test]
    async fn test_round_negative_integer() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ -6/3 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ -2/1 ]", "ROUND(-6/3) should be -2");
    }

    #[tokio::test]
    async fn test_round_operation_target_stack_error() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        let result = interp.execute("[ 1 2 3 ] .. ROUND").await;
        assert!(result.is_err(), "ROUND should not support Stack mode (..)");
    }

    #[tokio::test]
    async fn test_round_of_nil_passes_nil_through() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("NIL").await.unwrap();
        interp
            .execute("ROUND")
            .await
            .expect("ROUND of NIL should succeed and produce NIL");
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_nil(), "ROUND of NIL should yield NIL");
    }
}

#[cfg(test)]
mod num_tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_num_parse_error_stack_restoration() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 'hello' ]").await.unwrap();
        let result = interp.execute("NUM").await;
        assert!(result.is_err());
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1, "Stack should be restored after parse error");
    }

    #[tokio::test]
    async fn test_num_same_structure_error_stack_restoration() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 42 ]").await.unwrap();
        let result = interp.execute("NUM").await;

        assert!(result.is_err(), "NUM should error on number vector [42]");
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1, "Stack should be restored after error");
    }

    #[tokio::test]
    async fn test_num_nil_error_stack_restoration() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ nil ]").await.unwrap();
        let result = interp.execute("NUM").await;
        assert!(result.is_err());
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1, "Stack should be restored after nil error");
    }

    #[tokio::test]
    async fn test_num_operation_target_stack_error() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ '42' ] [ '123' ]").await.unwrap();
        let result = interp.execute(".. NUM").await;
        assert!(result.is_err());
        let stack = interp.get_stack();
        assert_eq!(
            stack.len(),
            2,
            "Stack should remain unchanged after Stack mode error"
        );
    }
}

#[cfg(test)]
mod interval_tests {
    use crate::interpreter::interval_ops::value_to_interval;
    use crate::interpreter::Interpreter;
    use crate::types::fraction::Fraction;

    #[tokio::test]
    async fn test_interval_creation_success_and_failure() {
        let mut interp = Interpreter::new();
        interp.execute("'math' IMPORT 1 2 INTERVAL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(format!("{}", stack[0]), "[1/1, 2/1]");

        let mut interp_fail = Interpreter::new();
        let result = interp_fail.execute("'math' IMPORT 2 1 INTERVAL").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_interval_basic_arithmetic() {
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 1 2 INTERVAL 3 4 INTERVAL +")
            .await
            .unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[4/1, 6/1]");

        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 1 2 INTERVAL 3 4 INTERVAL -")
            .await
            .unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[-3/1, -1/1]");

        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 1 2 INTERVAL 3 4 INTERVAL *")
            .await
            .unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[3/1, 8/1]");

        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT -1 2 INTERVAL 3 4 INTERVAL *")
            .await
            .unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[-4/1, 8/1]");

        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 2 4 INTERVAL 1 2 INTERVAL /")
            .await
            .unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[1/1, 4/1]");
    }

    #[tokio::test]
    async fn test_interval_division_by_zero_interval_bubbles() {
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 1 2 INTERVAL -1 1 INTERVAL /")
            .await
            .unwrap();
        assert!(interp.get_stack().last().unwrap().is_nil());
    }

    #[tokio::test]
    async fn test_sqrt_exact_cases() {
        let mut interp = Interpreter::new();
        interp.execute("'math' IMPORT 4 SQRT").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "2/1");

        let mut interp = Interpreter::new();
        interp.execute("'math' IMPORT 9/16 SQRT").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "3/4");
    }

    #[tokio::test]
    async fn test_sqrt_interval_soundness_and_eps() {
        // SQRT now returns an exact AlgebraicSqrt for rational scalar inputs
        // instead of an interval. Verify the result is an ExactScalar.
        let mut interp = Interpreter::new();
        interp.execute("'math' IMPORT 2 SQRT").await.unwrap();
        let val = &interp.get_stack()[0];
        assert!(
            matches!(&val.data, crate::types::ValueData::ExactScalar(er)
                if er.sqrt_radicand().map(|r| *r == crate::types::fraction::Fraction::from(2)).unwrap_or(false)),
            "SQRT(2) must be ExactScalar(AlgebraicSqrt {{ radicand: 2/1 }}), got: {val}"
        );

        // SQRT-EPS still returns an interval (interval path unchanged)
        let mut interp_eps = Interpreter::new();
        interp_eps
            .execute("'math' IMPORT 2 1/100 SQRT-EPS")
            .await
            .unwrap();
        let iv_eps =
            value_to_interval(&interp_eps.get_stack()[0]).expect("sqrt_eps(2) must be interval");
        assert!(iv_eps.width().le(&Fraction::new(1.into(), 100.into())));
    }

    #[tokio::test]
    async fn test_sqrt_interval_monotonicity() {
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 1 4 INTERVAL SQRT")
            .await
            .unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[1/1, 2/1]");
    }

    #[tokio::test]
    async fn test_interval_comparison_policy() {
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 1 2 INTERVAL 3 4 INTERVAL <")
            .await
            .unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "1/1");

        let mut interp_undetermined = Interpreter::new();
        interp_undetermined
            .execute("'math' IMPORT 2 3 INTERVAL 3 4 INTERVAL <")
            .await
            .unwrap();
        // SPEC §7.4.1 (revised): an undecidable comparison yields the logical
        // truth value `Unknown` (U), observed as `truthValue = unknown`, not
        // a `reason = undecidable` NIL.
        let result = &interp_undetermined.get_stack()[0];
        assert!(
            result.is_unknown(),
            "overlapping interval comparison projects to the logical Unknown"
        );
        assert_eq!(result.truth_value(), Some("unknown"));
        assert_eq!(format!("{}", result), "UNKNOWN");

        let mut interp_eq = Interpreter::new();
        interp_eq
            .execute("'math' IMPORT 1 5 INTERVAL 2 4 INTERVAL =")
            .await
            .unwrap();
        assert_eq!(format!("{}", interp_eq.get_stack()[0]), "0/1");
    }

    #[tokio::test]
    async fn test_mixed_arithmetic() {
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 1 2 3 INTERVAL +")
            .await
            .unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[3/1, 4/1]");

        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 2 3 5 INTERVAL *")
            .await
            .unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[6/1, 10/1]");
    }
}

#[cfg(test)]
mod nil_passthrough_tests {
    use crate::interpreter::Interpreter;

    async fn run(source: &str) -> Interpreter {
        let mut interp = Interpreter::new();
        interp.execute(source).await.unwrap();
        interp
    }

    #[tokio::test]
    async fn add_with_nil_left_yields_nil() {
        let interp = run("NIL 3 +").await;
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_nil(), "got {}", stack[0]);
    }

    #[tokio::test]
    async fn add_with_nil_right_yields_nil() {
        let interp = run("3 NIL +").await;
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_nil(), "got {}", stack[0]);
    }

    #[tokio::test]
    async fn sub_mul_div_with_nil_yield_nil() {
        let interp = run("NIL 5 -").await;
        assert!(interp.get_stack()[0].is_nil());
        let interp = run("NIL 5 *").await;
        assert!(interp.get_stack()[0].is_nil());
        let interp = run("NIL 5 /").await;
        assert!(interp.get_stack()[0].is_nil());
    }

    #[tokio::test]
    async fn div_by_nil_does_not_raise_division_by_zero() {
        let interp = run("5 NIL /").await;
        assert!(interp.get_stack()[0].is_nil());
    }

    #[tokio::test]
    async fn mod_with_nil_yields_nil() {
        let interp = run("NIL 3 MOD").await;
        assert!(interp.get_stack()[0].is_nil());
        let interp = run("3 NIL MOD").await;
        assert!(interp.get_stack()[0].is_nil());
    }

    #[tokio::test]
    async fn floor_ceil_round_of_nil_yield_nil() {
        let interp = run("NIL FLOOR").await;
        assert!(interp.get_stack()[0].is_nil());
        let interp = run("NIL CEIL").await;
        assert!(interp.get_stack()[0].is_nil());
        let interp = run("NIL ROUND").await;
        assert!(interp.get_stack()[0].is_nil());
    }

    #[tokio::test]
    async fn comparisons_with_nil_yield_nil() {
        let interp = run("NIL 3 <").await;
        assert!(interp.get_stack()[0].is_nil());
        let interp = run("3 NIL <=").await;
        assert!(interp.get_stack()[0].is_nil());
        let interp = run("NIL NIL =").await;
        assert!(interp.get_stack()[0].is_nil());
        let interp = run("NIL 3 >").await;
        assert!(interp.get_stack()[0].is_nil());
        let interp = run("3 NIL >=").await;
        assert!(interp.get_stack()[0].is_nil());
        let interp = run("NIL 3 <>").await;
        assert!(interp.get_stack()[0].is_nil());
    }

    #[tokio::test]
    async fn safe_divide_then_add_propagates_nil_through_pipeline() {
        let interp = run("[ 10 ] [ 0 ] ~ / 1 +").await;
        let stack = interp.get_stack();
        assert!(
            stack.last().unwrap().is_nil(),
            "expected NIL on top of stack after safe-divide and add; got {}",
            stack.last().unwrap()
        );
    }

    #[tokio::test]
    async fn or_nil_can_supply_fallback_after_passthrough() {
        let interp = run("[ 10 ] [ 0 ] ~ / 1 + 0 =>").await;
        let stack = interp.get_stack();
        assert!(
            !stack.last().unwrap().is_nil(),
            "OR-NIL should have replaced NIL with the fallback; got {}",
            stack.last().unwrap()
        );
    }
}

#[cfg(test)]
mod ai_first_comparison_tests {
    //! Tests for the AI-first comparison primitives GT, GTE, NEQ. These mirror
    //! LT / LTE / EQ and exist so an automated producer can emit the relation
    //! that matches its intent directly rather than rewriting it as a
    //! negation or operand swap.

    use crate::interpreter::Interpreter;

    async fn run(source: &str) -> Interpreter {
        let mut interp = Interpreter::new();
        interp.execute(source).await.unwrap();
        interp
    }

    fn bool_of(interp: &Interpreter) -> bool {
        // Boolean values are stored as Scalar(0|1) with a Boolean display
        // hint; the underlying Display impl prints the scalar.
        let v = &interp.get_stack()[0];
        let s = format!("{}", v);
        match s.as_str() {
            "1" | "1/1" => true,
            "0" | "0/1" => false,
            other => panic!("expected boolean (0 or 1), got {}", other),
        }
    }

    // ── canonical-name parity with LT/LTE/EQ ─────────────────────────────

    #[tokio::test]
    async fn gt_canonical_name_returns_true_when_strictly_greater() {
        let interp = run("2 1 GT").await;
        assert!(bool_of(&interp));
    }

    #[tokio::test]
    async fn gt_returns_false_on_equal_values() {
        let interp = run("1 1 GT").await;
        assert!(!bool_of(&interp));
    }

    #[tokio::test]
    async fn gte_canonical_name_returns_true_on_equal_values() {
        let interp = run("1 1 GTE").await;
        assert!(bool_of(&interp));
    }

    #[tokio::test]
    async fn gte_returns_false_when_strictly_less() {
        let interp = run("0 1 GTE").await;
        assert!(!bool_of(&interp));
    }

    #[tokio::test]
    async fn neq_canonical_name_returns_true_when_different() {
        let interp = run("1 2 NEQ").await;
        assert!(bool_of(&interp));
    }

    #[tokio::test]
    async fn neq_returns_false_when_equal() {
        let interp = run("3 3 NEQ").await;
        assert!(!bool_of(&interp));
    }

    // ── symbol-alias parity ──────────────────────────────────────────────

    #[tokio::test]
    async fn gt_symbol_alias_matches_canonical() {
        let interp = run("5 3 >").await;
        assert!(bool_of(&interp));
    }

    #[tokio::test]
    async fn gte_symbol_alias_matches_canonical() {
        let interp = run("3 3 >=").await;
        assert!(bool_of(&interp));
    }

    #[tokio::test]
    async fn neq_symbol_alias_matches_canonical() {
        let interp = run("1 2 <>").await;
        assert!(bool_of(&interp));
    }

    // ── exact rational comparison ────────────────────────────────────────

    #[tokio::test]
    async fn gt_compares_fractions_exactly() {
        let interp = run("7/2 17/5 GT").await;
        // 7/2 = 35/10, 17/5 = 34/10, so 7/2 > 17/5.
        assert!(bool_of(&interp));
    }

    // ── NEQ structural equality on vectors ───────────────────────────────

    #[tokio::test]
    async fn neq_returns_false_for_structurally_equal_vectors() {
        let interp = run("[ 1 2 3 ] [ 1 2 3 ] NEQ").await;
        assert!(!bool_of(&interp));
    }

    #[tokio::test]
    async fn neq_returns_true_for_structurally_different_vectors() {
        let interp = run("[ 1 2 3 ] [ 1 2 4 ] NEQ").await;
        assert!(bool_of(&interp));
    }

    // ── interval undecidability mirrors LT/LE ────────────────────────────

    #[tokio::test]
    async fn gt_on_disjoint_intervals_decides_true() {
        let interp = run("'math' IMPORT 3 4 INTERVAL 1 2 INTERVAL GT").await;
        let s = format!("{}", interp.get_stack()[0]);
        assert_eq!(s, "1/1");
    }

    #[tokio::test]
    async fn gte_on_disjoint_intervals_decides_false() {
        let interp = run("'math' IMPORT 0 1 INTERVAL 2 3 INTERVAL GTE").await;
        let s = format!("{}", interp.get_stack()[0]);
        assert_eq!(s, "0/1");
    }

    #[tokio::test]
    async fn gt_on_overlapping_intervals_projects_unknown() {
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 2 3 INTERVAL 2 4 INTERVAL GT")
            .await
            .unwrap();
        // SPEC §7.4.1 (revised): undecidable comparison yields the logical
        // truth value `Unknown` (U), not a `reason = undecidable` NIL.
        let result = &interp.get_stack()[0];
        assert!(
            result.is_unknown(),
            "overlapping interval GT yields Unknown"
        );
        assert_eq!(result.truth_value(), Some("unknown"));
    }

    #[tokio::test]
    async fn finite_cf_comparison_always_decides() {
        for (code, expected) in [("1 1 EQ", "1/1"), ("1 2 EQ", "0/1"), ("1 2 LT", "1/1")] {
            let mut interp = Interpreter::new();
            interp.execute(code).await.unwrap();
            let v = &interp.get_stack()[0];
            assert!(!v.is_unknown(), "`{code}` must decide, not be Unknown");
            assert_eq!(format!("{}", v), expected, "`{code}`");
        }
    }

    #[tokio::test]
    async fn equal_irrationals_compare_unknown() {
        // sqrt(2) - sqrt(2) is, structurally, a Gosper node the budget cannot
        // distinguish from 0; EQ / LT against 0 therefore yield Unknown (U).
        for code in ["2 SQRT 2 SQRT SUB 0 EQ", "2 SQRT 2 SQRT SUB 0 LT"] {
            let mut interp = Interpreter::new();
            interp
                .execute(&format!("'math' IMPORT {code}"))
                .await
                .unwrap();
            let v = &interp.get_stack()[0];
            assert!(v.is_unknown(), "`{code}` must be the logical Unknown");
            assert_eq!(v.truth_value(), Some("unknown"), "`{code}`");
            assert_eq!(format!("{}", v), "UNKNOWN", "`{code}`");
        }
    }

    #[tokio::test]
    async fn distinct_irrationals_decide_at_finite_prefix() {
        for (code, expected) in [("2 SQRT 3 SQRT EQ", "0/1"), ("2 SQRT 3 SQRT LT", "1/1")] {
            let mut interp = Interpreter::new();
            interp
                .execute(&format!("'math' IMPORT {code}"))
                .await
                .unwrap();
            let v = &interp.get_stack()[0];
            assert!(!v.is_unknown(), "`{code}` must decide, not be Unknown");
            assert_eq!(format!("{}", v), expected, "`{code}`");
        }
    }

    // ── NIL passthrough for the new ops (contract: nil_policy = Passthrough)

    #[tokio::test]
    async fn gt_with_nil_left_yields_nil() {
        let interp = run("NIL 1 GT").await;
        assert!(interp.get_stack()[0].is_nil());
    }

    #[tokio::test]
    async fn gte_with_nil_right_yields_nil() {
        let interp = run("1 NIL GTE").await;
        assert!(interp.get_stack()[0].is_nil());
    }

    #[tokio::test]
    async fn neq_with_two_nils_yields_nil() {
        // NEQ is NIL-passthrough, so NIL NEQ NIL is NIL — *not* FALSE.
        // (NIL is an absence value, not a member of an equivalence class.)
        let interp = run("NIL NIL <>").await;
        assert!(interp.get_stack()[0].is_nil());
    }

    // ── stack-mode sequence properties ───────────────────────────────────

    #[tokio::test]
    async fn gt_stack_mode_holds_for_strictly_decreasing_sequence() {
        let interp = run("5 4 3 2 4 .. GT").await;
        assert!(bool_of(&interp));
    }

    #[tokio::test]
    async fn gt_stack_mode_false_when_not_strictly_decreasing() {
        let interp = run("5 4 4 3 4 .. GT").await;
        assert!(!bool_of(&interp));
    }

    #[tokio::test]
    async fn gte_stack_mode_holds_for_nonincreasing_sequence() {
        let interp = run("5 4 4 3 4 .. GTE").await;
        assert!(bool_of(&interp));
    }

    #[tokio::test]
    async fn neq_stack_mode_holds_when_all_adjacent_pairs_differ() {
        let interp = run("1 2 3 1 4 .. NEQ").await;
        assert!(bool_of(&interp));
    }

    #[tokio::test]
    async fn neq_stack_mode_false_when_two_adjacent_values_match() {
        let interp = run("1 2 2 3 4 .. NEQ").await;
        assert!(!bool_of(&interp));
    }

    // ── KEEP modifier preserves operands ─────────────────────────────────

    #[tokio::test]
    async fn gt_keep_mode_preserves_both_operands() {
        let interp = run("2 1 ,, GT").await;
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 3, "KEEP must retain both operands plus result");
    }

    // ── SAFE projection preserves error category on malformed input ──────

    #[tokio::test]
    async fn gt_safe_mode_projects_structure_error_to_nil() {
        // Comparing a code block against a number is malformed → SAFE catches it.
        let interp = run("{ 1 } 1 ~ GT").await;
        assert!(interp.get_stack().last().unwrap().is_nil());
    }
}

#[cfg(test)]
mod comparison_budget_infrastructure_tests {
    //! Phase 6 infrastructure for SPEC §7.4.1's partial-quotient
    //! budget. Every Ajisai scalar currently on the stack is still
    //! a `Fraction`, so the ordering ops always decide and never
    //! project Undecidable. These tests pin the *current* behavior
    //! against regression as the refactor lands, and assert that the
    //! Undecidable / ComparisonBudget plumbing (NilReason +
    //! AbsenceOrigin) is wired correctly so Phase 7's non-Rational
    //! ExactReals will surface NIL with the right metadata when they
    //! exhaust the budget.
    use crate::error::NilReason;
    use crate::interpreter::Interpreter;
    use crate::semantic::AbsenceOrigin;
    use crate::types::Value;

    async fn run(source: &str) -> Interpreter {
        let mut interp = Interpreter::new();
        interp.execute(source).await.unwrap();
        interp
    }

    fn bool_of(interp: &Interpreter) -> bool {
        let v = &interp.get_stack()[0];
        let s = format!("{}", v);
        match s.as_str() {
            "1" | "1/1" => true,
            "0" | "0/1" => false,
            other => panic!("expected boolean (0 or 1), got {}", other),
        }
    }

    // ── Regression: every ordering decides on rational operands ──────────

    #[tokio::test]
    async fn lt_decides_on_rational_pair() {
        let interp = run("1/2 2/3 LT").await;
        assert!(bool_of(&interp));
    }

    #[tokio::test]
    async fn lte_decides_on_equal_reduced_rationals() {
        let interp = run("2/4 1/2 LTE").await;
        assert!(bool_of(&interp));
    }

    #[tokio::test]
    async fn gt_decides_on_negative_left() {
        let interp = run("-3/2 1/2 GT").await;
        assert!(!bool_of(&interp));
    }

    #[tokio::test]
    async fn gte_decides_on_large_rationals() {
        let interp = run("355/113 22/7 GTE").await;
        // 355/113 ≈ 3.14159292 < 22/7 ≈ 3.14285714 ⇒ GTE is false.
        assert!(!bool_of(&interp));
    }

    // ── Regression: STAK-mode property checks still produce a single bool

    #[tokio::test]
    async fn stak_lt_monotonic_sequence_is_true() {
        let interp = run("1 2 3 5 8 5 .. LT").await;
        assert!(bool_of(&interp));
    }

    #[tokio::test]
    async fn stak_lt_non_monotonic_sequence_is_false() {
        let interp = run("1 3 2 4 4 .. LT").await;
        assert!(!bool_of(&interp));
    }

    #[tokio::test]
    async fn stak_gte_non_increasing_sequence_is_true() {
        let interp = run("5 5 3 1 0 5 .. GTE").await;
        assert!(bool_of(&interp));
    }

    // ── NIL projection contract for the Undecidable case ─────────────────

    #[tokio::test]
    async fn undecidable_nil_carries_comparison_budget_origin() {
        // We can't yet drive the comparison path into the Undecidable
        // branch via runtime source (no non-Rational ExactReal scalar
        // is constructable yet — Phase 7 introduces that), so this
        // test pins the helper that the comparison.rs refactor calls:
        // building NIL with reason `Undecidable` must yield the
        // §7.4.1 origin `ComparisonBudget`.
        let v = Value::nil_with_reason(NilReason::Undecidable);
        let absence = v.absence_metadata().expect("nil carries absence");
        assert_eq!(absence.reason, Some(NilReason::Undecidable));
        assert_eq!(absence.origin, AbsenceOrigin::ComparisonBudget);
    }

    // ── NIL passthrough is unchanged ─────────────────────────────────────

    #[tokio::test]
    async fn lt_with_left_nil_passes_nil_through() {
        let interp = run("NIL 1 LT").await;
        assert!(interp.get_stack()[0].is_nil());
    }

    #[tokio::test]
    async fn lt_with_right_nil_passes_nil_through() {
        let interp = run("1 NIL LT").await;
        assert!(interp.get_stack()[0].is_nil());
    }
}

/// Phase 7 — EQ / NEQ Undecidable-NIL plumbing.
///
/// Phase 6 (PR #904) wired the `Undecidable` / `ComparisonBudget`
/// projection through the ordering path (`LT` / `LTE` / `GT` /
/// `GTE`) and explicitly left `EQ` / `NEQ` for Phase 7. This module
/// pins the new dispatch shape:
///
/// 1. `pairwise_eq` is three-valued (`Option<bool>`): rational
///    operands always decide; non-Rational `ExactReal` operands run
///    through `ExactReal::eq_with_budget` and may surface `None`.
/// 2. `apply_equality` projects `None` to the §7.4.1 Undecidable
///    NIL via the existing `push_undecidable_nil` helper.
/// 3. STAK-mode `EQ` / `NEQ` short-circuit on the first
///    NIL-producing pair (SPEC §7.4).
///
/// We can't yet construct a non-Rational `ExactReal` scalar value
/// from Ajisai source — `ValueData::Scalar` is still `Fraction`-
/// backed — so these tests:
///
/// * regress the rational-operand fast path through EQ / NEQ for
///   value equality, reduced-form equality, and structural fallback;
/// * pin the dispatch helpers (`ExactReal::eq_with_budget`) so the
///   non-Rational branch is exercised at the type-level boundary
///   that `apply_equality` will route through once subsequent phases
///   replace the scalar storage.
#[cfg(test)]
mod phase_seven_eq_budget_tests {
    use crate::error::NilReason;
    use crate::interpreter::Interpreter;
    use crate::semantic::AbsenceOrigin;
    use crate::types::continued_fraction::{ExactReal, DEFAULT_COMPARISON_BUDGET};
    use crate::types::fraction::Fraction;
    use num_bigint::BigInt;

    async fn run(source: &str) -> Interpreter {
        let mut interp = Interpreter::new();
        interp.execute(source).await.unwrap();
        interp
    }

    fn bool_of(interp: &Interpreter) -> bool {
        let v = &interp.get_stack()[0];
        let s = format!("{}", v);
        match s.as_str() {
            "1" | "1/1" => true,
            "0" | "0/1" => false,
            other => panic!("expected boolean (0 or 1), got {}", other),
        }
    }

    // ── Regression: EQ / NEQ still decide on rationals ───────────────────

    #[tokio::test]
    async fn eq_decides_value_equal_reduced_rationals() {
        let interp = run("2/4 1/2 EQ").await;
        assert!(bool_of(&interp));
    }

    #[tokio::test]
    async fn eq_decides_unequal_rationals() {
        let interp = run("1/2 2/3 EQ").await;
        assert!(!bool_of(&interp));
    }

    #[tokio::test]
    async fn neq_decides_unequal_rationals() {
        let interp = run("1/2 2/3 NEQ").await;
        assert!(bool_of(&interp));
    }

    #[tokio::test]
    async fn neq_decides_equal_reduced_rationals() {
        let interp = run("2/4 1/2 NEQ").await;
        assert!(!bool_of(&interp));
    }

    #[tokio::test]
    async fn eq_decides_large_rationals() {
        let interp = run("355/113 355/113 EQ").await;
        assert!(bool_of(&interp));
    }

    #[tokio::test]
    async fn eq_decides_negative_vs_positive() {
        let interp = run("-1/2 1/2 EQ").await;
        assert!(!bool_of(&interp));
    }

    // ── STAK-mode regression ─────────────────────────────────────────────

    #[tokio::test]
    async fn stak_eq_all_equal_is_true() {
        let interp = run("2/4 1/2 4/8 3 .. EQ").await;
        assert!(bool_of(&interp));
    }

    #[tokio::test]
    async fn stak_eq_with_one_distinct_is_false() {
        let interp = run("1/2 1/2 2/3 3 .. EQ").await;
        assert!(!bool_of(&interp));
    }

    #[tokio::test]
    async fn stak_neq_all_adjacent_unequal_is_true() {
        let interp = run("1 2 3 4 4 .. NEQ").await;
        assert!(bool_of(&interp));
    }

    #[tokio::test]
    async fn stak_neq_with_adjacent_duplicate_is_false() {
        let interp = run("1 2 2 3 4 .. NEQ").await;
        assert!(!bool_of(&interp));
    }

    // ── NIL passthrough is unchanged ─────────────────────────────────────

    #[tokio::test]
    async fn eq_with_left_nil_passes_nil_through() {
        let interp = run("NIL 1 EQ").await;
        assert!(interp.get_stack()[0].is_nil());
    }

    #[tokio::test]
    async fn neq_with_right_nil_passes_nil_through() {
        let interp = run("1 NIL NEQ").await;
        assert!(interp.get_stack()[0].is_nil());
    }

    // ── ExactReal-level dispatch boundary (Phase 7 hook) ─────────────────
    //
    // These cover the budgeted CF path that `pairwise_eq` /
    // `scalar_pair_eq` routes through whenever at least one operand
    // is non-Rational. `eq_with_budget` is exercised here so the
    // dispatch boundary is pinned even before a runtime path can
    // place such a value on the stack.

    fn rational(n: i64, d: i64) -> ExactReal {
        ExactReal::Rational(Fraction::new(BigInt::from(n), BigInt::from(d)))
    }

    #[test]
    fn exact_real_eq_with_budget_decides_equal_rationals() {
        assert_eq!(
            rational(2, 4).eq_with_budget(&rational(1, 2), DEFAULT_COMPARISON_BUDGET),
            Some(true)
        );
    }

    #[test]
    fn exact_real_eq_with_budget_decides_unequal_rationals() {
        assert_eq!(
            rational(1, 2).eq_with_budget(&rational(2, 3), DEFAULT_COMPARISON_BUDGET),
            Some(false)
        );
    }

    #[test]
    fn exact_real_eq_with_budget_decides_rational_vs_algebraic_sqrt() {
        // √2 is irrational; it can never equal 7/5 (or any rational).
        // The CF streams diverge in fewer than `DEFAULT_COMPARISON_BUDGET`
        // steps, so the result is decidable.
        let sqrt_two =
            ExactReal::from_sqrt_rational(Fraction::new(BigInt::from(2), BigInt::from(1)))
                .expect("sqrt(2) constructible");
        assert_eq!(
            sqrt_two.eq_with_budget(&rational(7, 5), DEFAULT_COMPARISON_BUDGET),
            Some(false)
        );
    }

    // ── Undecidable-NIL helper still has the §7.4.1 origin ───────────────
    //
    // `apply_equality` projects the `None` branch through
    // `push_undecidable_nil` — the same helper the ordering path
    // already uses. The contract is identical, so any future EQ /
    // NEQ Undecidable NIL surfaces the §7.4.1 metadata.

    #[tokio::test]
    async fn eq_undecidable_nil_carries_comparison_budget_origin() {
        let v = crate::types::Value::nil_with_reason(NilReason::Undecidable);
        let absence = v.absence_metadata().expect("nil carries absence");
        assert_eq!(absence.reason, Some(NilReason::Undecidable));
        assert_eq!(absence.origin, AbsenceOrigin::ComparisonBudget);
    }
}

/// SPEC §7.4.2 — `COMPARE-WITHIN` and the §4.5.0 `agreedPrefix` diagnosis.
///
/// `COMPARE-WITHIN` ( `[ a ] [ b ] [ budget ] -> [ -1 | 0 | 1 | UNKNOWN ]` )
/// makes the partial-quotient budget a first-class, user-controlled
/// parameter. The decided result is the exact sign of `a − b`; the
/// budget-undecided result is the logical `Unknown` (U) carrying
/// `diagnosis.agreedPrefix`, the number of leading partial quotients that
/// matched before the budget was exhausted.
#[cfg(test)]
mod compare_within_tests {
    use crate::interpreter::Interpreter;
    use crate::types::continued_fraction::{CmpOutcome, ExactReal};
    use crate::types::fraction::Fraction;
    use num_bigint::BigInt;

    async fn run(source: &str) -> Interpreter {
        let mut interp = Interpreter::new();
        interp.execute(source).await.unwrap();
        interp
    }

    fn rational(n: i64, d: i64) -> ExactReal {
        ExactReal::Rational(Fraction::new(BigInt::from(n), BigInt::from(d)))
    }

    // ── Unit level: the tracked three-way compare reports the prefix ─────

    #[test]
    fn tracked_undecided_reports_agreed_prefix_equal_to_budget() {
        // CF(1/2) = [0; 2], CF(1/3) = [0; 3]. They share index 0 (both 0)
        // and first differ at index 1. With budget 1 only index 0 is
        // consumed, so the order is undecided and the agreed prefix is the
        // full consumed budget, 1.
        assert_eq!(
            rational(1, 2).cmp_with_budget_tracked(&rational(1, 3), 1),
            CmpOutcome::Undecided { agreed_prefix: 1 }
        );
    }

    #[test]
    fn tracked_decides_when_budget_reaches_divergence() {
        // The same pair decides at budget 2 (index 1 differs: 2 vs 3),
        // 1/2 > 1/3.
        assert_eq!(
            rational(1, 2).cmp_with_budget_tracked(&rational(1, 3), 2),
            CmpOutcome::Decided(std::cmp::Ordering::Greater)
        );
    }

    #[test]
    fn tracked_equal_finite_decides_when_budget_reaches_termination() {
        // CF(1/2) = CF(2/4) = [0; 2]: index 0 = 0, index 1 = 2, then both
        // streams end at index 2. The raw tracked compare has no Fraction
        // fast path, so it only decides Equal once the budget reaches the
        // shared termination (budget >= 3); below that it is genuinely
        // undecided, reporting the matched prefix. (The COMPARE-WITHIN word
        // adds a finite fast path that decides regardless of budget — see
        // `compare_within_finite_decides_even_at_budget_one`.)
        assert_eq!(
            rational(1, 2).cmp_with_budget_tracked(&rational(2, 4), 3),
            CmpOutcome::Decided(std::cmp::Ordering::Equal)
        );
        assert_eq!(
            rational(1, 2).cmp_with_budget_tracked(&rational(2, 4), 2),
            CmpOutcome::Undecided { agreed_prefix: 2 }
        );
    }

    // ── Source level: decided signs ─────────────────────────────────────

    #[tokio::test]
    async fn compare_within_yields_minus_one_when_less() {
        let interp = run("1 2 16 COMPARE-WITHIN").await;
        assert_eq!(format!("{}", interp.get_stack()[0]), "-1/1");
    }

    #[tokio::test]
    async fn compare_within_yields_zero_when_equal() {
        let interp = run("2 2 16 COMPARE-WITHIN").await;
        assert_eq!(format!("{}", interp.get_stack()[0]), "0/1");
    }

    #[tokio::test]
    async fn compare_within_yields_one_when_greater() {
        let interp = run("2 1 16 COMPARE-WITHIN").await;
        assert_eq!(format!("{}", interp.get_stack()[0]), "1/1");
    }

    #[tokio::test]
    async fn compare_within_finite_decides_even_at_budget_one() {
        // Two finite rationals differ at a bounded index, so they decide
        // regardless of how small the budget is (SPEC §7.4.2).
        let interp = run("1/3 1/2 1 COMPARE-WITHIN").await;
        assert_eq!(format!("{}", interp.get_stack()[0]), "-1/1");
    }

    // ── Source level: budget-undecided → Unknown with agreedPrefix ──────

    #[tokio::test]
    async fn compare_within_equal_irrationals_yield_unknown_with_prefix() {
        // √2 − √2 is a Gosper node the budget cannot distinguish from 0,
        // so comparing it against 0 never decides → logical Unknown (U).
        let interp = run("'math' IMPORT 2 SQRT 2 SQRT SUB 0 8 COMPARE-WITHIN").await;
        let v = &interp.get_stack()[0];
        assert!(
            v.is_unknown(),
            "equal irrationals must yield Unknown, got {v}"
        );
        assert_eq!(v.truth_value(), Some("unknown"));

        // The Unknown result carries the machine-readable agreedPrefix.
        let absence = v.absence_metadata().expect("U carries absence metadata");
        let diagnosis = absence
            .diagnosis
            .as_ref()
            .expect("COMPARE-WITHIN U carries a diagnosis");
        let prefix = diagnosis
            .agreed_prefix
            .expect("diagnosis carries agreedPrefix");
        assert!(
            prefix <= 8,
            "agreedPrefix must not exceed the consumed budget, got {prefix}"
        );
    }

    // ── Source level: NIL passthrough (SPEC §7.12) ──────────────────────

    #[tokio::test]
    async fn compare_within_nil_left_passes_nil_through() {
        let interp = run("NIL 1 8 COMPARE-WITHIN").await;
        assert!(interp.get_stack()[0].is_nil());
    }

    #[tokio::test]
    async fn compare_within_nil_right_passes_nil_through() {
        let interp = run("1 NIL 8 COMPARE-WITHIN").await;
        assert!(interp.get_stack()[0].is_nil());
    }

    // ── Source level: malformed budget / operand → error (not U) ────────

    #[tokio::test]
    async fn compare_within_zero_budget_errors() {
        let mut interp = Interpreter::new();
        let result = interp.execute("1 2 0 COMPARE-WITHIN").await;
        assert!(result.is_err(), "zero budget is malformed use");
    }

    #[tokio::test]
    async fn compare_within_negative_budget_errors() {
        let mut interp = Interpreter::new();
        let result = interp.execute("1 2 -4 COMPARE-WITHIN").await;
        assert!(result.is_err(), "negative budget is malformed use");
    }

    #[tokio::test]
    async fn compare_within_non_numeric_operand_errors() {
        let mut interp = Interpreter::new();
        let result = interp.execute("{ 1 } 2 8 COMPARE-WITHIN").await;
        assert!(result.is_err(), "non-numeric operand is malformed use");
    }
}

#[cfg(test)]
mod ragged_broadcast_tests {
    use crate::interpreter::Interpreter;

    async fn eval(code: &str) -> String {
        let mut interp = Interpreter::new();
        interp.execute(code).await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1, "expected single result for: {code}");
        format!("{}", stack[0])
    }

    #[tokio::test]
    async fn test_scalar_mul_over_mixed_nested_vector() {
        let result = eval("[ 10 [ 1 2 3 ] 10 ] 10 *").await;
        assert_eq!(result, "[ 100/1 [ 10/1 20/1 30/1 ] 100/1 ]");
    }

    #[tokio::test]
    async fn test_scalar_mul_left_operand() {
        let result = eval("10 [ 10 [ 1 2 3 ] 10 ] *").await;
        assert_eq!(result, "[ 100/1 [ 10/1 20/1 30/1 ] 100/1 ]");
    }

    #[tokio::test]
    async fn test_scalar_add_over_mixed_nested_vector() {
        let result = eval("[ 1 [ 2 3 ] 4 ] 10 +").await;
        assert_eq!(result, "[ 11/1 [ 12/1 13/1 ] 14/1 ]");
    }

    #[tokio::test]
    async fn test_deeply_nested_ragged() {
        let result = eval("[ 1 [ 2 [ 3 4 ] ] ] 2 *").await;
        assert_eq!(result, "[ 2/1 [ 4/1 [ 6/1 8/1 ] ] ]");
    }

    #[tokio::test]
    async fn test_elementwise_ragged_same_structure() {
        let result = eval("[ 1 [ 2 3 ] ] [ 10 [ 20 30 ] ] *").await;
        assert_eq!(result, "[ 10/1 [ 40/1 90/1 ] ]");
    }

    #[tokio::test]
    async fn test_ragged_length_mismatch_errors() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 [ 2 3 ] ] [ 10 [ 20 30 40 ] ] *").await;
        assert!(result.is_err(), "mismatched nested lengths should error");
    }

    #[tokio::test]
    async fn test_regular_nested_still_works() {
        let result = eval("[ [ 1 2 ] [ 3 4 ] ] 10 *").await;
        assert_eq!(result, "[ [ 10/1 20/1 ] [ 30/1 40/1 ] ]");
    }

    #[tokio::test]
    async fn test_singleton_vector_sibling_preserved() {
        let result = eval("[ [ 1 ] 2 ] 10 *").await;
        assert_eq!(result, "[ [ 10/1 ] 20/1 ]");
    }
}

#[cfg(test)]
mod ragged_unary_tests {
    use crate::interpreter::Interpreter;

    async fn eval(code: &str) -> String {
        let mut interp = Interpreter::new();
        interp.execute(code).await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1, "expected single result for: {code}");
        format!("{}", stack[0])
    }

    #[tokio::test]
    async fn test_floor_over_mixed_nested_vector() {
        let result = eval("[ 7/2 [ 5/2 9/4 ] 3/2 ] FLOOR").await;
        assert_eq!(result, "[ 3/1 [ 2/1 2/1 ] 1/1 ]");
    }

    #[tokio::test]
    async fn test_not_over_mixed_nested_vector() {
        let result = eval("[ 0 [ 1 0 ] 5 ] NOT").await;
        assert_eq!(result, "[ 1/1 [ 0/1 1/1 ] 0/1 ]");
    }

    #[tokio::test]
    async fn test_mod_over_mixed_nested_vector() {
        let result = eval("[ 10 [ 7 8 ] 9 ] 3 %").await;
        assert_eq!(result, "[ 1/1 [ 1/1 2/1 ] 0/1 ]");
    }
}

#[cfg(test)]
mod ragged_equality_tests {
    use crate::types::Value;

    #[test]
    fn ragged_vector_not_equal_to_dense_tensor() {
        // Dense tensor [1 2] must not equal ragged nested vector [[1] 2].
        let dense = Value::from_tensor(vec![1i64.into(), 2i64.into()], vec![2]);
        let ragged = Value::from_children(vec![
            Value::from_children(vec![Value::from_int(1)]),
            Value::from_int(2),
        ]);
        assert_ne!(dense, ragged);
        assert_ne!(ragged, dense);
    }

    #[test]
    fn matching_nested_vector_equals_dense_tensor() {
        let dense = Value::from_tensor(
            vec![1i64.into(), 2i64.into(), 3i64.into(), 4i64.into()],
            vec![2, 2],
        );
        let nested = Value::from_children(vec![
            Value::from_children(vec![Value::from_int(1), Value::from_int(2)]),
            Value::from_children(vec![Value::from_int(3), Value::from_int(4)]),
        ]);
        assert_eq!(dense, nested);
    }
}

#[cfg(test)]
mod exact_scalar_tests {
    use crate::interpreter::Interpreter;
    use crate::types::continued_fraction::ExactReal;
    use crate::types::fraction::Fraction;
    use crate::types::ValueData;
    use num_bigint::BigInt;

    #[tokio::test]
    async fn sqrt_of_perfect_square_is_exact_rational() {
        // 4 MATH@SQRT = 2/1 exactly (Fraction fast path, not ExactScalar)
        let mut interp = Interpreter::new();
        interp.execute("'math' IMPORT 4 SQRT").await.unwrap();
        let f = interp.get_stack()[0]
            .as_scalar()
            .expect("SQRT(4) must be exact rational scalar");
        assert_eq!(*f, Fraction::new(BigInt::from(2), BigInt::from(1)));
    }

    #[tokio::test]
    async fn sqrt_of_irrational_produces_exact_scalar() {
        // 2 MATH@SQRT → ExactScalar(AlgebraicSqrt { radicand: 2/1 })
        let mut interp = Interpreter::new();
        interp.execute("'math' IMPORT 2 SQRT").await.unwrap();
        let val = &interp.get_stack()[0];
        assert!(
            matches!(&val.data, ValueData::ExactScalar(er)
                if er.is_algebraic_sqrt()),
            "SQRT(2) must be ExactScalar(AlgebraicSqrt), got: {val}"
        );
    }

    #[tokio::test]
    async fn sqrt_of_irrational_compares_equal_to_itself() {
        // Push √2 twice; EQ should return TRUE via CF data equality
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 2 SQRT 2 SQRT EQ")
            .await
            .unwrap();
        let val = &interp.get_stack()[0];
        assert!(val.is_truthy(), "√2 == √2 must be TRUE, got: {val}");
    }

    #[tokio::test]
    async fn sqrt_of_irrational_lt_comparison_correct() {
        // √2 ≈ 1.414 < 2
        let mut interp = Interpreter::new();
        interp.execute("'math' IMPORT 2 SQRT 2 LT").await.unwrap();
        let val = &interp.get_stack()[0];
        assert!(val.is_truthy(), "√2 < 2 must be TRUE, got: {val}");
    }

    #[tokio::test]
    async fn sqrt_squared_via_mul_returns_exact_scalar() {
        // √2 × √2 produces an exact value via Gosper bihomographic path.
        // The result is ExactScalar(Gosper); it should be a non-nil scalar.
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 2 SQRT 2 SQRT *")
            .await
            .unwrap();
        let val = &interp.get_stack()[0];
        assert!(
            val.is_scalar() && !val.is_nil(),
            "√2 × √2 must be a non-nil scalar, got: {val}"
        );
        // The display should be an approximate rational (Gosper node)
        let display = format!("{val}");
        assert!(
            !display.is_empty() && display != "NIL",
            "display must be non-nil"
        );
    }

    #[tokio::test]
    async fn exact_scalar_add_rational_produces_result() {
        // √2 + 1 → irrational result (ExactScalar)
        let mut interp = Interpreter::new();
        interp.execute("'math' IMPORT 2 SQRT 1 +").await.unwrap();
        let val = &interp.get_stack()[0];
        // Result is a Gosper Möbius transform — an ExactScalar
        assert!(
            val.is_scalar() && !val.is_nil(),
            "√2 + 1 must be a non-nil scalar, got: {val}"
        );
    }

    #[tokio::test]
    async fn exact_scalar_floor_is_exact_rational() {
        // floor(√2) = 1 exactly
        let mut interp = Interpreter::new();
        interp.execute("'math' IMPORT 2 SQRT FLOOR").await.unwrap();
        let val = &interp.get_stack()[0];
        let f = val.as_scalar().expect("floor(√2) must be exact rational");
        assert_eq!(f.to_i64(), Some(1), "floor(√2) must equal 1, got {f}");
    }

    #[tokio::test]
    async fn exact_scalar_ceil_is_exact_rational() {
        // ceil(√2) = 2 exactly
        let mut interp = Interpreter::new();
        interp.execute("'math' IMPORT 2 SQRT CEIL").await.unwrap();
        let val = &interp.get_stack()[0];
        let f = val.as_scalar().expect("ceil(√2) must be exact rational");
        assert_eq!(f.to_i64(), Some(2), "ceil(√2) must equal 2, got {f}");
    }

    #[tokio::test]
    async fn exact_scalar_round_is_exact_rational() {
        // round(√2) = 1 (√2 ≈ 1.414 → nearest integer is 1)
        let mut interp = Interpreter::new();
        interp.execute("'math' IMPORT 2 SQRT ROUND").await.unwrap();
        let val = &interp.get_stack()[0];
        let f = val.as_scalar().expect("round(√2) must be exact rational");
        assert_eq!(f.to_i64(), Some(1), "round(√2) must equal 1, got {f}");
    }

    #[tokio::test]
    async fn exact_scalar_floor_sqrt3_is_exact_rational() {
        // floor(√3) = 1 exactly
        let mut interp = Interpreter::new();
        interp.execute("'math' IMPORT 3 SQRT FLOOR").await.unwrap();
        let val = &interp.get_stack()[0];
        let f = val.as_scalar().expect("floor(√3) must be exact rational");
        assert_eq!(f.to_i64(), Some(1), "floor(√3) must equal 1, got {f}");
    }

    #[tokio::test]
    async fn exact_scalar_mod_rational_is_exact() {
        // √2 mod 1 = √2 - 1 (irrational, stays ExactScalar)
        let mut interp = Interpreter::new();
        interp.execute("'math' IMPORT 2 SQRT 1 MOD").await.unwrap();
        let val = &interp.get_stack()[0];
        assert!(
            val.is_scalar() && !val.is_nil(),
            "√2 mod 1 must be a non-nil scalar, got: {val}"
        );
        // Result should be less than 1
        let mut interp2 = Interpreter::new();
        interp2
            .execute("'math' IMPORT 2 SQRT 1 MOD 1 <")
            .await
            .unwrap();
        let cmp = &interp2.get_stack()[0];
        assert!(
            !cmp.is_nil() && cmp.as_scalar().map(|f| !f.is_zero()).unwrap_or(false),
            "(√2 mod 1) < 1 must be TRUE"
        );
    }
}

#[cfg(test)]
mod continued_fraction_role_tests {
    use crate::interpreter::Interpreter;
    use crate::types::display::{format_as_continued_fraction, format_with_hint};
    use crate::types::Interpretation;

    // `>CF` on a rational: 5/2 = [2; 2] -> "( 2 ( 2 ) )".
    #[tokio::test]
    async fn to_cf_rational_nested_form() {
        let mut interp = Interpreter::new();
        interp.execute("5/2 >CF").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let hints = interp.collect_stack_hints();
        assert_eq!(hints.last(), Some(&Interpretation::ContinuedFraction));
        let s = format_with_hint(&stack[0], Interpretation::ContinuedFraction);
        assert_eq!(s, "( 2 ( 2 ) )");
    }

    // `>CF` on √2 = [1; 2,2,2,...] -> lazy, truncated.
    #[tokio::test]
    async fn to_cf_sqrt2_truncated_form() {
        let mut interp = Interpreter::new();
        interp.execute("'math' IMPORT 2 SQRT >CF").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let s = format_as_continued_fraction(&stack[0]);
        assert!(s.starts_with("( 1"), "expected '( 1' prefix, got {s:?}");
        assert!(
            s.contains("( 1 ( 2 ( 2 "),
            "expected √2 expansion, got {s:?}"
        );
        assert!(s.contains("...)"), "expected truncation marker, got {s:?}");
        let opens = s.matches('(').count();
        let closes = s.matches(')').count();
        assert_eq!(opens, closes, "unbalanced parens in {s:?}");
    }

    // `>CF` only retags; the underlying value is byte-for-byte identical to
    // an untagged √2 (same structural rendering, no data mutation).
    #[tokio::test]
    async fn to_cf_preserves_value() {
        let mut tagged = Interpreter::new();
        tagged.execute("'math' IMPORT 2 SQRT >CF").await.unwrap();
        let mut plain = Interpreter::new();
        plain.execute("'math' IMPORT 2 SQRT").await.unwrap();

        let tagged_stack = tagged.get_stack();
        let plain_stack = plain.get_stack();
        assert_eq!(tagged_stack.len(), 1);
        assert_eq!(plain_stack.len(), 1);

        // The retagged value's underlying data renders identically to the
        // untagged √2 under the structural (RawNumber) role.
        assert_eq!(
            format_with_hint(&tagged_stack[0], Interpretation::RawNumber),
            format_with_hint(&plain_stack[0], Interpretation::RawNumber)
        );
    }
}
