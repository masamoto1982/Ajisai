#[cfg(test)]
mod tests {
    use crate::elastic::ElasticMode;
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
        assert!(matches!(
            interp.stack.last().unwrap().data,
            ValueData::Vector(_)
        ));
    }

    #[tokio::test]
    async fn test_cond_exhausted_error() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 5 ] { [ 0 ] < } { 'negative' } COND")
            .await;
        assert!(
            result.is_err(),
            "COND should fail without else: {:?}",
            result
        );
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
        assert!(
            result.is_err(),
            "COND should fail on odd blocks: {:?}",
            result
        );
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

    #[tokio::test]
    async fn test_cond_new_clause_style_multiple_branches() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 0 ]\n{ [ 0 ] < $ 'negative' }\n{ [ 0 ] = $ 'zero' }\n{ IDLE $ 'positive' }\nCOND")
            .await;
        assert!(
            result.is_ok(),
            "COND new style should succeed: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_cond_new_clause_style_else_branch() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 42 ]\n{ [ 0 ] < $ 'negative' }\n{ IDLE $ 'positive' }\nCOND")
            .await;
        assert!(
            result.is_ok(),
            "COND new-style else should succeed: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_cond_mixed_clause_styles_error() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 42 ] { [ 0 ] < $ 'negative' } { IDLE } { 'positive' } COND")
            .await;
        assert!(result.is_err(), "mixed clause styles should fail");
        let message = result.err().unwrap().to_string();
        assert!(
            message.contains("mixed clause styles are not allowed"),
            "unexpected error: {}",
            message
        );
    }

    #[tokio::test]
    async fn test_cond_clause_requires_exactly_one_separator() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 42 ] { [ 0 ] < $ 'a' $ 'b' } COND").await;
        assert!(result.is_err(), "multiple separators should fail");
        let message = result.err().unwrap().to_string();
        assert!(
            message.contains("exactly one '$' separator"),
            "unexpected error: {}",
            message
        );
    }

    #[tokio::test]
    async fn test_cond_clause_separator_left_side_required() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 42 ] { $ 'positive' } COND").await;
        assert!(result.is_err(), "empty guard should fail");
        let message = result.err().unwrap().to_string();
        assert!(
            message.contains("both guard and body are required around '$'"),
            "unexpected error: {}",
            message
        );
    }

    #[tokio::test]
    async fn test_cond_clause_separator_right_side_required() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 42 ] { IDLE $ } COND").await;
        assert!(result.is_err(), "empty body should fail");
        let message = result.err().unwrap().to_string();
        assert!(
            message.contains("both guard and body are required around '$'"),
            "unexpected error: {}",
            message
        );
    }

    #[tokio::test]
    async fn test_cond_new_clause_requires_one_clause_per_line() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 42 ] { [ 0 ] < $ 'negative' } { IDLE $ 'positive' } COND")
            .await;
        assert!(result.is_err(), "same-line multiple $ clauses should fail");
        let message = result.err().unwrap().to_string();
        assert!(
            message.contains("COND: $ clauses must be written one clause per line"),
            "unexpected error: {}",
            message
        );
    }

    #[tokio::test]
    async fn test_cond_hedged_prefetch_preserves_clause_order() {
        let mut interp = Interpreter::new();
        interp.set_elastic_mode(ElasticMode::HedgedSafe);
        let result = interp
            .execute("[ 1 ] { [ 1 ] = } { 'first' } { [ 1 ] = } { 'second' } COND")
            .await;
        assert!(result.is_ok(), "hedged COND should succeed: {:?}", result);
        let top = interp.stack.last().expect("stack top");
        let as_str = format!("{}", top);
        assert!(
            as_str.contains("first"),
            "first matching clause must win in order, got {}",
            as_str
        );
    }

    #[tokio::test]
    async fn test_cond_hedged_prefetch_metrics_increment() {
        let mut interp = Interpreter::new();
        interp.set_elastic_mode(ElasticMode::HedgedSafe);
        let result = interp
            .execute("[ 5 ] { TRUE } { 'first' } { TRUE } { 'second' } { IDLE } { 'else' } COND")
            .await;
        assert!(result.is_ok(), "hedged COND should succeed: {:?}", result);
        let m = interp.runtime_metrics();
        assert!(
            m.cond_guard_prefetch_count >= 1,
            "expected cond guard prefetch count to increase"
        );
    }
}

#[cfg(test)]
mod demo_word_tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueData;

    async fn setup_demo_words(interp: &mut Interpreter) {
        interp
            .execute("{ 'Hello' ,, PRINT } 'SAY-HELLO' DEF")
            .await
            .unwrap();
        interp
            .execute("{ 'World' ,, PRINT } 'SAY-WORLD' DEF")
            .await
            .unwrap();
        interp
            .execute("{ '!' ,, PRINT } 'SAY-BANG' DEF")
            .await
            .unwrap();
        interp.execute("{ { [ 1 ] = } { SAY-HELLO } { [ 2 ] = } { SAY-WORLD } { IDLE } { SAY-BANG } COND } 'GREET' DEF").await.unwrap();
        interp
            .execute("{ { GREET } MAP } 'GREET-ALL' DEF")
            .await
            .unwrap();
        let _ = interp.collect_output();
    }

    #[tokio::test]
    async fn test_cond_guard_handles_vector_boolean() {
        let mut interp = Interpreter::new();
        let r = interp
            .execute("[ 1 ] { [ 1 ] = } { 'yes' } { IDLE } { 'no' } COND")
            .await;
        assert!(
            r.is_ok(),
            "COND should handle vector boolean guard result: {:?}",
            r
        );
    }

    #[tokio::test]
    async fn test_greet_branch_1() {
        let mut interp = Interpreter::new();
        setup_demo_words(&mut interp).await;

        let r = interp.execute("[ 1 ] GREET").await;
        assert!(r.is_ok(), "GREET 1 failed: {:?}", r);
        let output = interp.collect_output();
        assert!(output.contains("Hello"), "Expected Hello, got: {}", output);
    }

    #[tokio::test]
    async fn test_greet_branch_2() {
        let mut interp = Interpreter::new();
        setup_demo_words(&mut interp).await;

        let r = interp.execute("[ 2 ] GREET").await;
        assert!(r.is_ok(), "GREET 2 failed: {:?}", r);
        let output = interp.collect_output();
        assert!(output.contains("World"), "Expected World, got: {}", output);
    }

    #[tokio::test]
    async fn test_greet_else_branch() {
        let mut interp = Interpreter::new();
        setup_demo_words(&mut interp).await;

        let r = interp.execute("[ 99 ] GREET").await;
        assert!(r.is_ok(), "GREET 99 failed: {:?}", r);
        let output = interp.collect_output();
        assert!(output.contains("!"), "Expected !, got: {}", output);
    }

    #[tokio::test]
    async fn test_greet_all() {
        let mut interp = Interpreter::new();
        setup_demo_words(&mut interp).await;

        let r = interp.execute("[ 1 2 3 ] GREET-ALL").await;
        assert!(r.is_ok(), "GREET-ALL failed: {:?}", r);
        let output = interp.collect_output();
        assert!(
            output.contains("Hello"),
            "Expected Hello in output, got: {}",
            output
        );
        assert!(
            output.contains("World"),
            "Expected World in output, got: {}",
            output
        );
        assert!(
            output.contains("!"),
            "Expected ! in output, got: {}",
            output
        );

        assert_eq!(interp.stack.len(), 1);
        let result = &interp.stack[0];
        assert_eq!(result.len(), 3);

        let bang = result.get_child(2).unwrap();
        assert!(
            matches!(bang.data, ValueData::Vector(_)),
            "Expected '!' to remain as a vector (string), got: {:?}",
            bang.data
        );
    }
}
