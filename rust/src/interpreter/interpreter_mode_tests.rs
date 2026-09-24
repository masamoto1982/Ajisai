//! Test suite for interpreter operand consumption.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_consume_mode_default() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 ] [ 2 ] +").await;
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
}
