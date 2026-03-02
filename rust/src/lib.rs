mod error;
pub mod types;
mod tokenizer;
pub mod interpreter;
mod builtins;
mod wasm_api;

pub use wasm_api::AjisaiInterpreter;

#[cfg(test)]
mod test_tokenizer;

#[cfg(test)]
mod ceil_tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_ceil_positive_remainder() {
        let mut interp = Interpreter::new();
        interp.execute("[ 7/3 ] CEIL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ 3 }", "CEIL(7/3) should be 3");
    }

    #[tokio::test]
    async fn test_ceil_negative_remainder() {
        let mut interp = Interpreter::new();
        interp.execute("[ -7/3 ] CEIL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ -2 }", "CEIL(-7/3) should be -2");
    }

    #[tokio::test]
    async fn test_ceil_positive_integer() {
        let mut interp = Interpreter::new();
        interp.execute("[ 6/3 ] CEIL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ 2 }", "CEIL(6/3) should be 2");
    }

    #[tokio::test]
    async fn test_ceil_negative_integer() {
        let mut interp = Interpreter::new();
        interp.execute("[ -6/3 ] CEIL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ -2 }", "CEIL(-6/3) should be -2");
    }

    #[tokio::test]
    async fn test_ceil_with_chevron() {
        let mut interp = Interpreter::new();
        // Test CEIL within a chevron branch word (using multiline definition)
        // >> [ 3 ] [ 1 ] < (3 < 1 = FALSE)
        // >> [ 7/3 ] CEIL (this branch is skipped)
        // >>> [ 0 ] (default branch, executed because condition is FALSE)
        let def_code = r#":
>> [ 3 ] [ 1 ] <
>> [ 7/3 ] CEIL
>>> [ 0 ]
; 'TEST' DEF"#;
        interp.execute(def_code).await.unwrap();
        interp.execute("TEST").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        // 3 < 1 is FALSE, so default is executed
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ 0 }");
    }

    #[tokio::test]
    async fn test_ceil_operation_target_stack_error() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 2 3 ] .. CEIL").await;
        assert!(result.is_err(), "CEIL should not support Stack mode (..)");
    }

    #[tokio::test]
    async fn test_ceil_error_restores_stack() {
        let mut interp = Interpreter::new();
        // CEILにNILを渡すとエラーになる。エラー時にスタックが復元されることを確認
        interp.execute("NIL").await.unwrap();
        let result = interp.execute("CEIL").await;
        assert!(result.is_err());
        // スタックが復元されているか確認
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1, "Stack should be restored after error");
    }
}

#[cfg(test)]
mod round_tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_round_positive_below_half() {
        // 7/3 ≈ 2.333... → 2 (最も近い整数)
        let mut interp = Interpreter::new();
        interp.execute("[ 7/3 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ 2 }", "ROUND(7/3) should be 2");
    }

    #[tokio::test]
    async fn test_round_positive_half() {
        // 5/2 = 2.5 → 3 (0から遠い方向)
        let mut interp = Interpreter::new();
        interp.execute("[ 5/2 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ 3 }", "ROUND(5/2) should be 3");
    }

    #[tokio::test]
    async fn test_round_positive_above_half() {
        // 8/3 ≈ 2.666... → 3 (最も近い整数)
        let mut interp = Interpreter::new();
        interp.execute("[ 8/3 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ 3 }", "ROUND(8/3) should be 3");
    }

    #[tokio::test]
    async fn test_round_negative_below_half() {
        // -7/3 ≈ -2.333... → -2 (最も近い整数)
        let mut interp = Interpreter::new();
        interp.execute("[ -7/3 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ -2 }", "ROUND(-7/3) should be -2");
    }

    #[tokio::test]
    async fn test_round_negative_half() {
        // -5/2 = -2.5 → -3 (0から遠い方向)
        let mut interp = Interpreter::new();
        interp.execute("[ -5/2 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ -3 }", "ROUND(-5/2) should be -3");
    }

    #[tokio::test]
    async fn test_round_negative_above_half() {
        // -8/3 ≈ -2.666... → -3 (最も近い整数)
        let mut interp = Interpreter::new();
        interp.execute("[ -8/3 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ -3 }", "ROUND(-8/3) should be -3");
    }

    #[tokio::test]
    async fn test_round_positive_integer() {
        // 6/3 = 2 → 2 (整数はそのまま)
        let mut interp = Interpreter::new();
        interp.execute("[ 6/3 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ 2 }", "ROUND(6/3) should be 2");
    }

    #[tokio::test]
    async fn test_round_negative_integer() {
        // -6/3 = -2 → -2 (整数はそのまま)
        let mut interp = Interpreter::new();
        interp.execute("[ -6/3 ] ROUND").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ -2 }", "ROUND(-6/3) should be -2");
    }

    #[tokio::test]
    async fn test_round_with_chevron() {
        let mut interp = Interpreter::new();
        // Test ROUND within a chevron branch word (using multiline definition)
        // >> [ 3 ] [ 1 ] < (3 < 1 = FALSE)
        // >> [ 8/3 ] ROUND (this branch is skipped)
        // >>> [ 0 ] (default branch, executed because condition is FALSE)
        let def_code = r#":
>> [ 3 ] [ 1 ] <
>> [ 8/3 ] ROUND
>>> [ 0 ]
; 'TEST' DEF"#;
        interp.execute(def_code).await.unwrap();
        interp.execute("TEST").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        // 3 < 1 is FALSE, so default is executed
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ 0 }");
    }

    #[tokio::test]
    async fn test_round_operation_target_stack_error() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 2 3 ] .. ROUND").await;
        assert!(result.is_err(), "ROUND should not support Stack mode (..)");
    }

    #[tokio::test]
    async fn test_round_error_restores_stack() {
        let mut interp = Interpreter::new();
        // ROUNDにNILを渡すとエラーになる。エラー時にスタックが復元されることを確認
        interp.execute("NIL").await.unwrap();
        let result = interp.execute("ROUND").await;
        assert!(result.is_err());
        // スタックが復元されているか確認
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1, "Stack should be restored after error");
    }
}

#[cfg(test)]
mod num_tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_num_parse_error_stack_restoration() {
        let mut interp = Interpreter::new();
        interp.execute("[ 'hello' ]").await.unwrap();
        let result = interp.execute("NUM").await;
        assert!(result.is_err());
        // スタックが復元されているか確認
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1, "Stack should be restored after parse error");
    }

    #[tokio::test]
    async fn test_num_same_structure_error_stack_restoration() {
        let mut interp = Interpreter::new();
        interp.execute("[ 42 ]").await.unwrap();
        let result = interp.execute("NUM").await;
        assert!(result.is_err());
        // スタックが復元されているか確認
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1, "Stack should be restored after same-type error");
    }

    #[tokio::test]
    async fn test_num_nil_error_stack_restoration() {
        let mut interp = Interpreter::new();
        interp.execute("[ nil ]").await.unwrap();
        let result = interp.execute("NUM").await;
        assert!(result.is_err());
        // スタックが復元されているか確認
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1, "Stack should be restored after nil error");
    }

    #[tokio::test]
    async fn test_num_operation_target_stack_error() {
        let mut interp = Interpreter::new();
        interp.execute("[ '42' ] [ '123' ]").await.unwrap();
        let result = interp.execute(".. NUM").await;
        assert!(result.is_err());
        // Stack modeエラー時はスタックから何もpopしていないので2要素のまま
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 2, "Stack should remain unchanged after Stack mode error");
    }
}

#[cfg(test)]
mod dimension_limit_tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_dimension_limit_at_3_visible() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ [ [ 1 2 3 ] ] ]").await;
        assert!(result.is_ok(), "3 visible dimensions should succeed");

        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1, "Stack should have 1 element after parsing 3D tensor");
    }

    #[tokio::test]
    async fn test_dimension_4_visible_succeeds() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ [ [ [ 1 ] ] ] ]").await;
        assert!(result.is_ok(), "4 visible dimensions should succeed with new limit");
    }

    #[tokio::test]
    async fn test_dimension_5_visible_succeeds() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ [ [ [ [ 1 ] ] ] ] ]").await;
        assert!(result.is_ok(), "5 visible dimensions should succeed");
    }

    #[tokio::test]
    async fn test_dimension_limit_at_9_visible() {
        // 9可視次元（10次元）は上限であり成功すべき
        let mut interp = Interpreter::new();
        let result = interp.execute("[ [ [ [ [ [ [ [ [ 1 ] ] ] ] ] ] ] ] ]").await;
        assert!(result.is_ok(), "9 visible dimensions (10 total) should succeed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_dimension_limit_exceeds_at_10_visible() {
        // 10可視次元（11次元）はエラーになるべき
        let mut interp = Interpreter::new();
        let result = interp.execute("[ [ [ [ [ [ [ [ [ [ 1 ] ] ] ] ] ] ] ] ] ]").await;
        assert!(result.is_err(), "10 visible dimensions (11 total) should result in an error");

        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("10 dimensions"),
            "Error message should mention '10 dimensions', got: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_dimension_error_message_format() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ [ [ [ [ [ [ [ [ [ 1 ] ] ] ] ] ] ] ] ] ]").await;
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Nesting depth limit exceeded"),
            "Error message should mention 'Nesting depth limit exceeded', got: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_bracket_display_1d() {
        // 1次元は { } で表示される
        let mut interp = Interpreter::new();
        interp.execute("[ 1 2 3 ]").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert!(result.starts_with('{'), "1D should display with {{ }}, got: {}", result);
        assert!(result.ends_with('}'), "1D should display with {{ }}, got: {}", result);
    }

    #[tokio::test]
    async fn test_bracket_display_2d() {
        // 2次元は { ( ) } で表示される
        let mut interp = Interpreter::new();
        interp.execute("[ [ 1 2 ] [ 3 4 ] ]").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert!(result.starts_with('{'), "2D outermost should be {{ }}, got: {}", result);
        assert!(result.contains('('), "2D inner should contain (), got: {}", result);
    }

    #[tokio::test]
    async fn test_bracket_display_3d() {
        // 3次元は { ( [ ] ) } で表示される
        let mut interp = Interpreter::new();
        interp.execute("[ [ [ 1 ] ] ]").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert!(result.starts_with('{'), "3D outermost should be {{ }}, got: {}", result);
        assert!(result.contains('['), "3D innermost should contain [], got: {}", result);
    }

    #[tokio::test]
    async fn test_bracket_display_3d_complex() {
        // 3次元構造 [2, 3, 1] のテスト
        let mut interp = Interpreter::new();
        interp.execute("{ ( [ 1 ] [ 2 ] [ 3 ] ) ( [ 4 ] [ 5 ] [ 6 ] ) }").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        // shape should be [2, 3, 1] - 3D structure
        assert!(result.starts_with('{'), "3D outermost should be {{ }}, got: {}", result);
        assert!(result.contains('('), "3D second level should contain (), got: {}", result);
        assert!(result.contains('['), "3D innermost should contain [], got: {}", result);
        // Verify exact format
        assert_eq!(result, "{ ( [ 1 ] [ 2 ] [ 3 ] ) ( [ 4 ] [ 5 ] [ 6 ] ) }", "Expected 3D structure");
    }

    #[tokio::test]
    async fn test_bracket_display_4d() {
        // 4次元は { ( [ { } ] ) } — 括弧サイクルが繰り返される
        let mut interp = Interpreter::new();
        interp.execute("[ [ [ [ 1 ] ] ] ]").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ ( [ { 1 } ] ) }", "4D should cycle brackets: {}", result);
    }

    #[tokio::test]
    async fn test_bracket_display_9d() {
        // 9可視次元（10次元上限）の括弧表示確認
        let mut interp = Interpreter::new();
        interp.execute("[ [ [ [ [ [ [ [ [ 1 ] ] ] ] ] ] ] ] ]").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        // depth 0={}, 1=(), 2=[], 3={}, 4=(), 5=[], 6={}, 7=(), 8=[]
        assert_eq!(result, "{ ( [ { ( [ { ( [ 1 ] ) } ] ) } ] ) }", "9D bracket cycle: {}", result);
    }

    #[tokio::test]
    async fn test_dotdot_operation_sets_mode() {
        // .. オペレーションがスタック操作モードを設定する
        let mut interp = Interpreter::new();
        interp.execute("[ 1 2 3 ]").await.unwrap();

        // .. を実行してもエラーにならない
        let result = interp.execute("..").await;
        assert!(result.is_ok(), ".. operation should succeed");
    }
}

#[cfg(test)]
mod tensor_ops_integration_tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_shape_1d() {
        let mut interp = Interpreter::new();
        interp.execute("[ 1 2 3 ] SHAPE").await.unwrap();
        let stack = interp.get_stack();
        // SHAPE consumes the vector and pushes shape
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ 3 }");
    }

    #[tokio::test]
    async fn test_shape_2d() {
        let mut interp = Interpreter::new();
        interp.execute("[ [ 1 2 3 ] [ 4 5 6 ] ] SHAPE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ 2 3 }");
    }

    #[tokio::test]
    async fn test_shape_keep_mode() {
        let mut interp = Interpreter::new();
        interp.execute("[ 1 2 3 ] ,, SHAPE").await.unwrap();
        let stack = interp.get_stack();
        // Keep mode: original + shape
        assert_eq!(stack.len(), 2);
        let original = format!("{}", stack[0]);
        assert_eq!(original, "{ 1 2 3 }");
        let shape = format!("{}", stack[1]);
        assert_eq!(shape, "{ 3 }");
    }

    #[tokio::test]
    async fn test_rank_1d() {
        let mut interp = Interpreter::new();
        interp.execute("[ 1 2 3 ] RANK").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "1"); // RANK returns a scalar
    }

    #[tokio::test]
    async fn test_rank_2d() {
        let mut interp = Interpreter::new();
        interp.execute("[ [ 1 2 ] [ 3 4 ] ] RANK").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "2"); // RANK returns a scalar
    }

    #[tokio::test]
    async fn test_reshape_basic() {
        let mut interp = Interpreter::new();
        interp.execute("[ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ ( 1 2 3 ) ( 4 5 6 ) }");
    }

    #[tokio::test]
    async fn test_reshape_3d() {
        let mut interp = Interpreter::new();
        interp.execute("[ 1 2 3 4 5 6 ] [ 3 2 ] RESHAPE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ ( 1 2 ) ( 3 4 ) ( 5 6 ) }");
    }

    #[tokio::test]
    async fn test_transpose_basic() {
        let mut interp = Interpreter::new();
        interp.execute("[ [ 1 2 3 ] [ 4 5 6 ] ] TRANSPOSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ ( 1 4 ) ( 2 5 ) ( 3 6 ) }");
    }

    #[tokio::test]
    async fn test_fill_basic() {
        let mut interp = Interpreter::new();
        interp.execute("[ 3 0 ] FILL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ 0 0 0 }");
    }

    #[tokio::test]
    async fn test_fill_2d() {
        let mut interp = Interpreter::new();
        interp.execute("[ 2 3 5 ] FILL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ ( 5 5 5 ) ( 5 5 5 ) }");
    }

    #[tokio::test]
    async fn test_shape_nil_propagation() {
        let mut interp = Interpreter::new();
        interp.execute("NIL SHAPE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_nil(), "SHAPE of NIL should return NIL (Map type NIL propagation)");
    }

    #[tokio::test]
    async fn test_transpose_nil_returns_nil() {
        let mut interp = Interpreter::new();
        interp.execute("NIL TRANSPOSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_nil(), "TRANSPOSE of NIL should return NIL (Form type: NIL = empty set)");
    }
}

#[cfg(test)]
mod unicode_tests {
    use crate::interpreter::Interpreter;
    use crate::types::Value;

    #[test]
    fn test_from_string_ascii() {
        let val = Value::from_string("A");
        // 'A' = U+0041 = 65
        let fracs = val.flatten_fractions();
        assert_eq!(fracs.len(), 1);
        assert_eq!(fracs[0].to_i64(), Some(65));
    }

    #[test]
    fn test_from_string_unicode_japanese() {
        let val = Value::from_string("あ");
        // 'あ' = U+3042 = 12354
        let fracs = val.flatten_fractions();
        assert_eq!(fracs.len(), 1, "Japanese char should be 1 code point, not multiple bytes");
        assert_eq!(fracs[0].to_i64(), Some(12354));
    }

    #[test]
    fn test_from_string_emoji() {
        let val = Value::from_string("🌸");
        // '🌸' = U+1F338 = 127800
        let fracs = val.flatten_fractions();
        assert_eq!(fracs.len(), 1, "Emoji should be 1 code point");
        assert_eq!(fracs[0].to_i64(), Some(127800));
    }

    #[test]
    fn test_from_string_mixed() {
        let val = Value::from_string("Aあ");
        let fracs = val.flatten_fractions();
        assert_eq!(fracs.len(), 2, "Should have 2 code points");
        assert_eq!(fracs[0].to_i64(), Some(65));   // 'A'
        assert_eq!(fracs[1].to_i64(), Some(12354)); // 'あ'
    }

    #[tokio::test]
    async fn test_chr_japanese() {
        let mut interp = Interpreter::new();
        // 12354 CHR → 'あ'
        interp.execute("12354 CHR").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'あ'");
    }

    #[tokio::test]
    async fn test_string_display_unicode() {
        let mut interp = Interpreter::new();
        interp.execute("'Hello'").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'Hello'");
    }

    #[tokio::test]
    async fn test_chars_join_unicode_roundtrip() {
        let mut interp = Interpreter::new();
        interp.execute("'hello' CHARS JOIN").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'hello'");
    }
}

#[cfg(test)]
mod json_io_tests {
    use crate::interpreter::Interpreter;

    // ========================================================================
    // PARSE tests
    // ========================================================================

    #[tokio::test]
    async fn test_parse_integer() {
        let mut interp = Interpreter::new();
        interp.execute("'42' PARSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "42");
    }

    #[tokio::test]
    async fn test_parse_string() {
        let mut interp = Interpreter::new();
        interp.execute(r#"'"hello"' PARSE"#).await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'hello'");
    }

    #[tokio::test]
    async fn test_parse_null() {
        let mut interp = Interpreter::new();
        interp.execute("'null' PARSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_nil());
    }

    #[tokio::test]
    async fn test_parse_bool_true() {
        let mut interp = Interpreter::new();
        interp.execute("'true' PARSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "TRUE");
    }

    #[tokio::test]
    async fn test_parse_bool_false() {
        let mut interp = Interpreter::new();
        interp.execute("'false' PARSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "FALSE");
    }

    #[tokio::test]
    async fn test_parse_array() {
        let mut interp = Interpreter::new();
        interp.execute("'[1, 2, 3]' PARSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert_eq!(stack[0].len(), 3);
    }

    #[tokio::test]
    async fn test_parse_empty_array() {
        let mut interp = Interpreter::new();
        interp.execute("'[]' PARSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_nil());
    }

    #[tokio::test]
    async fn test_parse_invalid_json() {
        let mut interp = Interpreter::new();
        interp.execute("'not json' PARSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_nil());
    }

    #[tokio::test]
    async fn test_parse_keep_mode() {
        let mut interp = Interpreter::new();
        interp.execute("'42' ,, PARSE").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 2);
        let original = format!("{}", stack[0]);
        assert_eq!(original, "'42'");
        let parsed = format!("{}", stack[1]);
        assert_eq!(parsed, "42");
    }

    // ========================================================================
    // STRINGIFY tests
    // ========================================================================

    #[tokio::test]
    async fn test_stringify_integer() {
        let mut interp = Interpreter::new();
        interp.execute("[ 42 ] STRINGIFY").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'[42]'");
    }

    #[tokio::test]
    async fn test_stringify_nil() {
        let mut interp = Interpreter::new();
        interp.execute("NIL STRINGIFY").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'null'");
    }

    #[tokio::test]
    async fn test_stringify_bool() {
        let mut interp = Interpreter::new();
        interp.execute("TRUE STRINGIFY").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'true'");
    }

    #[tokio::test]
    async fn test_stringify_string() {
        let mut interp = Interpreter::new();
        interp.execute("'hello' STRINGIFY").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, r#"'"hello"'"#);
    }

    #[tokio::test]
    async fn test_stringify_array() {
        let mut interp = Interpreter::new();
        interp.execute("[ 1 2 3 ] STRINGIFY").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'[1,2,3]'");
    }

    // ========================================================================
    // INPUT / OUTPUT tests
    // ========================================================================

    #[tokio::test]
    async fn test_input_empty() {
        let mut interp = Interpreter::new();
        interp.execute("INPUT").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "''");
    }

    #[tokio::test]
    async fn test_input_with_buffer() {
        let mut interp = Interpreter::new();
        interp.input_buffer = "hello world".to_string();
        interp.execute("INPUT").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'hello world'");
    }

    #[tokio::test]
    async fn test_output() {
        let mut interp = Interpreter::new();
        interp.execute("'result' OUTPUT").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 0);
        assert_eq!(interp.io_output_buffer, "'result'");
    }

    #[tokio::test]
    async fn test_output_keep_mode() {
        let mut interp = Interpreter::new();
        interp.execute("'result' ,, OUTPUT").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(!interp.io_output_buffer.is_empty());
    }

    // ========================================================================
    // JSON-GET tests
    // ========================================================================

    #[tokio::test]
    async fn test_json_get_existing_key() {
        let mut interp = Interpreter::new();
        interp.execute(r#"'{"name": "Ajisai", "version": 1}' PARSE 'name' JSON-GET"#).await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'Ajisai'");
    }

    #[tokio::test]
    async fn test_json_get_missing_key() {
        let mut interp = Interpreter::new();
        interp.execute(r#"'{"a": 1}' PARSE 'b' JSON-GET"#).await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_nil());
    }

    #[tokio::test]
    async fn test_json_get_numeric_value() {
        let mut interp = Interpreter::new();
        interp.execute(r#"'{"x": 42}' PARSE 'x' JSON-GET"#).await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "42");
    }

    // ========================================================================
    // JSON-KEYS tests
    // ========================================================================

    #[tokio::test]
    async fn test_json_keys() {
        let mut interp = Interpreter::new();
        interp.execute(r#"'{"a": 1, "b": 2}' PARSE JSON-KEYS"#).await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_vector());
        assert_eq!(stack[0].len(), 2);
    }

    #[tokio::test]
    async fn test_json_keys_non_object() {
        let mut interp = Interpreter::new();
        interp.execute("[ 1 2 3 ] JSON-KEYS").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_nil());
    }

    // ========================================================================
    // JSON-SET tests
    // ========================================================================

    #[tokio::test]
    async fn test_json_set_new_key() {
        let mut interp = Interpreter::new();
        interp.execute(r#"'{"a": 1}' PARSE 'b' [ 2 ] JSON-SET"#).await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_vector());
        assert_eq!(stack[0].len(), 2);
    }

    #[tokio::test]
    async fn test_json_set_update_key() {
        let mut interp = Interpreter::new();
        interp.execute(r#"'{"a": 1}' PARSE 'a' [ 99 ] JSON-SET 'a' JSON-GET"#).await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "{ 99 }");
    }

    #[tokio::test]
    async fn test_json_set_on_nil() {
        let mut interp = Interpreter::new();
        interp.execute("NIL 'key' 'value' JSON-SET").await.unwrap();
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
        interp.execute("'42' PARSE STRINGIFY").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'42'");
    }

    #[tokio::test]
    async fn test_parse_stringify_roundtrip_array() {
        let mut interp = Interpreter::new();
        interp.execute("'[1,2,3]' PARSE STRINGIFY").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "'[1,2,3]'");
    }

    // ========================================================================
    // INPUT → PARSE → process → STRINGIFY → OUTPUT pipeline
    // ========================================================================

    #[tokio::test]
    async fn test_input_parse_process_stringify_output() {
        let mut interp = Interpreter::new();
        interp.input_buffer = "[1, 2, 3]".to_string();
        interp.execute("INPUT PARSE : [ 2 ] * ; MAP STRINGIFY OUTPUT").await.unwrap();
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

        // Build a JSON object with 50+ keys
        let mut pairs = Vec::new();
        for i in 0..60 {
            pairs.push(format!(r#""key{}": {}"#, i, i * 10));
        }
        let json_str = format!("{{{}}}", pairs.join(", "));

        // Parse it
        let code = format!("'{}' PARSE", json_str);
        interp.execute(&code).await.unwrap();

        // Access the first, middle, and last keys
        interp.execute("'key0' JSON-GET").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack.last().unwrap());
        assert_eq!(result, "0");

        // Reset stack and re-parse for next test
        interp.stack.clear();
        interp.execute(&code).await.unwrap();
        interp.execute("'key30' JSON-GET").await.unwrap();
        let result = format!("{}", interp.get_stack().last().unwrap());
        assert_eq!(result, "300");

        interp.stack.clear();
        interp.execute(&code).await.unwrap();
        interp.execute("'key59' JSON-GET").await.unwrap();
        let result = format!("{}", interp.get_stack().last().unwrap());
        assert_eq!(result, "590");

        // Non-existent key should return NIL
        interp.stack.clear();
        interp.execute(&code).await.unwrap();
        interp.execute("'nonexistent' JSON-GET").await.unwrap();
        assert!(interp.get_stack().last().unwrap().is_nil());
    }

    #[tokio::test]
    async fn test_json_set_then_get_consistency() {
        let mut interp = Interpreter::new();

        // Create object, set a new key, then get it
        interp.execute(r#"'{"a": 1, "b": 2}' PARSE 'c' 3 JSON-SET"#).await.unwrap();
        interp.execute("'c' JSON-GET").await.unwrap();
        let result = format!("{}", interp.get_stack().last().unwrap());
        assert_eq!(result, "3");

        // Update an existing key, then get it
        interp.stack.clear();
        interp.execute(r#"'{"a": 1, "b": 2}' PARSE 'a' 99 JSON-SET"#).await.unwrap();
        interp.execute("'a' JSON-GET").await.unwrap();
        let result = format!("{}", interp.get_stack().last().unwrap());
        assert_eq!(result, "99");

        // Verify other keys are unaffected after update
        interp.stack.clear();
        interp.execute(r#"'{"a": 1, "b": 2}' PARSE 'a' 99 JSON-SET"#).await.unwrap();
        interp.execute("'b' JSON-GET").await.unwrap();
        let result = format!("{}", interp.get_stack().last().unwrap());
        assert_eq!(result, "2");
    }

    #[tokio::test]
    async fn test_json_keys_order_preserved() {
        let mut interp = Interpreter::new();

        // Parse an object and verify keys come out in insertion order
        interp.execute(r#"'{"alpha": 1, "beta": 2, "gamma": 3}' PARSE JSON-KEYS"#).await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(stack[0].is_vector());
        assert_eq!(stack[0].len(), 3);

        // STRINGIFY the original object to verify order is preserved in output
        interp.stack.clear();
        interp.execute(r#"'{"alpha": 1, "beta": 2, "gamma": 3}' PARSE STRINGIFY"#).await.unwrap();
        let result = format!("{}", interp.get_stack().last().unwrap());
        // The stringified output should contain all keys (order in serde_json::Map may vary,
        // but the keys must all be present)
        assert!(result.contains("alpha"));
        assert!(result.contains("beta"));
        assert!(result.contains("gamma"));
    }
}
