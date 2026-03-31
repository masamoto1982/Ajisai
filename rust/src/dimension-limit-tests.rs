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
        let result = interp.execute("[ [ [ [ 1 ] ] ] ]").await;
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
            .execute("[ [ [ [ [ [ [ [ [ 1 ] ] ] ] ] ] ] ] ]")
            .await;
        assert!(
            result.is_ok(),
            "9 visible dimensions (10 total) should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_dimension_limit_exceeds_at_10_visible() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        let result = interp
            .execute("[ [ [ [ [ [ [ [ [ [ 1 ] ] ] ] ] ] ] ] ] ]")
            .await;
        assert!(
            result.is_err(),
            "10 visible dimensions (11 total) should result in an error"
        );

        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("10 dimensions"),
            "Error message should mention '10 dimensions', got: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_dimension_error_message_format() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        let result = interp
            .execute("[ [ [ [ [ [ [ [ [ [ 1 ] ] ] ] ] ] ] ] ] ]")
            .await;
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Nesting depth limit exceeded"),
            "Error message should mention 'Nesting depth limit exceeded', got: {}",
            error_msg
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
        assert!(result.contains("[ 1 2 ]"), "2D inner should use [ ], got: {}", result);
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
            .execute("[ [ [ 1 ] [ 2 ] [ 3 ] ] [ [ 4 ] [ 5 ] [ 6 ] ] ]")
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
            result, "[ [ [ 1 ] [ 2 ] [ 3 ] ] [ [ 4 ] [ 5 ] [ 6 ] ] ]",
            "Expected 3D structure"
        );
    }

    #[tokio::test]
    async fn test_bracket_display_4d() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ [ [ [ 1 ] ] ] ]").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert_eq!(
            result, "[ [ [ [ 1 ] ] ] ]",
            "4D should keep [ ] brackets: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_bracket_display_9d() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp
            .execute("[ [ [ [ [ [ [ [ [ 1 ] ] ] ] ] ] ] ] ]")
            .await
            .unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert_eq!(
            result, "[ [ [ [ [ [ [ [ [ 1 ] ] ] ] ] ] ] ] ]",
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
}
