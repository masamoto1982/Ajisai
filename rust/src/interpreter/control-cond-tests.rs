#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueData;

    #[tokio::test]
    async fn test_cond_basic_two_branch() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ -5 ] { [ 0 ] < } { 'negative' } { IDLE } { 'positive' } COND")
            .await;
        assert!(result.is_ok(), "COND should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_cond_else_branch_runs() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 5 ] { [ 0 ] < } { 'negative' } { IDLE } { 'positive' } COND")
            .await;
        assert!(result.is_ok(), "COND should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        assert!(matches!(interp.stack.last().unwrap().data, ValueData::Vector(_)));
    }

    #[tokio::test]
    async fn test_cond_exhausted_error() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 5 ] { [ 0 ] < } { 'negative' } COND")
            .await;
        assert!(result.is_err(), "COND should fail without else: {:?}", result);
        let message = result.err().unwrap().to_string();
        assert!(
            message.contains("COND: all guards failed and no else clause"),
            "unexpected error: {}",
            message
        );
    }

    #[tokio::test]
    async fn test_cond_invalid_pair_count_error() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 ] { [ 0 ] < } COND").await;
        assert!(result.is_err(), "COND should fail on odd blocks: {:?}", result);
        let message = result.err().unwrap().to_string();
        assert!(
            message.contains("COND: expected even number of code blocks"),
            "unexpected error: {}",
            message
        );
    }

    #[tokio::test]
    async fn test_cond_keep_mode_no_duplicate() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ -5 ] ,, { [ 0 ] < } { 'negative' } { IDLE } { 'positive' } COND")
            .await;
        assert!(result.is_ok(), "COND with ,, should succeed: {:?}", result);
        assert_eq!(
            interp.stack.len(),
            2,
            "Keep mode should leave original + result (2 items), got {}",
            interp.stack.len()
        );
    }

    #[tokio::test]
    async fn test_cond_non_boolean_guard_error() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 1 ] { [ 2 ] } { 'x' } { IDLE } { 'else' } COND")
            .await;
        assert!(result.is_err(), "COND should fail for non-boolean guard");
        let message = result.err().unwrap().to_string();
        assert!(
            message.contains("COND: guard must return TRUE or FALSE"),
            "unexpected error: {}",
            message
        );
    }
}
