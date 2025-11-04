// rust/src/interpreter/dictionary.rs

use crate::interpreter::{Interpreter, WordDefinition};
use crate::interpreter::error::{AjisaiError, Result};
// [FIX 1] Import ExecutionLine from crate::types
use crate::types::{Token, BracketType, ValueType, ExecutionLine};
use std::collections::HashSet;

pub fn op_def(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    // 説明（オプション）を先にチェック
    let mut description = None;
    let has_description = if interp.stack.len() >= 3 {
        // スタックトップが文字列かチェック
        if let Some(top_val) = interp.stack.last() {
            matches!(top_val.val_type, ValueType::String(_))
        } else {
            false
        }
    } else {
        false
    };
    
    if has_description {
        if let Some(desc_val) = interp.stack.pop() {
            if let ValueType::String(s) = desc_val.val_type {
                description = Some(s);
            }
        }
    }
    
    // 名前を取得（文字列として）
    let name_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let name_str = match name_val.val_type {
        ValueType::String(s) => s,
        _ => return Err(AjisaiError::type_error("string 'name'", "other type")),
    };
    
    // 定義本体を取得
    let def_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    
    // 定義本体を文字列として取得
    let definition_str = match &def_val.val_type {
        ValueType::Vector(vec, _) => {
            if vec.len() == 1 {
                match &vec[0].val_type {
                    ValueType::String(s) => s.clone(),
                    _ => return Err(AjisaiError::type_error("string in vector", "other type")),
                }
            } else {
                return Err(AjisaiError::type_error("single-element vector", "multi-element vector"));
            }
        },
        _ => return Err(AjisaiError::type_error("vector with string", "other type")),
    };
    
    // トークン化して登録
    let custom_word_names: HashSet<String> = interp.dictionary.iter()
        .filter(|(_, def)| !def.is_builtin)
        .map(|(name, _)| name.clone())
        .collect();
    
    let tokens = crate::tokenizer::tokenize_with_custom_words(&definition_str, &custom_word_names)
        .map_err(|e| AjisaiError::from(format!("Tokenization error in DEF: {}", e)))?;
    
    op_def_inner(interp, &name_str, &tokens, description)
}


pub(crate) fn op_def_inner(interp: &mut Interpreter, name: &str, tokens: &[Token], description: Option<String>) -> Result<()> {
    let upper_name = name.to_uppercase();
    interp.output_buffer.push_str(&format!("[DEBUG] Defining word '{}'\n", upper_name));

    if let Some(old_def) = interp.dictionary.get(&upper_name) {
        for dep_name in &old_def.dependencies {
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
    Ok(())
}

fn parse_definition_body(tokens: &[Token], dictionary: &std::collections::HashMap<String, WordDefinition>) -> Result<Vec<ExecutionLine>> {
    let mut lines = Vec::new();
    let mut processed_tokens = Vec::new();
    
    // ガード文内の文字列をクォーテーションに変換
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
                processed_tokens.push(Token::VectorStart(BracketType::Square));
                processed_tokens.extend(inner_tokens);
                processed_tokens.push(Token::VectorEnd(BracketType::Square));
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
    // DELは 'NAME' を期待する
    let val = interp.stack.last().ok_or(AjisaiError::StackUnderflow)?;
    
    let name = match &val.val_type {
        ValueType::String(s) => s.clone(),
        _ => return Err(AjisaiError::type_error("string 'name'", "other type")),
    };

    let upper_name = name.to_uppercase();

    if let Some(removed_def) = interp.dictionary.remove(&upper_name) {
        for dep_name in &removed_def.dependencies {
            if let Some(dependents) = interp.dependents.get_mut(dep_name) {
                dependents.remove(&upper_name);
            }
        }
        interp.dependents.remove(&upper_name);
        
        interp.stack.pop(); // 'NAME' をポップ
        interp.output_buffer.push_str(&format!("Deleted word: {}\n", name));
        Ok(())
    } else {
        Err(AjisaiError::UnknownWord(upper_name))
    }
}

pub fn op_lookup(interp: &mut Interpreter) -> Result<()> {
    // LOOKUP (?) は 'NAME' を期待する
    let name_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let name_str = if let ValueType::String(s) = name_val.val_type {
        s.clone()
    } else {
        return Err(AjisaiError::type_error("string 'name'", name_val.val_type.to_string().as_str()));
    };

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
                format!("[ '' ] '{}' DEF", name_str) // 空の定義
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
