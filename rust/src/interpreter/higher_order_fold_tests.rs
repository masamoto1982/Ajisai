//! Test suite for `crate::interpreter::higher_order_fold`.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    fn top_scalar_i64(interp: &Interpreter) -> i64 {
        let top = interp.stack.last().expect("stack top");
        // A Boolean result reads as 1/0.
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
        let result = interp.execute("[ 1 2 3 4 ] [ 0 ] [ + ] FOLD").await;
        assert!(result.is_ok(), "FOLD should succeed: {:?}", result);
        assert_eq!(top_scalar_i64(&interp), 10);
    }

    #[tokio::test]
    async fn test_fold_of_an_absent_vector_is_that_absence() {
        // The Vector is data (LANG.FAILURE.PASSTHROUGH): an absent one is the
        // result, not the initial accumulator standing in for an empty fold.
        let mut interp = Interpreter::new();
        interp.execute("NIL [ 42 ] [ + ] FOLD").await.unwrap();
        assert!(interp.stack.last().is_some_and(|v| v.is_nil()));
    }
    /// `&` resolves to the same contract and executor as `AND`
    /// (LANG.SOURCE.NORMALIZE), including inside a predicate block.
    ///
    /// The operands are Booleans because `AND` is `booleanLogic`, whose input
    /// domain is two-valued (LANG.VALUES.TRUTH). This case used to read
    /// `{ [ 0 ] [ 1 ] AND }`, where `[ 0 ]` and `[ 1 ]` are vector literals
    /// rather than the index lenses they resemble; it passed only because
    /// `AND` coerced the scalars `0` and `1` to truth values and `FILTER`
    /// accepted the resulting singleton Vector as a predicate result. Both
    /// coercions are gone, so the case now states its subject directly.
    #[tokio::test]
    async fn test_filter_reaches_and_by_name() {
        let mut and_interp = Interpreter::new();
        let and_result = and_interp
            .execute("[ TRUE FALSE TRUE ] [ TRUE AND ] FILTER")
            .await;
        assert!(
            and_result.is_ok(),
            "FILTER with AND failed: {:?}",
            and_result
        );

        // AND and NOT carry no symbol: `&` is an ordinary name the
        // dictionary does not have, inside a block body like anywhere else.
        let mut alias_interp = Interpreter::new();
        let alias_result = alias_interp
            .execute("[ TRUE FALSE TRUE ] [ TRUE & ] FILTER")
            .await;
        assert!(
            alias_result.is_err(),
            "`&` must not be a spelling of AND: {:?}",
            alias_result
        );
    }

    /// A `booleanLogic` Word raises its registered `nonTruthValue` ERROR on a
    /// scalar operand: the Boolean and Scalar domains are disjoint, so `1` is
    /// not TRUE and `0` is not FALSE (LANG.VALUES.DISJOINT).
    #[tokio::test]
    async fn test_logic_words_reject_scalar_operands() {
        for source in ["1 1 AND", "FALSE 0 AND", "5 NOT", "TRUE 1 AND"] {
            let mut interp = Interpreter::new();
            let result = interp.execute(source).await;
            assert!(
                result.is_err(),
                "`{}` must raise nonTruthValue, got {:?}",
                source,
                result
            );
        }
    }

    /// A predicate block must decide in the Boolean domain: a scalar, a NIL,
    /// and a singleton Vector are each a nonconforming predicate result rather
    /// than a truth value (LANG.VALUES.TRUTH).
    #[tokio::test]
    async fn test_higher_order_predicates_reject_non_boolean() {
        for source in [
            "[ 1 2 3 ] [ 1 ] FILTER",
            "[ 1 2 3 ] [ NIL ] FILTER",
            "[ 1 2 3 ] [ [ TRUE ] ] FILTER",
        ] {
            let mut interp = Interpreter::new();
            let result = interp.execute(source).await;
            assert!(
                result.is_err(),
                "`{}` must reject a non-Boolean predicate result, got {:?}",
                source,
                result
            );
        }
    }
}
