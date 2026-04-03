#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueData;

    fn assert_stack_top_scalar(interp: &Interpreter, expected: i64, msg: &str) {
        let val = interp.stack.last().expect("Stack should not be empty");
        if let ValueData::Vector(children) = &val.data {
            assert_eq!(children.len(), 1, "{}: expected single-element vector", msg);
            let f = children[0].as_scalar().expect("Expected scalar");
            assert_eq!(f.to_i64().unwrap(), expected, "{}", msg);
        } else {
            panic!("{}: expected vector, got {:?}", msg, val.data);
        }
    }

    #[tokio::test]
    async fn test_branch_guard_sign_word() {
        let mut interp = Interpreter::new();
        let def = r#"{ $ { ,, [ 0 ] < } { [ -1 ] } $ { ,, [ 0 ] = } { [ 0 ] } $ { [ 1 ] } } 'SIGN' DEF"#;
        interp.execute(def).await.unwrap();

        interp.execute("[ -5 ] SIGN").await.unwrap();
        assert_stack_top_scalar(&interp, -1, "SIGN(-5)");
        interp.stack.clear();

        interp.execute("[ 0 ] SIGN").await.unwrap();
        assert_stack_top_scalar(&interp, 0, "SIGN(0)");
        interp.stack.clear();

        interp.execute("[ 3 ] SIGN").await.unwrap();
        assert_stack_top_scalar(&interp, 1, "SIGN(3)");
    }

    #[tokio::test]
    async fn test_branch_guard_requires_default_block() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 3 ] $ { ,, [ 0 ] < } { [ -1 ] * }").await;
        assert!(result.is_err(), "$ without final default block must fail");
    }

    #[tokio::test]
    async fn test_loop_guard_doubling() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 1 ] & { ,, [ 1000 ] < } { [ 2 ] * }")
            .await;
        assert!(result.is_ok(), "loop guard should succeed: {:?}", result);
        assert_stack_top_scalar(&interp, 1024, "double until >= 1000");
    }

    #[tokio::test]
    async fn test_loop_guard_limit_error() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 ] & { TRUE } { IDLE }").await;
        assert!(result.is_err(), "loop above 10,000 iterations must fail");
    }

    #[tokio::test]
    async fn test_idle_no_effect() {
        let mut interp = Interpreter::new();
        interp.execute("[ 3 ] IDLE").await.unwrap();
        assert_stack_top_scalar(&interp, 3, "IDLE should not change flow");
    }
}
