#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_now_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        let result = interp.execute(".. NOW").await;
        assert!(result.is_err(), "NOW should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("NOW") && err_msg.contains("Stack mode"),
            "Expected Stack mode error for NOW, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_datetime_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        let result = interp.execute("[ 1732531200 ] 'LOCAL' .. DATETIME").await;
        assert!(result.is_err(), "DATETIME should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("DATETIME") && err_msg.contains("Stack mode"),
            "Expected Stack mode error for DATETIME, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_timestamp_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        let result = interp
            .execute("[ [ 2024 11 25 14 0 0 ] ] 'LOCAL' .. TIMESTAMP")
            .await;
        assert!(result.is_err(), "TIMESTAMP should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("TIMESTAMP") && err_msg.contains("Stack mode"),
            "Expected Stack mode error for TIMESTAMP, got: {}",
            err_msg
        );
    }
}
