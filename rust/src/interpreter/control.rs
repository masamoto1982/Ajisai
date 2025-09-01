// rust/src/interpreter/control.rs (BracketType対応完全版)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType, Token, BracketType};

// EVAL - ベクトル内のコードを実行する
pub fn op_eval(interp: &mut Interpreter) -> Result<()> {
    let code_val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match code_val.val_type {
        ValueType::Vector(code_vec, _) => {
            let tokens = interp.vector_to_tokens(code_vec)?;
            interp.execute_tokens(&tokens)?;
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// DEF - 新しいワードを定義する
pub fn op_def(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::from("DEF requires vector and name"));
    }

    let name_val = interp.workspace.pop().unwrap();
    let code_val = interp.workspace.pop().unwrap();

    let name = match name_val.val_type {
        ValueType::String(s) => s.to_uppercase(),
        _ => return Err(AjisaiError::from("DEF requires string name")),
    };

    let tokens = match code_val.val_type {
        ValueType::Vector(v, bracket_type) => {
            // VectorStart + 内容 + VectorEnd の形でトークンを構築
            let mut tokens = vec![Token::VectorStart(bracket_type.clone())];
            for value in v {
                tokens.push(interp.value_to_token(value)?);
            }
            tokens.push(Token::VectorEnd(bracket_type));
            tokens
        },
        _ => return Err(AjisaiError::from("DEF requires vector")),
    };

    // 既存のワードチェック
    if let Some(existing) = interp.dictionary.get(&name) {
        if existing.is_builtin {
            return Err(AjisaiError::from(format!("Cannot redefine builtin word: {}", name)));
        }
    }

    // 依存関係チェック（保護されたワードの確認）
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

    // 古い依存関係をクリア
    if let Some(old_deps) = interp.get_word_dependencies(&name) {
        for dep in old_deps {
            if let Some(reverse_deps) = interp.dependencies.get_mut(&dep) {
                reverse_deps.remove(&name);
            }
        }
    }

    // 新しい依存関係を登録
    for token in &tokens {
        if let Token::Symbol(sym) = token {
            if interp.dictionary.contains_key(sym) && !interp.is_builtin_word(sym) {
                interp.dependencies.entry(sym.clone())
                    .or_insert_with(std::collections::HashSet::new)
                    .insert(name.clone());
            }
        }
    }

    // ワード定義を登録
    interp.dictionary.insert(name.clone(), crate::interpreter::WordDefinition {
        tokens,
        is_builtin: false,
        description: None,
        category: None,
    });

    interp.append_output(&format!("Defined word: {}\n", name));
    Ok(())
}

// DEL - ワードを削除する
pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match val.val_type {
        ValueType::String(name) => {
            let name = name.to_uppercase();
            
            // 組み込みワードの保護
            if let Some(def) = interp.dictionary.get(&name) {
                if def.is_builtin {
                    return Err(AjisaiError::from(format!("Cannot delete builtin word: {}", name)));
                }
            } else {
                return Err(AjisaiError::from(format!("Word '{}' not found", name)));
            }
            
            // 依存関係チェック（他のワードから使用されていないか確認）
            if let Some(dependents) = interp.dependencies.get(&name) {
                if !dependents.is_empty() {
                    let dependent_list: Vec<String> = dependents.iter().cloned().collect();
                    return Err(AjisaiError::ProtectedWord { 
                        name: name.clone(), 
                        dependents: dependent_list 
                    });
                }
            }
            
            // ワードを辞書から削除
            interp.dictionary.remove(&name);
            
            // 依存関係をクリア
            interp.dependencies.remove(&name);
            
            // 他のワードの依存関係からも削除
            for (_, deps) in interp.dependencies.iter_mut() {
                deps.remove(&name);
            }
            
            interp.append_output(&format!("Deleted word: {}\n", name));
            Ok(())
        },
        _ => Err(AjisaiError::type_error("string", "other type")),
    }
}

// JUMP - 条件付き分岐（GOTO相当） 
pub fn op_jump(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 3 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let else_target = interp.workspace.pop().unwrap();
    let if_target = interp.workspace.pop().unwrap();
    let condition = interp.workspace.pop().unwrap();
    
    // 条件評価
    let should_jump = match condition.val_type {
        ValueType::Boolean(b) => b,
        ValueType::Nil => false,
        _ => true,  // nil以外の値は真として扱う
    };
    
    // 条件に応じてターゲットを選択
    let target = if should_jump { if_target } else { else_target };
    
    match target.val_type {
        ValueType::String(word_name) => {
            // 同一ワード内制限でワード実行
            let current_word = interp.call_stack.last().cloned();
            interp.execute_word_leap(&word_name, current_word.as_deref())?;
            Ok(())
        },
        ValueType::Vector(code_vec, _) => {
            // 直接コードベクトルを実行
            let tokens = interp.vector_to_tokens(code_vec)?;
            interp.execute_tokens(&tokens)?;
            Ok(())
        },
        _ => Err(AjisaiError::type_error("string or vector", "other type")),
    }
}

// 条件付き実行 - 条件が真の場合のみ実行
pub fn op_when(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let action = interp.workspace.pop().unwrap();
    let condition = interp.workspace.pop().unwrap();
    
    // 条件評価
    let should_execute = match condition.val_type {
        ValueType::Boolean(b) => b,
        ValueType::Nil => false,
        _ => true,
    };
    
    if should_execute {
        match action.val_type {
            ValueType::String(word_name) => {
                // ワード名を実行
                interp.execute_word(&word_name)?;
            },
            ValueType::Vector(code_vec, _) => {
                // コードベクトルを直接実行
                let tokens = interp.vector_to_tokens(code_vec)?;
                interp.execute_tokens(&tokens)?;
            },
            _ => {
                // その他の値はそのままワークスペースに戻す
                interp.workspace.push(action);
            }
        }
    }
    
    Ok(())
}

// デフォルト値設定 - nil の場合にデフォルト値を使用
pub fn op_default(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let default_val = interp.workspace.pop().unwrap();
    let val = interp.workspace.pop().unwrap();
    
    if matches!(val.val_type, ValueType::Nil) {
        interp.workspace.push(default_val);
    } else {
        interp.workspace.push(val);
    }
    Ok(())
}

// NIL判定
pub fn op_nil_check(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    let is_nil = matches!(val.val_type, ValueType::Nil);
    interp.workspace.push(crate::types::Value { 
        val_type: ValueType::Boolean(is_nil) 
    });
    Ok(())
}

// NOT-NIL判定（KNOWN?と同等）
pub fn op_not_nil_check(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    let is_not_nil = !matches!(val.val_type, ValueType::Nil);
    interp.workspace.push(crate::types::Value { 
        val_type: ValueType::Boolean(is_not_nil) 
    });
    Ok(())
}
