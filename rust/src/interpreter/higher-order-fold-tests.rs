#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_fold_basic() {
        let mut interp = Interpreter::new();
        let code = r#"[ 1 2 3 4 ] [ 0 ] '+' FOLD"#;
        let result = interp.execute(code).await;
        assert!(result.is_ok(), "FOLD should succeed: {:?}", result);

        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_fold_nil_returns_initial() {
        let mut interp = Interpreter::new();
        let code = r#"NIL [ 42 ] '+' FOLD"#;
        let result = interp.execute(code).await;
        assert!(
            result.is_ok(),
            "FOLD on NIL should return initial value: {:?}",
            result
        );

        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_map_with_multiline_word() {
        let mut interp = Interpreter::new();
        let def_code = r#"{ [ 2 ] * } 'DOUBLE' DEF
{ DOUBLE [ 1 ] + } 'DOUBLE_PLUS_ONE' DEF"#;
        let def_result = interp.execute(def_code).await;
        assert!(def_result.is_ok(), "DEF should succeed: {:?}", def_result);

        let map_code = "[ 1 2 3 ] 'DOUBLE_PLUS_ONE' MAP";
        let result = interp.execute(map_code).await;

        assert!(
            result.is_ok(),
            "MAP with multiline word should succeed: {:?}",
            result
        );

        assert_eq!(
            interp.stack.len(),
            1,
            "Stack should have exactly 1 element, got {}",
            interp.stack.len()
        );
    }

    #[tokio::test]
    async fn test_map_preserves_stack_below() {
        let mut interp = Interpreter::new();
        let def_code = "{ [ 2 ] * } 'DOUBLE' DEF";
        let def_result = interp.execute(def_code).await;
        assert!(def_result.is_ok(), "DEF should succeed: {:?}", def_result);

        let code = "[ 100 ] [ 1 2 3 ] 'DOUBLE' MAP";
        let result = interp.execute(code).await;

        assert!(
            result.is_ok(),
            "MAP should preserve stack below: {:?}",
            result
        );

        assert_eq!(interp.stack.len(), 2, "Stack should have 2 elements");
    }

    #[tokio::test]
    async fn test_fold_preserves_stack_below() {
        let mut interp = Interpreter::new();
        let code = "[ 100 ] [ 1 2 3 4 ] [ 0 ] '+' FOLD";
        let result = interp.execute(code).await;

        assert!(
            result.is_ok(),
            "FOLD should preserve stack below: {:?}",
            result
        );

        assert_eq!(
            interp.stack.len(),
            2,
            "Stack should have 2 elements, got {}",
            interp.stack.len()
        );
    }

    #[tokio::test]
    async fn test_fold_with_user_word() {
        let mut interp = Interpreter::new();
        let def_code = "{ + } 'MYSUM' DEF";
        let def_result = interp.execute(def_code).await;
        assert!(def_result.is_ok(), "DEF should succeed: {:?}", def_result);

        let fold_code = "[ 1 2 3 4 ] [ 0 ] 'MYSUM' FOLD";
        let result = interp.execute(fold_code).await;

        assert!(
            result.is_ok(),
            "FOLD with custom word should succeed: {:?}",
            result
        );

        assert_eq!(
            interp.stack.len(),
            1,
            "Stack should have exactly 1 element, got {}",
            interp.stack.len()
        );
    }
}
