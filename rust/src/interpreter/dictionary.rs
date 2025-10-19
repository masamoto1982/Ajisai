// rust/src/interpreter/dictionary.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Token, ExecutionLine, ValueType, WordDefinition};
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

    op_def_inner(interp, &name_str, &tokens, None)
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

    let lines = parse_definition_body(tokens)?;
    
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

fn parse_definition_body(tokens: &[Token]) -> Result<Vec<ExecutionLine>> {
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
    Ok(ExecutionLine {
        body_tokens: tokens.to_vec(),
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
                format!("'{}' DEF", name_str)
            } else {
                if let Some(desc) = &def.description {
                    format!("{}\n'{}' '{}' DEF", definition, name_str, desc)
                } else {
                    format!("{}\n'{}' DEF", definition, name_str)
                }
            };
            interp.definition_to_load = Some(full_definition);
        }
        Ok(())
    } else {
        Err(AjisaiError::UnknownWord(name_str))
    }
}

pub fn parse_multiple_word_definitions(interp: &mut Interpreter, input: &str) -> Result<()> {
    let custom_word_names: HashSet<String> = interp.dictionary.iter()
        .filter(|(_, def)| !def.is_builtin)
        .map(|(name, _)| name.clone())
        .collect();
    
    let all_tokens = crate::tokenizer::tokenize_with_custom_words(input, &custom_word_names)
        .map_err(|e| AjisaiError::from(format!("Tokenization error: {}", e)))?;
    
    let mut current_def_block = Vec::new();
    let mut i = 0;
    
    while i < all_tokens.len() {
        match &all_tokens[i] {
            Token::DefBlockStart => {
                let (body_tokens, consumed) = interp.collect_def_block(&all_tokens, i)?;
                current_def_block = body_tokens;
                i += consumed;
            },
            Token::String(name) => {
                if !current_def_block.is_empty() {
                    if i + 1 < all_tokens.len() && all_tokens[i + 1] == Token::Symbol("DEF".to_string()) {
                        op_def_inner(interp, name, &current_def_block, None)?;
                        current_def_block.clear();
                        i += 2;
                    } else {
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            },
            _ => {
                i += 1;
            }
        }
    }
    
    Ok(())
}
