#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    fn top_scalar_i64(interp: &Interpreter) -> i64 {
        let top = interp.stack.last().expect("stack top");
        if let Some(f) = top.as_scalar() {
            return f.to_i64().expect("scalar i64");
        }
        let child = top.get_child(0).expect("vector[0]");
        child
            .as_scalar()
            .and_then(|f| f.to_i64())
            .expect("expected scalar i64 on stack top")
    }

    fn top_is_nil(interp: &Interpreter) -> bool {
        interp.stack.last().map(|v| v.is_nil()).unwrap_or(false)
    }

    #[tokio::test]
    async fn test_fold_basic() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 2 3 4 ] [ 0 ] '+' FOLD").await;
        assert!(result.is_ok(), "FOLD should succeed: {:?}", result);
        assert_eq!(top_scalar_i64(&interp), 10);
    }

    #[tokio::test]
    async fn test_fold_nil_returns_initial() {
        let mut interp = Interpreter::new();
        let result = interp.execute("NIL [ 42 ] '+' FOLD").await;
        assert!(result.is_ok(), "FOLD on NIL should return initial: {:?}", result);
        assert_eq!(top_scalar_i64(&interp), 42);
    }

    #[tokio::test]
    async fn test_unfold_basic_generation() {
        let mut interp = Interpreter::new();
        let code = "[ 1 ] { { [ 1 ] = } { [ 1 2 ] } { [ 2 ] = } { [ 2 3 ] } { [ 3 ] = } { [ 3 NIL ] } { IDLE } { NIL } COND } UNFOLD";
        let result = interp.execute(code).await;
        assert!(result.is_ok(), "UNFOLD basic should succeed: {:?}", result);
        let out = interp.stack.last().expect("result");
        assert_eq!(out.len(), 3);
    }

    #[tokio::test]
    async fn test_unfold_immediate_nil() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 ] { NIL } UNFOLD").await;
        assert!(result.is_ok(), "UNFOLD immediate NIL should succeed: {:?}", result);
        assert!(top_is_nil(&interp));
    }

    #[tokio::test]
    async fn test_unfold_invalid_format_error() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 ] { [ 1 ] } UNFOLD").await;
        assert!(result.is_err(), "UNFOLD invalid format must fail");
    }

    #[tokio::test]
    async fn test_unfold_non_termination_error() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 ] { [ 1 1 ] } UNFOLD").await;
        assert!(result.is_err(), "UNFOLD non-termination must fail");
    }

    #[tokio::test]
    async fn test_unfold_user_word_and_stack_preservation() {
        let mut interp = Interpreter::new();
        interp
            .execute("{ { [ 1 ] = } { [ 1 2 ] } { [ 2 ] = } { [ 2 3 ] } { [ 3 ] = } { [ 3 NIL ] } { IDLE } { NIL } COND } 'NEXT' DEF")
            .await
            .unwrap();
        let result = interp.execute("[ 99 ] [ 1 ] 'NEXT' UNFOLD").await;
        assert!(result.is_ok(), "UNFOLD user word should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 2);
        let below = &interp.stack[0];
        let below_num = below
            .as_scalar()
            .and_then(|f| f.to_i64())
            .or_else(|| below.get_child(0).and_then(|v| v.as_scalar()).and_then(|f| f.to_i64()));
        assert_eq!(below_num, Some(99));
    }

    #[tokio::test]
    async fn test_any_basic_and_nil_and_user_word() {
        let mut interp = Interpreter::new();
        let ok = interp
            .execute("[ 1 3 5 8 ] { [ 2 ] MOD [ 0 ] = } ANY")
            .await;
        assert!(ok.is_ok(), "ANY basic failed: {:?}", ok);
        assert_eq!(top_scalar_i64(&interp), 1);

        let mut interp2 = Interpreter::new();
        let ok2 = interp2.execute("NIL { [ 2 ] MOD [ 0 ] = } ANY").await;
        assert!(ok2.is_ok(), "ANY NIL failed: {:?}", ok2);
        assert_eq!(top_scalar_i64(&interp2), 0);

        let mut interp3 = Interpreter::new();
        interp3
            .execute("{ [ 2 ] MOD [ 0 ] = } 'IS_EVEN' DEF")
            .await
            .unwrap();
        let ok3 = interp3.execute("[ 1 3 6 ] 'IS_EVEN' ANY").await;
        assert!(ok3.is_ok(), "ANY user word failed: {:?}", ok3);
        assert_eq!(top_scalar_i64(&interp3), 1);
    }

    #[tokio::test]
    async fn test_any_short_circuit() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 2 3 ] { { [ 1 ] = } { TRUE } { IDLE } { [ 1 ] [ 0 ] / } COND } ANY").await;
        assert!(result.is_ok(), "ANY should short-circuit before divide-by-zero: {:?}", result);
        assert_eq!(top_scalar_i64(&interp), 1);
    }

    #[tokio::test]
    async fn test_all_basic_nil_short_circuit_user_word() {
        let mut interp = Interpreter::new();
        let ok = interp
            .execute("[ 2 4 6 8 ] { [ 2 ] MOD [ 0 ] = } ALL")
            .await;
        assert!(ok.is_ok(), "ALL basic failed: {:?}", ok);
        assert_eq!(top_scalar_i64(&interp), 1);

        let mut interp2 = Interpreter::new();
        let ok2 = interp2.execute("NIL { [ 2 ] MOD [ 0 ] = } ALL").await;
        assert!(ok2.is_ok(), "ALL NIL failed: {:?}", ok2);
        assert_eq!(top_scalar_i64(&interp2), 1);

        let mut interp3 = Interpreter::new();
        let ok3 = interp3
            .execute("[ 2 3 4 ] { { [ 2 ] = } { FALSE } { IDLE } { [ 1 ] [ 0 ] / } COND } ALL")
            .await;
        assert!(ok3.is_ok(), "ALL should short-circuit on first FALSE: {:?}", ok3);
        assert_eq!(top_scalar_i64(&interp3), 0);

        let mut interp4 = Interpreter::new();
        interp4
            .execute("{ [ 2 ] MOD [ 0 ] = } 'IS_EVEN' DEF")
            .await
            .unwrap();
        let ok4 = interp4.execute("[ 2 4 5 ] 'IS_EVEN' ALL").await;
        assert!(ok4.is_ok(), "ALL user word failed: {:?}", ok4);
        assert_eq!(top_scalar_i64(&interp4), 0);
    }

    #[tokio::test]
    async fn test_count_cases_and_user_word() {
        let mut interp = Interpreter::new();
        assert!(interp
            .execute("[ 1 2 3 4 5 ] { [ 2 ] MOD [ 0 ] = } COUNT")
            .await
            .is_ok());
        assert_eq!(top_scalar_i64(&interp), 2);

        let mut interp2 = Interpreter::new();
        assert!(interp2
            .execute("[ 2 4 6 ] { [ 2 ] MOD [ 0 ] = } COUNT")
            .await
            .is_ok());
        assert_eq!(top_scalar_i64(&interp2), 3);

        let mut interp3 = Interpreter::new();
        assert!(interp3
            .execute("[ 1 3 5 ] { [ 2 ] MOD [ 0 ] = } COUNT")
            .await
            .is_ok());
        assert_eq!(top_scalar_i64(&interp3), 0);

        let mut interp4 = Interpreter::new();
        assert!(interp4.execute("NIL { [ 2 ] MOD [ 0 ] = } COUNT").await.is_ok());
        assert_eq!(top_scalar_i64(&interp4), 0);

        let mut interp5 = Interpreter::new();
        interp5
            .execute("{ [ 2 ] MOD [ 0 ] = } 'IS_EVEN' DEF")
            .await
            .unwrap();
        assert!(interp5.execute("[ 1 2 4 7 ] 'IS_EVEN' COUNT").await.is_ok());
        assert_eq!(top_scalar_i64(&interp5), 2);
    }


    #[tokio::test]
    async fn test_count_percent_alias_matches_mod() {
        let mut mod_interp = Interpreter::new();
        let mod_result = mod_interp
            .execute("[ 1 2 3 4 5 6 ] { [ 2 ] MOD [ 0 ] = } COUNT")
            .await;
        assert!(mod_result.is_ok(), "COUNT with MOD failed: {:?}", mod_result);
        let mod_count = top_scalar_i64(&mod_interp);

        let mut alias_interp = Interpreter::new();
        let alias_result = alias_interp
            .execute("[ 1 2 3 4 5 6 ] { [ 2 ] % [ 0 ] = } COUNT")
            .await;
        assert!(
            alias_result.is_ok(),
            "COUNT with % alias failed: {:?}",
            alias_result
        );
        let alias_count = top_scalar_i64(&alias_interp);

        assert_eq!(alias_count, mod_count);
    }

    #[tokio::test]
    async fn test_filter_ampersand_alias_matches_and() {
        let mut and_interp = Interpreter::new();
        let and_result = and_interp
            .execute("[ [ TRUE TRUE ] [ TRUE FALSE ] [ FALSE TRUE ] ] { [ 0 ] [ 1 ] AND } FILTER")
            .await;
        assert!(and_result.is_ok(), "FILTER with AND failed: {:?}", and_result);

        let mut alias_interp = Interpreter::new();
        let alias_result = alias_interp
            .execute("[ [ TRUE TRUE ] [ TRUE FALSE ] [ FALSE TRUE ] ] { [ 0 ] [ 1 ] & } FILTER")
            .await;
        assert!(
            alias_result.is_ok(),
            "FILTER with & alias failed: {:?}",
            alias_result
        );

        assert_eq!(alias_interp.stack, and_interp.stack);
    }

    #[tokio::test]
    async fn test_scan_add_mul_nil_user_word_and_stack_preserve() {
        let mut interp = Interpreter::new();
        assert!(interp.execute("[ 1 2 3 4 ] [ 0 ] '+' SCAN").await.is_ok());
        assert_eq!(interp.stack.last().expect("result").len(), 4);

        let mut interp2 = Interpreter::new();
        assert!(interp2.execute("[ 1 2 3 4 ] [ 1 ] '*' SCAN").await.is_ok());
        assert_eq!(interp2.stack.last().expect("result").len(), 4);

        let mut interp3 = Interpreter::new();
        assert!(interp3.execute("NIL [ 0 ] '+' SCAN").await.is_ok());
        assert!(top_is_nil(&interp3));

        let mut interp4 = Interpreter::new();
        interp4.execute("{ + } 'MYSUM' DEF").await.unwrap();
        assert!(interp4.execute("[ 1 2 3 ] [ 0 ] 'MYSUM' SCAN").await.is_ok());
        assert_eq!(interp4.stack.last().expect("result").len(), 3);

        let mut interp5 = Interpreter::new();
        assert!(interp5
            .execute("[ 100 ] [ 1 2 3 ] [ 0 ] '+' SCAN")
            .await
            .is_ok());
        assert_eq!(interp5.stack.len(), 2);
    }
}
