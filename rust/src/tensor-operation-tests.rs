#[cfg(test)]
mod tensor_ops_integration_tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_shape_1d() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 1 2 3 ] SHAPE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ 3 ]");
    }

    #[tokio::test]
    async fn test_shape_2d() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp
            .execute("[ [ 1 2 3 ] [ 4 5 6 ] ] SHAPE")
            .await
            .unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ 2 3 ]");
    }

    #[tokio::test]
    async fn test_shape_keep_mode() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 1 2 3 ] ,, SHAPE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 2);
        let original = format!("{}", stack[0]);
        assert_eq!(original, "[ 1 2 3 ]");
        let shape = format!("{}", stack[1]);
        assert_eq!(shape, "[ 3 ]");
    }

    #[tokio::test]
    async fn test_rank_1d() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 1 2 3 ] RANK").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "1");
    }

    #[tokio::test]
    async fn test_rank_2d() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ [ 1 2 ] [ 3 4 ] ] RANK").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "2");
    }

    #[tokio::test]
    async fn test_reshape_basic() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp
            .execute("[ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE")
            .await
            .unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ [ 1 2 3 ] [ 4 5 6 ] ]");
    }

    #[tokio::test]
    async fn test_reshape_3d() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp
            .execute("[ 1 2 3 4 5 6 ] [ 3 2 ] RESHAPE")
            .await
            .unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ [ 1 2 ] [ 3 4 ] [ 5 6 ] ]");
    }

    #[tokio::test]
    async fn test_transpose_basic() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp
            .execute("[ [ 1 2 3 ] [ 4 5 6 ] ] TRANSPOSE")
            .await
            .unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ [ 1 4 ] [ 2 5 ] [ 3 6 ] ]");
    }

    #[tokio::test]
    async fn test_fill_basic() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 3 0 ] FILL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ 0 0 0 ]");
    }

    #[tokio::test]
    async fn test_fill_2d() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 2 3 5 ] FILL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ [ 5 5 5 ] [ 5 5 5 ] ]");
    }

    #[tokio::test]
    async fn test_add_broadcast_row_vector_to_matrix() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp
            .execute("[ [ 1 2 3 ] [ 4 5 6 ] ] [ 10 20 30 ] +")
            .await
            .unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ [ 11 22 33 ] [ 14 25 36 ] ]");
    }

    #[tokio::test]
    async fn test_add_broadcast_column_vector_to_matrix() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp
            .execute("[ [ 1 2 3 ] [ 4 5 6 ] ] [ [ 100 ] [ 200 ] ] +")
            .await
            .unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ [ 101 102 103 ] [ 204 205 206 ] ]");
    }

    #[tokio::test]
    async fn test_shape_nil_propagation() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("NIL SHAPE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(
            stack[0].is_nil(),
            "SHAPE of NIL should return NIL (Map type NIL propagation)"
        );
    }

    #[tokio::test]
    async fn test_transpose_nil_returns_nil() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("NIL TRANSPOSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(
            stack[0].is_nil(),
            "TRANSPOSE of NIL should return NIL (Form type: NIL = empty set)"
        );
    }
}

#[cfg(test)]
mod unicode_tests {
    use crate::interpreter::Interpreter;
    use crate::types::Value;

    #[test]
    fn test_from_string_ascii() {
        let val = Value::from_string("A");
        let fracs = val.collect_fractions_flat();
        assert_eq!(fracs.len(), 1);
        assert_eq!(fracs[0].to_i64(), Some(65));
    }

    #[test]
    fn test_from_string_unicode_japanese() {
        let val = Value::from_string("あ");
        let fracs = val.collect_fractions_flat();
        assert_eq!(
            fracs.len(),
            1,
            "Japanese char should be 1 code point, not multiple bytes"
        );
        assert_eq!(fracs[0].to_i64(), Some(12354));
    }

    #[test]
    fn test_from_string_emoji() {
        let val = Value::from_string("🌸");
        let fracs = val.collect_fractions_flat();
        assert_eq!(fracs.len(), 1, "Emoji should be 1 code point");
        assert_eq!(fracs[0].to_i64(), Some(127800));
    }

    #[test]
    fn test_from_string_mixed() {
        let val = Value::from_string("Aあ");
        let fracs = val.collect_fractions_flat();
        assert_eq!(fracs.len(), 2, "Should have 2 code points");
        assert_eq!(fracs[0].to_i64(), Some(65));
        assert_eq!(fracs[1].to_i64(), Some(12354));
    }

    #[tokio::test]
    async fn test_chr_japanese() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("12354 CHR").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'あ'");
    }

    #[tokio::test]
    async fn test_string_display_unicode() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("'Hello'").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'Hello'");
    }

    #[tokio::test]
    async fn test_chars_join_unicode_roundtrip() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("'hello' CHARS JOIN").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'hello'");
    }
}
