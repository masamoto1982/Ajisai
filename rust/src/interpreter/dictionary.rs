// rust/src/interpreter/dictionary.rs
//
// 統一分数アーキテクチャ版の辞書操作
//
// すべての値は Vec<Fraction> として表現される。
// 文字列は分数のベクタ（各要素がコードポイント）として格納される。

use crate::interpreter::{Interpreter, WordDefinition, OperationTarget};
use crate::interpreter::helpers::get_word_name_from_value;
use crate::error::{AjisaiError, Result};
use crate::types::{Token, ExecutionLine, DisplayHint};
use std::collections::HashSet;

/// 値を文字列として解釈する
///
/// 統一分数アーキテクチャ: すべての値は分数のベクタなので、
/// 各分数をコードポイントとして文字列に変換する。
fn value_to_string(val: &crate::types::Value) -> Result<String> {
    if val.data.is_empty() {
        return Err(AjisaiError::from("Cannot convert NIL to string"));
    }

    let chars: String = val.data.iter()
        .filter_map(|f| {
            f.to_i64().and_then(|n| {
                if n >= 0 && n <= 0x10FFFF {
                    char::from_u32(n as u32)
                } else {
                    None
                }
            })
        })
        .collect();

    Ok(chars)
}

/// 値が文字列として解釈可能かチェック
///
/// DisplayHint が String の場合、または有効なコードポイントの範囲内にある場合
fn is_string_like(val: &crate::types::Value) -> bool {
    if val.data.is_empty() {
        return false;
    }

    // DisplayHint が String の場合は確実に文字列
    if val.display_hint == DisplayHint::String {
        return true;
    }

    // すべての要素が有効なコードポイント範囲にあるかチェック
    val.data.iter().all(|f| {
        f.to_i64().map(|n| n >= 0 && n <= 0x10FFFF).unwrap_or(false)
    })
}

pub fn op_def(interp: &mut Interpreter) -> Result<()> {
    // DEFはStackモードをサポートしない（辞書操作ワード）
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("DEF does not support Stack mode (..)"));
    }

    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    // 説明（オプション）を先にチェック
    // 説明ありの場合: [ベクタ] ['NAME'] ['説明']
    // 説明なしの場合: [ベクタ] ['NAME']
    let mut description = None;

    let has_description = if interp.stack.len() >= 3 {
        // トップ2つが文字列的な値の場合のみ、説明ありと判定
        if let Some(top_val) = interp.stack.last() {
            if is_string_like(top_val) {
                // 次（2番目）も文字列的かチェック
                if let Some(second_val) = interp.stack.get(interp.stack.len() - 2) {
                    is_string_like(second_val)
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    if has_description {
        if let Some(desc_val) = interp.stack.pop() {
            // 文字列を取得（統一分数アーキテクチャ: 直接変換）
            if let Ok(s) = value_to_string(&desc_val) {
                description = Some(s);
            }
        }
    }

    // スタックから名前を取得
    let name_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let name_str = get_word_name_from_value(&name_val)?;

    // 定義本体を取得
    let def_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 定義本体を文字列として取得（統一分数アーキテクチャ）
    let definition_str = value_to_string(&def_val)?;

    // 定義本体をトークン化
    let custom_word_names: HashSet<String> = interp.dictionary.iter()
        .filter(|(_, def)| !def.is_builtin)
        .map(|(name, _)| name.clone())
        .collect();
    
    let tokens = crate::tokenizer::tokenize_with_custom_words(&definition_str, &custom_word_names)
        .map_err(|e| AjisaiError::from(format!("Tokenization error in DEF: {}", e)))?;

    // 内部定義関数を呼び出し
    op_def_inner(interp, &name_str, &tokens, description)
}

pub(crate) fn op_def_inner(interp: &mut Interpreter, name: &str, tokens: &[Token], description: Option<String>) -> Result<()> {
    let upper_name = name.to_uppercase();

    // 組み込みワードは再定義不可（! があっても不可）
    if let Some(existing) = interp.dictionary.get(&upper_name) {
        if existing.is_builtin {
            interp.force_flag = false;
            return Err(AjisaiError::from(format!(
                "Cannot redefine built-in word: {}", upper_name
            )));
        }

        // カスタムワードの再定義: 依存関係チェック
        let dependents = interp.get_dependents(&upper_name);

        if !dependents.is_empty() && !interp.force_flag {
            let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
            interp.force_flag = false;
            return Err(AjisaiError::from(format!(
                "Cannot redefine '{}': referenced by {}. Use ! [ ... ] '{}' DEF to force.",
                upper_name, dep_list, upper_name
            )));
        }

        // 警告メッセージを準備（依存関係があった場合）
        if !dependents.is_empty() {
            let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
            interp.output_buffer.push_str(&format!(
                "Warning: '{}' was redefined. Affected words: {}\n",
                upper_name, dep_list
            ));
        }

        // 既存のカスタムワードの依存関係をクリーンアップ
        for dep_name in &existing.dependencies {
            if let Some(dependents) = interp.dependents.get_mut(dep_name) {
                dependents.remove(&upper_name);
            }
        }
    }

    let lines = parse_definition_body(tokens, &interp.dictionary)?;
    
    let mut new_dependencies = HashSet::new();
    for line in &lines {
        for token in line.body_tokens.iter() {
            if let Token::Symbol(s) = token {
                let upper_s = s.to_uppercase();
                if interp.dictionary.contains_key(&upper_s) && !interp.dictionary.get(&upper_s).unwrap().is_builtin {
                    new_dependencies.insert(upper_s);
                }
            }
        }
    }
    
    for dep_name in &new_dependencies {
        interp.dependents.entry(dep_name.clone()).or_default().insert(upper_name.clone());
    }
    
    let new_def = WordDefinition {
        lines,
        is_builtin: false,
        description,
        dependencies: new_dependencies,
        original_source: None,
    };

    interp.dictionary.insert(upper_name.clone(), new_def);
    interp.output_buffer.push_str(&format!("Defined word: {}\n", name));
    interp.force_flag = false;  // フラグをリセット
    Ok(())
}

fn parse_definition_body(tokens: &[Token], dictionary: &std::collections::HashMap<String, WordDefinition>) -> Result<Vec<ExecutionLine>> {
    let mut lines = Vec::new();
    let mut processed_tokens = Vec::new();
    
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::String(s) if s.starts_with('\'') && s.ends_with('\'') => {
                // シングルクォート文字列をクォーテーションとして扱う
                let inner = &s[1..s.len()-1];
                // カスタムワード名のセットを作成
                let custom_word_names: HashSet<String> = dictionary.iter()
                    .filter(|(_, def)| !def.is_builtin)
                    .map(|(name, _)| name.clone())
                    .collect();
                    
                // 内部をトークン化
                let inner_tokens = crate::tokenizer::tokenize_with_custom_words(inner, &custom_word_names)
                    .map_err(|e| AjisaiError::from(format!("Error tokenizing quotation: {}", e)))?;
                processed_tokens.push(Token::VectorStart);
                processed_tokens.extend(inner_tokens);
                processed_tokens.push(Token::VectorEnd);
            },
            Token::LineBreak => {
                if !processed_tokens.is_empty() {
                    let execution_line = ExecutionLine {
                        body_tokens: processed_tokens.clone(),
                    };
                    lines.push(execution_line);
                    processed_tokens.clear();
                }
            },
            _ => {
                processed_tokens.push(tokens[i].clone());
            }
        }
        i += 1;
    }
    
    if !processed_tokens.is_empty() {
        let execution_line = ExecutionLine {
            body_tokens: processed_tokens,
        };
        lines.push(execution_line);
    }
    
    if lines.is_empty() {
        return Err(AjisaiError::from("Word definition cannot be empty"));
    }
    
    Ok(lines)
}

pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    // DELはStackモードをサポートしない（辞書操作ワード）
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("DEL does not support Stack mode (..)"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 統一分数アーキテクチャ: 値を文字列として解釈
    let name = get_word_name_from_value(&val)?;

    let upper_name = name.to_uppercase();

    // 組み込みワードは削除不可（! があっても不可）
    if let Some(def) = interp.dictionary.get(&upper_name) {
        if def.is_builtin {
            interp.force_flag = false;  // フラグをリセット
            return Err(AjisaiError::from(format!(
                "Cannot delete built-in word: {}", upper_name
            )));
        }
    } else {
        interp.force_flag = false;  // フラグをリセット
        return Err(AjisaiError::from(format!(
            "Word '{}' is not defined", upper_name
        )));
    }

    // 依存関係のチェック
    let dependents = interp.get_dependents(&upper_name);

    if !dependents.is_empty() && !interp.force_flag {
        // 依存関係があり、強制フラグがない場合はエラー
        let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
        return Err(AjisaiError::from(format!(
            "Cannot delete '{}': referenced by {}. Use ! '{}' DEL to force.",
            upper_name, dep_list, upper_name
        )));
    }

    // 削除実行
    if let Some(removed_def) = interp.dictionary.remove(&upper_name) {
        for dep_name in &removed_def.dependencies {
            if let Some(deps) = interp.dependents.get_mut(dep_name) {
                deps.remove(&upper_name);
            }
        }
        interp.dependents.remove(&upper_name);

        // 他のワードの依存関係リストからも削除
        for deps in interp.dependents.values_mut() {
            deps.remove(&upper_name);
        }

        // 警告メッセージ（依存関係があった場合）
        if !dependents.is_empty() {
            let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
            interp.output_buffer.push_str(&format!(
                "Warning: '{}' was deleted. Affected words: {}\n",
                upper_name, dep_list
            ));
        }

        interp.output_buffer.push_str(&format!("Deleted word: {}\n", name));
    }

    interp.force_flag = false;  // フラグをリセット
    Ok(())
}

pub fn op_lookup(interp: &mut Interpreter) -> Result<()> {
    // ?はStackモードをサポートしない（辞書操作ワード）
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("? (LOOKUP) does not support Stack mode (..)"));
    }

    // LOOKUP (?) は 'NAME' を期待する
    let name_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 統一分数アーキテクチャ: 値を文字列として解釈
    let name_str = get_word_name_from_value(&name_val)?;

    let upper_name = name_str.to_uppercase();
    
    if let Some(def) = interp.dictionary.get(&upper_name) {
        if def.is_builtin {
            let detailed_info = crate::builtins::get_builtin_detail(&upper_name);
            interp.definition_to_load = Some(detailed_info);
            return Ok(());
        }
        
        if let Some(original_source) = &def.original_source {
            interp.definition_to_load = Some(original_source.clone());
        } else {
            let definition = interp.get_word_definition_tokens(&upper_name).unwrap_or_default();
            let full_definition = if definition.is_empty() {
                format!("[ '' ] '{}' DEF", name_str)
            } else {
                if let Some(desc) = &def.description {
                    format!("[ '{}' ] '{}' '{}' DEF", definition, name_str, desc)
                } else {
                    format!("[ '{}' ] '{}' DEF", definition, name_str)
                }
            };
            interp.definition_to_load = Some(full_definition);
        }
        Ok(())
    } else {
        Err(AjisaiError::UnknownWord(name_str))
    }
}

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_cannot_override_builtin_word() {
        let mut interp = Interpreter::new();
        // 組み込みワードGETを上書きしようとする
        let result = interp.execute("[ '[ 1 ] +' ] 'GET' DEF").await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Cannot redefine built-in word"),
                "Expected error message to contain 'Cannot redefine built-in word', got: {}", err_msg);
    }

    #[tokio::test]
    #[ignore] // TODO: Fix for unified fraction architecture
    async fn test_can_override_custom_word() {
        let mut interp = Interpreter::new();
        // カスタムワードは上書き可能
        let result1 = interp.execute("[ '[ 2 ] *' ] 'DOUBLE' DEF").await;
        assert!(result1.is_ok(), "First definition should succeed");

        let result2 = interp.execute("[ '[ 3 ] *' ] 'DOUBLE' DEF").await;
        assert!(result2.is_ok(), "Overriding custom word should succeed");

        let result3 = interp.execute("[ 5 ] DOUBLE").await;
        assert!(result3.is_ok(), "Executing redefined word should succeed");

        // スタックトップが [ 15 ] であることを確認（Vector）
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
        if let Some(val) = interp.stack.last() {
            assert_eq!(val.data.len(), 1, "Result should have one element");
            // 15 は分数として 15/1 で表現される
            assert_eq!(val.data[0].numerator, num_bigint::BigInt::from(15), "Expected 15, got {}", val.data[0].numerator);
            assert_eq!(val.data[0].denominator, num_bigint::BigInt::from(1), "Expected denominator 1");
        }
    }

    #[tokio::test]
    async fn test_cannot_override_other_builtin_words() {
        let mut interp = Interpreter::new();

        // 複数の組み込みワードを上書きしようとする
        let builtin_words = vec!["INSERT", "REPLACE", "MAP", "FILTER", "PRINT"];

        for word in builtin_words {
            let code = format!("[ '[ 1 ] +' ] '{}' DEF", word);
            let result = interp.execute(&code).await;
            assert!(result.is_err(), "Should not be able to override builtin word: {}", word);
            let err_msg = result.unwrap_err().to_string();
            assert!(err_msg.contains("Cannot redefine built-in word"),
                    "Expected error for {}, got: {}", word, err_msg);
        }
    }

    #[tokio::test]
    async fn test_def_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        // Stackモード（..）でDEFを呼び出した場合はエラー
        let result = interp.execute("[ '[ 2 ] *' ] 'DOUBLE' .. DEF").await;
        assert!(result.is_err(), "DEF should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("DEF") && err_msg.contains("Stack mode"),
                "Expected Stack mode error for DEF, got: {}", err_msg);
    }

    #[tokio::test]
    async fn test_del_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        // まず定義
        interp.execute("[ '[ 2 ] *' ] 'DOUBLE' DEF").await.unwrap();

        // Stackモード（..）でDELを呼び出した場合はエラー
        let result = interp.execute("'DOUBLE' .. DEL").await;
        assert!(result.is_err(), "DEL should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("DEL") && err_msg.contains("Stack mode"),
                "Expected Stack mode error for DEL, got: {}", err_msg);
    }

    #[tokio::test]
    async fn test_lookup_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        // まず定義
        interp.execute("[ '[ 2 ] *' ] 'DOUBLE' DEF").await.unwrap();

        // Stackモード（..）で?を呼び出した場合はエラー
        let result = interp.execute("'DOUBLE' .. ?").await;
        assert!(result.is_err(), "? (LOOKUP) should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("?") && err_msg.contains("Stack mode"),
                "Expected Stack mode error for ?, got: {}", err_msg);
    }
}
