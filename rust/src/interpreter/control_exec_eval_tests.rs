//! Test suite for `crate::interpreter::control` (EXEC/EVAL).

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    #[tokio::test]
    async fn test_exec_empty_stack_error() {
        let mut interp = Interpreter::new();

        let result = interp.execute("EXEC").await;

        assert!(result.is_err(), "EXEC on empty stack should fail");
    }

    #[tokio::test]
    async fn test_eval_empty_stack_error() {
        let mut interp = Interpreter::new();

        let result = interp.execute("EVAL").await;

        assert!(result.is_err(), "EVAL on empty stack should fail");
    }
}
