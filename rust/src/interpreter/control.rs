// rust/src/interpreter/control.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}, WordDefinition};
use crate::types::{Token, ExecutionLine};

pub fn op_def(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 { return Err(AjisaiError::from("DEF requires a definition block and a name")); }

    let name_val = interp.workspace.pop().unwrap();
    let body_val = interp.workspace.pop().unwrap();

    let name = if let Ok(s) = get_string_from_value(&name_val) { s.to_uppercase() } 
               else { return Err(AjisaiError::type_error("string for word name", "other type")); };
    
    let tokens = if let Ok(t) = get_tokens_from_value(&body_val) { t }
                 else { return Err(AjisaiError::type_error("definition block for word body", "other type")); };

    let lines = parse_definition_body(&tokens)?;

    interp.dictionary.insert(name.clone(), WordDefinition {
        lines,
        is_builtin: false,
        description: None,
    });
    interp.output_buffer.push_str(&format!("Defined word: {}\n", name));
    Ok(())
}

fn get_string_from_value(value: &crate::types::Value) -> Result<String> {
    match &value.val_type {
        crate::types::ValueType::Vector(v, _) if v.len() == 1 => {
             if let crate::types::ValueType::String(s) = &v[0].val_type { Ok(s.clone()) }
             else { Err(AjisaiError::type_error("string", "other type")) }
        },
        _ => Err(AjisaiError::type_error("single-element vector with string", "other type")),
    }
}

fn get_tokens_from_value(value: &crate::types::Value) -> Result<Vec<Token>> {
    match &value.val_type {
        crate::types::ValueType::Vector(v, _) if v.len() == 1 => {
             if let crate::types::ValueType::Symbol(s) = &v[0].val_type {
                 // This is a placeholder. We need a new ValueType for definition blocks.
                 // For now, we'll assume the symbol contains the raw tokens.
                 // This needs a proper implementation.
                 crate::tokenizer::tokenize_with_custom_words(s, &std::collections::HashSet::new())
                    .map_err(AjisaiError::from)
             }
             else { Err(AjisaiError::type_error("definition block", "other type")) }
        },
        _ => Err(AjisaiError::type_error("single-element vector with definition block", "other type")),
    }
}

fn parse_definition_body(tokens: &[Token]) -> Result<Vec<ExecutionLine>> {
    tokens.split(|tok| matches!(tok, Token::LineBreak))
        .filter(|line| !line.is_empty())
        .map(|line_tokens| {
            let mut repeat_count = 1;
            let mut delay_ms = 0;
            let mut body_end = line_tokens.len();

            // Parse modifiers from the end
            let mut temp_end = line_tokens.len();
            while temp_end > 0 {
                if let Token::Modifier(m_str) = &line_tokens[temp_end - 1] {
                    if m_str.ends_with('x') {
                        repeat_count = m_str[..m_str.len()-1].parse().unwrap_or(1);
                    } else if m_str.ends_with("ms") {
                        delay_ms = m_str[..m_str.len()-2].parse().unwrap_or(0);
                    } else if m_str.ends_with('s') {
                        delay_ms = m_str[..m_str.len()-1].parse::<u64>().unwrap_or(0) * 1000;
                    }
                    temp_end -= 1;
                } else {
                    break;
                }
            }
            body_end = temp_end;

            let main_tokens = &line_tokens[..body_end];
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
    let name = get_string_from_value(&val)?;
    if interp.dictionary.remove(&name.to_uppercase()).is_some() {
        interp.output_buffer.push_str(&format!("Deleted word: {}\n", name));
    }
    Ok(())
}
