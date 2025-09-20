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

impl Interpreter {
    pub(crate) fn contains_nested_definition(&self, tokens: &[Token]) -> bool {
        tokens.iter().any(|t| matches!(t, Token::Symbol(s) if s == "INNER_DEF_LINE"))
    }
    
    pub(crate) fn parse_nested_definition_body(&self, tokens: &[Token]) -> Result<Vec<ExecutionLine>> {
        let mut lines = Vec::new();
        let mut i = 0;
        
        while i < tokens.len() {
            if let Token::Symbol(s) = &tokens[i] {
                if s == "INNER_DEF_LINE" {
                    let (line, consumed) = self.parse_single_inner_line(&tokens[i..])?;
                    lines.push(line);
                    i += consumed;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
        
        // デフォルト行の存在チェック
        let has_default = lines.iter().any(|line| line.condition_tokens.is_empty());
        if !has_default {
            return Err(AjisaiError::from("Nested definition must have at least one default line (without condition)"));
        }
        
        self.output_buffer.push_str(&format!("[DEBUG] Nested definition validation passed. {} lines total\n", lines.len()));
        Ok(lines)
    }
    
    fn parse_single_inner_line(&self, tokens: &[Token]) -> Result<(ExecutionLine, usize)> {
        // INNER_DEF_LINE から END_INNER_DEF_LINE までを解析
        let mut i = 1; // Skip INNER_DEF_LINE
        let mut condition_tokens = Vec::new();
        let mut body_tokens = Vec::new();
        let mut repeat_count = 1;
        let mut delay_ms = 0;
        let mut is_default_line = false;
        
        // WITH_CONDITION または DEFAULT_LINE かチェック
        if i < tokens.len() {
            if let Token::Symbol(s) = &tokens[i] {
                if s == "DEFAULT_LINE" {
                    is_default_line = true;
                    i += 1;
                } else if s == "WITH_CONDITION" {
                    i += 1;
                } else {
                    return Err(AjisaiError::from("Invalid inner definition line format"));
                }
            }
        }
        
        // トークンを解析
        while i < tokens.len() {
            match &tokens[i] {
                Token::Symbol(s) if s == "END_INNER_DEF_LINE" => {
                    break;
                },
                Token::GuardSeparator => {
                    // ガードセパレータが見つかった場合、それまでが条件部
                    i += 1;
                    // 残りが処理部（修飾子を除く）
                    break;
                },
                Token::Modifier(m) => {
                    // 修飾子の解析
                    self.parse_modifier(m, &mut repeat_count, &mut delay_ms);
                    i += 1;
                },
                _ => {
                    if is_default_line || condition_tokens.is_empty() {
                        // デフォルト行または条件部
                        if !is_default_line && !tokens[i..].iter().any(|t| matches!(t, Token::GuardSeparator)) {
                            // ガードセパレータがない場合はデフォルト行として扱う
                            is_default_line = true;
                        }
                        
                        if is_default_line {
                            body_tokens.push(tokens[i].clone());
                        } else {
                            condition_tokens.push(tokens[i].clone());
                        }
                    } else {
                        body_tokens.push(tokens[i].clone());
                    }
                    i += 1;
                }
            }
        }
        
        // ガードセパレータ以降の処理部を取得
        if !is_default_line {
            while i < tokens.len() {
                match &tokens[i] {
                    Token::Symbol(s) if s == "END_INNER_DEF_LINE" => break,
                    Token::Modifier(m) => {
                        self.parse_modifier(m, &mut repeat_count, &mut delay_ms);
                        i += 1;
                    },
                    _ => {
                        body_tokens.push(tokens[i].clone());
                        i += 1;
                    }
                }
            }
        }
        
        // END_INNER_DEF_LINE をスキップ
        if i < tokens.len() {
            i += 1;
        }
        
        let final_condition_tokens = if is_default_line { Vec::new() } else { condition_tokens };
        
        Ok((ExecutionLine {
            condition_tokens: final_condition_tokens,
            body_tokens,
            repeat_count,
            delay_ms,
        }, i))
    }
    
    fn parse_modifier(&self, modifier: &str, repeat_count: &mut i64, delay_ms: &mut u64) {
        if modifier.ends_with('x') {
            if let Ok(count) = modifier[..modifier.len()-1].parse::<i64>() {
                *repeat_count = count;
            }
        } else if modifier.ends_with("ms") {
            if let Ok(ms) = modifier[..modifier.len()-2].parse::<u64>() {
                *delay_ms = ms;
            }
        } else if modifier.ends_with('s') {
            if let Ok(s) = modifier[..modifier.len()-1].parse::<u64>() {
                *delay_ms = s * 1000;
            }
        }
    }
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
