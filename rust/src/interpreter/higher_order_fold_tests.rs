//! Test suite for `crate::interpreter::higher_order_fold`.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    fn top_scalar_i64(interp: &Interpreter) -> i64 {
        let top = interp.stack.last().expect("stack top");
        // A Boolean predicate result (ANY/ALL) reads as 1/0.
        if let Some(b) = top.as_truth() {
            return if b { 1 } else { 0 };
        }
        if let Some(f) = top.as_scalar() {
            return f.to_i64().expect("scalar i64");
        }
        let child = top.child(0).expect("vector[0]");
        if let Some(b) = child.as_truth() {
            return if b { 1 } else { 0 };
        }
        child
            .as_scalar()
            .and_then(|f| f.to_i64())
            .expect("expected scalar i64 on stack top")
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
        assert!(
            result.is_ok(),
            "FOLD on NIL should return initial: {:?}",
            result
        );
        assert_eq!(top_scalar_i64(&interp), 42);
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
    async fn test_filter_ampersand_alias_matches_and() {
        let mut and_interp = Interpreter::new();
        let and_result = and_interp
            .execute("[ [ TRUE TRUE ] [ TRUE FALSE ] [ FALSE TRUE ] ] { [ 0 ] [ 1 ] AND } FILTER")
            .await;
        assert!(
            and_result.is_ok(),
            "FILTER with AND failed: {:?}",
            and_result
        );

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
}
