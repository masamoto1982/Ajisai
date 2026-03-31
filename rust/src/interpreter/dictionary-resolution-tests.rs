#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

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
    async fn test_path_short_name_no_collision() {
        // Short name resolves when no collision exists
        let mut interp = Interpreter::new();
        let sample_words = vec![("SAY-HELLO-WORLD", "[ 42 ]", "test word")];
        restore_sample_words(&mut interp, &sample_words);
        let _ = interp.collect_output();

        let result = interp.execute("SAY-HELLO-WORLD").await;
        assert!(result.is_ok(), "Short name should resolve: {:?}", result.err());
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_path_dict_at_word() {
        // DEMO@WORD resolves custom word
        let mut interp = Interpreter::new();
        let sample_words = vec![("SAY-HELLO-WORLD", "[ 42 ]", "test word")];
        restore_sample_words(&mut interp, &sample_words);
        let _ = interp.collect_output();

        let result = interp.execute("DEMO@SAY-HELLO-WORLD").await;
        assert!(result.is_ok(), "DEMO@SAY-HELLO-WORLD should resolve: {:?}", result.err());
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_path_user_dict_word() {
        // USER@DEMO@WORD resolves custom word
        let mut interp = Interpreter::new();
        let sample_words = vec![("SAY-HELLO-WORLD", "[ 42 ]", "test word")];
        restore_sample_words(&mut interp, &sample_words);
        let _ = interp.collect_output();

        let result = interp.execute("USER@DEMO@SAY-HELLO-WORLD").await;
        assert!(result.is_ok(), "USER@DEMO@SAY-HELLO-WORLD should resolve: {:?}", result.err());
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_path_fully_qualified_user() {
        // DICT@USER@DEMO@WORD resolves custom word
        let mut interp = Interpreter::new();
        let sample_words = vec![("SAY-HELLO-WORLD", "[ 42 ]", "test word")];
        restore_sample_words(&mut interp, &sample_words);
        let _ = interp.collect_output();

        let result = interp.execute("DICT@USER@DEMO@SAY-HELLO-WORLD").await;
        assert!(result.is_ok(), "DICT@USER@DEMO@SAY-HELLO-WORLD should resolve: {:?}", result.err());
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_path_module_at_word() {
        // MUSIC@PLAY resolves module word
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();

        let result = interp.execute("MUSIC@C4").await;
        assert!(result.is_ok(), "MUSIC@C4 should resolve: {:?}", result.err());
        if let Some(val) = interp.stack.last() {
            assert_eq!(val.as_scalar().unwrap().to_i64().unwrap(), 264);
        }
    }

    #[tokio::test]
    async fn test_path_dict_module_word() {
        // DICT@MUSIC@C4 resolves module sample word
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();

        let result = interp.execute("DICT@MUSIC@C4").await;
        assert!(result.is_ok(), "DICT@MUSIC@C4 should resolve: {:?}", result.err());
        if let Some(val) = interp.stack.last() {
            assert_eq!(val.as_scalar().unwrap().to_i64().unwrap(), 264);
        }
    }

    #[tokio::test]
    async fn test_path_core_at_word() {
        // CORE@GET resolves built-in word
        let mut interp = Interpreter::new();
        interp.execute("[ 10 20 30 ]").await.unwrap();

        let result = interp.execute("[ 1 ] CORE@GET").await;
        assert!(result.is_ok(), "CORE@GET should resolve: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_path_dict_core_word() {
        // DICT@CORE@GET resolves built-in word
        let mut interp = Interpreter::new();
        interp.execute("[ 10 20 30 ]").await.unwrap();

        let result = interp.execute("[ 1 ] DICT@CORE@GET").await;
        assert!(result.is_ok(), "DICT@CORE@GET should resolve: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_path_case_insensitive() {
        // Case normalization: music@play → MUSIC@PLAY
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();

        let result = interp.execute("music@c4").await;
        assert!(result.is_ok(), "music@c4 should resolve (case insensitive): {:?}", result.err());
        if let Some(val) = interp.stack.last() {
            assert_eq!(val.as_scalar().unwrap().to_i64().unwrap(), 264);
        }
    }

    #[tokio::test]
    async fn test_path_case_insensitive_user() {
        // Case normalization for custom words
        let mut interp = Interpreter::new();
        let sample_words = vec![("SAY-HELLO-WORLD", "[ 42 ]", "test word")];
        restore_sample_words(&mut interp, &sample_words);
        let _ = interp.collect_output();

        let result = interp.execute("demo@say-hello-world").await;
        assert!(result.is_ok(), "demo@say-hello-world should resolve: {:?}", result.err());
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_ambiguous_word_error() {
        // Word existing in both module and custom should produce ambiguity error
        let mut interp = Interpreter::new();
        interp.execute("{ [ 999 ] } 'C4' DEF").await.unwrap();
        let _ = interp.collect_output();

        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();

        // C4 now exists in both MUSIC (sample) and DEMO (custom)
        let result = interp.execute("C4").await;
        assert!(result.is_err(), "C4 should be ambiguous");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Ambiguous"), "Expected ambiguity error, got: {}", err_msg);
        assert!(err_msg.contains("MUSIC@C4"), "Should mention MUSIC@C4: {}", err_msg);
        assert!(err_msg.contains("DEMO@C4"), "Should mention DEMO@C4: {}", err_msg);
    }

    #[tokio::test]
    async fn test_ambiguous_resolved_by_qualified_path() {
        // Ambiguous word resolved via qualified path
        let mut interp = Interpreter::new();
        interp.execute("{ [ 999 ] } 'C4' DEF").await.unwrap();
        let _ = interp.collect_output();

        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();

        // MUSIC@C4 should resolve to 264
        let result = interp.execute("MUSIC@C4").await;
        assert!(result.is_ok(), "MUSIC@C4 should resolve: {:?}", result.err());
        if let Some(val) = interp.stack.last() {
            assert_eq!(val.as_scalar().unwrap().to_i64().unwrap(), 264);
        }

        // DEMO@C4 should resolve to 999
        let result = interp.execute("DEMO@C4").await;
        assert!(result.is_ok(), "DEMO@C4 should resolve: {:?}", result.err());
        if let Some(val) = interp.stack.last() {
            let scalar = val
                .as_scalar()
                .or_else(|| {
                    val.as_vector()
                        .and_then(|children| children.first())
                        .and_then(|child| child.as_scalar())
                })
                .expect("DEMO@C4 should be numeric");
            assert_eq!(scalar.to_i64().unwrap(), 999);
        }
    }

    #[tokio::test]
    async fn test_builtin_not_ambiguous() {
        // Built-in words are never ambiguous, even if custom word with same name exists
        let mut interp = Interpreter::new();

        // GET is a built-in. Even if we somehow had a custom GET (blocked by protection),
        // the built-in always wins without ambiguity
        let result = interp.execute("[ 10 20 30 ] [ 0 ] GET").await;
        assert!(result.is_ok(), "Built-in GET should always work: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_module_builtin_word_via_qualified_path() {
        // Module built-in words (MUSIC@PLAY etc.) via qualified path
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();

        // MUSIC@SEQ is a module built-in (not a sample word)
        let result = interp.execute("[ 440 ] MUSIC@SEQ MUSIC@PLAY").await;
        assert!(result.is_ok(), "MUSIC@SEQ MUSIC@PLAY should work: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_split_path_unit() {
        use crate::interpreter::Interpreter;

        // Test the split_path function directly
        let (layers, word) = Interpreter::split_path("MUSIC@PLAY");
        assert_eq!(layers, vec!["MUSIC"]);
        assert_eq!(word, "PLAY");

        let (layers, word) = Interpreter::split_path("USER@DEMO@SAY-HELLO");
        assert_eq!(layers, vec!["USER", "DEMO"]);
        assert_eq!(word, "SAY-HELLO");

        let (layers, word) = Interpreter::split_path("DICT@USER@DEMO@SAY-HELLO");
        assert_eq!(layers, vec!["DICT", "USER", "DEMO"]);
        assert_eq!(word, "SAY-HELLO");

        let (layers, word) = Interpreter::split_path("SAY-HELLO");
        assert!(layers.is_empty());
        assert_eq!(word, "SAY-HELLO");

        // Case insensitive
        let (layers, word) = Interpreter::split_path("music@play");
        assert_eq!(layers, vec!["MUSIC"]);
        assert_eq!(word, "PLAY");
    }
}
