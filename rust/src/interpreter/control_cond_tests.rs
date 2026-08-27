//! Test suite for `crate::interpreter::control_cond`.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_cond_exhausted_error() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 5 ] [ [ [ 0 ] < ] [ 'negative' ] ] COND")
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
        let result = interp.execute("[ 1 ] [ [ [ 0 ] < ] ] COND").await;
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
    async fn test_cond_clause_requires_exactly_one_separator() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 42 ] [ [ [ 0 ] < | 'a' | 'b' ] ] COND")
            .await;
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
        let result = interp.execute("[ 42 ] [ [ | 'positive' ] ] COND").await;
        assert!(result.is_err(), "empty guard should fail");
        let message = result.err().unwrap().to_string();
        assert!(
            message.contains("both guard and body are required around '|'"),
            "unexpected error: {}",
            message
        );
    }

    /// Clause order is observable: the first matching guard wins.
    #[tokio::test]
    async fn test_cond_first_matching_clause_wins() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 1 ] [ [ [ 1 ] = ] [ 'first' ] [ [ 1 ] = ] [ 'second' ] ] COND")
            .await;
        assert!(result.is_ok(), "COND should succeed: {:?}", result);
        let as_str = format!("{}", interp.stack.last().expect("stack top"));
        assert!(
            as_str.contains("first"),
            "first matching clause must win in order, got {}",
            as_str
        );
    }
}

#[cfg(test)]
mod example_words_tests {
    use crate::interpreter::Interpreter;

    async fn setup_example_words(interp: &mut Interpreter) {
        interp
            .execute("[ 'Hello' KEEP PRINT ] 'SAY-HELLO' DEF")
            .await
            .unwrap();
        interp
            .execute("[ 'World' KEEP PRINT ] 'SAY-WORLD' DEF")
            .await
            .unwrap();
        interp
            .execute("[ '!' KEEP PRINT ] 'SAY-BANG' DEF")
            .await
            .unwrap();
        interp
            .execute("[ SAY-HELLO SAY-WORLD SAY-BANG ] 'GREET' DEF")
            .await
            .unwrap();
        interp
            .execute("[ [ [ [ 15 ] MOD [ 0 ] = ] [ 'FizzBuzz' PRINT ] [ [ 3 ] MOD [ 0 ] = ] [ 'Fizz' PRINT ] [ [ 5 ] MOD [ 0 ] = ] [ 'Buzz' PRINT ] [ TRUE ] [ KEEP PRINT ] ] COND ] 'FIZZBUZZ' DEF")
            .await
            .unwrap();
        let _ = interp.collect_output();
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
        for dep in ["SAY-HELLO", "SAY-WORLD", "SAY-BANG"] {
            let dependents = interp.dependents.get(dep);
            assert!(
                dependents.is_some_and(|d| d.contains("GREET")),
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

    /// `IDLE` fires where it is written, not at the end.
    ///
    /// The dispatcher used to lift every `IDLE` clause out of the sequence and
    /// try it only after every other guard had failed, so a clause written
    /// *below* an `IDLE` could win — the one place where reading a COND top to
    /// bottom gave the wrong answer. Reaching an `IDLE` means nothing above it
    /// fired, which is exactly when `IDLE` is defined to fire.
    #[tokio::test]
    async fn idle_fires_in_the_order_it_is_written() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("TRUE\n[\n[ IDLE | 1 ]\n[ TRUE | 0 ]\n]\nCOND")
            .await;
        assert!(result.is_ok(), "{:?}", result);
        assert_eq!(
            interp.stack[0].as_scalar().unwrap().to_i64().unwrap(),
            1,
            "the IDLE written first must win over the TRUE guard below it"
        );
    }

    #[tokio::test]
    async fn a_clause_above_idle_still_wins() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("5\n[\n[ 99 GT | 'a' ]\n[ IDLE | 'b' ]\n[ 0 GT | 'c' ]\n]\nCOND")
            .await;
        assert!(result.is_ok(), "{:?}", result);
        assert_eq!(interp.stack[0].as_text().unwrap(), "b");
    }

    /// Two or more `|` clauses on one physical line used to be rejected before
    /// evaluation. Layout is presentation now, so the same COND written flat
    /// denotes the same program.
    #[tokio::test]
    async fn clauses_on_one_line_denote_the_same_program() {
        let mut flat = Interpreter::new();
        let mut tall = Interpreter::new();
        assert!(flat
            .execute("5 [ [ 3 GT | 'big' ] [ IDLE | 'small' ] ] COND")
            .await
            .is_ok());
        assert!(tall
            .execute("5\n[\n[ 3 GT | 'big' ]\n[ IDLE | 'small' ]\n]\nCOND")
            .await
            .is_ok());
        assert_eq!(
            flat.stack[0].as_text().unwrap(),
            tall.stack[0].as_text().unwrap()
        );
        assert_eq!(flat.stack[0].as_text().unwrap(), "big");
    }

    /// A multi-line COND nested inside a Word body used to fail at the *call*
    /// with "Unclosed code block": the body was split into execution lines at
    /// every line break, including the ones interior to the block, so the
    /// first line was the unclosed fragment `{ { 0 GT | 1 }`.
    #[tokio::test]
    async fn a_word_body_may_hold_a_multi_line_cond() {
        let mut interp = Interpreter::new();
        let define = interp
            .execute("[ [ [ [ 0 GT | 1 ]\n[ IDLE | 0 ]\n] COND ] MAP ] 'STEPFN' DEF")
            .await;
        assert!(define.is_ok(), "{:?}", define);
        interp.collect_output();

        let call = interp.execute("[ -1 2 ] STEPFN").await;
        assert!(call.is_ok(), "{:?}", call);
        let result = &interp.stack[0];
        assert_eq!(
            result.child(0).unwrap().as_scalar().unwrap().to_i64(),
            Some(0)
        );
        assert_eq!(
            result.child(1).unwrap().as_scalar().unwrap().to_i64(),
            Some(1)
        );
    }

    /// The same rule for a vector literal: a break inside `[ ]` is interior to
    /// one value, not a separator between two statements.
    #[tokio::test]
    async fn a_word_body_may_hold_a_multi_line_vector() {
        let mut interp = Interpreter::new();
        assert!(interp.execute("[ [ 1\n2\n3 ] ] 'V3' DEF").await.is_ok());
        interp.collect_output();
        assert!(interp.execute("V3").await.is_ok());
        assert_eq!(interp.stack[0].len(), 3);
    }

    /// And the rule it must not break: at the body's own level a line break
    /// still ends a statement.
    #[tokio::test]
    async fn a_break_at_body_level_still_separates_statements() {
        let mut interp = Interpreter::new();
        assert!(interp.execute("[ 1 2 ADD\n10 MUL ] 'W' DEF").await.is_ok());
        interp.collect_output();
        assert!(interp.execute("W").await.is_ok());
        assert_eq!(interp.stack[0].as_scalar().unwrap().to_i64(), Some(30));
    }
}
