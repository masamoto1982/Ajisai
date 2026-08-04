//! Test suite for interpreter word-definition behavior.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueData;

    #[tokio::test]
    async fn test_map_with_increment() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("{ [ 1 ] + } 'INC' DEF [ 1 2 3 ] 'INC' MAP")
            .await;
        assert!(
            result.is_ok(),
            "MAP with increment function should succeed: {:?}",
            result
        );

        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            assert!(val.is_vector(), "Expected vector result");
            assert_eq!(val.len(), 3, "Result should have 3 elements");
            let extract = |i: usize| -> String {
                val.child(i)
                    .expect("child within len")
                    .as_scalar()
                    .expect("Expected scalar")
                    .numerator()
                    .to_string()
            };
            assert_eq!(extract(0), "2", "First element should be 2");
            assert_eq!(extract(1), "3", "Second element should be 3");
            assert_eq!(extract(2), "4", "Third element should be 4");
        }
    }

    #[tokio::test]
    async fn test_nested_empty_vector_is_a_value() {
        let mut interp = Interpreter::new();

        interp
            .execute("[ [ ] ]")
            .await
            .expect("`[ [ ] ]` is a value");
        let outer = interp.stack.last().expect("a result");
        assert_eq!(outer.len(), 1, "one element, which is the empty vector");
        assert_eq!(outer.child(0).expect("the element").len(), 0);
    }

    #[tokio::test]
    async fn test_empty_brackets_are_the_empty_vector() {
        let mut interp = Interpreter::new();

        interp.execute("[ ]").await.expect("`[ ]` is a value");
        let val = interp.stack.last().expect("a result");
        assert!(!val.is_nil(), "the empty vector is not an absence");
        assert_eq!(val.len(), 0);
    }

    // The dependency guard on DEF/DEL is unconditional: the vocabulary has no
    // force modifier, so these tests pin the only two outcomes that exist —
    // allowed when nothing refers to the word, refused when something does.
    // They previously drove a `force_flag`, but nothing could ever set it, so
    // only the refusing side was ever exercised.

    #[tokio::test]
    async fn test_del_without_dependents_succeeds() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await.unwrap();

        let result = interp.execute("'DOUBLE' DEL").await;
        assert!(result.is_ok());
        assert!(!interp.user_words.contains_key("DOUBLE"));
    }

    #[tokio::test]
    async fn test_del_with_dependents_is_refused() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await.unwrap();
        interp
            .execute("{ DOUBLE DOUBLE } 'QUAD' DEF")
            .await
            .unwrap();

        let result = interp.execute("'DOUBLE' DEL").await;
        let message = result
            .expect_err("DEL of a referenced word is refused")
            .to_string();
        assert!(
            message.contains("Cannot delete") && message.contains("QUAD"),
            "refusal must name the dependents: {message}"
        );
        assert!(interp.user_words.contains_key("DOUBLE"));
    }

    /// Deleting the dependent first clears the guard — the refusal is about the
    /// dependency, not about the word being undeletable.
    #[tokio::test]
    async fn test_del_succeeds_once_the_dependent_is_gone() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await.unwrap();
        interp
            .execute("{ DOUBLE DOUBLE } 'QUAD' DEF")
            .await
            .unwrap();

        interp.execute("'QUAD' DEL").await.unwrap();
        interp.execute("'DOUBLE' DEL").await.unwrap();
        assert!(!interp.user_words.contains_key("DOUBLE"));
    }

    #[tokio::test]
    async fn test_def_with_dependents_is_refused() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await.unwrap();
        interp
            .execute("{ DOUBLE DOUBLE } 'QUAD' DEF")
            .await
            .unwrap();

        let result = interp.execute("{ [ 3 ] * } 'DOUBLE' DEF").await;
        let message = result
            .expect_err("redefining a referenced word is refused")
            .to_string();
        assert!(
            message.contains("Cannot redefine") && message.contains("QUAD"),
            "refusal must name the dependents: {message}"
        );
    }

    #[tokio::test]
    async fn test_del_of_a_builtin_is_refused() {
        let mut interp = Interpreter::new();

        let result = interp.execute("'+' DEL").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_nil_keyword_works() {
        let mut interp = Interpreter::new();

        let result = interp.execute("NIL").await;
        assert!(result.is_ok(), "NIL keyword should work: {:?}", result);

        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil(), "Expected NIL, got {:?}", val);
        }
    }

    #[tokio::test]
    async fn test_nil_in_vector() {
        let mut interp = Interpreter::new();

        let result = interp.execute("[ 1 NIL 2 ]").await;
        assert!(result.is_ok(), "NIL in vector should work: {:?}", result);

        assert_eq!(interp.stack.len(), 1);

        let vec_val = &interp.stack[0];
        assert_eq!(
            vec_val.shape(),
            vec![3],
            "Vector should have 3 elements including NIL"
        );
        assert!(vec_val.is_vector(), "Expected vector");
        assert_eq!(vec_val.len(), 3, "Data should have 3 elements");
        let second = vec_val.child(1).expect("len==3 implies child(1) exists");
        assert!(second.is_nil(), "Second element should be NIL");
    }

    #[tokio::test]
    async fn test_nil_is_value() {
        use crate::types::Value;

        let nil = Value::nil();
        assert!(nil.is_nil(), "Value::nil() should be NIL");
        assert!(nil.shape().is_empty(), "NIL should be scalar (empty shape)");
        assert!(
            matches!(nil.data, ValueData::Nil),
            "NIL should be ValueData::Nil"
        );
    }

    #[tokio::test]
    async fn test_nil_arithmetic_propagation() {
        let nil = crate::types::fraction::Fraction::nil();
        let one = crate::types::fraction::Fraction::from(1);
        let result = nil.add(&one);
        assert!(result.is_nil(), "NIL + 1 should be NIL");

        let result = one.mul(&nil);
        assert!(result.is_nil(), "1 * NIL should be NIL");
    }

    #[tokio::test]
    async fn test_nil_and_true_returns_nil() {
        let mut interp = Interpreter::new();
        let result = interp.execute("NIL TRUE AND").await;
        assert!(result.is_ok(), "NIL AND TRUE should work: {:?}", result);
        let val = interp.stack.pop().unwrap();
        assert!(
            val.is_nil(),
            "NIL AND TRUE should return NIL, got {:?}",
            val
        );
    }

    #[tokio::test]
    async fn test_nil_or_false_returns_nil() {
        let mut interp = Interpreter::new();
        let result = interp.execute("NIL FALSE OR").await;
        assert!(result.is_ok(), "NIL OR FALSE should work: {:?}", result);
        let val = interp.stack.pop().unwrap();
        assert!(
            val.is_nil(),
            "NIL OR FALSE should return NIL, got {:?}",
            val
        );
    }

    #[tokio::test]
    async fn test_not_nil_returns_nil() {
        let mut interp = Interpreter::new();
        let result = interp.execute("NIL NOT").await;
        assert!(result.is_ok(), "NOT NIL should work: {:?}", result);
        let val = interp.stack.pop().unwrap();
        assert!(val.is_nil(), "NOT NIL should return NIL, got {:?}", val);
    }

    #[tokio::test]
    async fn test_true_and_nil_returns_nil() {
        let mut interp = Interpreter::new();
        let result = interp.execute("TRUE NIL AND").await;
        assert!(result.is_ok(), "TRUE NIL AND should work: {:?}", result);
        let val = interp.stack.pop().unwrap();
        assert!(
            val.is_nil(),
            "TRUE NIL AND should return NIL, got {:?}",
            val
        );
    }

    #[tokio::test]
    async fn test_nested_call_chain_4_levels_ok() {
        let mut interp = Interpreter::new();
        interp.execute("{ B } 'A' DEF").await.unwrap();
        interp.execute("{ C } 'B' DEF").await.unwrap();
        interp.execute("{ D } 'C' DEF").await.unwrap();
        interp.execute("{ [ 1 ] } 'D' DEF").await.unwrap();

        let result = interp.execute("A").await;
        assert!(
            result.is_ok(),
            "4-level nested call chain should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_deep_call_chain_succeeds() {
        let mut interp = Interpreter::new();
        interp.execute("{ B } 'A' DEF").await.unwrap();
        interp.execute("{ C } 'B' DEF").await.unwrap();
        interp.execute("{ D } 'C' DEF").await.unwrap();
        interp.execute("{ E } 'D' DEF").await.unwrap();
        interp.execute("{ [ 1 ] } 'E' DEF").await.unwrap();

        let result = interp.execute("A").await;
        assert!(
            result.is_ok(),
            "Deep call chain should succeed: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_direct_recursion_hits_execution_limit() {
        let mut interp = Interpreter::new();
        interp.max_execution_steps = 64;
        interp.execute("{ REC } 'REC' DEF").await.unwrap();

        let result = interp.execute("REC").await;
        assert!(
            result.is_err(),
            "Direct recursion should hit execution limit"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Execution step limit"),
            "Error message should mention execution step limit: {}",
            err_msg
        );
    }

    // Task 6: infinite recursion must surface as a recoverable AjisaiError
    // and never as a WASM trap from Rust stack exhaustion. The depth guard
    // fires before the execution-step guard when steps are left high.
    #[tokio::test]
    async fn test_infinite_recursion_hits_recursion_depth_limit() {
        let mut interp = Interpreter::new();
        // Leave the step limit at default so the depth guard fires first.
        interp.execute("{ REC } 'REC' DEF").await.unwrap();

        let result = interp.execute("REC").await;
        assert!(
            result.is_err(),
            "Direct recursion should produce an error, not a trap"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("recursion limit exceeded"),
            "Error message should mention recursion limit: {}",
            err_msg
        );
    }

    // Task 6: the depth guard must not regress legal deep call chains
    // (non-recursive). 5-level chain stays comfortably under the cap.
    #[tokio::test]
    async fn test_depth_guard_does_not_break_legal_deep_chain() {
        let mut interp = Interpreter::new();
        interp.execute("{ B } 'A' DEF").await.unwrap();
        interp.execute("{ C } 'B' DEF").await.unwrap();
        interp.execute("{ D } 'C' DEF").await.unwrap();
        interp.execute("{ E } 'D' DEF").await.unwrap();
        interp.execute("{ [ 1 ] } 'E' DEF").await.unwrap();

        let result = interp.execute("A").await;
        assert!(
            result.is_ok(),
            "Legal 5-level chain should not trigger the depth guard: {:?}",
            result
        );
        // call_depth must be back to 0 after a successful run.
        assert_eq!(
            interp.call_depth, 0,
            "call_depth should reset to 0 on success"
        );
    }

    // Task 6: after a recursion-limit error, call_depth must reset to 0 so
    // the next program is not penalized by the previous run's leftover state.
    #[tokio::test]
    async fn test_call_depth_resets_after_recursion_error() {
        let mut interp = Interpreter::new();
        interp.execute("{ REC } 'REC' DEF").await.unwrap();
        let _ = interp.execute("REC").await;
        assert_eq!(
            interp.call_depth, 0,
            "call_depth should unwind back to 0 once the error has propagated"
        );

        // Subsequent normal execution must still work.
        interp.execute("{ [ 1 ] } 'OK' DEF").await.unwrap();
        let result = interp.execute("OK").await;
        assert!(
            result.is_ok(),
            "After a recursion error a normal call should still succeed: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_call_chain_state_resets_after_completion() {
        let mut interp = Interpreter::new();
        interp.execute("{ B } 'A' DEF").await.unwrap();
        interp.execute("{ [ 1 ] } 'B' DEF").await.unwrap();

        let result1 = interp.execute("A").await;
        assert!(result1.is_ok(), "First call should succeed");

        let result2 = interp.execute("A").await;
        assert!(
            result2.is_ok(),
            "Second call should succeed (call_stack should reset)"
        );
    }

    #[tokio::test]
    async fn test_execution_limit_error_message() {
        let mut interp = Interpreter::new();
        interp.max_execution_steps = 64;
        interp.execute("{ REC } 'REC' DEF").await.unwrap();
        let result = interp.execute("REC").await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Execution step limit"),
            "Error message should mention execution step limit: {}",
            err_msg
        );
    }
    #[tokio::test]
    async fn test_def_with_code_block_body() {
        // DEF is strictly `{ body } 'NAME' DEF`. The body is a code block, not
        // a data array of source strings.
        let mut interp = Interpreter::new();
        interp
            .execute("{ [ 2 ] * } 'DOUBLE' DEF")
            .await
            .expect("DEF with code-block body should succeed");
        let result = interp.execute("[ 21 ] DOUBLE").await;
        assert!(result.is_ok(), "DOUBLE should run: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        let val = interp.stack.last().expect("result present");
        assert!(val.is_vector(), "Expected vector result");
        assert_eq!(val.len(), 1);
        assert_eq!(
            val.child(0)
                .expect("child")
                .as_scalar()
                .expect("scalar")
                .numerator()
                .to_string(),
            "42"
        );
    }

    #[tokio::test]
    async fn test_def_with_multiline_code_block_body() {
        // A multi-line `{ }` body executes one source line at a time:
        // `[ 10 ] -> +1 -> *2 = 22`.
        let mut interp = Interpreter::new();
        interp
            .execute("{\n[ 1 ] +\n[ 2 ] *\n} 'INCDOUBLE' DEF")
            .await
            .expect("multi-line code-block body should succeed");
        let result = interp.execute("[ 10 ] INCDOUBLE").await;
        assert!(result.is_ok(), "INCDOUBLE should run: {:?}", result);
        let val = interp.stack.last().expect("result present");
        assert_eq!(
            val.child(0)
                .expect("child")
                .as_scalar()
                .expect("scalar")
                .numerator()
                .to_string(),
            "22"
        );
    }

    #[tokio::test]
    async fn test_def_rejects_data_array_body() {
        // A data array `[ ... ]` is no longer accepted as a definition body;
        // only a code block is. Canonical code data must first be reflected.
        let mut interp = Interpreter::new();
        let result = interp.execute("[ '[ 2 ] *' ] 'DOUBLE' DEF").await;
        assert!(
            result.is_err(),
            "DEF with a data-array body should be rejected"
        );
    }
    #[tokio::test]
    async fn test_def_ignores_leftover_string_like_value() {
        // A leftover string-like value below the body must not shift argument
        // interpretation: DEF reads exactly the top two positions (name, body).
        let mut interp = Interpreter::new();
        interp
            .execute("'leftover' { [ 2 ] * } 'DOUBLE' DEF")
            .await
            .expect("leftover string must not disturb DEF args");
        // The leftover value is still on the stack, untouched.
        let result = interp.execute("[ 21 ] DOUBLE").await;
        assert!(result.is_ok(), "DOUBLE should run: {:?}", result);
        let val = interp.stack.last().expect("result present");
        assert_eq!(
            val.child(0)
                .expect("child")
                .as_scalar()
                .expect("scalar")
                .numerator()
                .to_string(),
            "42"
        );
    }
}
