//! Test suite for tensor dimension-limit enforcement (`crate::interpreter::tensor_ops`).

#[cfg(test)]
mod dimension_limit_tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_dimension_limit_at_3_visible() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        let result = interp.execute("[ [ [ 1 2 3 ] ] ]").await;
        assert!(result.is_ok(), "3 visible dimensions should succeed");

        let stack = interp.get_stack();
        assert_eq!(
            stack.len(),
            1,
            "Stack should have 1 element after parsing 3D tensor"
        );
    }

    #[tokio::test]
    async fn test_dimension_4_visible_succeeds() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        let result = interp.execute("[ [ [ [ 1/1 ] ] ] ]").await;
        assert!(
            result.is_ok(),
            "4 visible dimensions should succeed with new limit"
        );
    }

    #[tokio::test]
    async fn test_dimension_5_visible_succeeds() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        let result = interp.execute("[ [ [ [ [ 1 ] ] ] ] ]").await;
        assert!(result.is_ok(), "5 visible dimensions should succeed");
    }

    #[tokio::test]
    async fn test_dimension_limit_at_9_visible() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        let result = interp
            .execute("[ [ [ [ [ [ [ [ [ 1/1 ] ] ] ] ] ] ] ] ]")
            .await;
        assert!(
            result.is_ok(),
            "9 visible dimensions (10 total) should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_dimension_10_visible_succeeds() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        let result = interp
            .execute("[ [ [ [ [ [ [ [ [ [ 1 ] ] ] ] ] ] ] ] ] ]")
            .await;
        assert!(
            result.is_ok(),
            "10 visible dimensions should succeed after removing the dimension limit"
        );
    }

    #[tokio::test]
    async fn test_deeply_nested_vector_succeeds() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        let result = interp
            .execute("[ [ [ [ [ [ [ [ [ [ [ [ 1 ] ] ] ] ] ] ] ] ] ] ] ]")
            .await;
        assert!(
            result.is_ok(),
            "Deeply nested vectors should succeed after removing the dimension limit"
        );
    }

    #[tokio::test]
    async fn test_bracket_display_1d() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 1 2 3 ]").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert!(
            result.starts_with('['),
            "1D should display with [ ], got: {}",
            result
        );
        assert!(
            result.ends_with(']'),
            "1D should display with [ ], got: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_bracket_display_2d() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ [ 1 2 ] [ 3 4 ] ]").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert!(
            result.starts_with('['),
            "2D outermost should be [ ], got: {}",
            result
        );
        assert!(
            result.contains("[ 1/1 2/1 ]"),
            "2D inner should use [ ], got: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_bracket_display_3d() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ [ [ 1 ] ] ]").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert!(
            result.starts_with('['),
            "3D outermost should be [ ], got: {}",
            result
        );
        assert!(
            result.contains('['),
            "3D innermost should contain [], got: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_bracket_display_3d_complex() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp
            .execute("[ [ [ 1/1 ] [ 2/1 ] [ 3/1 ] ] [ [ 4/1 ] [ 5/1 ] [ 6/1 ] ] ]")
            .await
            .unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert!(
            result.starts_with('['),
            "3D outermost should be [ ], got: {}",
            result
        );
        assert!(
            result.contains('['),
            "3D innermost should contain [], got: {}",
            result
        );
        assert_eq!(
            result, "[ [ [ 1/1 ] [ 2/1 ] [ 3/1 ] ] [ [ 4/1 ] [ 5/1 ] [ 6/1 ] ] ]",
            "Expected 3D structure"
        );
    }

    #[tokio::test]
    async fn test_bracket_display_4d() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ [ [ [ 1/1 ] ] ] ]").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert_eq!(
            result, "[ [ [ [ 1/1 ] ] ] ]",
            "4D should keep [ ] brackets: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_bracket_display_9d() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp
            .execute("[ [ [ [ [ [ [ [ [ 1/1 ] ] ] ] ] ] ] ] ]")
            .await
            .unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert_eq!(
            result, "[ [ [ [ [ [ [ [ [ 1/1 ] ] ] ] ] ] ] ] ]",
            "9D unified [ ] display: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_dotdot_operation_sets_mode() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 1 2 3 ]").await.unwrap();

        let result = interp.execute("..").await;
        assert!(result.is_ok(), ".. operation should succeed");
    }

    // Regression: deeply nested vector literals must be rejected before they
    // build a value whose recursive display/drop overflows the native stack
    // (an unrecoverable abort / WASM trap). See MAX_VECTOR_NESTING_DEPTH.
    fn nested_literal(depth: usize) -> String {
        format!("{}1{}", "[ ".repeat(depth), " ]".repeat(depth))
    }

    #[tokio::test]
    async fn test_excessive_vector_nesting_errors_not_aborts() {
        let mut interp = Interpreter::new();
        let result = interp.execute(&nested_literal(5000)).await;
        assert!(
            result.is_err(),
            "5000-deep vector nesting must be a recoverable error, not a stack-overflow abort"
        );
        assert!(result.unwrap_err().to_string().contains("nesting too deep"));
    }

    #[tokio::test]
    async fn test_excessive_nesting_survivable_and_recovers() {
        // After the deep-nesting error, the interpreter stays usable: it did not
        // build (and then have to recursively drop) the pathological value.
        let mut interp = Interpreter::new();
        assert!(interp.execute(&nested_literal(5000)).await.is_err());
        assert!(
            interp.execute("[ 1 2 3 ]").await.is_ok(),
            "interpreter must remain usable after a deep-nesting rejection"
        );
    }

    #[tokio::test]
    async fn test_moderate_vector_nesting_still_succeeds() {
        // Well within the cap: ordinary nested data keeps working.
        let mut interp = Interpreter::new();
        assert!(
            interp.execute(&nested_literal(32)).await.is_ok(),
            "32-deep nesting is far below the cap and must succeed"
        );
    }
}
