// rust/src/interpreter/control.rs
//
// 【責務】
// 制御フロー操作（TIMES、WAIT）を実装する。
// カスタムワードの繰り返し実行や遅延実行をサポートする。

use crate::interpreter::{Interpreter};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{get_integer_from_value, get_word_name_from_value};

/// TIMES - ワードをN回繰り返し実行する
///
/// 【責務】
/// - 指定されたカスタムワードを指定回数繰り返し実行
/// - ビルトインワードには使用不可（カスタムワードのみ）
///
/// 【使用法】
/// - `'MYWORD' [5] TIMES` → MYWORDを5回実行
///
/// 【引数スタック】
/// - [count]: 実行回数（単一要素ベクタの整数）
/// - ['name']: ワード名（単一要素ベクタの文字列）
///
/// 【戻り値スタック】
/// - なし（ワードの実行結果がスタックに残る）
///
/// 【エラー】
/// - ワードが存在しない場合
/// - ビルトインワードを指定した場合
/// - カウントが整数でない場合
pub(crate) fn execute_times(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::from("TIMES requires word name and count. Usage: 'WORD' [ n ] TIMES"));
    }

    let count_val = interp.stack.pop().unwrap();
    let name_val = interp.stack.pop().unwrap();

    let count = get_integer_from_value(&count_val)?;
    let word_name = get_word_name_from_value(&name_val)?;

    if let Some(def) = interp.dictionary.get(&word_name) {
        if def.is_builtin {
            return Err(AjisaiError::from("TIMES can only be used with custom words"));
        }
    } else {
        return Err(AjisaiError::UnknownWord(word_name));
    }

    // TIMES内のループでは「変化なしエラー」チェックを無効化
    // （FOLDなどの高階関数と同様に繰り返し演算を行うため）
    let saved_no_change_check = interp.disable_no_change_check;
    interp.disable_no_change_check = true;

    let result = (|| {
        for _ in 0..count {
            interp.execute_word_core(&word_name)?;
        }
        Ok(())
    })();

    // フラグを復元
    interp.disable_no_change_check = saved_no_change_check;

    result
}

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_times_basic() {
        let mut interp = Interpreter::new();

        // Define INC word: adds 1 to the top of stack
        interp.execute("[ ': [ 1 ] +' ] 'INC' DEF").await.unwrap();

        // Start with 0, call INC 5 times -> should be 5
        let result = interp.execute("[ 0 ] 'INC' [ 5 ] TIMES").await;

        assert!(result.is_ok(), "TIMES should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        // Check the value is 5
        if let Some(val) = interp.stack.last() {
            let debug_str = format!("{:?}", val);
            assert!(debug_str.contains("5"), "Result should be 5");
        }
    }

    #[tokio::test]
    async fn test_times_zero_count() {
        let mut interp = Interpreter::new();

        // Define a word
        interp.execute("[ ': [ 1 ] +' ] 'INC' DEF").await.unwrap();

        // Start with 10, call INC 0 times -> should still be 10
        let result = interp.execute("[ 10 ] 'INC' [ 0 ] TIMES").await;

        assert!(result.is_ok(), "TIMES with 0 count should succeed");
        assert_eq!(interp.stack.len(), 1);

        if let Some(val) = interp.stack.last() {
            let debug_str = format!("{:?}", val);
            assert!(debug_str.contains("10"), "Result should be 10");
        }
    }

    #[tokio::test]
    async fn test_times_unknown_word_error() {
        let mut interp = Interpreter::new();

        // Try TIMES with undefined word
        let result = interp.execute("[ 0 ] 'UNDEFINED' [ 3 ] TIMES").await;

        assert!(result.is_err(), "TIMES with undefined word should fail");
    }

    #[tokio::test]
    async fn test_times_builtin_word_error() {
        let mut interp = Interpreter::new();

        // Try TIMES with builtin word (should fail)
        let result = interp.execute("[ 0 ] 'DUP' [ 3 ] TIMES").await;

        assert!(result.is_err(), "TIMES with builtin word should fail");
    }

    #[tokio::test]
    async fn test_times_with_multiline_word() {
        let mut interp = Interpreter::new();

        // Define a word with multiple lines (simpler than guard clauses)
        // This tests that TIMES correctly calls words with multiple execution lines
        let def = r#"[ ':
[ 1 ] +
[ 1 ] +' ] 'ADD_TWO' DEF"#;
        let def_result = interp.execute(def).await;
        assert!(def_result.is_ok(), "DEF should succeed: {:?}", def_result);

        // Start with 0, call 2 times -> 0 +2 +2 = 4
        let result = interp.execute("[ 0 ] 'ADD_TWO' [ 2 ] TIMES").await;

        assert!(result.is_ok(), "TIMES with multiline word should succeed: {:?}", result);

        if let Some(val) = interp.stack.last() {
            let debug_str = format!("{:?}", val);
            assert!(debug_str.contains("4"), "Result should be 4, got: {}", debug_str);
        }
    }

    #[tokio::test]
    async fn test_times_with_operation_target() {
        let mut interp = Interpreter::new();

        // Define a word that uses .. (operation target) to sum multiple elements
        // .. [ 2 ] + means: take 2 elements from stack and add them
        interp.execute("[ ': .. [ 2 ] +' ] 'SUM2' DEF").await.unwrap();

        // Push 3 elements, then sum them pairwise twice
        // [1] [2] [3] -> SUM2 -> [1] [5] -> SUM2 -> [6]
        let result = interp.execute("[ 1 ] [ 2 ] [ 3 ] 'SUM2' [ 2 ] TIMES").await;

        assert!(result.is_ok(), "TIMES with operation target should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        if let Some(val) = interp.stack.last() {
            let debug_str = format!("{:?}", val);
            assert!(debug_str.contains("6"), "Result should be 6, got: {}", debug_str);
        }
    }
}
