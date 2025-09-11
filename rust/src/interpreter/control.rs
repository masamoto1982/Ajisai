// rust/src/interpreter/control.rs (ビルドエラー完全修正版)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType, Token, Value, BracketType};
use std::collections::HashSet;
use num_traits::Zero;

pub fn op_if_select(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 3 { return Err(AjisaiError::WorkspaceUnderflow); }
    let false_action = interp.workspace.pop().unwrap();
    let true_action = interp.workspace.pop().unwrap();
    let condition = interp.workspace.pop().unwrap();
    
    let selected_action = if is_truthy(&condition) { true_action } else { false_action };
    
    match selected_action.val_type {
        ValueType::Vector(action_values, _) => {
            let tokens = vector_to_tokens(action_values)?;
            interp.execute_tokens(&tokens)
        },
        _ => {
            interp.workspace.push(selected_action);
            Ok(())
        }
    }
}

fn vector_to_tokens(values: Vec<Value>) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    for value in values {
        match value.val_type {
            ValueType::Vector(inner_values, bracket_type) => {
                tokens.push(Token::VectorStart(bracket_type.clone()));
                tokens.extend(vector_to_tokens(inner_values)?);
                tokens.push(Token::VectorEnd(bracket_type));
            },
            _ => tokens.push(value_to_token(value)?),
        }
    }
    Ok(tokens)
}

fn is_truthy(value: &Value) -> bool {
    match &value.val_type {
        ValueType::Boolean(b) => *b,
        ValueType::Nil => false,
        ValueType::Number(n) => !n.numerator.is_zero(),
        ValueType::String(s) => !s.is_empty(),
        ValueType::Vector(v, _) => {
            if v.len() == 1 { is_truthy(&v[0]) } else { !v.is_empty() }
        },
        ValueType::Symbol(_) => true,
    }
}

fn value_to_token(value: Value) -> Result<Token> {
    match value.val_type {
        ValueType::Number(_) => Ok(Token::Number(format!("{}", value))),
        ValueType::String(s) => Ok(Token::String(s)),
        ValueType::Boolean(b) => Ok(Token::Boolean(b)),
        ValueType::Symbol(s) => Ok(Token::Symbol(s)),
        ValueType::Nil => Ok(Token::Nil),
        ValueType::Vector(_, _) => Err(AjisaiError::from("Cannot convert nested vector to single token")),
    }
}

pub fn op_def(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 { return Err(AjisaiError::from("DEF requires vector and name")); }
    let name_val = interp.workspace.pop().unwrap();
    let code_val = interp.workspace.pop().unwrap();
    
    let name = match name_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => match &v[0].val_type {
            ValueType::String(s) => s.to_uppercase(),
            _ => return Err(AjisaiError::from("DEF name must be a string"))
        },
        _ => return Err(AjisaiError::from("DEF name must be a single-element vector containing a string")),
    };

    let tokens = match code_val.val_type {
        ValueType::Vector(v, _) => vector_to_tokens(v)?,
        _ => return Err(AjisaiError::from("DEF requires vector for its body")),
    };

    if let Some(existing) = interp.dictionary.get(&name) {
        if existing.is_builtin { return Err(AjisaiError::from(format!("Cannot redefine builtin word: {}", name))); }
    }
    
    interp.dictionary.insert(name, crate::interpreter::WordDefinition {
        tokens,
        is_builtin: false,
        description: None,
        category: None,
        repeat_count: 1,
    });

    Ok(())
}

pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop().ok_or(AjisaiError::WorkspaceUnderflow)?;
    let name = match val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => match &v[0].val_type {
            ValueType::String(s) => s.to_uppercase(),
            _ => return Err(AjisaiError::type_error("string", "other type"))
        },
        _ => return Err(AjisaiError::type_error("single-element vector with string", "other type")),
    };

    if interp.dictionary.get(&name).map_or(false, |d| d.is_builtin) {
        return Err(AjisaiError::from(format!("Cannot delete builtin word: {}", name)));
    }
    if interp.dependencies.get(&name).map_or(false, |deps| !deps.is_empty()) {
        let dependents = interp.dependencies.get(&name).unwrap().iter().cloned().collect();
        return Err(AjisaiError::ProtectedWord { name, dependents });
    }
    
    interp.dictionary.remove(&name);
    Ok(())
}
