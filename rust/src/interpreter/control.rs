// rust/src/interpreter/control.rs (完全版)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType};

// 雇用司書 - 新しい部署を設立（DEF相当）
pub fn op_hire(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::from("雇用 requires vector and name"));
    }

    let name_val = interp.workspace.pop().unwrap();
    let code_val = interp.workspace.pop().unwrap();

    let name = match name_val.val_type {
        ValueType::String(s) => s.to_uppercase(),
        _ => return Err(AjisaiError::from("雇用 requires string name")),
    };

    let tokens = match code_val.val_type {
        ValueType::Vector(v) => {
            interp.vector_to_tokens(v)?
        },
        _ => return Err(AjisaiError::from("雇用 requires vector")),
    };

    if let Some(existing) = interp.dictionary.get(&name) {
        if existing.is_builtin {
            return Err(AjisaiError::from(format!("Cannot redefine builtin librarian: {}", name)));
        }
    }

    if interp.dictionary.contains_key(&name) {
        if let Some(dependents) = interp.dependencies.get(&name) {
            if !dependents.is_empty() {
                let dependent_list: Vec<String> = dependents.iter().cloned().collect();
                return Err(AjisaiError::ProtectedWord { 
                    name: name.clone(), 
                    dependents: dependent_list 
                });
            }
        }
    }

    if let Some(old_deps) = interp.get_word_dependencies(&name) {
        for dep in old_deps {
            if let Some(reverse_deps) = interp.dependencies.get_mut(&dep) {
                reverse_deps.remove(&name);
            }
        }
    }

    for token in &tokens {
        if let crate::types::Token::Symbol(sym) = token {
            if interp.dictionary.contains_key(sym) && !interp.is_builtin_word(sym) {
                interp.dependencies.entry(sym.clone())
                    .or_insert_with(std::collections::HashSet::new)
                    .insert(name.clone());
            }
        }
    }

    interp.dictionary.insert(name.clone(), crate::interpreter::WordDefinition {
        tokens,
        is_builtin: false,
        description: None,
        category: None,
    });

    interp.append_output(&format!("Hired librarian: {}\n", name));
    Ok(())
}

// 解雇司書 - 部署を解散（DEL相当）
pub fn op_fire(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match val.val_type {
        ValueType::String(name) => {
            let name = name.to_uppercase();
            
            if let Some(def) = interp.dictionary.get(&name) {
                if def.is_builtin {
                    return Err(AjisaiError::from(format!("Cannot fire builtin librarian: {}", name)));
                }
            }
            
            if let Some(dependents) = interp.dependencies.get(&name) {
                if !dependents.is_empty() {
                    let dependent_list: Vec<String> = dependents.iter().cloned().collect();
                    return Err(AjisaiError::ProtectedWord { 
                        name: name.clone(), 
                        dependents: dependent_list 
                    });
                }
            }
            
            interp.dictionary.remove(&name);
            interp.dependencies.remove(&name);
            
            for (_, deps) in interp.dependencies.iter_mut() {
                deps.remove(&name);
            }
            
            interp.append_output(&format!("Fired librarian: {}\n", name));
            Ok(())
        },
        _ => Err(AjisaiError::type_error("string", "other type")),
    }
}

// 交代司書 - 司書交代（GOTO相当） 
pub fn op_handover(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 3 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let else_target = interp.workspace.pop().unwrap();
    let if_target = interp.workspace.pop().unwrap();
    let condition = interp.workspace.pop().unwrap();
    
    let should_jump = match condition.val_type {
        ValueType::Boolean(b) => b,
        ValueType::Nil => false,
        _ => true,
    };
    
    let target = if should_jump { if_target } else { else_target };
    
    match target.val_type {
        ValueType::String(librarian_name) => {
            // 同一ワード内制限でワード実行
            let current_word = interp.call_stack.last().cloned();
            interp.execute_word_leap(&librarian_name, current_word.as_deref())?;
            Ok(())
        },
        ValueType::Vector(code_vec) => {
            // 直接コード実行
            let tokens = interp.vector_to_tokens(code_vec)?;
            interp.execute_tokens(&tokens)?;
            Ok(())
        },
        _ => Err(AjisaiError::type_error("string or vector", "other type")),
    }
}
