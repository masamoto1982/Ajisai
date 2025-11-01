// rust/src/interpreter/dictionary.rs

use crate::interpreter::{Interpreter, WordDefinition};
use crate::interpreter::error::{AjisaiError, Result};
use crate::types::{Token, ValueType, ExecutionLine, Value};
use std::collections::HashSet;
use std::fmt::Write;

/// Value を Token に変換する
fn value_to_token(val: &Value) -> Result<Token> {
    match &val.val_type {
        ValueType::Number(_) => {
            Ok(Token::Number(val.to_string()))
        },
        ValueType::String(s) => Ok(Token::String(s.clone())),
        ValueType::Boolean(b) => Ok(Token::Boolean(*b)),
        ValueType::Symbol(s) => Ok(Token::Symbol(s.clone())),
        ValueType::Nil => Ok(Token::Nil),
        ValueType::GuardSeparator => Ok(Token::GuardSeparator),  // ★ 追加
        ValueType::LineBreak => Ok(Token::LineBreak),            // ★ 追加
        ValueType::Vector(_, _) => Err(AjisaiError::from("Cannot convert nested vector root to single token")),
    }
}

/// Vec<Value> を Vec<Token> に再帰的に変換する
fn values_to_tokens(values: &[Value]) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    for val in values {
        match &val.val_type {
            ValueType::Vector(v, bt) => {
                tokens.push(Token::VectorStart(bt.clone()));
                tokens.extend(values_to_tokens(v)?);
                tokens.push(Token::VectorEnd(bt.clone()));
            },
            _ => {
                tokens.push(value_to_token(val)?);
            }
        }
    }
    Ok(tokens)
}

pub fn op_def(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let mut description: Option<String> = None;
    let name_str: String;

    let val1 = interp.stack.pop().unwrap();
    
    if let ValueType::String(s1) = val1.val_type {
        if let Some(val2) = interp.stack.last() {
             if let ValueType::String(s2) = &val2.val_type {
                description = Some(s1);
                name_str = s2.clone();
                interp.stack.pop();
             } else {
                name_str = s1;
             }
        } else {
             name_str = s1;
        }
    } else {
        interp.stack.push(val1);
        return Err(AjisaiError::type_error("string 'name' or 'description'", "other type"));
    }

    let def_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    
    let definition_values = match def_val.val_type {
        ValueType::Vector(vec, _) => vec,
        _ => return Err(AjisaiError::type_error("vector (quotation)", "other type")),
    };

    let tokens = values_to_tokens(&definition_values)?;
    
    op_def_inner(interp, &name_str, &tokens, description)
}

pub(crate) fn op_def_inner(interp: &mut Interpreter, name: &str, tokens: &[Token], description: Option<String>) -> Result<()> {
    let upper_name = name.to_uppercase();
    
    writeln!(interp.debug_buffer, "[DEBUG] Defining word '{}' with tokens: {:?}", upper_name, tokens).unwrap();

    if let Some(old_def) = interp.dictionary.get(&upper_name) {
        for dep_name in &old_def.dependencies {
            if let Some(dependents) = interp.dependents.get_mut(dep_name) {
                dependents.remove(&upper_name);
            }
        }
    }

    let mut lines = Vec::new();
    let mut current_line = Vec::new();
    
    for token in tokens {
        if let Token::LineBreak = token {
            if !current_line.is_empty() {
                lines.push(ExecutionLine { body_tokens: current_line });
                current_line = Vec::new();
            }
        } else {
            current_line.push(token.clone());
        }
    }
    
    if !current_line.is_empty() {
        lines.push(ExecutionLine { body_tokens: current_line });
    }
    
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

pub fn op_del(interp: &mut Interpreter) -> Result<()> {
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
        
        interp.stack.pop();
        interp.output_buffer.push_str(&format!("Deleted word: {}\n", name));
        Ok(())
    } else {
        Err(AjisaiError::UnknownWord(upper_name))
    }
}

pub fn op_lookup(interp: &mut Interpreter) -> Result<()> {
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
            let desc = def.description.as_deref().unwrap_or(""); 
            let full_definition = format!("[ {} ] '{}' '{}' DEF", definition, name_str, desc);
            interp.definition_to_load = Some(full_definition);
        }
        Ok(())
    } else {
        Err(AjisaiError::UnknownWord(name_str))
    }
}
