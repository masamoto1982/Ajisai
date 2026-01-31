// rust/src/interpreter/control.rs
//
// 統一分数アーキテクチャ版の制御フロー操作
// コードブロック (: ... ;) 対応
//
// 【責務】
// TIMES、WAIT などの制御フロー操作を実装する。
// カスタムワードの繰り返し実行や遅延実行をサポートする。

use crate::interpreter::Interpreter;
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::get_integer_from_value;
use crate::types::{Value, ValueData, DisplayHint};

/// 値を文字列として解釈する（内部ヘルパー）
fn value_as_string(val: &Value) -> Option<String> {
    fn collect_chars(val: &Value) -> Vec<char> {
        match &val.data {
            ValueData::Nil => vec![],
            ValueData::Scalar(f) => {
                f.to_i64().and_then(|n| {
                    if n >= 0 && n <= 0x10FFFF {
                        char::from_u32(n as u32)
                    } else {
                        None
                    }
                }).map(|c| vec![c]).unwrap_or_default()
            }
            ValueData::Vector(children) => {
                children.iter().flat_map(|c| collect_chars(c)).collect()
            }
            ValueData::CodeBlock(_) => vec![],
        }
    }

    let chars = collect_chars(val);
    if chars.is_empty() {
        None
    } else {
        Some(chars.into_iter().collect())
    }
}

/// 値が文字列として扱えるかチェック
fn is_string_value(val: &Value) -> bool {
    val.display_hint == DisplayHint::String && !val.is_nil()
}

/// TIMES - コードブロックまたはワード名をN回繰り返し実行する
///
/// 【責務】
/// - 指定されたコードブロック（: ... ;）またはカスタムワード名を指定回数繰り返し実行
/// - ビルトインワードには使用不可
///
/// 【使用法】
/// - `: [ 1 ] + ; [5] TIMES` → コードブロックを5回実行（新構文）
/// - `'MYWORD' [5] TIMES` → カスタムワードを5回実行（ワード名）
///
/// 【引数スタック】
/// - [count]: 実行回数（単一要素ベクタの整数）
/// - : code ; または 'word_name': コードブロックまたはワード名
///
/// 【戻り値スタック】
/// - なし（実行結果がスタックに残る）
///
/// 【エラー】
/// - ビルトインワードを指定した場合
/// - カウントが整数でない場合
pub(crate) fn execute_times(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::from(
            "TIMES requires code and count. Usage: : code ; [ n ] TIMES"
        ));
    }

    let count_val = interp.stack.pop().unwrap();
    let code_val = interp.stack.pop().unwrap();

    let count = get_integer_from_value(&count_val)?;

    // TIMES内のループでは「変化なしエラー」チェックを無効化
    let saved_no_change_check = interp.disable_no_change_check;
    interp.disable_no_change_check = true;

    // コードブロックまたは文字列（ワード名）を取得
    let result = if let Some(tokens) = code_val.as_code_block() {
        // コードブロックの場合
        let tokens = tokens.clone();
        (|| {
            for _ in 0..count {
                let (_, _) = interp.execute_section_core(&tokens, 0)?;
            }
            Ok(())
        })()
    } else if is_string_value(&code_val) {
        // 文字列の場合はワード名として扱う
        let word_name = value_as_string(&code_val)
            .ok_or_else(|| AjisaiError::structure_error("code block (: ... ;) or word name", "other format"))?;
        let upper_word_name = word_name.to_uppercase();

        // ワード名として辞書を検索
        if let Some(def) = interp.dictionary.get(&upper_word_name) {
            if def.is_builtin {
                interp.disable_no_change_check = saved_no_change_check;
                return Err(AjisaiError::from("TIMES can only be used with custom words, not builtin words"));
            }

            // カスタムワードを繰り返し実行
            (|| {
                for _ in 0..count {
                    interp.execute_word_core(&upper_word_name)?;
                }
                Ok(())
            })()
        } else {
            // 辞書にない場合はエラー
            interp.disable_no_change_check = saved_no_change_check;
            return Err(AjisaiError::UnknownWord(upper_word_name));
        }
    } else {
        interp.disable_no_change_check = saved_no_change_check;
        interp.stack.push(code_val);
        interp.stack.push(count_val);
        return Err(AjisaiError::from(
            "TIMES requires a code block (: ... ;) or word name. Usage: : code ; [ n ] TIMES"
        ));
    };

    // フラグを復元
    interp.disable_no_change_check = saved_no_change_check;

    result
}

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueData;

    #[tokio::test]
    async fn test_times_basic() {
        let mut interp = Interpreter::new();

        // Define INC word: adds 1 to the top of stack
        // [ [ 1 ] + ] means: push 1, then add
        interp.execute("[ [ 1 ] + ] 'INC' DEF").await.unwrap();

        // Start with 0, call INC 5 times -> should be 5
        let result = interp.execute("[ 0 ] 'INC' [ 5 ] TIMES").await;

        assert!(result.is_ok(), "TIMES should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        // Check the value is [ 5 ] (Vector containing scalar 5)
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 5, "Result should be 5");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_times_zero_count() {
        let mut interp = Interpreter::new();

        // Define a word
        interp.execute("[ [ 1 ] + ] 'INC' DEF").await.unwrap();

        // Start with 10, call INC 0 times -> should still be 10
        let result = interp.execute("[ 10 ] 'INC' [ 0 ] TIMES").await;

        assert!(result.is_ok(), "TIMES with 0 count should succeed");
        assert_eq!(interp.stack.len(), 1);

        // Check the value is [ 10 ] (Vector containing scalar 10)
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
        let result = interp.execute("[ 0 ] 'PRINT' [ 3 ] TIMES").await;

        assert!(result.is_err(), "TIMES with builtin word should fail");
    }

    #[tokio::test]
    async fn test_times_with_multiline_word() {
        let mut interp = Interpreter::new();

        // Define a word that adds 2 (adds 1 twice)
        let def = r#"[ [ 1 ] + [ 1 ] + ] 'ADD_TWO' DEF"#;
        let def_result = interp.execute(def).await;
        assert!(def_result.is_ok(), "DEF should succeed: {:?}", def_result);

        // Start with 0, call 2 times -> 0 +2 +2 = 4
        let result = interp.execute("[ 0 ] 'ADD_TWO' [ 2 ] TIMES").await;

        assert!(result.is_ok(), "TIMES with multiline word should succeed: {:?}", result);

        // Check the value is [ 4 ] (Vector containing scalar 4)
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 4, "Result should be 4");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_times_accumulate() {
        let mut interp = Interpreter::new();

        // Define a word that adds 10: [ 10 ] +
        interp.execute("[ [ 10 ] + ] 'ADD10' DEF").await.unwrap();

        // Start with 5, add 10 three times: 5 -> 15 -> 25 -> 35
        let result = interp.execute("[ 5 ] 'ADD10' [ 3 ] TIMES").await;

        assert!(result.is_ok(), "TIMES with ADD10 should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        // Check the value is [ 35 ] (Vector containing scalar 35)
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 35, "Result should be 35");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_times_with_code_block() {
        let mut interp = Interpreter::new();

        // Execute code block (new syntax with : ... ;)
        // [ 0 ] : [ 1 ] + ; [ 5 ] TIMES -> [ 5 ]
        let result = interp.execute("[ 0 ] : [ 1 ] + ; [ 5 ] TIMES").await;

        assert!(result.is_ok(), "TIMES with code block should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        if let Some(val) = interp.stack.last() {
            let debug_str = format!("{:?}", val);
            assert!(debug_str.contains("5"), "Result should be 5, got: {}", debug_str);
        }
    }

    #[tokio::test]
    async fn test_times_with_code_block_complex() {
        let mut interp = Interpreter::new();

        // More complex code block: [ 2 ] * (doubling)
        // [ 1 ] : [ 2 ] * ; [ 4 ] TIMES -> [ 16 ] (1 * 2 * 2 * 2 * 2 = 16)
        let result = interp.execute("[ 1 ] : [ 2 ] * ; [ 4 ] TIMES").await;

        assert!(result.is_ok(), "TIMES with code block multiplication should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");

        if let Some(val) = interp.stack.last() {
            let debug_str = format!("{:?}", val);
            assert!(debug_str.contains("16"), "Result should be 16, got: {}", debug_str);
        }
    }

    #[tokio::test]
    async fn test_times_in_custom_word_with_word_name() {
        let mut interp = Interpreter::new();

        // Define INC first
        interp.execute("[ [ 1 ] + ] 'INC' DEF").await.unwrap();

        // Use TIMES with word name (no nested quotes needed)
        let result = interp.execute("[ 0 ] 'INC' [ 5 ] TIMES").await;

        assert!(result.is_ok(), "Should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);

        // Check the value is [ 5 ] (Vector containing scalar 5)
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 5, "Result should be 5");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    // === Code Block - コードブロック構文を使用するTIMESのテスト ===

    #[tokio::test]
    async fn test_code_block_push() {
        let mut interp = Interpreter::new();

        // コードブロックがスタックに正しくプッシュされることを確認
        let result = interp.execute("[ 0 ] : [ 1 ] + ;").await;

        assert!(result.is_ok(), "Code block should parse successfully");
        assert_eq!(interp.stack.len(), 2, "Should have 2 items on stack: [0] and code block");
        assert!(interp.stack[1].as_code_block().is_some(), "Second item should be a code block");
    }

    #[tokio::test]
    async fn test_times_with_code_block_increment() {
        let mut interp = Interpreter::new();

        // コードブロック構文を使ったTIMES
        // : [ 1 ] + ; means: push 1, then add
        let result = interp.execute("[ 0 ] : [ 1 ] + ; [ 5 ] TIMES").await;

        assert!(result.is_ok(), "TIMES with code block should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);

        // Check the value is [ 5 ] (Vector containing scalar 5)
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 5, "Result should be 5");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_times_with_code_block_doubling() {
        let mut interp = Interpreter::new();

        // より複雑なコードブロック: [ 2 ] * (2倍)
        // [ 1 ] : [ 2 ] * ; [ 4 ] TIMES -> [ 16 ] (1 * 2 * 2 * 2 * 2 = 16)
        let result = interp.execute("[ 1 ] : [ 2 ] * ; [ 4 ] TIMES").await;

        assert!(result.is_ok(), "TIMES with code block multiplication should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);

        // Check the value is [ 16 ] (Vector containing scalar 16)
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 16, "Result should be 16");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }
}
