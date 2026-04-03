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
        assert_eq!(val.len(), 1, "Expected single element");
        assert!(
            !val.as_scalar().expect("Expected scalar").is_zero(),
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
    async fn test_route_branch_true_case() {
        let mut interp = Interpreter::new();

        // Condition: 3 < 5 → true → execute action (define ANSWER)
        let code = r#"
$ { [ 3 ] [ 5 ] < } { [ 42 ] 'ANSWER' DEF } $ { [ 0 ] 'ZERO' DEF }
"#;

        let result = interp.execute(code).await;
        assert!(
            result.is_ok(),
            "$ branch should succeed: {:?}",
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
    async fn test_route_branch_false_case() {
        let mut interp = Interpreter::new();

        // Condition: 5 < 3 → false → default (define SMALL)
        let code = r#"
$ { [ 5 ] [ 3 ] < } { [ 100 ] 'BIG' DEF } $ { [ -1 ] 'SMALL' DEF }
"#;

        let result = interp.execute(code).await;
        assert!(
            result.is_ok(),
            "$ branch (false case) should succeed: {:?}",
            result
        );

        assert!(
            !interp.user_words.contains_key("BIG"),
            "BIG should not be defined"
        );
        assert!(
            interp.user_words.contains_key("SMALL"),
            "SMALL should be defined"
        );

        let call_result = interp.execute("SMALL").await;
        assert!(
            call_result.is_ok(),
            "Calling SMALL should succeed: {:?}",
            call_result.err()
        );
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
    }

    #[tokio::test]
    async fn test_route_default_only() {
        let mut interp = Interpreter::new();

        // Single code block (odd count = default only)
        let code = "$ { [ 999 ] 'DEFAULT' DEF }";

        let result = interp.execute(code).await;
        assert!(
            result.is_ok(),
            "$ default-only should succeed: {:?}",
            result
        );

        assert!(
            interp.user_words.contains_key("DEFAULT"),
            "DEFAULT should be defined"
        );

        let call_result = interp.execute("DEFAULT").await;
        assert!(
            call_result.is_ok(),
            "Calling DEFAULT should succeed: {:?}",
            call_result.err()
        );
    }

    #[tokio::test]
    async fn test_route_with_existing_user_word() {
        let mut interp = Interpreter::new();

        let def_code = "{ [ 2 ] * } 'DOUBLE' DEF";
        let result = interp.execute(def_code).await;
        assert!(
            result.is_ok(),
            "DOUBLE definition should succeed: {:?}",
            result
        );

        // Condition: 5 < 10 → true → define PROCESS using DOUBLE
        let route_code = r#"
$ { [ 5 ] [ 10 ] < } { [ 3 ] DOUBLE 'PROCESS' DEF } $ { [ 0 ] 'NOPROCESS' DEF }
"#;
        let result = interp.execute(route_code).await;
        assert!(
            result.is_ok(),
            "$ with existing word should succeed: {:?}",
            result
        );

        assert!(
            interp.user_words.contains_key("PROCESS"),
            "PROCESS should be defined"
        );

        let call_result = interp.execute("PROCESS").await;
        assert!(
            call_result.is_ok(),
            "Calling PROCESS should succeed: {:?}",
            call_result.err()
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

    #[tokio::test]
    async fn test_route_branch_vs_sequential() {
        let mut interp = Interpreter::new();

        // $: condition 3 < 5 → true → [ 100 ]
        let route_code = "[ 3 ] $ { ,, [ 5 ] < } { [ 100 ] } $ { [ 0 ] }";

        let result = interp.execute(route_code).await;
        assert!(
            result.is_ok(),
            "$ branch should succeed: {:?}",
            result
        );

        // The condition evaluated [3] [5] < = TRUE, which is on the stack.
        // $ sees code blocks. The TRUE from condition evaluation is the flow.
        // Let's verify the result.
        assert!(!interp.stack.is_empty(), "Stack should not be empty");

        interp.stack.clear();

        // Sequential: just pushes values
        let sequential_code = r#"
[ 3 ] [ 5 ] <
[100]
[0]
"#;

        let result = interp.execute(sequential_code).await;
        assert!(
            result.is_ok(),
            "Sequential lines should succeed: {:?}",
            result
        );

        assert_eq!(interp.stack.len(), 3, "Stack should have three elements");
    }

    #[tokio::test]
    async fn test_route_multi_condition_default() {
        let mut interp = Interpreter::new();

        // [ 0 ]: condition 0 < 5 → true → action: multiply by 100
        let code = "[ 0 ] $ { ,, [ 5 ] < } { [ 100 ] * } $ { ,, [ 10 ] < } { [ 200 ] * } $ { [ 999 ] + }";

        let result = interp.execute(code).await;
        assert!(
            result.is_ok(),
            "$ multi-condition should succeed: {:?}",
            result
        );

        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                assert_eq!(
                    children[0]
                        .as_scalar()
                        .expect("Expected scalar")
                        .numerator()
                        .to_string(),
                    "0",
                    "0 * 100 = 0"
                );
            } else {
                panic!("Expected vector result");
            }
        }
    }

}
