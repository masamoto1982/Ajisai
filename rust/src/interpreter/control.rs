// rust/src/interpreter/control.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Token, ExecutionLine, ValueType, WordDefinition, Value, BracketType};
use std::collections::HashSet;
use num_traits::{ToPrimitive, One};

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
        for token in line.condition_tokens.iter().chain(line.body_tokens.iter()) {
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
    let guard_position = tokens.iter().position(|t| matches!(t, Token::GuardSeparator));
    
    let (condition_tokens, body_tokens) = if let Some(guard_pos) = guard_position {
        (tokens[..guard_pos].to_vec(), tokens[guard_pos+1..].to_vec())
    } else {
        (Vec::new(), tokens.to_vec())
    };
    
    Ok(ExecutionLine {
        condition_tokens,
        body_tokens,
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
    
    let mut definitions = Vec::new();
    let mut i = 0;
    let mut current_body_start = 0;
    
    while i < all_tokens.len() {
        if let Token::Symbol(s) = &all_tokens[i] {
            if s.to_uppercase() == "DEF" {
                if i == 0 || current_body_start >= i - 1 {
                    return Err(AjisaiError::from("DEF requires a body and name"));
                }
                
                let name = match &all_tokens[i - 1] {
                    Token::String(s) => s.clone(),
                    _ => return Err(AjisaiError::from("DEF requires a string name")),
                };
                
                let mut description = None;
                let mut next_start = i + 1;
                
                if i + 1 < all_tokens.len() {
                    if let Token::String(desc) = &all_tokens[i + 1] {
                        description = Some(desc.clone());
                        next_start = i + 2;
                    }
                }
                
                let body_end = i - 1;
                let body_tokens: Vec<Token> = all_tokens[current_body_start..body_end].to_vec();
                
                definitions.push((name, body_tokens, description));
                
                while next_start < all_tokens.len() && matches!(all_tokens[next_start], Token::LineBreak) {
                    next_start += 1;
                }
                current_body_start = next_start;
                i = next_start;
                continue;
            }
        }
        i += 1;
    }
    
    if definitions.is_empty() {
        return Err(AjisaiError::from("No DEF keyword found"));
    }
    
    for (name, body_tokens, description) in definitions {
        op_def_inner(interp, &name, &body_tokens, description)?;
    }
    
    Ok(())
}

pub(crate) fn execute_times(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::from("TIMES requires word name and count. Usage: 'WORD' [ n ] TIMES"));
    }

    let count_val = interp.stack.pop().unwrap();
    let name_val = interp.stack.pop().unwrap();

    let count = match &count_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) if n.denominator == num_bigint::BigInt::one() => {
                    n.numerator.to_i64().ok_or_else(|| AjisaiError::from("Count too large"))?
                },
                _ => return Err(AjisaiError::type_error("integer", "other type")),
            }
        },
        _ => return Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    };

    let word_name = match &name_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::String(s) => s.clone(),
                _ => return Err(AjisaiError::type_error("string", "other type")),
            }
        },
        _ => return Err(AjisaiError::type_error("single-element vector with string", "other type")),
    };

    let upper_name = word_name.to_uppercase();
    
    if let Some(def) = interp.dictionary.get(&upper_name) {
        if def.is_builtin {
            return Err(AjisaiError::from("TIMES can only be used with custom words"));
        }
    } else {
        return Err(AjisaiError::UnknownWord(word_name));
    }

    for _ in 0..count {
        interp.execute_word_sync(&upper_name)?;
    }

    Ok(())
}

pub(crate) fn execute_wait(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::from("WAIT requires word name and delay. Usage: 'WORD' [ ms ] WAIT"));
    }

    let delay_val = interp.stack.pop().unwrap();
    let name_val = interp.stack.pop().unwrap();

    let delay_ms = match &delay_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) if n.denominator == num_bigint::BigInt::one() => {
                    n.numerator.to_u64().ok_or_else(|| AjisaiError::from("Delay too large"))?
                },
                _ => return Err(AjisaiError::type_error("integer", "other type")),
            }
        },
        _ => return Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    };

    let word_name = match &name_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::String(s) => s.clone(),
                _ => return Err(AjisaiError::type_error("string", "other type")),
            }
        },
        _ => return Err(AjisaiError::type_error("single-element vector with string", "other type")),
    };

    let upper_name = word_name.to_uppercase();
    
    if let Some(def) = interp.dictionary.get(&upper_name) {
        if def.is_builtin {
            return Err(AjisaiError::from("WAIT can only be used with custom words"));
        }
    } else {
        return Err(AjisaiError::UnknownWord(word_name));
    }

    interp.output_buffer.push_str(&format!("[DEBUG] Would wait {}ms before executing '{}'\n", delay_ms, word_name));
    
    interp.execute_word_sync(&upper_name)?;

    Ok(())
}

fn value_to_token(value: &Value) -> Result<Token> {
    match &value.val_type {
        ValueType::Number(f) => Ok(Token::Number(if f.denominator == One::one() {
            f.numerator.to_string()
        } else {
            format!("{}/{}", f.numerator, f.denominator)
        })),
        ValueType::String(s) => Ok(Token::String(s.clone())),
        ValueType::Boolean(b) => Ok(Token::Boolean(*b)),
        ValueType::Symbol(s) => Ok(Token::Symbol(s.clone())),
        ValueType::Nil => Ok(Token::Nil),
        ValueType::Vector(_, _) => Err(AjisaiError::from("Cannot convert nested vector directly to a single token for EVAL")),
        _ => Err(AjisaiError::from("Cannot convert this value to a token for EVAL")),
    }
}

fn values_to_tokens_recursive(values: &[Value], tokens: &mut Vec<Token>) -> Result<()> {
    for value in values {
        match &value.val_type {
            ValueType::Vector(inner_values, bracket_type) => {
                tokens.push(Token::VectorStart(bracket_type.clone()));
                values_to_tokens_recursive(inner_values, tokens)?;
                tokens.push(Token::VectorEnd(bracket_type.clone()));
            }
            _ => {
                tokens.push(value_to_token(value)?);
            }
        }
    }
    Ok(())
}

pub fn op_eval(interp: &mut Interpreter) -> Result<()> {
    let code_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let values = match code_val.val_type {
        ValueType::Vector(v, _) => v,
        _ => return Err(AjisaiError::type_error("vector", "other type")),
    };

    let mut tokens = Vec::new();
    values_to_tokens_recursive(&values, &mut tokens)?;
    
    interp.execute_tokens_sync(&tokens)
}
