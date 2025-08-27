// rust/src/interpreter/control.rs

use crate::interpreter::{Interpreter, error::{LPLError, Result}};
use crate::types::{ValueType, Token};

// 雇用司書 - 新しい部署を設立（DEF相当）
pub fn op_hire(interp: &mut Interpreter) -> Result<()> {
    if interp.bookshelf.len() < 2 {
        return Err(LPLError::from("雇用 requires vector and name"));
    }

    let name_val = interp.bookshelf.pop().unwrap();
    let code_val = interp.bookshelf.pop().unwrap();

    let name = match name_val.val_type {
        ValueType::String(s) => s.to_uppercase(),
        _ => return Err(LPLError::from("雇用 requires string name")),
    };

    let tokens = match code_val.val_type {
        ValueType::Vector(v) => {
            interp.vector_to_tokens(v)?
        },
        _ => return Err(LPLError::from("雇用 requires vector")),
    };

    // 既存のワードチェック
    if let Some(existing) = interp.dictionary.get(&name) {
        if existing.is_builtin {
            return Err(LPLError::from(format!("Cannot redefine builtin librarian: {}", name)));
        }
    }

    // 依存関係チェック（保護されたワードの確認）
    if interp.dictionary.contains_key(&name) {
        if let Some(dependents) = interp.dependencies.get(&name) {
            if !dependents.is_empty() {
                let dependent_list: Vec<String> = dependents.iter().cloned().collect();
                return Err(LPLError::ProtectedWord { 
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

    // ワード定義を登録（新フィールド追加）
    interp.dictionary.insert(name.clone(), crate::interpreter::WordDefinition {
        tokens,
        is_builtin: false,
        description: None,  // 従来の雇用では説明なし
        category: None,
        hidden: Some(false),  // 新フィールド
        english_name: None,   // 新フィールド
        japanese_name: None,  // 新フィールド
    });

    interp.append_output(&format!("Hired librarian: {}\n", name));
    Ok(())
}

// 解雇司書 - 部署を解散（DEL相当）
pub fn op_fire(interp: &mut Interpreter) -> Result<()> {
    let val = interp.bookshelf.pop()
        .ok_or(LPLError::BookshelfUnderflow)?;
    
    match val.val_type {
        ValueType::String(name) => {
            let name = name.to_uppercase();
            
            // 組み込みワードの保護
            if let Some(def) = interp.dictionary.get(&name) {
                if def.is_builtin {
                    return Err(LPLError::from(format!("Cannot fire builtin librarian: {}", name)));
                }
            } else {
                return Err(LPLError::from(format!("Librarian '{}' not found", name)));
            }
            
            // 依存関係チェック（他のワードから使用されていないか確認）
            if let Some(dependents) = interp.dependencies.get(&name) {
                if !dependents.is_empty() {
                    let dependent_list: Vec<String> = dependents.iter().cloned().collect();
                    return Err(LPLError::ProtectedWord { 
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
            
            interp.append_output(&format!("Fired librarian: {}\n", name));
            Ok(())
        },
        _ => Err(LPLError::type_error("string", "other type")),
    }
}

// 交代司書 - 司書交代（条件付きGOTO相当） 
pub fn op_handover(interp: &mut Interpreter) -> Result<()> {
    if interp.bookshelf.len() < 3 {
        return Err(LPLError::BookshelfUnderflow);
    }
    
    let else_target = interp.bookshelf.pop().unwrap();
    let if_target = interp.bookshelf.pop().unwrap();
    let condition = interp.bookshelf.pop().unwrap();
    
    // 条件評価
    let should_jump = match condition.val_type {
        ValueType::Boolean(b) => b,
        ValueType::Nil => false,
        _ => true,  // nil以外の値は真として扱う
    };
    
    // 条件に応じてターゲットを選択
    let target = if should_jump { if_target } else { else_target };
    
    match target.val_type {
        ValueType::String(librarian_name) => {
            // 同一ワード内制限でワード実行
            let current_word = interp.call_stack.last().cloned();
            interp.execute_word_leap(&librarian_name, current_word.as_deref())?;
            Ok(())
        },
        ValueType::Vector(code_vec) => {
            // 直接コードベクトルを実行
            let tokens = interp.vector_to_tokens(code_vec)?;
            interp.execute_tokens(&tokens)?;
            Ok(())
        },
        _ => Err(LPLError::type_error("string or vector", "other type")),
    }
}

// 条件付き実行司書 - 条件が真の場合のみ実行
pub fn op_when(interp: &mut Interpreter) -> Result<()> {
    if interp.bookshelf.len() < 2 {
        return Err(LPLError::BookshelfUnderflow);
    }
    
    let action = interp.bookshelf.pop().unwrap();
    let condition = interp.bookshelf.pop().unwrap();
    
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
            ValueType::Vector(code_vec) => {
                // コードベクトルを直接実行
                let tokens = interp.vector_to_tokens(code_vec)?;
                interp.execute_tokens(&tokens)?;
            },
            _ => {
                // その他の値はそのまま書架に戻す
                interp.bookshelf.push(action);
            }
        }
    }
    
    Ok(())
}

// デフォルト値設定司書 - nil の場合にデフォルト値を使用
pub fn op_default(interp: &mut Interpreter) -> Result<()> {
    if interp.bookshelf.len() < 2 {
        return Err(LPLError::BookshelfUnderflow);
    }
    
    let default_val = interp.bookshelf.pop().unwrap();
    let val = interp.bookshelf.pop().unwrap();
    
    if matches!(val.val_type, ValueType::Nil) {
        interp.bookshelf.push(default_val);
    } else {
        interp.bookshelf.push(val);
    }
    Ok(())
}

// NIL判定司書
pub fn op_nil_check(interp: &mut Interpreter) -> Result<()> {
    let val = interp.bookshelf.pop()
        .ok_or(LPLError::BookshelfUnderflow)?;
    
    let is_nil = matches!(val.val_type, ValueType::Nil);
    interp.bookshelf.push(crate::types::Value { 
        val_type: ValueType::Boolean(is_nil) 
    });
    Ok(())
}

// NOT-NIL判定司書（KNOWN?と同等）
pub fn op_not_nil_check(interp: &mut Interpreter) -> Result<()> {
    let val = interp.bookshelf.pop()
        .ok_or(LPLError::BookshelfUnderflow)?;
    
    let is_not_nil = !matches!(val.val_type, ValueType::Nil);
    interp.bookshelf.push(crate::types::Value { 
        val_type: ValueType::Boolean(is_not_nil) 
    });
    Ok(())
}
