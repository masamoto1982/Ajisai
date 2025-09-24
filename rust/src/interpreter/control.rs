// rust/src/interpreter/control.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Token, ExecutionLine, Value, ValueType, WordDefinition};
use std::collections::HashSet;

pub fn op_def(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 { return Err(AjisaiError::from("DEF requires a definition block and a name")); }

    let name_val = interp.workspace.pop().unwrap();
    let body_val = interp.workspace.pop().unwrap();

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

    op_def_inner(interp, &tokens, &name_str)
}

pub(crate) fn op_def_inner(interp: &mut Interpreter, tokens: &[Token], name: &str) -> Result<()> {
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

    let lines = parse_definition_body(interp, tokens)?;
    
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
        description: None,
        dependencies: new_dependencies,
    };
    
    interp.dictionary.insert(upper_name.clone(), new_def);
    interp.output_buffer.push_str(&format!("Defined word: {}\n", name));
    Ok(())
}

fn parse_definition_body(_interp: &mut Interpreter, tokens: &[Token]) -> Result<Vec<ExecutionLine>> {
    let mut lines = Vec::new();
    let mut current_pos = 0;
    
    while current_pos < tokens.len() {
        if let Some(start_pos) = tokens[current_pos..].iter().position(|t| matches!(t, Token::DefBlockStart)) {
            let block_start = current_pos + start_pos;
            
            let mut depth = 1;
            let mut end_pos_opt = None;
            for (i, token) in tokens.iter().enumerate().skip(block_start + 1) {
                match token {
                    Token::DefBlockStart => depth += 1,
                    Token::DefBlockEnd => {
                        depth -= 1;
                        if depth == 0 { end_pos_opt = Some(i); break; }
                    },
                    _ => {}
                }
            }

            if let Some(block_end) = end_pos_opt {
                let line_tokens = &tokens[block_start + 1 .. block_end];
                
                let mut modifier_pos = block_end + 1;
                let mut repeat_count = 1;
                let mut delay_ms = 0;
                while modifier_pos < tokens.len() {
                    if let Token::Modifier(m_str) = &tokens[modifier_pos] {
                        if m_str.ends_with('x') { if let Ok(count) = m_str[..m_str.len()-1].parse::<i64>() { repeat_count = count; } } 
                        else if m_str.ends_with("ms") { if let Ok(ms) = m_str[..m_str.len()-2].parse::<u64>() { delay_ms = ms; } }
                        else if m_str.ends_with('s') { if let Ok(s) = m_str[..m_str.len()-1].parse::<u64>() { delay_ms = s * 1000; } }
                        modifier_pos += 1;
                    } else { break; }
                }

                let (condition_tokens, body_tokens) = 
                    if let Some(guard_pos) = line_tokens.iter().position(|t| matches!(t, Token::GuardSeparator)) {
                        (line_tokens[..guard_pos].to_vec(), line_tokens[guard_pos+1..].to_vec())
                    } else {
                        (Vec::new(), line_tokens.to_vec())
                    };
                
                lines.push(ExecutionLine { condition_tokens, body_tokens, repeat_count, delay_ms });
                current_pos = modifier_pos;
            } else {
                return Err(AjisaiError::from("Mismatched : and ; in definition body"));
            }
        } else { break; }
    }
    
    if !lines.iter().any(|line| line.condition_tokens.is_empty()) {
        return Err(AjisaiError::from("Custom word definition must have at least one default line (without a $ guard)"));
    }
    
    Ok(lines)
}

pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.last().ok_or(AjisaiError::WorkspaceUnderflow)?;
    
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
        
        interp.workspace.pop();
        interp.output_buffer.push_str(&format!("Deleted word: {}\n", name));
        Ok(())
    } else {
        Err(AjisaiError::UnknownWord(upper_name))
    }
}

pub fn op_lookup(interp: &mut Interpreter) -> Result<()> {
    let name_val = interp.workspace.pop().ok_or(AjisaiError::WorkspaceUnderflow)?;

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
    if let Some(definition) = interp.get_word_definition_tokens(&upper_name) {
        let full_definition = format!("{} '{}' DEF", definition, name_str);
        interp.definition_to_load = Some(full_definition);
        Ok(())
    } else {
        Err(AjisaiError::UnknownWord(name_str))
    }
}
