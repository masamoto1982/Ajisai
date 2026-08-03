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
