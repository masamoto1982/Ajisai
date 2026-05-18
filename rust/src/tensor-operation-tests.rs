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
        assert_eq!(result, "[ 3/1 ]");
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
        assert_eq!(result, "[ 2/1 3/1 ]");
    }

    #[tokio::test]
    async fn test_shape_keep_mode() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 1 2 3 ] ,, SHAPE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 2);
        let original = format!("{}", stack[0]);
        assert_eq!(original, "[ 1/1 2/1 3/1 ]");
        let shape = format!("{}", stack[1]);
        assert_eq!(shape, "[ 3/1 ]");
    }

    #[tokio::test]
    async fn test_rank_1d() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 1 2 3 ] RANK").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "1/1");
    }

    #[tokio::test]
    async fn test_rank_2d() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ [ 1 2 ] [ 3 4 ] ] RANK").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "2/1");
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
        assert_eq!(result, "[ [ 1/1 2/1 3/1 ] [ 4/1 5/1 6/1 ] ]");
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
        assert_eq!(result, "[ [ 1/1 2/1 ] [ 3/1 4/1 ] [ 5/1 6/1 ] ]");
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
        assert_eq!(result, "[ [ 1/1 4/1 ] [ 2/1 5/1 ] [ 3/1 6/1 ] ]");
    }

    #[tokio::test]
    async fn test_fill_basic() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 3 0 ] FILL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ 0/1 0/1 0/1 ]");
    }

    #[tokio::test]
    async fn test_fill_2d() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
        interp.execute("[ 2 3 5 ] FILL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[ [ 5/1 5/1 5/1 ] [ 5/1 5/1 5/1 ] ]");
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
        assert_eq!(result, "[ [ 11/1 22/1 33/1 ] [ 14/1 25/1 36/1 ] ]");
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
        assert_eq!(result, "[ [ 101/1 102/1 103/1 ] [ 204/1 205/1 206/1 ] ]");
    }

    #[tokio::test]
    async fn test_percent_alias_matches_mod_basic() {
        let mut mod_interp = Interpreter::new();
        mod_interp
            .execute("'json' IMPORT 'io' IMPORT")
            .await
            .unwrap();
        mod_interp.execute("[ 7 ] [ 3 ] MOD").await.unwrap();
        let mod_result = format!("{}", mod_interp.get_stack().last().unwrap());

        let mut alias_interp = Interpreter::new();
        alias_interp
            .execute("'json' IMPORT 'io' IMPORT")
            .await
            .unwrap();
        alias_interp.execute("[ 7 ] [ 3 ] %").await.unwrap();
        let alias_result = format!("{}", alias_interp.get_stack().last().unwrap());

        assert_eq!(mod_result, "[ 1/1 ]");
        assert_eq!(alias_result, mod_result);
    }

    #[tokio::test]
    async fn test_percent_alias_matches_mod_broadcast() {
        let mut mod_interp = Interpreter::new();
        mod_interp
            .execute("'json' IMPORT 'io' IMPORT")
            .await
            .unwrap();
        mod_interp.execute("[ 1 2 3 4 5 ] [ 2 ] MOD").await.unwrap();
        let mod_result = format!("{}", mod_interp.get_stack().last().unwrap());

        let mut alias_interp = Interpreter::new();
        alias_interp
            .execute("'json' IMPORT 'io' IMPORT")
            .await
            .unwrap();
        alias_interp.execute("[ 1 2 3 4 5 ] [ 2 ] %").await.unwrap();
        let alias_result = format!("{}", alias_interp.get_stack().last().unwrap());

        assert_eq!(alias_result, mod_result);
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

#[cfg(test)]
mod sparse_tensor_fast_path_integration_tests {
    use crate::interpreter::Interpreter;

    fn sparse_fraction_vector_literal() -> String {
        let mut lanes = vec!["0".to_string(); 64];
        lanes[5] = "3/2".to_string();
        lanes[40] = "-5/3".to_string();
        format!("[ {} ]", lanes.join(" "))
    }

    #[tokio::test]
    async fn sparse_candidate_tensor_scalar_mul_matches_dense_semantics() {
        let mut interp = Interpreter::new();
        interp
            .execute(&format!("{} 2 *", sparse_fraction_vector_literal()))
            .await
            .unwrap();
        let result = interp.get_stack().last().unwrap();
        assert_eq!(format!("{}", result.child(5).unwrap()), "3/1");
        assert_eq!(format!("{}", result.child(40).unwrap()), "-10/3");
        for index in 0..64 {
            if index != 5 && index != 40 {
                assert_eq!(format!("{}", result.child(index).unwrap()), "0/1");
            }
        }
    }

    #[tokio::test]
    async fn sparse_candidate_same_shape_mul_matches_dense_semantics() {
        let mut interp = Interpreter::new();
        let rhs = "[ ".to_string() + &vec!["2"; 64].join(" ") + " ]";
        interp
            .execute(&format!("{} {} *", sparse_fraction_vector_literal(), rhs))
            .await
            .unwrap();
        let result = interp.get_stack().last().unwrap();
        assert_eq!(format!("{}", result.child(5).unwrap()), "3/1");
        assert_eq!(format!("{}", result.child(40).unwrap()), "-10/3");
        for index in 0..64 {
            if index != 5 && index != 40 {
                assert_eq!(format!("{}", result.child(index).unwrap()), "0/1");
            }
        }
    }
}
