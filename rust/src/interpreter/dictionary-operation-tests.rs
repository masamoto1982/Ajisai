#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueData;

    #[tokio::test]
    async fn test_cannot_override_builtin_word() {
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("[ [ [ 1 ] + ] ] 'GET' DEF").await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Cannot redefine built-in word"),
            "Expected error message to contain 'Cannot redefine built-in word', got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_can_override_user_word() {
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();

        let result1 = interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await;
        assert!(result1.is_ok(), "First definition should succeed");

        let result2 = interp.execute("{ [ 3 ] * } 'DOUBLE' DEF").await;
        assert!(result2.is_ok(), "Overriding user word should succeed");

        let result3 = interp.execute("[ 5 ] DOUBLE").await;
        assert!(result3.is_ok(), "Executing redefined word should succeed");

        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 15, "Result should be 15");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_cannot_override_other_builtin_words() {
        let mut interp = Interpreter::new();

        let builtin_words = vec!["INSERT", "REPLACE", "MAP", "FILTER", "PRINT"];

        for word in builtin_words {
            let code = format!("[ [ 1 ] + ] '{}' DEF", word);
            let result = interp.execute(&code).await;
            assert!(
                result.is_err(),
                "Should not be able to override builtin word: {}",
                word
            );
            let err_msg = result.unwrap_err().to_string();
            assert!(
                err_msg.contains("Cannot redefine built-in word"),
                "Expected error for {}, got: {}",
                word,
                err_msg
            );
        }
    }

    #[tokio::test]
    async fn test_def_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        let result = interp.execute("[ [ [ 2 ] * ] ] 'DOUBLE' .. DEF").await;
        assert!(result.is_err(), "DEF should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("DEF") && err_msg.contains("Stack mode"),
            "Expected Stack mode error for DEF, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_del_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        interp
            .execute("[ [ [ 2 ] * ] ] 'DOUBLE' DEF")
            .await
            .unwrap();

        let result = interp.execute("'DOUBLE' .. DEL").await;
        assert!(result.is_err(), "DEL should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("DEL") && err_msg.contains("Stack mode"),
            "Expected Stack mode error for DEL, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_lookup_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        interp
            .execute("[ [ [ 2 ] * ] ] 'DOUBLE' DEF")
            .await
            .unwrap();

        let result = interp.execute("'DOUBLE' .. ?").await;
        assert!(result.is_err(), "? (LOOKUP) should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("?") && err_msg.contains("Stack mode"),
            "Expected Stack mode error for ?, got: {}",
            err_msg
        );
    }

    fn restore_sample_words(interp: &mut Interpreter, sample_words: &[(&str, &str, &str)]) {
        use crate::tokenizer;

        for (name, definition, description) in sample_words {
            let tokens = tokenizer::tokenize(definition)
                .unwrap_or_else(|e| panic!("Failed to tokenize {}: {}", name, e));

            crate::interpreter::execute_def::op_def_inner(interp, name, &tokens, Some(description.to_string()))
                .unwrap_or_else(|e| panic!("Failed to define {}: {}", name, e));
        }

        interp
            .rebuild_dependencies()
            .expect("Failed to rebuild dependencies");
    }

    #[tokio::test]
    async fn test_del_sample_user_words() {
        let mut interp = Interpreter::new();

        let sample_words = vec![
            ("C4", "264", "純正律 C4"),
            ("D4", "C4 9 * 8 /", "純正律 D4"),
            ("E4", "C4 5 * 4 /", "純正律 E4"),
        ];
        restore_sample_words(&mut interp, &sample_words);

        assert!(interp.user_words.contains_key("C4"));
        assert!(interp.user_words.contains_key("D4"));
        assert!(interp.user_words.contains_key("E4"));

        let result = interp.execute("'D4' DEL").await;
        assert!(result.is_ok(), "Should delete D4: {:?}", result.err());
        assert!(!interp.user_words.contains_key("D4"));

        let result = interp.execute("'C4' DEL").await;
        assert!(result.is_err(), "Should not delete C4 (has dependents)");

        let result = interp.execute("! 'C4' DEL").await;
        assert!(result.is_ok(), "Should force delete C4: {:?}", result.err());
        assert!(!interp.user_words.contains_key("C4"));
    }

    #[tokio::test]
    async fn test_del_sample_user_words_with_fqn() {

        let mut interp = Interpreter::new();

        let sample_words = vec![
            ("C4", "264", "純正律 C4"),
            ("D4", "C4 9 * 8 /", "純正律 D4"),
            ("E4", "C4 5 * 4 /", "純正律 E4"),
        ];
        restore_sample_words(&mut interp, &sample_words);

        assert!(interp.user_words.contains_key("D4"));


        let result = interp.execute("'DEMO@D4' DEL").await;
        assert!(result.is_ok(), "Should delete D4 via FQN: {:?}", result.err());
        assert!(!interp.user_words.contains_key("D4"));


        let result = interp.execute("'DEMO@NONEXISTENT' DEL").await;
        assert!(result.is_err(), "Should error for non-existent FQN word");


        let result = interp.execute("'DEMO@C4' DEL").await;
        assert!(result.is_err(), "Should not delete C4 via FQN (has dependents)");


        let result = interp.execute("! 'DEMO@C4' DEL").await;
        assert!(result.is_ok(), "Should force delete C4 via FQN: {:?}", result.err());
        assert!(!interp.user_words.contains_key("C4"));
    }

    #[tokio::test]
    async fn test_execute_restored_sample_words() {
        let mut interp = Interpreter::new();

        let sample_words = vec![
            ("C4", "264", "純正律 C4"),
            ("D4", "C4 9 * 8 /", "純正律 D4"),
        ];
        restore_sample_words(&mut interp, &sample_words);

        let result = interp.execute("C4").await;
        assert!(
            result.is_ok(),
            "Executing C4 should succeed: {:?}",
            result.err()
        );
        assert_eq!(interp.stack.len(), 1);

        let result = interp.execute("D4").await;
        assert!(
            result.is_ok(),
            "Executing D4 should succeed: {:?}",
            result.err()
        );
        assert_eq!(interp.stack.len(), 2);
    }

    #[tokio::test]
    async fn test_sample_words_in_vector_literal_play() {




        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();


        let result = interp.execute("[ C4 D4 E4 ] MUSIC@SEQ MUSIC@PLAY").await;
        assert!(
            result.is_ok(),
            "[ C4 D4 E4 ] MUSIC@SEQ MUSIC@PLAY should succeed: {:?}",
            result.err()
        );

        let output = interp.collect_output();

        assert!(
            output.contains("AUDIO:"),
            "Should contain AUDIO command, got: {}",
            output
        );
    }

    #[tokio::test]
    async fn test_sample_words_scalar_output() {

        let mut interp = Interpreter::new();

        let sample_words = vec![
            ("C4", "264", "純正律 C4"),
            ("D4", "C4 9 * 8 /", "純正律 D4"),
        ];
        restore_sample_words(&mut interp, &sample_words);
        let _ = interp.collect_output();

        let _ = interp.execute("C4").await.unwrap();
        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            assert!(
                val.as_scalar().is_some(),
                "C4 should push a scalar, not a vector"
            );
            assert_eq!(val.as_scalar().unwrap().to_i64().unwrap(), 264);
        }

        let _ = interp.execute("D4").await.unwrap();
        assert_eq!(interp.stack.len(), 2);
        if let Some(val) = interp.stack.last() {
            assert!(
                val.as_scalar().is_some(),
                "D4 should push a scalar, not a vector"
            );
            assert_eq!(val.as_scalar().unwrap().to_i64().unwrap(), 297);
        }
    }

    #[tokio::test]
    async fn test_builtin_symbols_remain_strings_in_vector() {


        let mut interp = Interpreter::new();



        let result = interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await;
        assert!(
            result.is_ok(),
            "Code block DEF should work: {:?}",
            result.err()
        );

        let result = interp.execute("[ 5 ] DOUBLE").await;
        assert!(
            result.is_ok(),
            "Executing DOUBLE should succeed: {:?}",
            result.err()
        );
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1);
                assert_eq!(children[0].as_scalar().unwrap().to_i64().unwrap(), 10);
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_user_word_resolved_in_nested_vector() {

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();




        let result = interp
            .execute("[ [ C4 E4 G4 ] ] MUSIC@SIM MUSIC@PLAY")
            .await;
        assert!(
            result.is_ok(),
            "Nested vector with user words should work: {:?}",
            result.err()
        );

        let output = interp.collect_output();
        assert!(output.contains("AUDIO:"), "Should contain AUDIO command");
    }

    #[tokio::test]
    async fn test_def_with_vector_duality() {
        let mut interp = Interpreter::new();


        let result = interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await;
        assert!(
            result.is_ok(),
            "DEF with vector should succeed: {:?}",
            result
        );

        let result = interp.execute("[ 5 ] DOUBLE").await;
        assert!(
            result.is_ok(),
            "Executing DOUBLE should succeed: {:?}",
            result
        );

        assert_eq!(interp.stack.len(), 1, "Stack should have 1 element");
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 10, "Result should be 10");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_def_with_module_collision_warns() {

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();

        let result = interp.execute("{ [ 999 ] } 'C4' DEF").await;
        assert!(result.is_ok(), "DEF should succeed even with module collision: {:?}", result.err());
        let output = interp.collect_output();
        assert!(output.contains("Warning"),
            "Should warn about the collision: {}", output);
        assert!(output.contains("MUSIC@C4"),
            "Warning should mention MUSIC@C4: {}", output);
    }

    #[tokio::test]
    async fn test_import_keeps_user_word_qualified() {

        let mut interp = Interpreter::new();


        interp.execute("{ [ 999 ] } 'C4' DEF").await.unwrap();
        assert!(interp.user_words.contains_key("C4"));


        interp.execute("'music' IMPORT").await.unwrap();
        let output = interp.collect_output();

        assert!(interp.user_words.contains_key("C4"),
            "User word C4 should remain in DEMO after IMPORT");
        assert!(output.contains("Warning"),
            "Should warn about the conflict: {}", output);


        let result = interp.execute("C4").await;
        assert!(result.is_err(), "C4 should be ambiguous");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Ambiguous"),
            "Expected ambiguity error, got: {}", err_msg);


        let result = interp.execute("DEMO@C4").await;
        assert!(result.is_ok(), "Qualified DEMO@C4 should work: {:?}", result.err());
        if let Some(val) = interp.stack.last() {
            let scalar = val
                .as_scalar()
                .or_else(|| {
                    val.as_vector()
                        .and_then(|children| children.first())
                        .and_then(|child| child.as_scalar())
                })
                .expect("DEMO@C4 should resolve to a numeric value");
            assert_eq!(scalar.to_i64().unwrap(), 999,
                "DEMO@C4 should remain the user-defined value");
        }


        let result = interp.execute("MUSIC@C4").await;
        assert!(result.is_ok(), "Qualified MUSIC@C4 should work: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_module_word_resolves_without_conflict() {
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();

        let result = interp.execute("C4").await;
        assert!(result.is_ok(), "C4 should work: {:?}", result.err());
        if let Some(val) = interp.stack.last() {
            assert_eq!(val.as_scalar().unwrap().to_i64().unwrap(), 264,
                "C4 should be 264 (module sample)");
        }
    }

    #[tokio::test]
    async fn test_module_first_builtin_still_protected() {

        let mut interp = Interpreter::new();
        let result = interp.execute("{ [ 1 ] } 'GET' DEF").await;
        assert!(result.is_err(), "Should not be able to override built-in GET");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Cannot redefine built-in word"),
            "Expected BuiltinProtection error, got: {}", err_msg);
    }

}
