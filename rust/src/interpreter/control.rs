// rust/src/interpreter/control.rs (完全修正版)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType, Value};
use num_traits::Zero;

pub fn op_if_select(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 3 { return Err(AjisaiError::WorkspaceUnderflow); }
    let false_action = interp.workspace.pop().unwrap();
    let true_action = interp.workspace.pop().unwrap();
    let condition = interp.workspace.pop().unwrap();
    
    let selected_action = if is_truthy(&condition) { true_action } else { false_action };
    
    match selected_action.val_type {
        ValueType::Quotation(tokens) => {
            interp.execute_tokens(&tokens)
        },
        _ => {
             Err(AjisaiError::type_error("quotation", "other type"))
        }
    }
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
        ValueType::Quotation(t) => !t.is_empty(),
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
        ValueType::Quotation(tokens) => tokens,
        _ => return Err(AjisaiError::from("DEF requires a quotation for its body")),
    };

    if let Some(existing) = interp.dictionary.get(&name) {
        if existing.is_builtin { return Err(AjisaiError::from(format!("Cannot redefine builtin word: {}", name))); }
    }
    
    interp.dictionary.insert(name.clone(), crate::interpreter::WordDefinition {
        tokens,
        is_builtin: false,
        description: None,
        category: None,
        repeat_count: 1,
    });
    interp.output_buffer.push_str(&format!("Defined word: {}\n", name));

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
    
    if interp.dictionary.remove(&name).is_some() {
        interp.output_buffer.push_str(&format!("Deleted word: {}\n", name));
    }
    Ok(())
}
