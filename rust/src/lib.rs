// rust/src/lib.rs

mod error;
mod types;
mod tokenizer;
mod interpreter;
mod builtins;
mod wasm_api;

// `pub use` に `#[wasm_bindgen]` は適用できないため削除。
// `AjisaiInterpreter` 構造体自体が `wasm_api.rs` の中で `#[wasm_bindgen]` されているため、
// この `use` を介して正しくエクスポートされます。
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
    async fn test_ceil_with_guard() {
        let mut interp = Interpreter::new();
        // Test CEIL within a guarded word (using multiline definition)
        // : [ 1 ] [ 3 ] > (1 > 3 = FALSE)
        // : [ 7/3 ] CEIL (this branch is skipped)
        // : [ 0 ] (default branch, executed because condition is FALSE)
        interp.execute("[ ': [ 1 ] [ 3 ] >\n: [ 7/3 ] CEIL\n: [ 0 ]' ] 'TEST' DEF").await.unwrap();
        interp.execute("TEST").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        // 1 > 3 is FALSE, so default is executed
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
    async fn test_round_with_guard() {
        let mut interp = Interpreter::new();
        // Test ROUND within a guarded word (using multiline definition)
        // : [ 1 ] [ 3 ] > (1 > 3 = FALSE)
        // : [ 8/3 ] ROUND (this branch is skipped)
        // : [ 0 ] (default branch, executed because condition is FALSE)
        interp.execute("[ ': [ 1 ] [ 3 ] >\n: [ 8/3 ] ROUND\n: [ 0 ]' ] 'TEST' DEF").await.unwrap();
        interp.execute("TEST").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        // 1 > 3 is FALSE, so default is executed
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
        // 3次元（可視限界）は成功すべき
        let mut interp = Interpreter::new();
        let result = interp.execute("[ [ [ 1 2 3 ] ] ]").await;
        assert!(result.is_ok(), "3 visible dimensions should succeed");

        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1, "Stack should have 1 element after parsing 3D tensor");
    }

    #[tokio::test]
    async fn test_dimension_limit_exceeds_at_4_visible() {
        // 4次元（可視）はエラーになるべき
        let mut interp = Interpreter::new();
        let result = interp.execute("[ [ [ [ 1 ] ] ] ]").await;
        assert!(result.is_err(), "4 visible dimensions should result in an error");

        // エラーメッセージに "3 visible dimensions" が含まれることを確認
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("3 visible dimensions"),
            "Error message should mention '3 visible dimensions', got: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_dimension_error_message_mentions_dimension_0() {
        // エラーメッセージに "dimension 0: the stack" が含まれることを確認
        let mut interp = Interpreter::new();
        let result = interp.execute("[ [ [ [ 1 ] ] ] ]").await;
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("dimension 0: the stack"),
            "Error message should mention 'dimension 0: the stack', got: {}",
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
    async fn test_dotdot_operation_sets_mode() {
        // .. オペレーションがスタック操作モードを設定する
        let mut interp = Interpreter::new();
        interp.execute("[ 1 2 3 ]").await.unwrap();

        // .. を実行してもエラーにならない
        let result = interp.execute("..").await;
        assert!(result.is_ok(), ".. operation should succeed");
    }
}
