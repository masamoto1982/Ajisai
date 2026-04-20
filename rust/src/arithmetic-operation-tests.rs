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
        assert_eq!(result, "[ 3 ]", "CEIL(7/3) should be 3");
    }

    #[tokio::test]
    async fn test_ceil_negative_remainder() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ -7/3 ] CEIL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ -2 ]", "CEIL(-7/3) should be -2");
    }

    #[tokio::test]
    async fn test_ceil_positive_integer() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 6/3 ] CEIL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ 2 ]", "CEIL(6/3) should be 2");
    }

    #[tokio::test]
    async fn test_ceil_negative_integer() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ -6/3 ] CEIL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ -2 ]", "CEIL(-6/3) should be -2");
    }

    #[tokio::test]
    async fn test_ceil_operation_target_stack_error() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        let result = interp.execute("[ 1 2 3 ] .. CEIL").await;
        assert!(result.is_err(), "CEIL should not support Stack mode (..)");
    }

    #[tokio::test]
    async fn test_ceil_error_restores_stack() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("NIL").await.unwrap();
        let result = interp.execute("CEIL").await;
        assert!(result.is_err());
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1, "Stack should be restored after error");
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
        assert_eq!(result, "[ 2 ]", "ROUND(7/3) should be 2");
    }

    #[tokio::test]
    async fn test_round_positive_half() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 5/2 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ 3 ]", "ROUND(5/2) should be 3");
    }

    #[tokio::test]
    async fn test_round_positive_above_half() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 8/3 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ 3 ]", "ROUND(8/3) should be 3");
    }

    #[tokio::test]
    async fn test_round_negative_below_half() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ -7/3 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ -2 ]", "ROUND(-7/3) should be -2");
    }

    #[tokio::test]
    async fn test_round_negative_half() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ -5/2 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ -3 ]", "ROUND(-5/2) should be -3");
    }

    #[tokio::test]
    async fn test_round_negative_above_half() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ -8/3 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ -3 ]", "ROUND(-8/3) should be -3");
    }

    #[tokio::test]
    async fn test_round_positive_integer() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 6/3 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ 2 ]", "ROUND(6/3) should be 2");
    }

    #[tokio::test]
    async fn test_round_negative_integer() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ -6/3 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ -2 ]", "ROUND(-6/3) should be -2");
    }

    #[tokio::test]
    async fn test_round_operation_target_stack_error() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        let result = interp.execute("[ 1 2 3 ] .. ROUND").await;
        assert!(result.is_err(), "ROUND should not support Stack mode (..)");
    }

    #[tokio::test]
    async fn test_round_error_restores_stack() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("NIL").await.unwrap();
        let result = interp.execute("ROUND").await;
        assert!(result.is_err());
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1, "Stack should be restored after error");
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
        interp.execute("1 2 INTERVAL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(format!("{}", stack[0]), "[1, 2]");

        let mut interp_fail = Interpreter::new();
        let result = interp_fail.execute("2 1 INTERVAL").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_interval_basic_arithmetic() {
        let mut interp = Interpreter::new();
        interp.execute("1 2 INTERVAL 3 4 INTERVAL +").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[4, 6]");

        let mut interp = Interpreter::new();
        interp.execute("1 2 INTERVAL 3 4 INTERVAL -").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[-3, -1]");

        let mut interp = Interpreter::new();
        interp.execute("1 2 INTERVAL 3 4 INTERVAL *").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[3, 8]");

        let mut interp = Interpreter::new();
        interp
            .execute("-1 2 INTERVAL 3 4 INTERVAL *")
            .await
            .unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[-4, 8]");

        let mut interp = Interpreter::new();
        interp.execute("2 4 INTERVAL 1 2 INTERVAL /").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[1, 4]");
    }

    #[tokio::test]
    async fn test_interval_division_by_zero_interval_fails() {
        let mut interp = Interpreter::new();
        let result = interp.execute("1 2 INTERVAL -1 1 INTERVAL /").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sqrt_exact_cases() {
        let mut interp = Interpreter::new();
        interp.execute("4 SQRT").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "2");

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
        assert_eq!(format!("{}", interp.get_stack()[0]), "[1, 2]");
    }

    #[tokio::test]
    async fn test_interval_comparison_policy() {
        let mut interp = Interpreter::new();
        interp.execute("1 2 INTERVAL 3 4 INTERVAL <").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[ 1 ]");

        let mut interp_undetermined = Interpreter::new();
        let result = interp_undetermined
            .execute("2 3 INTERVAL 3 4 INTERVAL <")
            .await;
        assert!(result.is_err());

        let mut interp_eq = Interpreter::new();
        interp_eq
            .execute("1 5 INTERVAL 2 4 INTERVAL =")
            .await
            .unwrap();
        assert_eq!(format!("{}", interp_eq.get_stack()[0]), "[ 0 ]");
    }

    #[tokio::test]
    async fn test_mixed_arithmetic() {
        let mut interp = Interpreter::new();
        interp.execute("1 2 3 INTERVAL +").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[3, 4]");

        let mut interp = Interpreter::new();
        interp.execute("2 3 5 INTERVAL *").await.unwrap();
        assert_eq!(format!("{}", interp.get_stack()[0]), "[6, 10]");
    }
}
