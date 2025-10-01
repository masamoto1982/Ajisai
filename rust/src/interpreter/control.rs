// rust/src/interpreter/control.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Token, ExecutionLine, ValueType, WordDefinition}; // Value を削除
use std::collections::HashSet;

pub fn op_def(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 { return Err(AjisaiError::from("DEF requires a definition block and a name")); }

    let name_val = interp.stack.pop().unwrap();
    let body_val = interp.stack.pop().unwrap();

    let name_str = if let ValueType::Vector(v, _) = name_val.val_type {
        if v.len() == 1 {
            if let ValueType::String(s) = &v[0].val_type {
                s.clone()
            } else {
                return Err(AjisaiError::type_error("string for word name", "other type"));
            }
        } else {
            return Err(AjisaiError::type_error("single-element vector", "multi-element vector"));
        }
    } else {
        return Err(AjisaiError::type_error("vector for word name", "other type"));
    };
    
    let tokens = if let ValueType::DefinitionBody(t) = body_val.val_type {
        t
    } else {
        return Err(AjisaiError::type_error("definition block for word body", "other type"));
    };

    op_def_inner(interp, &tokens, &name_str, None, None)
}

pub(crate) fn op_def_inner(interp: &mut Interpreter, tokens: &[Token], name: &str, description: Option<String>, original_source: Option<String>) -> Result<()> {
    let upper_name = name.to_uppercase();
    interp.output_buffer.push_str(&format!("[DEBUG] Defining word '{}'\n", upper_name));

    // 以前の定義があれば、古い依存関係を削除
    if let Some(old_def) = interp.dictionary.get(&upper_name) {
        for dep_name in &old_def.dependencies {
            if let Some(dependents) = interp.dependents.get_mut(dep_name) {
                dependents.remove(&upper_name);
            }
        }
    }

    let lines = parse_definition_body_new_syntax(interp, tokens)?;
    
    // 新しい依存関係を計算
    let mut new_dependencies = HashSet::new();
    for line in &lines {
        for token in line.condition_tokens.iter().chain(line.body_tokens.iter()) {
            if let Token::Symbol(s) = token {
                let upper_s = s.to_uppercase();
                if interp.dictionary.contains_key(&upper_s) && !interp.dictionary.get(&upper_s).unwrap().is_builtin {
                    new_dependencies.insert(upper_s);
                }
            }
        }
    }
    
    // 新しい依存関係を登録
    for dep_name in &new_dependencies {
        interp.dependents.entry(dep_name.clone()).or_default().insert(upper_name.clone());
    }
    
    let new_def = WordDefinition {
        lines,
        is_builtin: false,
        description,
        dependencies: new_dependencies,
        original_source,
    };
    
    interp.dictionary.insert(upper_name.clone(), new_def);
    interp.output_buffer.push_str(&format!("Defined word: {}\n", name));
    Ok(())
}

// 新構文用のパーサー: 改行ベース + : 条件分岐
fn parse_definition_body_new_syntax(_interp: &mut Interpreter, tokens: &[Token]) -> Result<Vec<ExecutionLine>> {
    let mut lines = Vec::new();
    let mut current_line_tokens = Vec::new();
    
    for token in tokens {
        match token {
            Token::LineBreak => {
                if !current_line_tokens.is_empty() {
                    let execution_line = parse_single_execution_line(&current_line_tokens)?;
                    lines.push(execution_line);
                    current_line_tokens.clear();
                }
            },
            _ => {
                current_line_tokens.push(token.clone());
            }
        }
    }
    
    // 最終行の処理
    if !current_line_tokens.is_empty() {
        let execution_line = parse_single_execution_line(&current_line_tokens)?;
        lines.push(execution_line);
    }
    
    if lines.is_empty() {
        return Err(AjisaiError::from("Word definition cannot be empty"));
    }
    
    Ok(lines)
}

fn parse_single_execution_line(tokens: &[Token]) -> Result<ExecutionLine> {
    // 修飾子（3x, 100msなど）を検出
    let mut repeat_count = 1i64;
    let mut delay_ms = 0u64;
    let mut modifier_positions = Vec::new();
    
    for (i, token) in tokens.iter().enumerate() {
        if let Token::Modifier(m_str) = token {
            modifier_positions.push(i);
            if m_str.ends_with('x') {
                if let Ok(count) = m_str[..m_str.len()-1].parse::<i64>() {
                    repeat_count = count;
                }
            } else if m_str.ends_with("ms") {
                if let Ok(ms) = m_str[..m_str.len()-2].parse::<u64>() {
                    delay_ms = ms;
                }
            } else if m_str.ends_with('s') {
                if let Ok(s) = m_str[..m_str.len()-1].parse::<u64>() {
                    delay_ms = s * 1000;
                }
            }
        }
    }
    
    // 修飾子を除いた実行部分を取得
    let execution_tokens: Vec<Token> = tokens.iter().enumerate() // mut を削除
        .filter(|(i, _)| !modifier_positions.contains(i))
        .map(|(_, token)| token.clone())
        .collect();
    
    // : による条件分岐の検出
    let guard_position = execution_tokens.iter().position(|t| matches!(t, Token::GuardSeparator));
    
    let (condition_tokens, body_tokens) = if let Some(guard_pos) = guard_position {
        (execution_tokens[..guard_pos].to_vec(), execution_tokens[guard_pos+1..].to_vec())
    } else {
        (Vec::new(), execution_tokens)
    };
    
    Ok(ExecutionLine {
        condition_tokens,
        body_tokens,
        repeat_count,
        delay_ms,
    })
}

pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.last().ok_or(AjisaiError::StackUnderflow)?;
    
    let name = match &val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::String(s) => s.clone(),
                _ => return Err(AjisaiError::type_error("string", "other type")),
            }
        },
        _ => return Err(AjisaiError::type_error("single-element vector with string", "other type")),
    };

    let upper_name = name.to_uppercase();

    if let Some(removed_def) = interp.dictionary.remove(&upper_name) {
        for dep_name in &removed_def.dependencies {
            if let Some(dependents) = interp.dependents.get_mut(dep_name) {
                dependents.remove(&upper_name);
            }
        }
        interp.dependents.remove(&upper_name);
        
        interp.stack.pop();
        interp.output_buffer.push_str(&format!("Deleted word: {}\n", name));
        Ok(())
    } else {
        Err(AjisaiError::UnknownWord(upper_name))
    }
}

pub fn op_lookup(interp: &mut Interpreter) -> Result<()> {
    let name_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let name_str = if let ValueType::Vector(v, _) = name_val.val_type {
        if v.len() == 1 {
            if let ValueType::String(s) = &v[0].val_type {
                s.clone()
            } else {
                return Err(AjisaiError::type_error("string for word name", "other type"));
            }
        } else {
            return Err(AjisaiError::type_error("single-element vector", "multi-element vector"));
        }
    } else {
        return Err(AjisaiError::type_error("vector for word name", "other type"));
    };

    let upper_name = name_str.to_uppercase();
    if let Some(def) = interp.dictionary.get(&upper_name) {
        // 元ソースコードがあればそれを優先、なければトークンから再構成
        let definition = if let Some(original_source) = &def.original_source {
            original_source.clone()
        } else {
            // フォールバック：既存の方式でトークンから再構成
            interp.get_word_definition_tokens(&upper_name).unwrap_or_default()
        };
        
        let full_definition = if definition.is_empty() {
            // 説明なしの場合
            format!("'{}' DEF", name_str)
        } else {
            // 説明ありの場合
            if let Some(desc) = &def.description {
                format!("{} '{}' '{}' DEF", definition, name_str, desc)
            } else {
                format!("{} '{}' DEF", definition, name_str)
            }
        };
        
        interp.definition_to_load = Some(full_definition);
        Ok(())
    } else {
        Err(AjisaiError::UnknownWord(name_str))
    }
}

pub fn parse_multiple_word_definitions(interp: &mut Interpreter, input: &str) -> Result<()> {
    let lines: Vec<&str> = input.lines().collect();
    let mut current_word_lines = Vec::new();
    let mut definition_start_line = 0;
    let mut found_first_content = false;
    
    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        // 空行やコメント行の処理
        if trimmed.is_empty() || trimmed.starts_with('#') {
            if found_first_content {
                current_word_lines.push(line.to_string());
            }
            continue;
        }
        
        // DEF パターンの検出
        if trimmed.ends_with(" DEF") || trimmed.contains(" DEF ") {
            // ワード定義を実行
            let def_parts = extract_word_name_and_description(trimmed)?;
            let word_name = def_parts.0;
            let description = def_parts.1;
            
            let word_source = lines[definition_start_line..line_num].join("\n");
            define_word_from_lines(interp, &current_word_lines, &word_name, description, Some(word_source))?;
            
            // 次のワードのための準備
            current_word_lines.clear();
            definition_start_line = line_num + 1;
            found_first_content = false;
        } else {
            // 通常の行を追加
            if !found_first_content {
                found_first_content = true;
                definition_start_line = line_num;
            }
            current_word_lines.push(line.to_string());
        }
    }
    
    // 最後にDEFがなかった場合のエラーチェック
    if !current_word_lines.is_empty() {
        return Err(AjisaiError::from("Word definition without DEF keyword"));
    }
    
    Ok(())
}

fn extract_word_name_and_description(def_line: &str) -> Result<(String, Option<String>)> {
    let trimmed = def_line.trim();
    
    // DEFの位置を探す
    let def_pos = if let Some(pos) = trimmed.rfind(" DEF") {
        if pos + 4 == trimmed.len() {
            pos
        } else {
            return Err(AjisaiError::from("Invalid DEF syntax. DEF must be at the end of the line"));
        }
    } else {
        return Err(AjisaiError::from("DEF keyword not found"));
    };
    
    let before_def = trimmed[..def_pos].trim();
    
    // シングルクォートで囲まれた文字列を抽出
    let mut strings = Vec::new();
    let mut current_pos = 0;
    
    while current_pos < before_def.len() {
        if before_def.chars().nth(current_pos) == Some('\'') {
            // 開始クォートを見つけた
            let start = current_pos + 1;
            if let Some(end_relative) = before_def[start..].find('\'') {
                let end = start + end_relative;
                strings.push(before_def[start..end].to_string());
                current_pos = end + 1;
            } else {
                return Err(AjisaiError::from("Unclosed quote in DEF line"));
            }
        } else {
            current_pos += 1;
        }
    }
    
    match strings.len() {
        1 => Ok((strings[0].clone(), None)),
        2 => Ok((strings[0].clone(), Some(strings[1].clone()))),
        _ => Err(AjisaiError::from("Invalid DEF syntax. Use 'NAME' DEF or 'NAME' 'DESCRIPTION' DEF")),
    }
}

fn define_word_from_lines(interp: &mut Interpreter, lines: &[String], name: &str, description: Option<String>, original_source: Option<String>) -> Result<()> {
    let definition_text = lines.join("\n");
    
    // カスタムワード名を収集
    let custom_word_names: HashSet<String> = interp.dictionary.iter()
        .filter(|(_, def)| !def.is_builtin)
        .map(|(name, _)| name.clone())
        .collect();
    
    // トークン化
    let tokens = crate::tokenizer::tokenize_with_custom_words(&definition_text, &custom_word_names)
        .map_err(|e| AjisaiError::from(format!("Tokenization error: {}", e)))?;
    
    // 定義実行
    op_def_inner(interp, &tokens, name, description, original_source)
}
