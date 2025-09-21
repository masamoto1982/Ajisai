// rust/src/interpreter/control.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Token, ExecutionLine, ValueType, WordDefinition};

pub fn op_def(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 { return Err(AjisaiError::from("DEF requires a definition block and a name")); }

    let name_val = interp.workspace.pop().unwrap();
    let body_val = interp.workspace.pop().unwrap();

    let name = if let ValueType::Vector(v, _) = name_val.val_type {
        if v.len() == 1 {
            if let ValueType::String(s) = &v[0].val_type {
                s.to_uppercase()
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

    interp.output_buffer.push_str(&format!("[DEBUG] Defining word '{}' with {} tokens\n", name, tokens.len()));
    
    // ネストした定義構造かどうかを判定
    if interp.contains_nested_definition(&tokens) {
        let lines = interp.parse_nested_definition_body(&tokens)?;
        interp.output_buffer.push_str(&format!("[DEBUG] Parsed {} nested lines for word '{}'\n", lines.len(), name));
        
        interp.dictionary.insert(name.clone(), WordDefinition {
            lines,
            is_builtin: false,
            description: None,
        });
    } else {
        // 従来の単一ライン定義
        let lines = parse_definition_body(interp, &tokens)?;
        interp.output_buffer.push_str(&format!("[DEBUG] Parsed {} lines for word '{}'\n", lines.len(), name));
        
        interp.dictionary.insert(name.clone(), WordDefinition {
            lines,
            is_builtin: false,
            description: None,
        });
    }
    
    interp.output_buffer.push_str(&format!("Defined word: {}\n", name));
    Ok(())
}

fn parse_definition_body(interp: &mut Interpreter, tokens: &[Token]) -> Result<Vec<ExecutionLine>> {
    let line_groups: Vec<&[Token]> = tokens.split(|tok| matches!(tok, Token::LineBreak))
        .filter(|line| !line.is_empty())
        .collect();
    
    interp.output_buffer.push_str(&format!("[DEBUG] Split definition into {} lines\n", line_groups.len()));
    
    let lines: Result<Vec<ExecutionLine>> = line_groups.iter().enumerate()
        .map(|(line_num, line_tokens)| {
            interp.output_buffer.push_str(&format!("[DEBUG] Processing line {}: {} tokens\n", line_num + 1, line_tokens.len()));
            
            let mut repeat_count = 1;
            let mut delay_ms = 0;
            
            let mut modifier_tokens = 0;
            for token in line_tokens.iter().rev() {
                if let Token::Modifier(m_str) = token {
                    interp.output_buffer.push_str(&format!("[DEBUG] Line {} modifier: {}\n", line_num + 1, m_str));
                    if m_str.ends_with('x') {
                        if let Ok(count) = m_str[..m_str.len()-1].parse::<i64>() {
                            repeat_count = count;
                            interp.output_buffer.push_str(&format!("[DEBUG] Line {} repeat count: {}\n", line_num + 1, count));
                        }
                    } else if m_str.ends_with("ms") {
                        if let Ok(ms) = m_str[..m_str.len()-2].parse::<u64>() {
                            delay_ms = ms;
                            interp.output_buffer.push_str(&format!("[DEBUG] Line {} delay: {}ms\n", line_num + 1, ms));
                        }
                    } else if m_str.ends_with('s') {
                        if let Ok(s) = m_str[..m_str.len()-1].parse::<u64>() {
                            delay_ms = s * 1000;
                            interp.output_buffer.push_str(&format!("[DEBUG] Line {} delay: {}s ({}ms)\n", line_num + 1, s, delay_ms));
                        }
                    }
                    modifier_tokens += 1;
                } else {
                    break;
                }
            }
            let main_tokens = &line_tokens[..line_tokens.len() - modifier_tokens];
            interp.output_buffer.push_str(&format!("[DEBUG] Line {} main tokens: {}, modifier tokens: {}\n", line_num + 1, main_tokens.len(), modifier_tokens));

            let (condition_tokens, body_tokens) = 
                if let Some(pos) = main_tokens.iter().position(|t| matches!(t, Token::GuardSeparator)) {
                    interp.output_buffer.push_str(&format!("[DEBUG] Line {} has condition (guard at position {})\n", line_num + 1, pos));
                    (main_tokens[..pos].to_vec(), main_tokens[pos+1..].to_vec())
                } else {
                    interp.output_buffer.push_str(&format!("[DEBUG] Line {} is default line (no condition)\n", line_num + 1));
                    (Vec::new(), main_tokens.to_vec())
                };

            Ok(ExecutionLine { condition_tokens, body_tokens, repeat_count, delay_ms })
        })
        .collect();
    
    let lines = lines?;
    
    // デフォルト行（条件なし）の存在チェック
    let default_lines: Vec<usize> = lines.iter().enumerate()
        .filter(|(_, line)| line.condition_tokens.is_empty())
        .map(|(i, _)| i + 1)
        .collect();
    
    interp.output_buffer.push_str(&format!("[DEBUG] Default lines found: {:?}\n", default_lines));
    
    if default_lines.is_empty() {
        interp.output_buffer.push_str("[DEBUG] ERROR: No default line found!\n");
        return Err(AjisaiError::from("Custom word definition must have at least one default line (without condition)"));
    }
    
    interp.output_buffer.push_str(&format!("[DEBUG] Definition validation passed. {} lines total, {} default lines\n", lines.len(), default_lines.len()));
    Ok(lines)
}

pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop().ok_or(AjisaiError::WorkspaceUnderflow)?;
    let name = if let ValueType::Vector(v, _) = val.val_type {
        if v.len() == 1 {
            if let ValueType::String(s) = &v[0].val_type {
                s.clone()
            } else {
                return Err(AjisaiError::type_error("string", "other type"));
            }
        } else {
            return Err(AjisaiError::type_error("single-element vector", "multi-element vector"));
        }
    } else {
        return Err(AjisaiError::type_error("vector", "other type"));
    };

    if interp.dictionary.remove(&name.to_uppercase()).is_some() {
        interp.output_buffer.push_str(&format!("Deleted word: {}\n", name));
    }
    Ok(())
}
