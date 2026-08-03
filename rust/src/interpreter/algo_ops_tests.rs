//! Test suite for `crate::interpreter::algo_ops` (ALGO UNIQUE/CONTAINS/INDEX-OF).

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn index_of_returns_position() {
        let mut interp = Interpreter::new();
        interp
            .execute("[ 10 20 30 ] 20 INDEX-OF")
            .await
            .expect("should succeed");
        assert_eq!(interp.stack[0].as_scalar().unwrap().to_i64().unwrap(), 1);
    }

    #[tokio::test]
    async fn index_of_missing_is_bubble() {
        let mut interp = Interpreter::new();
        interp
            .execute("[ 10 20 30 ] 99 INDEX-OF")
            .await
            .expect("a search miss is a Bubble, not an error");
        assert_eq!(interp.stack.len(), 1);
        assert!(interp.stack[0].is_nil());
    }
}
