// rust/src/interpreter/control.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Token, ExecutionLine, Value, ValueType, WordDefinition};

pub fn op_def(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 { return Err(AjisaiError::from("DEF requires a definition block and a name")); }

    let name_val = interp.workspace.pop().unwrap();
    let body_val = interp.workspace.pop().unwrap();

    // 名前を取得
    let name = match name_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => {
            if let ValueType::String(s) = &v[0].val_type {
                s.to_uppercase()
            } else {
                return Err(AjisaiError::type_error("string for word name", "other type"));
            }
        },
        _ => return Err(AjisaiError::type_error("vector with string for word name", "other type")),
    };
    
    // 定義本体のトークンを取得
    let tokens = if let ValueType::DefinitionBody(t) = body_val.val_type {
        t
    } else {
        return Err(AjisaiError::type_error("definition block for word body", "other type"));
    };

    interp.output_buffer.push_str(&format!("[DEBUG] Defining word '{}' with {} tokens in body\n", name, tokens.len()));
    
    // トークン列を解析して実行ラインのリストを作成
    let lines = parse_definition_body(interp, &tokens)?;
    interp.output_buffer.push_str(&format!("[DEBUG] Parsed {} execution lines for word '{}'\n", lines.len(), name));
        
    interp.dictionary.insert(name.clone(), WordDefinition {
        lines,
        is_builtin: false,
        description: None,
    });
    
    interp.output_buffer.push_str(&format!("Defined word: {}\n", name));
    Ok(())
}

fn parse_definition_body(interp: &mut Interpreter, tokens: &[Token]) -> Result<Vec<ExecutionLine>> {
    let mut lines = Vec::new();
    let mut current_pos = 0;
    
    // 外側の定義ブロックの中身を、内側の : ... ; ごとに分割していく
    while current_pos < tokens.len() {
        if let Some(start_pos) = tokens[current_pos..].iter().position(|t| matches!(t, Token::DefBlockStart)) {
            let block_start = current_pos + start_pos;
            
            // 対応する ; を探す
            let mut depth = 1;
            let mut end_pos_opt = None;
            for (i, token) in tokens.iter().enumerate().skip(block_start + 1) {
                match token {
                    Token::DefBlockStart => depth += 1,
                    Token::DefBlockEnd => {
                        depth -= 1;
                        if depth == 0 {
                            end_pos_opt = Some(i);
                            break;
                        }
                    },
                    _ => {}
                }
            }

            if let Some(block_end) = end_pos_opt {
                // : と ; の間のトークンを取得
                let line_tokens = &tokens[block_start + 1 .. block_end];
                
                // 修飾子をパース
                let mut modifier_pos = block_end + 1;
                let mut repeat_count = 1;
                let mut delay_ms = 0;
                while modifier_pos < tokens.len() {
                    if let Token::Modifier(m_str) = &tokens[modifier_pos] {
                        interp.output_buffer.push_str(&format!("[DEBUG] Found modifier: {}\n", m_str));
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
                        modifier_pos += 1;
                    } else {
                        break;
                    }
                }

                // ガード($)で条件部と処理部を分離
                let (condition_tokens, body_tokens) = 
                    if let Some(guard_pos) = line_tokens.iter().position(|t| matches!(t, Token::GuardSeparator)) {
                        (line_tokens[..guard_pos].to_vec(), line_tokens[guard_pos+1..].to_vec())
                    } else {
                        // ガードがなければ、全てが処理部（デフォルト行）
                        (Vec::new(), line_tokens.to_vec())
                    };
                
                lines.push(ExecutionLine { condition_tokens, body_tokens, repeat_count, delay_ms });
                current_pos = modifier_pos;
            } else {
                return Err(AjisaiError::from("Mismatched : and ; in definition body"));
            }
        } else {
            // : が見つからなければ終了
            break;
        }
    }
    
    // デフォルト行（条件なし）の存在チェック
    if !lines.iter().any(|line| line.condition_tokens.is_empty()) {
        return Err(AjisaiError::from("Custom word definition must have at least one default line (without a $ guard)"));
    }
    
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
