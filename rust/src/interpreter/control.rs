use std::collections::HashSet;
use crate::interpreter::{Interpreter, WordDefinition, error::{AjisaiError, Result}};
use crate::types::{ValueType, Token};

pub fn op_def(interp: &mut Interpreter, _description: Option<String>) -> Result<()> {
    // 内部処理用のDEF（ユーザーは直接使わない）
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let name_val = interp.stack.pop().unwrap();
    let body_val = interp.stack.pop().unwrap();

    match (&name_val.val_type, &body_val.val_type) {
        (ValueType::String(name), ValueType::Quotation(body_tokens)) => {
            let name = name.to_uppercase();

            // ビルトインワードは再定義不可
            if let Some(existing) = interp.dictionary.get(&name) {
                if existing.is_builtin {
                    return Err(AjisaiError::from(format!("Cannot redefine builtin word: {}", name)));
                }
            }

            // 依存関係チェック
            if interp.dictionary.contains_key(&name) {
                if let Some(dependents) = interp.dependencies.get(&name) {
                    if !dependents.is_empty() {
                        let dependent_list: Vec<String> = dependents.iter().cloned().collect();
                        return Err(AjisaiError::ProtectedWord {
                            name: name.clone(),
                            dependents: dependent_list,
                        });
                    }
                }
            }

            // 新しい依存関係を収集
            let mut new_dependencies = HashSet::new();
            for token in body_tokens {
                if let Token::Symbol(s) = token {
                    if interp.dictionary.contains_key(s) {
                        new_dependencies.insert(s.clone());
                    }
                }
            }

            // 依存関係を更新
            for dep_name in &new_dependencies {
                interp.dependencies
                    .entry(dep_name.clone())
                    .or_insert_with(HashSet::new)
                    .insert(name.clone());
            }

            // 新しいワードを登録
            interp.dictionary.insert(name.clone(), WordDefinition {
                tokens: body_tokens.clone(),
                is_builtin: false,
                description: None,
            });

            Ok(())
        }
        _ => Err(AjisaiError::type_error("quotation and string", "other types")),
    }
}

pub fn op_if(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 3 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let else_branch = interp.stack.pop().unwrap();
    let then_branch = interp.stack.pop().unwrap();
    let condition = interp.stack.pop().unwrap();

    let (then_tokens, else_tokens) = match (&then_branch.val_type, &else_branch.val_type) {
        (ValueType::Quotation(t), ValueType::Quotation(e)) => (t, e),
        _ => return Err(AjisaiError::type_error("two quotations", "other types")),
    };

    let tokens_to_execute = match condition.val_type {
        ValueType::Boolean(true) => then_tokens,
        ValueType::Boolean(false) | ValueType::Nil => else_tokens,
        _ => return Err(AjisaiError::type_error("boolean or nil", "other type")),
    };
    
    interp.execute_tokens_with_context(tokens_to_execute)
}

pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    match val.val_type {
        ValueType::String(name) => {
            let name = name.to_uppercase();
            
            // ビルトインワードは削除不可
            if let Some(def) = interp.dictionary.get(&name) {
                if def.is_builtin {
                    return Err(AjisaiError::from(format!("Cannot delete builtin word: {}", name)));
                }
            }
            
            // 依存関係チェック
            if let Some(dependents) = interp.dependencies.get(&name) {
                if !dependents.is_empty() {
                    let dependent_list: Vec<String> = dependents.iter().cloned().collect();
                    return Err(AjisaiError::ProtectedWord {
                        name: name.clone(),
                        dependents: dependent_list,
                    });
                }
            }
            
            interp.dictionary.remove(&name);
            interp.dependencies.remove(&name);
            Ok(())
        },
        _ => Err(AjisaiError::type_error("string", "other type")),
    }
}

pub fn op_call(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    match val.val_type {
        ValueType::Quotation(tokens) => {
            interp.execute_tokens_with_context(&tokens)
        },
        _ => Err(AjisaiError::type_error("quotation", "other type")),
    }
}
