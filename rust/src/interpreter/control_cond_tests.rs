//! Test suite for `crate::interpreter::control_cond`.

#[cfg(test)]
mod tests {
    #[cfg(feature = "elastic-engine")]
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
            .execute("[ 0 ]\n{ [ 0 ] < | 'negative' }\n{ [ 0 ] = | 'zero' }\n{ IDLE | 'positive' }\nCOND")
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
            .execute("[ 42 ]\n{ [ 0 ] < | 'negative' }\n{ IDLE | 'positive' }\nCOND")
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
            .execute("[ 42 ] { [ 0 ] < | 'negative' } { IDLE } { 'positive' } COND")
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
        let result = interp.execute("[ 42 ] { [ 0 ] < | 'a' | 'b' } COND").await;
        assert!(result.is_err(), "multiple separators should fail");
        let message = result.err().unwrap().to_string();
        assert!(
            message.contains("exactly one '|' separator"),
            "unexpected error: {}",
            message
        );
    }

    #[tokio::test]
    async fn test_cond_clause_separator_left_side_required() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 42 ] { | 'positive' } COND").await;
        assert!(result.is_err(), "empty guard should fail");
        let message = result.err().unwrap().to_string();
        assert!(
            message.contains("both guard and body are required around '|'"),
            "unexpected error: {}",
            message
        );
    }

    #[tokio::test]
    async fn test_cond_clause_separator_right_side_required() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 42 ] { IDLE | } COND").await;
        assert!(result.is_err(), "empty body should fail");
        let message = result.err().unwrap().to_string();
        assert!(
            message.contains("both guard and body are required around '|'"),
            "unexpected error: {}",
            message
        );
    }

    #[tokio::test]
    async fn test_cond_new_clause_requires_one_clause_per_line() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 42 ] { [ 0 ] < | 'negative' } { IDLE | 'positive' } COND")
            .await;
        assert!(result.is_err(), "same-line multiple | clauses should fail");
        let message = result.err().unwrap().to_string();
        assert!(
            message.contains("COND: | clauses must be written one clause per line"),
            "unexpected error: {}",
            message
        );
    }

    #[cfg(feature = "elastic-engine")]
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

    #[cfg(feature = "elastic-engine")]
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
mod example_words_tests {
    use crate::interpreter::Interpreter;

    async fn setup_example_words(interp: &mut Interpreter) {
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
        interp
            .execute("{ SAY-HELLO SAY-WORLD SAY-BANG } 'GREET' DEF")
            .await
            .unwrap();
        interp
            .execute("{ { [ 15 ] MOD [ 0 ] = } { 'FizzBuzz' PRINT } { [ 3 ] MOD [ 0 ] = } { 'Fizz' PRINT } { [ 5 ] MOD [ 0 ] = } { 'Buzz' PRINT } { TRUE } { ,, PRINT } COND } 'FIZZBUZZ' DEF")
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
    async fn test_greet_chains_say_words() {
        let mut interp = Interpreter::new();
        setup_example_words(&mut interp).await;

        // GREET teaches dependency management: it is nothing but the three
        // SAY words chained together, so calling it prints all three pieces.
        let r = interp.execute("GREET").await;
        assert!(r.is_ok(), "GREET failed: {:?}", r);
        let output = interp.collect_output();
        assert!(output.contains("Hello"), "Expected Hello, got: {}", output);
        assert!(output.contains("World"), "Expected World, got: {}", output);
        assert!(output.contains("!"), "Expected !, got: {}", output);
    }

    #[tokio::test]
    async fn test_greet_depends_on_say_words() {
        let mut interp = Interpreter::new();
        setup_example_words(&mut interp).await;

        // The dependency graph must record that GREET relies on each SAY word.
        for dep in ["EXAMPLE@SAY-HELLO", "EXAMPLE@SAY-WORLD", "EXAMPLE@SAY-BANG"] {
            let dependents = interp.dependents.get(dep);
            assert!(
                dependents.is_some_and(|d| d.contains("EXAMPLE@GREET")),
                "{} should be referenced by EXAMPLE@GREET; got {:?}",
                dep,
                dependents
            );
        }
    }

    #[tokio::test]
    async fn test_fizzbuzz_multiple_of_three() {
        let mut interp = Interpreter::new();
        setup_example_words(&mut interp).await;

        let r = interp.execute("[ 9 ] FIZZBUZZ").await;
        assert!(r.is_ok(), "FIZZBUZZ 9 failed: {:?}", r);
        let output = interp.collect_output();
        assert!(
            output.contains("Fizz") && !output.contains("FizzBuzz"),
            "Expected Fizz, got: {}",
            output
        );
    }

    #[tokio::test]
    async fn test_fizzbuzz_multiple_of_five() {
        let mut interp = Interpreter::new();
        setup_example_words(&mut interp).await;

        let r = interp.execute("[ 10 ] FIZZBUZZ").await;
        assert!(r.is_ok(), "FIZZBUZZ 10 failed: {:?}", r);
        let output = interp.collect_output();
        assert!(output.contains("Buzz"), "Expected Buzz, got: {}", output);
    }

    #[tokio::test]
    async fn test_fizzbuzz_multiple_of_fifteen() {
        let mut interp = Interpreter::new();
        setup_example_words(&mut interp).await;

        // The 15 guard must win over the 3 and 5 guards thanks to its order.
        let r = interp.execute("[ 30 ] FIZZBUZZ").await;
        assert!(r.is_ok(), "FIZZBUZZ 30 failed: {:?}", r);
        let output = interp.collect_output();
        assert!(
            output.contains("FizzBuzz"),
            "Expected FizzBuzz, got: {}",
            output
        );
    }

    #[tokio::test]
    async fn test_fizzbuzz_else_prints_number() {
        let mut interp = Interpreter::new();
        setup_example_words(&mut interp).await;

        let r = interp.execute("[ 7 ] FIZZBUZZ").await;
        assert!(r.is_ok(), "FIZZBUZZ 7 failed: {:?}", r);
        let output = interp.collect_output();
        assert!(output.contains('7'), "Expected 7, got: {}", output);
        assert!(
            !output.contains("Fizz") && !output.contains("Buzz"),
            "Expected no Fizz/Buzz, got: {}",
            output
        );

        // The value remains on the stack after COND.
        assert_eq!(interp.stack.len(), 1);
        let result = &interp.stack[0];
        let scalar = result
            .as_scalar()
            .cloned()
            .or_else(|| result.child(0).and_then(|c| c.as_scalar().cloned()))
            .expect("FIZZBUZZ should leave the numeric value on the stack");
        assert_eq!(scalar.to_i64().unwrap(), 7);
    }
}
