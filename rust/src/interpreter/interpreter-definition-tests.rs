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
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 3, "Result should have 3 elements");
                assert_eq!(
                    children[0]
                        .as_scalar()
                        .expect("Expected scalar")
                        .numerator()
                        .to_string(),
                    "2",
                    "First element should be 2"
                );
                assert_eq!(
                    children[1]
                        .as_scalar()
                        .expect("Expected scalar")
                        .numerator()
                        .to_string(),
                    "3",
                    "Second element should be 3"
                );
                assert_eq!(
                    children[2]
                        .as_scalar()
                        .expect("Expected scalar")
                        .numerator()
                        .to_string(),
                    "4",
                    "Third element should be 4"
                );
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_map_stack_mode() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("{ [ 2 ] * } 'DOUBLE' DEF [ 1 ] [ 2 ] [ 3 ] [ 3 ] 'DOUBLE' .. MAP")
            .await;
        assert!(
            result.is_ok(),
            "MAP in Stack mode should work: {:?}",
            result
        );

        assert_eq!(interp.stack.len(), 3);
    }

    #[tokio::test]
    async fn test_empty_vector_error() {
        let mut interp = Interpreter::new();

        let result = interp.execute("[ [ ] ]").await;
        assert!(result.is_err(), "Empty vector should be an error");
        assert!(result.unwrap_err().to_string().contains("Empty vector"));
    }

    #[tokio::test]
    async fn test_empty_brackets_error() {
        let mut interp = Interpreter::new();

        let result = interp.execute("[ ]").await;
        assert!(result.is_err(), "Empty brackets should be an error");
        assert!(result.unwrap_err().to_string().contains("Empty vector"));
    }


    #[tokio::test]
    async fn test_force_flag_del_without_dependents() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await.unwrap();

        let result = interp.execute("'DOUBLE' DEL").await;
        assert!(result.is_ok());
        assert!(!interp.user_words.contains_key("DOUBLE"));
    }

    #[tokio::test]
    async fn test_force_flag_del_with_dependents_error() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await.unwrap();
        interp
            .execute("{ DOUBLE DOUBLE } 'QUAD' DEF")
            .await
            .unwrap();

        let result = interp.execute("'DOUBLE' DEL").await;
        assert!(result.is_err());
        assert!(interp.user_words.contains_key("DOUBLE"));
    }

    #[tokio::test]
    async fn test_force_flag_del_with_dependents_forced() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await.unwrap();
        interp
            .execute("{ DOUBLE DOUBLE } 'QUAD' DEF")
            .await
            .unwrap();

        let result = interp.execute("! 'DOUBLE' DEL").await;
        assert!(result.is_ok());
        assert!(!interp.user_words.contains_key("DOUBLE"));
        assert!(interp.output_buffer.contains("Warning"));
    }

    #[tokio::test]
    async fn test_force_flag_def_with_dependents_error() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await.unwrap();
        interp
            .execute("{ DOUBLE DOUBLE } 'QUAD' DEF")
            .await
            .unwrap();

        let result = interp.execute("{ [ 3 ] * } 'DOUBLE' DEF").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_force_flag_def_with_dependents_forced() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await.unwrap();
        interp
            .execute("{ DOUBLE DOUBLE } 'QUAD' DEF")
            .await
            .unwrap();

        let result = interp.execute("! [ [ 3 ] * ] 'DOUBLE' DEF").await;
        assert!(result.is_ok());
        assert!(interp.output_buffer.contains("Warning"));
    }

    #[tokio::test]
    async fn test_force_flag_builtin_always_error() {
        let mut interp = Interpreter::new();

        let result = interp.execute("! '+' DEL").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_force_flag_reset_after_other_word() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await.unwrap();
        interp
            .execute("{ DOUBLE DOUBLE } 'QUAD' DEF")
            .await
            .unwrap();

        interp.execute("!").await.unwrap();
        interp.execute("[ 1 2 ] LENGTH").await.unwrap();
        let result = interp.execute("'DOUBLE' DEL").await;
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
        if let ValueData::Vector(children) = &vec_val.data {
            assert_eq!(children.len(), 3, "Data should have 3 elements");
            assert!(children[1].is_nil(), "Second element should be NIL");
        } else {
            panic!("Expected vector");
        }
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
    async fn test_false_and_nil_returns_false() {
        let mut interp = Interpreter::new();
        let result = interp.execute("FALSE NIL AND").await;
        assert!(result.is_ok(), "FALSE AND NIL should work: {:?}", result);
        let val = interp.stack.pop().unwrap();
        assert!(!val.is_nil(), "FALSE AND NIL should return FALSE, not NIL");
        assert!(!val.is_truthy(), "FALSE AND NIL should be falsy");
    }

    #[tokio::test]
    async fn test_false_and_nil_alias_returns_false() {
        let mut interp = Interpreter::new();
        let result = interp.execute("FALSE NIL &").await;
        assert!(result.is_ok(), "FALSE NIL & should work: {:?}", result);
        let val = interp.stack.pop().unwrap();
        assert!(!val.is_nil(), "FALSE NIL & should return FALSE, not NIL");
        assert!(!val.is_truthy(), "FALSE NIL & should be falsy");
    }

    #[tokio::test]
    async fn test_true_and_nil_alias_returns_nil() {
        let mut interp = Interpreter::new();
        let result = interp.execute("TRUE NIL &").await;
        assert!(result.is_ok(), "TRUE NIL & should work: {:?}", result);
        let val = interp.stack.pop().unwrap();
        assert!(val.is_nil(), "TRUE NIL & should return NIL, got {:?}", val);
    }

    #[tokio::test]
    async fn test_true_or_nil_returns_true() {
        let mut interp = Interpreter::new();
        let result = interp.execute("TRUE NIL OR").await;
        assert!(result.is_ok(), "TRUE OR NIL should work: {:?}", result);
        let val = interp.stack.pop().unwrap();
        assert!(!val.is_nil(), "TRUE OR NIL should return TRUE, not NIL");
        assert!(val.is_truthy(), "TRUE OR NIL should be truthy");
    }


    #[tokio::test]
    async fn test_nested_call_chain_4_levels_ok() {
        let mut interp = Interpreter::new();
        interp.execute("{ B } 'A' DEF").await.unwrap();
        interp.execute("{ C } 'B' DEF").await.unwrap();
        interp.execute("{ D } 'C' DEF").await.unwrap();
        interp.execute("{ [ 1 ] } 'D' DEF").await.unwrap();

        let result = interp.execute("A").await;
        assert!(result.is_ok(), "4-level nested call chain should succeed: {:?}", result);
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
        assert!(result.is_ok(), "Deep call chain should succeed: {:?}", result);
    }

    #[tokio::test]
    async fn test_direct_recursion_hits_execution_limit() {
        let mut interp = Interpreter::new();
        interp.max_execution_steps = 64;
        interp.execute("{ REC } 'REC' DEF").await.unwrap();

        let result = interp.execute("REC").await;
        assert!(result.is_err(), "Direct recursion should hit execution limit");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Execution step limit"),
            "Error message should mention execution step limit: {}",
            err_msg
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
}
