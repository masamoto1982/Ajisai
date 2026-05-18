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
    use crate::error::NilReason;
    use crate::interpreter::interval_ops::value_to_interval;
    use crate::interpreter::Interpreter;
    use crate::semantic::AbsenceOrigin;
    use crate::types::fraction::Fraction;

    #[tokio::test]
    async fn test_interval_creation_success_and_failure() {
        let mut interp = Interpreter::new();
        interp.execute("1 2 INTERVAL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(format!("{}", stack[0]), "[1/1, 2/1]");

        let mut interp_fail = Interpreter::new();
        let result = interp_fail.execute("2 1 INTERVAL").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_interval_basic_arithmetic() {
        let mut interp = Interpreter::new();
        interp.execute("1 2 INTERVAL 3 4 INTERVAL +").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[4/1, 6/1]");

        let mut interp = Interpreter::new();
        interp.execute("1 2 INTERVAL 3 4 INTERVAL -").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[-3/1, -1/1]");

        let mut interp = Interpreter::new();
        interp.execute("1 2 INTERVAL 3 4 INTERVAL *").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[3/1, 8/1]");

        let mut interp = Interpreter::new();
        interp
            .execute("-1 2 INTERVAL 3 4 INTERVAL *")
            .await
            .unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[-4/1, 8/1]");

        let mut interp = Interpreter::new();
        interp.execute("2 4 INTERVAL 1 2 INTERVAL /").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[1/1, 4/1]");
    }

    #[tokio::test]
    async fn test_interval_division_by_zero_interval_bubbles() {
        let mut interp = Interpreter::new();
        interp
            .execute("1 2 INTERVAL -1 1 INTERVAL /")
            .await
            .unwrap();
        assert!(interp.get_stack().last().unwrap().is_nil());
    }

    #[tokio::test]
    async fn test_sqrt_exact_cases() {
        let mut interp = Interpreter::new();
        interp.execute("4 SQRT").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "2/1");

        let mut interp = Interpreter::new();
        interp.execute("9/16 SQRT").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "3/4");
    }

    #[tokio::test]
    async fn test_sqrt_interval_soundness_and_eps() {
        let mut interp = Interpreter::new();
        interp.execute("2 SQRT").await.unwrap();
        let iv = value_to_interval(&interp.get_stack()[0]).expect("sqrt(2) must be interval");
        let two = Fraction::from(2);
        assert!(iv.lo.mul(&iv.lo).le(&two));
        assert!(iv.hi.mul(&iv.hi).ge(&two));
        assert!(iv.lo.le(&iv.hi));

        let mut interp_eps = Interpreter::new();
        interp_eps.execute("2 1/100 SQRT_EPS").await.unwrap();
        let iv_eps =
            value_to_interval(&interp_eps.get_stack()[0]).expect("sqrt_eps(2) must be interval");
        assert!(iv_eps.width().le(&Fraction::new(1.into(), 100.into())));
    }

    #[tokio::test]
    async fn test_sqrt_interval_monotonicity() {
        let mut interp = Interpreter::new();
        interp.execute("1 4 INTERVAL SQRT").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[1/1, 2/1]");
    }

    #[tokio::test]
    async fn test_interval_comparison_policy() {
        let mut interp = Interpreter::new();
        interp.execute("1 2 INTERVAL 3 4 INTERVAL <").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "1/1");

        let mut interp_undetermined = Interpreter::new();
        interp_undetermined
            .execute("2 3 INTERVAL 3 4 INTERVAL <")
            .await
            .unwrap();
        let absence = interp_undetermined.get_stack()[0]
            .absence_metadata()
            .expect("overlapping interval comparison projects to NIL");
        assert_eq!(absence.reason, Some(NilReason::Undecidable));
        assert_eq!(absence.origin, AbsenceOrigin::ComparisonBudget);

        let mut interp_eq = Interpreter::new();
        interp_eq
            .execute("1 5 INTERVAL 2 4 INTERVAL =")
            .await
            .unwrap();
        assert_eq!(format!("{}", interp_eq.get_stack()[0]), "0/1");
    }

    #[tokio::test]
    async fn test_mixed_arithmetic() {
        let mut interp = Interpreter::new();
        interp.execute("1 2 3 INTERVAL +").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[3/1, 4/1]");

        let mut interp = Interpreter::new();
        interp.execute("2 3 5 INTERVAL *").await.unwrap();
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

    use crate::error::NilReason;
    use crate::interpreter::Interpreter;
    use crate::semantic::AbsenceOrigin;

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
        let interp = run("3 4 INTERVAL 1 2 INTERVAL GT").await;
        let s = format!("{}", interp.get_stack()[0]);
        assert_eq!(s, "1/1");
    }

    #[tokio::test]
    async fn gte_on_disjoint_intervals_decides_false() {
        let interp = run("0 1 INTERVAL 2 3 INTERVAL GTE").await;
        let s = format!("{}", interp.get_stack()[0]);
        assert_eq!(s, "0/1");
    }

    #[tokio::test]
    async fn gt_on_overlapping_intervals_projects_undecidable_nil() {
        let mut interp = Interpreter::new();
        interp
            .execute("2 3 INTERVAL 2 4 INTERVAL GT")
            .await
            .unwrap();
        let absence = interp.get_stack()[0]
            .absence_metadata()
            .expect("overlapping interval comparison projects to NIL");
        assert_eq!(absence.reason, Some(NilReason::Undecidable));
        assert_eq!(absence.origin, AbsenceOrigin::ComparisonBudget);
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
