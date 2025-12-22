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
        assert_eq!(result, "[3]", "CEIL(7/3) should be 3");
    }

    #[tokio::test]
    async fn test_ceil_negative_remainder() {
        let mut interp = Interpreter::new();
        interp.execute("[ -7/3 ] CEIL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[-2]", "CEIL(-7/3) should be -2");
    }

    #[tokio::test]
    async fn test_ceil_positive_integer() {
        let mut interp = Interpreter::new();
        interp.execute("[ 6/3 ] CEIL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[2]", "CEIL(6/3) should be 2");
    }

    #[tokio::test]
    async fn test_ceil_negative_integer() {
        let mut interp = Interpreter::new();
        interp.execute("[ -6/3 ] CEIL").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        let result = format!("{}", stack[0]);
        assert_eq!(result, "[-2]", "CEIL(-6/3) should be -2");
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
        assert_eq!(result, "[0]");
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
        // CEILに非数値を渡すとエラーになる。エラー時にスタックが復元されることを確認
        interp.execute("[ 'test' ]").await.unwrap();
        let result = interp.execute("CEIL").await;
        assert!(result.is_err());
        // スタックが復元されているか確認
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1, "Stack should be restored after error");
    }
}
