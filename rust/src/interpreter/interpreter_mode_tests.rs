//! Test suite for interpreter modifier/target modes.

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
            "Stack should have 3 elements after keep mode then reset: {:?}",
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

}
