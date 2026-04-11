#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueData;

    #[tokio::test]
    async fn test_stack_get_basic() {
        let mut interp = Interpreter::new();

        let code = "[5] [0] .. GET";

        let result = interp.execute(code).await;
        assert!(result.is_ok());
        assert_eq!(
            interp.stack.len(),
            1,
            "Stack+Consume GET should consume stack and push result only"
        );
    }

    #[tokio::test]
    async fn test_stack_get_with_guard_and_comparison() {
        let mut interp = Interpreter::new();

        let code = "[10] [20] [30] [1] ,, .. GET [20] =";

        let result = interp.execute(code).await;
        assert!(result.is_ok());
        assert_eq!(interp.stack.len(), 4);
        let val = &interp.stack[3];
        // Comparison returns a single-element vector boolean [ TRUE ]
        assert_eq!(val.len(), 1, "Expected single-element vector boolean");
        let inner = val.get_child(0).expect("Expected inner element");
        assert!(
            !inner.as_scalar().expect("Expected scalar in result").is_zero(),
            "Expected TRUE from comparison"
        );
    }

    #[tokio::test]
    async fn test_simple_addition() {
        let mut interp = Interpreter::new();

        let code = "[2] [3] +";

        let result = interp.execute(code).await;
        assert!(
            result.is_ok(),
            "Simple addition should succeed: {:?}",
            result
        );

        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
    }

    #[tokio::test]
    async fn test_definition_and_call() {
        let mut interp = Interpreter::new();

        let code = r#"
{ [2] [3] + } 'ADDTEST' DEF
ADDTEST
"#;

        let result = interp.execute(code).await;
        assert!(
            result.is_ok(),
            "Definition and call should succeed: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_default_line_without_colon() {
        let mut interp = Interpreter::new();

        let code = "[5] [3] +";

        let result = interp.execute(code).await;
        assert!(
            result.is_ok(),
            "Default line without colon should succeed: {:?}",
            result
        );

        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                assert_eq!(
                    children[0]
                        .as_scalar()
                        .expect("Expected scalar")
                        .numerator()
                        .to_string(),
                    "8",
                    "Result should be 8"
                );
            } else {
                panic!("Expected vector result from addition");
            }
        }
    }

    #[tokio::test]
    async fn test_def_with_new_syntax() {
        let mut interp = Interpreter::new();

        let code = "{ [ 42 ] } 'ANSWER' DEF";

        let result = interp.execute(code).await;
        assert!(
            result.is_ok(),
            "DEF with new syntax should succeed: {:?}",
            result
        );

        assert!(
            interp.user_words.contains_key("ANSWER"),
            "ANSWER should be defined"
        );

        let call_result = interp.execute("ANSWER").await;
        assert!(call_result.is_ok(), "Calling ANSWER should succeed");
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
    }

    #[tokio::test]
    async fn test_multiple_lines_without_colon() {
        let mut interp = Interpreter::new();

        let code = r#"
[1] [2] +
[3] *
"#;

        let result = interp.execute(code).await;
        assert!(
            result.is_ok(),
            "Multiple lines without colon should succeed: {:?}",
            result
        );

        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                assert_eq!(
                    children[0]
                        .as_scalar()
                        .expect("Expected scalar")
                        .numerator()
                        .to_string(),
                    "9",
                    "Result should be 9"
                );
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_sequential_execution() {
        let mut interp = Interpreter::new();

        let code = r#"
[10] [20] +
[5] *
"#;

        let result = interp.execute(code).await;
        assert!(
            result.is_ok(),
            "Sequential lines should succeed: {:?}",
            result
        );

        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                assert_eq!(
                    children[0]
                        .as_scalar()
                        .expect("Expected scalar")
                        .numerator()
                        .to_string(),
                    "150",
                    "Result should be 150"
                );
            } else {
                panic!("Expected vector result");
            }
        }
    }

}
