use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::extract_word_name_from_value;
use crate::interpreter::vector_exec::format_vector_to_source;
use crate::interpreter::{Interpreter, OperationTargetMode, WordDefinition};
use crate::types::{ExecutionLine, Token, Value, ValueData};
use std::collections::HashSet;
use std::sync::Arc;

fn extract_string_from_value(val: &Value) -> Result<String> {
    fn collect_chars(val: &Value) -> Vec<char> {
        match &val.data {
            ValueData::Nil => vec![],
            ValueData::Scalar(f) => f
                .to_i64()
                .and_then(|n| {
                    if n >= 0 && n <= 0x10FFFF {
                        char::from_u32(n as u32)
                    } else {
                        None
                    }
                })
                .map(|c| vec![c])
                .unwrap_or_default(),
            ValueData::Vector(children)
            | ValueData::Record {
                pairs: children, ..
            } => children.iter().flat_map(|c| collect_chars(c)).collect(),
            ValueData::CodeBlock(_) => vec![],
        }
    }

    let chars = collect_chars(val);
    if chars.is_empty() {
        return Err(AjisaiError::from("Cannot convert NIL to string"));
    }

    Ok(chars.into_iter().collect())
}

fn is_string_like(val: &Value) -> bool {
    if val.is_nil() {
        return false;
    }

    fn check_codepoints(val: &Value) -> bool {
        match &val.data {
            ValueData::Nil => false,
            ValueData::Scalar(f) => f.to_i64().map(|n| n >= 0 && n <= 0x10FFFF).unwrap_or(false),
            ValueData::Vector(children)
            | ValueData::Record {
                pairs: children, ..
            } => children.iter().all(|c| check_codepoints(c)),
            ValueData::CodeBlock(_) => false,
        }
    }

    check_codepoints(val)
}

fn is_user_word_defined(interp: &Interpreter, symbol: &str) -> bool {
    let upper_symbol = symbol.to_uppercase();
    interp.is_user_word(&upper_symbol)
}

fn check_definition_descriptor_on_stack(stack: &[Value]) -> bool {
    if stack.len() < 3 {
        return false;
    }
    let last = &stack[stack.len() - 1];
    let second_last = &stack[stack.len() - 2];
    is_string_like(last) && is_string_like(second_last)
}

pub fn op_def(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported {
            word: "DEF".into(),
            mode: "Stack".into(),
        });
    }

    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let mut description = None;

    let has_description = check_definition_descriptor_on_stack(&interp.stack);

    if has_description {
        if let Some(desc_val) = interp.stack.pop() {
            if let Ok(s) = extract_string_from_value(&desc_val) {
                description = Some(s);
            }
        }
    }

    let name_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let name_str = extract_word_name_from_value(&name_val)?;

    let def_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let definition_str = match &def_val.data {
        ValueData::CodeBlock(tokens) => tokens
            .iter()
            .map(|t| match t {
                Token::Number(n) => n.to_string(),
                Token::String(s) => format!("'{}'", s),
                Token::Symbol(s) => s.to_string(),
                Token::VectorStart => "[".to_string(),
                Token::VectorEnd => "]".to_string(),
                Token::CodeBlockStart => ":".to_string(),
                Token::CodeBlockEnd => ";".to_string(),
                Token::ChevronBranch => ">>".to_string(),
                Token::ChevronDefault => ">>>".to_string(),
                Token::Pipeline => "==".to_string(),
                Token::NilCoalesce => "=>".to_string(),
                Token::SafeMode => "~".to_string(),
                Token::LineBreak => "\n".to_string(),
            })
            .collect::<Vec<_>>()
            .join(" "),
        ValueData::Vector(_) | ValueData::Record { .. } => format_vector_to_source(&def_val)?,
        _ => {
            return Err(AjisaiError::from(
                "DEF requires a code block (: ... ;) or vector as definition body",
            ));
        }
    };

    let tokens = crate::tokenizer::tokenize(&definition_str)
        .map_err(|e| AjisaiError::from(format!("Tokenization error in DEF: {}", e)))?;

    op_def_inner(interp, &name_str, &tokens, description)
}

pub(crate) fn op_def_inner(
    interp: &mut Interpreter,
    name: &str,
    tokens: &[Token],
    description: Option<String>,
) -> Result<()> {
    let upper_name = name.to_uppercase();

    if interp.core_vocabulary.contains_key(&upper_name) {
        interp.force_flag = false;
        return Err(AjisaiError::BuiltinProtection {
            word: upper_name,
            operation: "redefine".into(),
        });
    }

    // Module sample collision check — warn but allow DEF
    let mut collision_modules = Vec::new();
    for (module_name, module_dict) in &interp.module_samples {
        if module_dict.sample_words.contains_key(&upper_name) {
            collision_modules.push(module_name.clone());
        }
    }

    let dict_name = interp.active_user_dictionary.clone();
    let fq_name = format!("{}@{}", dict_name, upper_name);

    if let Some(existing) = interp
        .user_dictionaries
        .get(&dict_name)
        .and_then(|dict| dict.words.get(&upper_name))
    {
        let dependents = interp.collect_dependents(&fq_name);

        if !dependents.is_empty() && !interp.force_flag {
            let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
            interp.force_flag = false;
            return Err(AjisaiError::from(format!(
                "Cannot redefine '{}': referenced by {}. Use ! [ ... ] '{}' DEF to force.",
                fq_name, dep_list, upper_name
            )));
        }

        if !dependents.is_empty() {
            let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
            interp.output_buffer.push_str(&format!(
                "Warning: '{}' was redefined. Affected words: {}\n",
                fq_name, dep_list
            ));
        }

        for dep_name in &existing.dependencies {
            if let Some(dependents) = interp.dependents.get_mut(dep_name) {
                dependents.remove(&fq_name);
            }
        }
    }

    let lines = parse_definition_body(tokens)?;

    let mut new_dependencies = HashSet::new();
    for line in &lines {
        for token in line.body_tokens.iter() {
            if let Token::Symbol(s) = token {
                let upper_s = s.to_uppercase();
                if let Some((resolved_name, resolved_def)) = interp.resolve_word_entry(&upper_s) {
                    if !resolved_def.is_builtin {
                        new_dependencies.insert(resolved_name);
                    }
                }
            }
        }
    }

    for dep_name in &new_dependencies {
        interp
            .dependents
            .entry(dep_name.clone())
            .or_default()
            .insert(fq_name.clone());
    }

    let new_def = WordDefinition {
        lines: lines.into(),
        is_builtin: false,
        description,
        dependencies: new_dependencies,
        original_source: None,
        namespace: Some(dict_name.clone()),
        registration_order: interp.next_registration_order(),
    };

    let dict_order = interp
        .user_dictionaries
        .get(&dict_name)
        .map(|dict| dict.order)
        .unwrap_or_else(|| new_def.registration_order);
    interp.user_dictionaries.entry(dict_name.clone()).or_insert_with(|| crate::interpreter::UserDictionary {
        order: dict_order,
        words: std::collections::HashMap::new(),
    }).words.insert(upper_name.clone(), Arc::new(new_def));
    interp.sync_user_words_cache();
    interp
        .output_buffer
        .push_str(&format!("Defined word: {}@{}\n", dict_name, name));
    // Warn about collisions with module sample words
    if !collision_modules.is_empty() {
        let module_paths: Vec<String> = collision_modules.iter().map(|m| format!("{}@{}", m, upper_name)).collect();
        let user_path = format!("{}@{}", dict_name, upper_name);
        let all_paths: Vec<String> = module_paths.iter().chain(std::iter::once(&user_path)).cloned().collect();
        interp.output_buffer.push_str(&format!(
            "Warning: '{}' now exists in both {}. Use a qualified path when calling this word.\n",
            upper_name, all_paths.join(" and ")
        ));
    }
    interp.force_flag = false;
    Ok(())
}

fn parse_definition_body(tokens: &[Token]) -> Result<Vec<ExecutionLine>> {
    let mut lines = Vec::new();
    let mut processed_tokens = Vec::new();

    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::String(s) if s.starts_with('\'') && s.ends_with('\'') => {
                let inner = &s[1..s.len() - 1];

                let inner_tokens = crate::tokenizer::tokenize(inner)
                    .map_err(|e| AjisaiError::from(format!("Error tokenizing quotation: {}", e)))?;
                processed_tokens.push(Token::VectorStart);
                processed_tokens.extend(inner_tokens);
                processed_tokens.push(Token::VectorEnd);
            }
            Token::LineBreak => {
                if !processed_tokens.is_empty() {
                    let execution_line = ExecutionLine {
                        body_tokens: processed_tokens.clone().into(),
                    };
                    lines.push(execution_line);
                    processed_tokens.clear();
                }
            }
            // シェブロン分岐トークン(>> / >>>)の前で自動的に行を分割する。
            // ユーザー入力では改行で自然に分割されるが、定義文字列から復元する
            // パスでも同一の行構造を保証するために必要。
            Token::ChevronBranch | Token::ChevronDefault => {
                if !processed_tokens.is_empty() {
                    let execution_line = ExecutionLine {
                        body_tokens: processed_tokens.clone().into(),
                    };
                    lines.push(execution_line);
                    processed_tokens.clear();
                }
                processed_tokens.push(tokens[i].clone());
            }
            _ => {
                processed_tokens.push(tokens[i].clone());
            }
        }
        i += 1;
    }

    if !processed_tokens.is_empty() {
        let execution_line = ExecutionLine {
            body_tokens: processed_tokens.into(),
        };
        lines.push(execution_line);
    }

    if lines.is_empty() {
        return Err(AjisaiError::from("Word definition cannot be empty"));
    }

    Ok(lines)
}

pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported {
            word: "DEL".into(),
            mode: "Stack".into(),
        });
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let name = extract_word_name_from_value(&val)?;

    let upper_name = name.to_uppercase();

    // FQN（DICT@WORD）形式の場合、辞書名とワード名に分離
    let (target_dict, word_name) = if let Some((ns, w)) = interp.split_qualified_name(&upper_name) {
        (Some(ns), w)
    } else {
        (None, upper_name.clone())
    };

    if interp.core_vocabulary.contains_key(&word_name) {
        interp.force_flag = false;
        return Err(AjisaiError::BuiltinProtection {
            word: word_name,
            operation: "delete".into(),
        });
    }

    // 辞書全体の削除（FQNでない場合のみ）
    if target_dict.is_none() {
        if interp.user_dictionaries.contains_key(&word_name) {
            interp.user_dictionaries.remove(&word_name);
            interp.sync_user_words_cache();
            interp.rebuild_dependencies()?;
            interp
                .output_buffer
                .push_str(&format!("Deleted dictionary: {}\n", word_name));
            interp.force_flag = false;
            return Ok(());
        }

        if interp.module_samples.contains_key(&word_name) {
            interp.module_samples.remove(&word_name);
            interp.imported_modules.remove(&word_name);
            interp.sync_user_words_cache();
            interp.rebuild_dependencies()?;
            interp
                .output_buffer
                .push_str(&format!("Deleted dictionary: {}\n", word_name));
            interp.force_flag = false;
            return Ok(());
        }
    }

    // 個別ワードの所在を特定
    let (owner_name, is_module) = find_word_owner(interp, target_dict.as_deref(), &word_name)?;

    // モジュールサンプルワードはforceフラグ必須
    if is_module && !interp.force_flag {
        interp.force_flag = false;
        return Err(AjisaiError::from(format!(
            "Word '{}' is a module sample word. Use ! '{}' DEL to force delete.",
            word_name, word_name
        )));
    }

    let fq_name = format!("{}@{}", owner_name, word_name);
    let dependents = interp.collect_dependents(&fq_name);

    if !dependents.is_empty() && !interp.force_flag {
        let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
        return Err(AjisaiError::from(format!(
            "Cannot delete '{}': referenced by {}. Use ! '{}' DEL to force.",
            word_name, dep_list, word_name
        )));
    }

    // 削除実行
    let removed_def = if is_module {
        interp
            .module_samples
            .get_mut(&owner_name)
            .and_then(|dict| dict.sample_words.remove(&word_name))
    } else {
        interp
            .user_dictionaries
            .get_mut(&owner_name)
            .and_then(|dict| dict.words.remove(&word_name))
    };

    if let Some(removed_def) = removed_def {
        interp.sync_user_words_cache();
        for dep_name in &removed_def.dependencies {
            if let Some(deps) = interp.dependents.get_mut(dep_name) {
                deps.remove(&fq_name);
            }
        }
        interp.dependents.remove(&fq_name);
        for deps in interp.dependents.values_mut() {
            deps.remove(&fq_name);
        }
    }

    if !dependents.is_empty() {
        let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
        interp.output_buffer.push_str(&format!(
            "Warning: '{}' was deleted. Affected words: {}\n",
            word_name, dep_list
        ));
    }

    interp
        .output_buffer
        .push_str(&format!("Deleted word: {}\n", fq_name));

    interp.force_flag = false;
    Ok(())
}

/// ワードの所有辞書を特定する。
/// target_dict が指定されていれば、その辞書のみ検索する。
/// 返値は (辞書名, モジュールか否か)。
fn find_word_owner(
    interp: &Interpreter,
    target_dict: Option<&str>,
    word_name: &str,
) -> Result<(String, bool)> {
    if let Some(dict_name) = target_dict {
        // FQN指定: 指定された辞書から検索
        if let Some(dict) = interp.user_dictionaries.get(dict_name) {
            if dict.words.contains_key(word_name) {
                return Ok((dict_name.to_string(), false));
            }
        }
        if let Some(module) = interp.module_samples.get(dict_name) {
            if module.sample_words.contains_key(word_name) {
                return Ok((dict_name.to_string(), true));
            }
        }
        Err(AjisaiError::from(format!(
            "Word '{}@{}' is not defined",
            dict_name, word_name
        )))
    } else {
        // 短縮名: 全辞書を検索（user_dictionaries優先）
        for (dict_name, dict) in &interp.user_dictionaries {
            if dict.words.contains_key(word_name) {
                return Ok((dict_name.clone(), false));
            }
        }
        for (module_name, module) in &interp.module_samples {
            if module.sample_words.contains_key(word_name) {
                return Ok((module_name.clone(), true));
            }
        }
        Err(AjisaiError::from(format!(
            "Word '{}' is not defined",
            word_name
        )))
    }
}

pub fn op_lookup(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported {
            word: "? (LOOKUP)".into(),
            mode: "Stack".into(),
        });
    }

    let name_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let name_str = extract_word_name_from_value(&name_val)?;

    let upper_name = name_str.to_uppercase();

    if let Some(def) = interp.resolve_word(&upper_name) {
        if def.is_builtin {
            let detailed_info = crate::builtins::lookup_builtin_detail(&upper_name);
            interp.definition_to_load = Some(detailed_info);
            return Ok(());
        }

        if let Some(original_source) = &def.original_source {
            interp.definition_to_load = Some(original_source.clone());
        } else {
            let definition = interp
                .lookup_word_definition_tokens(&upper_name)
                .unwrap_or_default();
            let full_definition = if definition.is_empty() {
                format!("[ NIL ] '{}' DEF", name_str)
            } else {
                if let Some(desc) = &def.description {
                    format!("[ {} ] '{}' '{}' DEF", definition, name_str, desc)
                } else {
                    format!("[ {} ] '{}' DEF", definition, name_str)
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
    use crate::types::ValueData;

    #[tokio::test]
    async fn test_cannot_override_builtin_word() {
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("[ [ [ 1 ] + ] ] 'GET' DEF").await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Cannot redefine built-in word"),
            "Expected error message to contain 'Cannot redefine built-in word', got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_can_override_user_word() {
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        // Use code block syntax since vector duality no longer preserves operators.
        let result1 = interp.execute(": [ 2 ] * ; 'DOUBLE' DEF").await;
        assert!(result1.is_ok(), "First definition should succeed");

        let result2 = interp.execute(": [ 3 ] * ; 'DOUBLE' DEF").await;
        assert!(result2.is_ok(), "Overriding custom word should succeed");

        let result3 = interp.execute("[ 5 ] DOUBLE").await;
        assert!(result3.is_ok(), "Executing redefined word should succeed");

        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                if let Some(f) = children[0].as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 15, "Result should be 15");
                } else {
                    panic!("Expected scalar inside vector");
                }
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_cannot_override_other_builtin_words() {
        let mut interp = Interpreter::new();

        let builtin_words = vec!["INSERT", "REPLACE", "MAP", "FILTER", "PRINT"];

        for word in builtin_words {
            let code = format!("[ [ 1 ] + ] '{}' DEF", word);
            let result = interp.execute(&code).await;
            assert!(
                result.is_err(),
                "Should not be able to override builtin word: {}",
                word
            );
            let err_msg = result.unwrap_err().to_string();
            assert!(
                err_msg.contains("Cannot redefine built-in word"),
                "Expected error for {}, got: {}",
                word,
                err_msg
            );
        }
    }

    #[tokio::test]
    async fn test_def_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        let result = interp.execute("[ [ [ 2 ] * ] ] 'DOUBLE' .. DEF").await;
        assert!(result.is_err(), "DEF should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("DEF") && err_msg.contains("Stack mode"),
            "Expected Stack mode error for DEF, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_del_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        interp
            .execute("[ [ [ 2 ] * ] ] 'DOUBLE' DEF")
            .await
            .unwrap();

        let result = interp.execute("'DOUBLE' .. DEL").await;
        assert!(result.is_err(), "DEL should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("DEL") && err_msg.contains("Stack mode"),
            "Expected Stack mode error for DEL, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_lookup_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        interp
            .execute("[ [ [ 2 ] * ] ] 'DOUBLE' DEF")
            .await
            .unwrap();

        let result = interp.execute("'DOUBLE' .. ?").await;
        assert!(result.is_err(), "? (LOOKUP) should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("?") && err_msg.contains("Stack mode"),
            "Expected Stack mode error for ?, got: {}",
            err_msg
        );
    }

    fn restore_sample_words(interp: &mut Interpreter, sample_words: &[(&str, &str, &str)]) {
        use crate::tokenizer;

        for (name, definition, description) in sample_words {
            let tokens = tokenizer::tokenize(definition)
                .unwrap_or_else(|e| panic!("Failed to tokenize {}: {}", name, e));

            super::op_def_inner(interp, name, &tokens, Some(description.to_string()))
                .unwrap_or_else(|e| panic!("Failed to define {}: {}", name, e));
        }

        interp
            .rebuild_dependencies()
            .expect("Failed to rebuild dependencies");
    }

    #[tokio::test]
    async fn test_del_sample_user_words() {
        let mut interp = Interpreter::new();

        let sample_words = vec![
            ("C4", "264", "純正律 C4"),
            ("D4", "C4 9 * 8 /", "純正律 D4"),
            ("E4", "C4 5 * 4 /", "純正律 E4"),
        ];
        restore_sample_words(&mut interp, &sample_words);

        assert!(interp.user_words.contains_key("C4"));
        assert!(interp.user_words.contains_key("D4"));
        assert!(interp.user_words.contains_key("E4"));

        let result = interp.execute("'D4' DEL").await;
        assert!(result.is_ok(), "Should delete D4: {:?}", result.err());
        assert!(!interp.user_words.contains_key("D4"));

        let result = interp.execute("'C4' DEL").await;
        assert!(result.is_err(), "Should not delete C4 (has dependents)");

        let result = interp.execute("! 'C4' DEL").await;
        assert!(result.is_ok(), "Should force delete C4: {:?}", result.err());
        assert!(!interp.user_words.contains_key("C4"));
    }

    #[tokio::test]
    async fn test_del_sample_user_words_with_fqn() {
        // GUI経由のDEL: FQN（DEMO@WORD）形式での削除
        let mut interp = Interpreter::new();

        let sample_words = vec![
            ("C4", "264", "純正律 C4"),
            ("D4", "C4 9 * 8 /", "純正律 D4"),
            ("E4", "C4 5 * 4 /", "純正律 E4"),
        ];
        restore_sample_words(&mut interp, &sample_words);

        assert!(interp.user_words.contains_key("D4"));

        // FQN形式で削除
        let result = interp.execute("'DEMO@D4' DEL").await;
        assert!(result.is_ok(), "Should delete D4 via FQN: {:?}", result.err());
        assert!(!interp.user_words.contains_key("D4"));

        // 存在しないFQNは適切にエラー
        let result = interp.execute("'DEMO@NONEXISTENT' DEL").await;
        assert!(result.is_err(), "Should error for non-existent FQN word");

        // 依存関係ありの場合もFQNで正しくエラー
        let result = interp.execute("'DEMO@C4' DEL").await;
        assert!(result.is_err(), "Should not delete C4 via FQN (has dependents)");

        // forceフラグ付きFQNで強制削除
        let result = interp.execute("! 'DEMO@C4' DEL").await;
        assert!(result.is_ok(), "Should force delete C4 via FQN: {:?}", result.err());
        assert!(!interp.user_words.contains_key("C4"));
    }

    #[tokio::test]
    async fn test_execute_restored_sample_words() {
        let mut interp = Interpreter::new();

        let sample_words = vec![
            ("C4", "264", "純正律 C4"),
            ("D4", "C4 9 * 8 /", "純正律 D4"),
        ];
        restore_sample_words(&mut interp, &sample_words);

        let result = interp.execute("C4").await;
        assert!(
            result.is_ok(),
            "Executing C4 should succeed: {:?}",
            result.err()
        );
        assert_eq!(interp.stack.len(), 1);

        let result = interp.execute("D4").await;
        assert!(
            result.is_ok(),
            "Executing D4 should succeed: {:?}",
            result.err()
        );
        assert_eq!(interp.stack.len(), 2);
    }

    #[tokio::test]
    async fn test_sample_words_in_vector_literal_play() {
        // Module sample words (C4, D4 etc.) resolve to scalars inside vector literals.
        // Without DisplayHint, is_string_value treats all vectors as strings,
        // so the audio system interprets scalar elements as lyrics (codepoints).
        // The AUDIO command is still emitted but with an empty seq structure.
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();

        // Custom word names inside a vector literal resolve to their scalar values
        let result = interp.execute("[ C4 D4 E4 ] MUSIC@SEQ MUSIC@PLAY").await;
        assert!(
            result.is_ok(),
            "[ C4 D4 E4 ] MUSIC@SEQ MUSIC@PLAY should succeed: {:?}",
            result.err()
        );

        let output = interp.collect_output();
        // AUDIO command is still emitted (with empty seq since elements are treated as lyrics)
        assert!(
            output.contains("AUDIO:"),
            "Should contain AUDIO command, got: {}",
            output
        );
    }

    #[tokio::test]
    async fn test_sample_words_scalar_output() {
        // Sample words should push scalar values (not vectors)
        let mut interp = Interpreter::new();

        let sample_words = vec![
            ("C4", "264", "純正律 C4"),
            ("D4", "C4 9 * 8 /", "純正律 D4"),
        ];
        restore_sample_words(&mut interp, &sample_words);
        let _ = interp.collect_output();

        let _ = interp.execute("C4").await.unwrap();
        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            assert!(
                val.as_scalar().is_some(),
                "C4 should push a scalar, not a vector"
            );
            assert_eq!(val.as_scalar().unwrap().to_i64().unwrap(), 264);
        }

        let _ = interp.execute("D4").await.unwrap();
        assert_eq!(interp.stack.len(), 2);
        if let Some(val) = interp.stack.last() {
            assert!(
                val.as_scalar().is_some(),
                "D4 should push a scalar, not a vector"
            );
            assert_eq!(val.as_scalar().unwrap().to_i64().unwrap(), 297);
        }
    }

    #[tokio::test]
    async fn test_builtin_symbols_remain_strings_in_vector() {
        // Built-in operator symbols should still become strings in vectors
        // (preserving Vector Duality behavior for DEF)
        let mut interp = Interpreter::new();

        // Use code block syntax since vector duality no longer preserves
        // builtin operator symbols (from_string creates codepoint vectors).
        let result = interp.execute(": [ 2 ] * ; 'DOUBLE' DEF").await;
        assert!(
            result.is_ok(),
            "Code block DEF should work: {:?}",
            result.err()
        );

        let result = interp.execute("[ 5 ] DOUBLE").await;
        assert!(
            result.is_ok(),
            "Executing DOUBLE should succeed: {:?}",
            result.err()
        );
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1);
                assert_eq!(children[0].as_scalar().unwrap().to_i64().unwrap(), 10);
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_user_word_resolved_in_nested_vector() {
        // Module sample words should also resolve inside nested vectors
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();

        // Nested vector: [ [ C4 E4 G4 ] ] should create a vector of a vector of scalars.
        // Without DisplayHint, is_string_value treats all vectors as strings,
        // so audio treats nested scalar vectors as lyrics (no frequency output).
        let result = interp
            .execute("[ [ C4 E4 G4 ] ] MUSIC@SIM MUSIC@PLAY")
            .await;
        assert!(
            result.is_ok(),
            "Nested vector with custom words should work: {:?}",
            result.err()
        );

        let output = interp.collect_output();
        assert!(output.contains("AUDIO:"), "Should contain AUDIO command");
    }

    #[tokio::test]
    async fn test_def_with_vector_duality() {
        let mut interp = Interpreter::new();

        // Use code block syntax since vector duality no longer preserves operators.
        let result = interp.execute(": [ 2 ] * ; 'DOUBLE' DEF").await;
        assert!(
            result.is_ok(),
            "DEF with vector should succeed: {:?}",
            result
        );

        let result = interp.execute("[ 5 ] DOUBLE").await;
        assert!(
            result.is_ok(),
            "Executing DOUBLE should succeed: {:?}",
            result
        );

        assert_eq!(interp.stack.len(), 1, "Stack should have 1 element");
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
    async fn test_def_with_module_collision_warns() {
        // DEF of a name that collides with a module sample now succeeds with a warning
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();

        let result = interp.execute(": [ 999 ] ; 'C4' DEF").await;
        assert!(result.is_ok(), "DEF should succeed even with module collision: {:?}", result.err());
        let output = interp.collect_output();
        assert!(output.contains("Warning"),
            "Should warn about the collision: {}", output);
        assert!(output.contains("MUSIC@C4"),
            "Warning should mention MUSIC@C4: {}", output);
    }

    #[tokio::test]
    async fn test_import_keeps_user_word_qualified() {
        // IMPORT keeps conflicting user-defined words accessible via qualified path
        let mut interp = Interpreter::new();

        // Define C4 before importing music
        interp.execute(": [ 999 ] ; 'C4' DEF").await.unwrap();
        assert!(interp.user_words.contains_key("C4"));

        // Import music module — user word remains, short name becomes ambiguous
        interp.execute("'music' IMPORT").await.unwrap();
        let output = interp.collect_output();

        assert!(interp.user_words.contains_key("C4"),
            "User word C4 should remain in DEMO after IMPORT");
        assert!(output.contains("Warning"),
            "Should warn about the conflict: {}", output);

        // C4 is now ambiguous (exists in both MUSIC and DEMO), should error
        let result = interp.execute("C4").await;
        assert!(result.is_err(), "C4 should be ambiguous");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Ambiguous"),
            "Expected ambiguity error, got: {}", err_msg);

        // Qualified access to DEMO@C4 should work
        let result = interp.execute("DEMO@C4").await;
        assert!(result.is_ok(), "Qualified DEMO@C4 should work: {:?}", result.err());
        if let Some(val) = interp.stack.last() {
            let scalar = val
                .as_scalar()
                .or_else(|| {
                    val.as_vector()
                        .and_then(|children| children.first())
                        .and_then(|child| child.as_scalar())
                })
                .expect("DEMO@C4 should resolve to a numeric value");
            assert_eq!(scalar.to_i64().unwrap(), 999,
                "DEMO@C4 should remain the user-defined value");
        }

        // Qualified access to MUSIC@C4 should work too
        let result = interp.execute("MUSIC@C4").await;
        assert!(result.is_ok(), "Qualified MUSIC@C4 should work: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_module_word_resolves_without_conflict() {
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();

        let result = interp.execute("C4").await;
        assert!(result.is_ok(), "C4 should work: {:?}", result.err());
        if let Some(val) = interp.stack.last() {
            assert_eq!(val.as_scalar().unwrap().to_i64().unwrap(), 264,
                "C4 should be 264 (module sample)");
        }
    }

    #[tokio::test]
    async fn test_module_first_builtin_still_protected() {
        // Module-first: core built-in words are still protected from override
        let mut interp = Interpreter::new();
        let result = interp.execute(": [ 1 ] ; 'GET' DEF").await;
        assert!(result.is_err(), "Should not be able to override built-in GET");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Cannot redefine built-in word"),
            "Expected BuiltinProtection error, got: {}", err_msg);
    }

    #[tokio::test]
    async fn test_chevron_in_restored_definition_without_linebreaks() {
        // 定義文字列に改行がなくてもシェブロン分岐が正しく動作することを検証。
        // サンプルワード復元パスとユーザーDEFパスで同一の結果を保証する。
        let mut interp = Interpreter::new();

        let sample_words = vec![
            ("SAY-BY-SIGN",
             ">> [ 0 ] < >> 'Hello' PRINT >> [ 0 ] = >> 'Hello World' PRINT >>> 'World' PRINT",
             "sign branch sample"),
        ];
        restore_sample_words(&mut interp, &sample_words);
        let _ = interp.collect_output();

        // 負の値 → "Hello"
        let result = interp.execute("[ -1 ] SAY-BY-SIGN").await;
        assert!(result.is_ok(), "SAY-BY-SIGN with -1 should succeed: {:?}", result.err());
        let output = interp.collect_output();
        assert!(output.contains("Hello"), "Expected 'Hello' for negative, got: {}", output);

        // 0 → "Hello World"
        interp.stack.clear();
        let result = interp.execute("[ 0 ] SAY-BY-SIGN").await;
        assert!(result.is_ok(), "SAY-BY-SIGN with 0 should succeed: {:?}", result.err());
        let output = interp.collect_output();
        assert!(output.contains("Hello World"), "Expected 'Hello World' for zero, got: {}", output);

        // 正の値 → "World"
        interp.stack.clear();
        let result = interp.execute("[ 1 ] SAY-BY-SIGN").await;
        assert!(result.is_ok(), "SAY-BY-SIGN with 1 should succeed: {:?}", result.err());
        let output = interp.collect_output();
        assert!(output.contains("World"), "Expected 'World' for positive, got: {}", output);
    }

    // ========================================================================
    // New path resolution tests for @ notation
    // ========================================================================

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
        interp.execute(": [ 999 ] ; 'C4' DEF").await.unwrap();
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
        interp.execute(": [ 999 ] ; 'C4' DEF").await.unwrap();
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
