use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::get_word_name_from_value;
use crate::interpreter::vector_exec::vector_to_source;
use crate::interpreter::{Interpreter, OperationTargetMode, WordDefinition};
use crate::types::{ExecutionLine, Token, Value, ValueData};
use std::collections::HashSet;
use std::sync::Arc;

fn value_to_string(val: &Value) -> Result<String> {
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

fn is_custom_word_defined(interp: &Interpreter, symbol: &str) -> bool {
    let upper_symbol = symbol.to_uppercase();
    interp.is_custom_word(&upper_symbol)
}

fn has_definition_description(stack: &[Value]) -> bool {
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

    let has_description = has_definition_description(&interp.stack);

    if has_description {
        if let Some(desc_val) = interp.stack.pop() {
            if let Ok(s) = value_to_string(&desc_val) {
                description = Some(s);
            }
        }
    }

    let name_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let name_str = get_word_name_from_value(&name_val)?;

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
                Token::ScopeDirective(name) => format!("@{}", name),
            })
            .collect::<Vec<_>>()
            .join(" "),
        ValueData::Vector(_) | ValueData::Record { .. } => vector_to_source(&def_val)?,
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

    if interp.builtin_dictionary.contains_key(&upper_name) {
        interp.force_flag = false;
        return Err(AjisaiError::BuiltinProtection {
            word: upper_name,
            operation: "redefine".into(),
        });
    }

    if let Some(existing) = interp.dictionary.get(&upper_name) {
        let dependents = interp.get_dependents(&upper_name);

        if !dependents.is_empty() && !interp.force_flag {
            let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
            interp.force_flag = false;
            return Err(AjisaiError::from(format!(
                "Cannot redefine '{}': referenced by {}. Use ! [ ... ] '{}' DEF to force.",
                upper_name, dep_list, upper_name
            )));
        }

        if !dependents.is_empty() {
            let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
            interp.output_buffer.push_str(&format!(
                "Warning: '{}' was redefined. Affected words: {}\n",
                upper_name, dep_list
            ));
        }

        for dep_name in &existing.dependencies {
            if let Some(dependents) = interp.dependents.get_mut(dep_name) {
                dependents.remove(&upper_name);
            }
        }
    }

    let lines = parse_definition_body(tokens)?;

    let mut new_dependencies = HashSet::new();
    for line in &lines {
        for token in line.body_tokens.iter() {
            if let Token::Symbol(s) = token {
                let upper_s = s.to_uppercase();
                if is_custom_word_defined(interp, s) {
                    new_dependencies.insert(upper_s);
                }
            }
        }
    }

    for dep_name in &new_dependencies {
        interp
            .dependents
            .entry(dep_name.clone())
            .or_default()
            .insert(upper_name.clone());
    }

    let new_def = WordDefinition {
        lines: lines.into(),
        is_builtin: false,
        description,
        dependencies: new_dependencies,
        original_source: None,
    };

    interp
        .dictionary
        .insert(upper_name.clone(), Arc::new(new_def));
    interp
        .output_buffer
        .push_str(&format!("Defined word: {}\n", name));
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

    let name = get_word_name_from_value(&val)?;

    let upper_name = name.to_uppercase();

    if interp.builtin_dictionary.contains_key(&upper_name) {
        interp.force_flag = false;
        return Err(AjisaiError::BuiltinProtection {
            word: upper_name.clone(),
            operation: "delete".into(),
        });
    }

    // Check if word exists in dictionary (user-defined)
    let in_dictionary = interp.dictionary.contains_key(&upper_name);

    // Check if word exists in module samples
    let in_module_samples = interp
        .module_samples
        .values()
        .any(|md| md.contains_key(&upper_name));

    if !in_dictionary && !in_module_samples {
        interp.force_flag = false;
        return Err(AjisaiError::from(format!(
            "Word '{}' is not defined",
            upper_name
        )));
    }

    // Module sample words require force flag
    if !in_dictionary && in_module_samples && !interp.force_flag {
        interp.force_flag = false;
        return Err(AjisaiError::from(format!(
            "Word '{}' is a module sample word. Use ! '{}' DEL to force delete.",
            upper_name, upper_name
        )));
    }

    let dependents = interp.get_dependents(&upper_name);

    if !dependents.is_empty() && !interp.force_flag {
        let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
        return Err(AjisaiError::from(format!(
            "Cannot delete '{}': referenced by {}. Use ! '{}' DEL to force.",
            upper_name, dep_list, upper_name
        )));
    }

    // Delete from dictionary (user-defined)
    if let Some(removed_def) = interp.dictionary.remove(&upper_name) {
        for dep_name in &removed_def.dependencies {
            if let Some(deps) = interp.dependents.get_mut(dep_name) {
                deps.remove(&upper_name);
            }
        }
        interp.dependents.remove(&upper_name);

        for deps in interp.dependents.values_mut() {
            deps.remove(&upper_name);
        }

        if !dependents.is_empty() {
            let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
            interp.output_buffer.push_str(&format!(
                "Warning: '{}' was deleted. Affected words: {}\n",
                upper_name, dep_list
            ));
        }

        interp
            .output_buffer
            .push_str(&format!("Deleted word: {}\n", name));
    } else if in_module_samples {
        // Delete from module samples (force flag required, already checked)
        for module_dict in interp.module_samples.values_mut() {
            if let Some(removed_def) = module_dict.remove(&upper_name) {
                for dep_name in &removed_def.dependencies {
                    if let Some(deps) = interp.dependents.get_mut(dep_name) {
                        deps.remove(&upper_name);
                    }
                }
                interp.dependents.remove(&upper_name);
                for deps in interp.dependents.values_mut() {
                    deps.remove(&upper_name);
                }
                break;
            }
        }

        if !dependents.is_empty() {
            let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
            interp.output_buffer.push_str(&format!(
                "Warning: '{}' was deleted. Affected words: {}\n",
                upper_name, dep_list
            ));
        }

        interp
            .output_buffer
            .push_str(&format!("Deleted word: {}\n", name));
    }

    interp.force_flag = false;
    Ok(())
}

pub fn op_lookup(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported {
            word: "? (LOOKUP)".into(),
            mode: "Stack".into(),
        });
    }

    let name_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let name_str = get_word_name_from_value(&name_val)?;

    let upper_name = name_str.to_uppercase();

    if let Some(def) = interp.resolve_word(&upper_name) {
        if def.is_builtin {
            let detailed_info = crate::builtins::get_builtin_detail(&upper_name);
            interp.definition_to_load = Some(detailed_info);
            return Ok(());
        }

        if let Some(original_source) = &def.original_source {
            interp.definition_to_load = Some(original_source.clone());
        } else {
            let definition = interp
                .get_word_definition_tokens(&upper_name)
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
    async fn test_can_override_custom_word() {
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
    async fn test_del_sample_custom_words() {
        let mut interp = Interpreter::new();

        let sample_words = vec![
            ("C4", "264", "純正律 C4"),
            ("D4", "C4 9 * 8 /", "純正律 D4"),
            ("E4", "C4 5 * 4 /", "純正律 E4"),
        ];
        restore_sample_words(&mut interp, &sample_words);

        assert!(interp.dictionary.contains_key("C4"));
        assert!(interp.dictionary.contains_key("D4"));
        assert!(interp.dictionary.contains_key("E4"));

        let result = interp.execute("'D4' DEL").await;
        assert!(result.is_ok(), "Should delete D4: {:?}", result.err());
        assert!(!interp.dictionary.contains_key("D4"));

        let result = interp.execute("'C4' DEL").await;
        assert!(result.is_err(), "Should not delete C4 (has dependents)");

        let result = interp.execute("! 'C4' DEL").await;
        assert!(result.is_ok(), "Should force delete C4: {:?}", result.err());
        assert!(!interp.dictionary.contains_key("C4"));
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
        // Custom words (C4, D4 etc.) resolve to scalars inside vector literals.
        // Without DisplayHint, is_string_value treats all vectors as strings,
        // so the audio system interprets scalar elements as lyrics (codepoints).
        // The AUDIO command is still emitted but with an empty seq structure.
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();

        let sample_words = vec![
            ("C4", "264", "純正律 C4"),
            ("D4", "C4 9 * 8 /", "純正律 D4"),
            ("E4", "C4 5 * 4 /", "純正律 E4"),
        ];
        restore_sample_words(&mut interp, &sample_words);
        let _ = interp.get_output();

        // Custom word names inside a vector literal resolve to their scalar values
        let result = interp.execute("[ C4 D4 E4 ] MUSIC::SEQ MUSIC::PLAY").await;
        assert!(
            result.is_ok(),
            "[ C4 D4 E4 ] MUSIC::SEQ MUSIC::PLAY should succeed: {:?}",
            result.err()
        );

        let output = interp.get_output();
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
        let _ = interp.get_output();

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
    async fn test_custom_word_resolved_in_nested_vector() {
        // Custom words should also resolve inside nested vectors
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();

        let sample_words = vec![
            ("C4", "264", "純正律 C4"),
            ("E4", "330", "純正律 E4"),
            ("G4", "396", "純正律 G4"),
        ];
        restore_sample_words(&mut interp, &sample_words);
        let _ = interp.get_output();

        // Nested vector: [ [ C4 E4 G4 ] ] should create a vector of a vector of scalars.
        // Without DisplayHint, is_string_value treats all vectors as strings,
        // so audio treats nested scalar vectors as lyrics (no frequency output).
        let result = interp
            .execute("[ [ C4 E4 G4 ] ] MUSIC::SIM MUSIC::PLAY")
            .await;
        assert!(
            result.is_ok(),
            "Nested vector with custom words should work: {:?}",
            result.err()
        );

        let output = interp.get_output();
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
}
