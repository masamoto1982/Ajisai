//! Test suite for `crate::interpreter::algo_ops` (ALGO UNIQUE/CONTAINS/INDEX-OF).

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn unique_dedupes_preserving_order() {
        let mut interp = Interpreter::new();
        interp
            .execute("'algo' IMPORT [ 3 1 3 2 1 ] UNIQUE")
            .await
            .expect("should succeed");
        assert_eq!(interp.stack.len(), 1);
        let v = interp.stack[0].as_vector_view().expect("vector result");
        let nums: Vec<i64> = v
            .iter()
            .map(|e| e.as_scalar().unwrap().to_i64().unwrap())
            .collect();
        assert_eq!(nums, vec![3, 1, 2]);
    }

    #[tokio::test]
    async fn unique_collapses_all_equal() {
        let mut interp = Interpreter::new();
        interp
            .execute("'algo' IMPORT [ 5 5 5 ] UNIQUE")
            .await
            .expect("should succeed");
        let v = interp.stack[0].as_vector_view().expect("vector result");
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].as_scalar().unwrap().to_i64().unwrap(), 5);
    }

    #[tokio::test]
    async fn unique_non_vector_errors() {
        let mut interp = Interpreter::new();
        let result = interp.execute("'algo' IMPORT 42 UNIQUE").await;
        assert!(result.is_err(), "UNIQUE of a non-vector is malformed use");
    }

    #[tokio::test]
    async fn contains_reports_membership() {
        let mut interp = Interpreter::new();
        interp
            .execute("'algo' IMPORT [ 1 2 3 ] 2 CONTAINS")
            .await
            .expect("should succeed");
        assert_eq!(interp.stack[0].as_truth(), Some(true));

        interp.stack.clear();
        interp
            .execute("'algo' IMPORT [ 1 2 3 ] 9 CONTAINS")
            .await
            .expect("should succeed");
        assert_eq!(interp.stack[0].as_truth(), Some(false));
    }

    #[tokio::test]
    async fn index_of_returns_position() {
        let mut interp = Interpreter::new();
        interp
            .execute("'algo' IMPORT [ 10 20 30 ] 20 INDEX-OF")
            .await
            .expect("should succeed");
        assert_eq!(interp.stack[0].as_scalar().unwrap().to_i64().unwrap(), 1);
    }

    #[tokio::test]
    async fn index_of_missing_is_bubble() {
        let mut interp = Interpreter::new();
        interp
            .execute("'algo' IMPORT [ 10 20 30 ] 99 INDEX-OF")
            .await
            .expect("a search miss is a Bubble, not an error");
        assert_eq!(interp.stack.len(), 1);
        assert!(interp.stack[0].is_nil());
    }

    #[tokio::test]
    async fn index_of_can_fall_back_with_or_nil() {
        let mut interp = Interpreter::new();
        interp
            .execute("'algo' IMPORT -1 [ 1 2 3 ] 9 INDEX-OF ^")
            .await
            .expect("should succeed");
        assert_eq!(interp.stack[0].as_scalar().unwrap().to_i64().unwrap(), -1);
    }

    #[tokio::test]
    async fn stack_mode_is_rejected() {
        let mut interp = Interpreter::new();
        let result = interp.execute("'algo' IMPORT [ 1 1 2 ] .. UNIQUE").await;
        assert!(result.is_err(), "UNIQUE should reject Stack mode");
        assert!(result.unwrap_err().to_string().contains("Stack mode"));
    }

    #[tokio::test]
    async fn keep_mode_retains_operands() {
        let mut interp = Interpreter::new();
        interp
            .execute("'algo' IMPORT [ 1 2 3 ] 2 ,, CONTAINS")
            .await
            .expect("keep mode should succeed");
        // vector + target retained, plus the boolean result
        assert_eq!(interp.stack.len(), 3);
        assert_eq!(interp.stack[2].as_truth(), Some(true));
    }
}
