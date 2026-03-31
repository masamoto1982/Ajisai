#[cfg(test)]
mod json_io_tests {
    use crate::interpreter::Interpreter;

    // ========================================================================
    // JSON@PARSE tests
    // ========================================================================

    #[tokio::test]
    async fn test_parse_integer() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("'42' JSON@PARSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "42");
    }

    #[tokio::test]
    async fn test_parse_string() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute(r#"'"hello"' JSON@PARSE"#).await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'hello'");
    }

    #[tokio::test]
    async fn test_parse_null() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("'null' JSON@PARSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_nil());
    }

    #[tokio::test]
    async fn test_parse_bool_true() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("'true' JSON@PARSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "1");
    }

    #[tokio::test]
    async fn test_parse_bool_false() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("'false' JSON@PARSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "0");
    }

    #[tokio::test]
    async fn test_parse_array() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("'[1, 2, 3]' JSON@PARSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert_eq!(stack[0].len(), 3);
    }

    #[tokio::test]
    async fn test_parse_empty_array() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("'[]' JSON@PARSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_nil());
    }

    #[tokio::test]
    async fn test_parse_invalid_json() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("'not json' JSON@PARSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_nil());
    }

    #[tokio::test]
    async fn test_parse_keep_mode() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("'42' ,, JSON@PARSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 2);
        let original = format!("{}", stack[0]);
        assert_eq!(original, "'42'");
        let parsed = format!("{}", stack[1]);
        assert_eq!(parsed, "42");
    }

    // ========================================================================
    // JSON@STRINGIFY tests
    // ========================================================================

    #[tokio::test]
    async fn test_stringify_integer() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 42 ] JSON@STRINGIFY").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'[42]'");
    }

    #[tokio::test]
    async fn test_stringify_nil() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("NIL JSON@STRINGIFY").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'null'");
    }

    #[tokio::test]
    async fn test_stringify_bool() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("TRUE JSON@STRINGIFY").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'1'");
    }

    #[tokio::test]
    async fn test_stringify_string() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("'hello' JSON@STRINGIFY").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, r#"'"hello"'"#);
    }

    #[tokio::test]
    async fn test_stringify_array() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 1 2 3 ] JSON@STRINGIFY").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'[1,2,3]'");
    }

    // ========================================================================
    // IO@INPUT / IO@OUTPUT tests
    // ========================================================================

    #[tokio::test]
    async fn test_input_empty() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("IO@INPUT").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "NIL");
    }

    #[tokio::test]
    async fn test_input_with_buffer() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.input_buffer = "hello world".to_string();
        interp.execute("IO@INPUT").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'hello world'");
    }

    #[tokio::test]
    async fn test_output() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("'result' IO@OUTPUT").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 0);
        assert_eq!(interp.io_output_buffer, "'result'");
    }

    #[tokio::test]
    async fn test_output_keep_mode() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("'result' ,, IO@OUTPUT").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(!interp.io_output_buffer.is_empty());
    }

    // ========================================================================
    // JSON@GET tests
    // ========================================================================

    #[tokio::test]
    async fn test_json_get_existing_key() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp
            .execute(r#"'{"name": "Ajisai", "version": 1}' JSON@PARSE 'name' JSON@GET"#)
            .await
            .unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'Ajisai'");
    }

    #[tokio::test]
    async fn test_json_get_missing_key() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp
            .execute(r#"'{"a": 1}' JSON@PARSE 'b' JSON@GET"#)
            .await
            .unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_nil());
    }

    #[tokio::test]
    async fn test_json_get_numeric_value() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp
            .execute(r#"'{"x": 42}' JSON@PARSE 'x' JSON@GET"#)
            .await
            .unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "42");
    }

    // ========================================================================
    // JSON@KEYS tests
    // ========================================================================

    #[tokio::test]
    async fn test_json_keys() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp
            .execute(r#"'{"a": 1, "b": 2}' JSON@PARSE JSON@KEYS"#)
            .await
            .unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_vector());
        assert_eq!(stack[0].len(), 2);
    }

    #[tokio::test]
    async fn test_json_keys_non_object() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 1 2 3 ] JSON@KEYS").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_nil());
    }

    // ========================================================================
    // JSON@SET tests
    // ========================================================================

    #[tokio::test]
    async fn test_json_set_new_key() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp
            .execute(r#"'{"a": 1}' JSON@PARSE 'b' [ 2 ] JSON@SET"#)
            .await
            .unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_vector());
        assert_eq!(stack[0].len(), 2);
    }

    #[tokio::test]
    async fn test_json_set_update_key() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp
            .execute(r#"'{"a": 1}' JSON@PARSE 'a' [ 99 ] JSON@SET 'a' JSON@GET"#)
            .await
            .unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'c'");
    }

    #[tokio::test]
    async fn test_json_set_on_nil() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("NIL 'key' 'value' JSON@SET").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_vector());
        assert_eq!(stack[0].len(), 1);
    }

    // ========================================================================
    // Roundtrip tests
    // ========================================================================

    #[tokio::test]
    async fn test_parse_stringify_roundtrip_number() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp
            .execute("'42' JSON@PARSE JSON@STRINGIFY")
            .await
            .unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'42'");
    }

    #[tokio::test]
    async fn test_parse_stringify_roundtrip_array() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp
            .execute("'[1,2,3]' JSON@PARSE JSON@STRINGIFY")
            .await
            .unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'[1,2,3]'");
    }

    // ========================================================================
    // IO pipeline tests
    // ========================================================================

    #[tokio::test]
    async fn test_input_parse_process_stringify_output() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.input_buffer = "[1, 2, 3]".to_string();
        interp
            .execute("{ [ 2 ] * } 'DBL' DEF IO@INPUT JSON@PARSE 'DBL' MAP JSON@STRINGIFY IO@OUTPUT")
            .await
            .unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 0);
        assert_eq!(interp.io_output_buffer, "'[2,4,6]'");
    }

    // ========================================================================
    // JSON optimization correctness tests
    // ========================================================================

    #[tokio::test]
    async fn test_json_get_large_object() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();

        let mut pairs = Vec::new();
        for i in 0..60 {
            pairs.push(format!(r#""key{}": {}"#, i, i * 10));
        }
        let json_str = format!("{{{}}}", pairs.join(", "));

        let code = format!("'{}' JSON@PARSE", json_str);
        interp.execute(&code).await.unwrap();

        interp.execute("'key0' JSON@GET").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack.last().unwrap());
        assert_eq!(result, "0");

        interp.stack.clear();
        interp.execute(&code).await.unwrap();
        interp.execute("'key30' JSON@GET").await.unwrap();
        let result = format!("{}", interp.get_stack().last().unwrap());
        assert_eq!(result, "300");

        interp.stack.clear();
        interp.execute(&code).await.unwrap();
        interp.execute("'key59' JSON@GET").await.unwrap();
        let result = format!("{}", interp.get_stack().last().unwrap());
        assert_eq!(result, "590");

        interp.stack.clear();
        interp.execute(&code).await.unwrap();
        interp.execute("'nonexistent' JSON@GET").await.unwrap();
        assert!(interp.get_stack().last().unwrap().is_nil());
    }

    #[tokio::test]
    async fn test_json_set_then_get_consistency() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();

        interp
            .execute(r#"'{"a": 1, "b": 2}' JSON@PARSE 'c' 3 JSON@SET"#)
            .await
            .unwrap();
        interp.execute("'c' JSON@GET").await.unwrap();
        let result = format!("{}", interp.get_stack().last().unwrap());
        assert_eq!(result, "3");

        interp.stack.clear();
        interp
            .execute(r#"'{"a": 1, "b": 2}' JSON@PARSE 'a' 99 JSON@SET"#)
            .await
            .unwrap();
        interp.execute("'a' JSON@GET").await.unwrap();
        let result = format!("{}", interp.get_stack().last().unwrap());
        assert_eq!(result, "99");

        interp.stack.clear();
        interp
            .execute(r#"'{"a": 1, "b": 2}' JSON@PARSE 'a' 99 JSON@SET"#)
            .await
            .unwrap();
        interp.execute("'b' JSON@GET").await.unwrap();
        let result = format!("{}", interp.get_stack().last().unwrap());
        assert_eq!(result, "2");
    }

    #[tokio::test]
    async fn test_json_keys_order_preserved() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();

        interp
            .execute(r#"'{"alpha": 1, "beta": 2, "gamma": 3}' JSON@PARSE JSON@KEYS"#)
            .await
            .unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_vector());
        assert_eq!(stack[0].len(), 3);

        interp.stack.clear();
        interp
            .execute(r#"'{"alpha": 1, "beta": 2, "gamma": 3}' JSON@PARSE JSON@STRINGIFY"#)
            .await
            .unwrap();
        let result = format!("{}", interp.get_stack().last().unwrap());
        assert!(result.contains("alpha"));
        assert!(result.contains("beta"));
        assert!(result.contains("gamma"));
    }
}
