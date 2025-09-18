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

    let lines = parse_definition_body(&tokens)?;

    interp.dictionary.insert(name.clone(), WordDefinition {
        lines,
        is_builtin: false,
        description: None,
    });
    interp.output_buffer.push_str(&format!("Defined word: {}\n", name));
    Ok(())
}

fn parse_definition_body(tokens: &[Token]) -> Result<Vec<ExecutionLine>> {
    tokens.split(|tok| matches!(tok, Token::LineBreak))
        .filter(|line| !line.is_empty())
        .map(|line_tokens| {
            let mut repeat_count = 1;
            let mut delay_ms = 0;
            
            let mut modifier_tokens = 0;
            for token in line_tokens.iter().rev() {
                if let Token::Modifier(m_str) = token {
                    if m_str.ends_with('x') {
                        repeat_count = m_str[..m_str.len()-1].parse().unwrap_or(1);
                    } else if m_str.ends_with("ms") {
                        delay_ms = m_str[..m_str.len()-2].parse().unwrap_or(0);
                    } else if m_str.ends_with('s') {
                        delay_ms = m_str[..m_str.len()-1].parse::<u64>().unwrap_or(0) * 1000;
                    }
                    modifier_tokens += 1;
                } else {
                    break;
                }
            }
            let main_tokens = &line_tokens[..line_tokens.len() - modifier_tokens];

            let (condition_tokens, body_tokens) = 
                if let Some(pos) = main_tokens.iter().position(|t| matches!(t, Token::GuardSeparator)) {
                    (main_tokens[..pos].to_vec(), main_tokens[pos+1..].to_vec())
                } else {
                    (Vec::new(), main_tokens.to_vec())
                };

            Ok(ExecutionLine { condition_tokens, body_tokens, repeat_count, delay_ms })
        })
        .collect()
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
