#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_keep_mode_basic() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[1] [2] ,, +").await;
        assert!(
            result.is_ok(),
            "Keep mode addition should succeed: {:?}",
            result
        );

        assert_eq!(
            interp.stack.len(),
            3,
            "Stack should have 3 elements after keep mode operation"
        );
    }

    #[tokio::test]
    async fn test_modifiers_order_independent_stack_keep() {
        let mut interp = Interpreter::new();

        let result1 = interp.execute("[1] [2] [3] [3] .. ,, +").await;
        assert!(
            result1.is_ok(),
            "Stack+Keep mode (.. ,,) should succeed: {:?}",
            result1
        );
        let stack1 = interp.stack.clone();

        interp.execute_reset().unwrap();

        let result2 = interp.execute("[1] [2] [3] [3] ,, .. +").await;
        assert!(
            result2.is_ok(),
            "Stack+Keep mode (,, ..) should succeed: {:?}",
            result2
        );
        let stack2 = interp.stack.clone();

        assert_eq!(
            stack1.len(),
            stack2.len(),
            "Both modifier orders should produce same stack length: {} vs {}",
            stack1.len(),
            stack2.len()
        );
    }

    #[tokio::test]
    async fn test_consume_mode_default() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[1] [2] +").await;
        assert!(
            result.is_ok(),
            "Default consume mode should work: {:?}",
            result
        );

        assert_eq!(
            interp.stack.len(),
            1,
            "Stack should have 1 element after consume mode operation"
        );
    }

    #[tokio::test]
    async fn test_explicit_consume_mode() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[1] [2] , +").await;
        assert!(
            result.is_ok(),
            "Explicit consume mode should work: {:?}",
            result
        );

        assert_eq!(
            interp.stack.len(),
            1,
            "Stack should have 1 element after explicit consume mode"
        );
    }

    #[tokio::test]
    async fn test_mode_reset_after_word() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[1] [2] ,, + [3] +").await;
        assert!(result.is_ok(), "Mode should reset after word: {:?}", result);

        assert_eq!(
            interp.stack.len(),
            3,
            "Stack should have 3 elements: {:?}",
            interp.stack
        );
    }

    #[tokio::test]
    async fn test_keep_mode_with_mul() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[3] [4] ,, *").await;
        assert!(
            result.is_ok(),
            "Keep mode multiplication should succeed: {:?}",
            result
        );

        assert_eq!(
            interp.stack.len(),
            3,
            "Stack should have 3 elements after keep mode multiplication"
        );
    }

    #[tokio::test]
    async fn test_keep_mode_with_sub() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[10] [3] ,, -").await;
        assert!(
            result.is_ok(),
            "Keep mode subtraction should succeed: {:?}",
            result
        );

        assert_eq!(
            interp.stack.len(),
            3,
            "Stack should have 3 elements after keep mode subtraction"
        );
    }

    #[tokio::test]
    async fn test_keep_mode_with_div() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[12] [4] ,, /").await;
        assert!(
            result.is_ok(),
            "Keep mode division should succeed: {:?}",
            result
        );

        assert_eq!(
            interp.stack.len(),
            3,
            "Stack should have 3 elements after keep mode division"
        );
    }


    #[tokio::test]
    async fn test_safe_mode_normal_execution() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 2 3 ] [ 1 ] ~ GET").await;
        assert!(
            result.is_ok(),
            "Safe mode should not affect normal execution: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 2);
        assert!(
            !interp.stack[1].is_nil(),
            "Result should not be NIL on success"
        );
    }

    #[tokio::test]
    async fn test_safe_mode_error_returns_nil() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 2 3 ] [ 10 ] ~ GET").await;
        assert!(
            result.is_ok(),
            "Safe mode should suppress error: {:?}",
            result
        );
        assert_eq!(
            interp.stack.len(),
            3,
            "Stack should have 3 elements (restored + NIL): {:?}",
            interp.stack
        );
        assert!(
            interp.stack.last().unwrap().is_nil(),
            "Top should be NIL on error"
        );
    }

    #[tokio::test]
    async fn test_safe_mode_with_nil_coalesce() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 2 3 ] [ 10 ] ~ GET => [ 0 ]").await;
        assert!(
            result.is_ok(),
            "Safe mode with nil coalesce should work: {:?}",
            result
        );
        assert!(
            !interp.stack.last().unwrap().is_nil(),
            "Top should be the default value [0], not NIL"
        );
    }

    #[tokio::test]
    async fn test_safe_mode_division_by_zero() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 10 ] [ 0 ] ~ /").await;
        assert!(
            result.is_ok(),
            "Safe mode should suppress division by zero: {:?}",
            result
        );
        assert_eq!(
            interp.stack.len(),
            3,
            "Stack should have 3 elements: {:?}",
            interp.stack
        );
        assert!(
            interp.stack.last().unwrap().is_nil(),
            "Top should be NIL on division by zero"
        );
    }

    #[tokio::test]
    async fn test_safe_mode_stack_restore_on_error() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 100 ] [ 1 2 3 ] [ 10 ] ~ GET").await;
        assert!(
            result.is_ok(),
            "Safe mode should suppress error: {:?}",
            result
        );
        assert_eq!(
            interp.stack.len(),
            4,
            "Stack should have 4 elements: {:?}",
            interp.stack
        );
        assert!(
            interp.stack.last().unwrap().is_nil(),
            "Top of stack should be NIL"
        );
    }

    #[tokio::test]
    async fn test_safe_mode_resets_after_word() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 2 3 ] [ 10 ] ~ GET").await;
        assert!(result.is_ok());

        let result2 = interp.execute("[ 1 2 3 ] [ 10 ] GET").await;
        assert!(result2.is_err(), "Second GET without ~ should fail");
    }

    #[tokio::test]
    async fn test_safe_mode_with_keep_mode() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 2 3 ] [ 10 ] ~ ,, GET").await;
        assert!(
            result.is_ok(),
            "Safe mode with keep mode should work: {:?}",
            result
        );
        assert!(interp.stack.last().unwrap().is_nil(), "Top should be NIL");
    }

    #[tokio::test]
    async fn test_safe_mode_unknown_word() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 ] ~ NONEXISTENT").await;
        assert!(
            result.is_ok(),
            "Safe mode should suppress unknown word error: {:?}",
            result
        );
        assert_eq!(
            interp.stack.len(),
            2,
            "Stack should have 2 elements: {:?}",
            interp.stack
        );
        assert!(
            interp.stack.last().unwrap().is_nil(),
            "Top should be NIL for unknown word"
        );
    }

    #[tokio::test]
    async fn test_safe_mode_with_reverse_singleton() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 ] ~ REVERSE").await;
        assert!(
            result.is_ok(),
            "Safe mode should allow REVERSE on singleton vector: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1, "Stack should remain unchanged: {:?}", interp.stack);
        assert!(interp.stack.last().unwrap().is_vector(), "Top should be the reversed vector");
    }
}
