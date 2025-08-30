// rust/src/interpreter/control.rs (完全版・一文字漢字ワード対応)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType, Token};

// 招妖精 - 新しい妖精を招き寄せる（DEF相当）
// 注意：mod.rsでの特別処理により、説明付き招待は事前に処理される
pub fn op_summon(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::from("招 requires vector and name"));
    }

    let name_val = interp.workspace.pop().unwrap();
    let code_val = interp.workspace.pop().unwrap();

    let name = match name_val.val_type {
        ValueType::String(s) => s.to_uppercase(),
        _ => return Err(AjisaiError::from("招 requires string name")),
    };

    let tokens = match code_val.val_type {
    ValueType::Vector(v) => {
        let mut tokens = vec![Token::VectorStart];
        for value in v {
            tokens.push(interp.value_to_token(value)?);
        }
        tokens.push(Token::VectorEnd);
        tokens
    },
    _ => return Err(AjisaiError::from("招 requires vector")),
};

    // 既存のワードチェック
    if let Some(existing) = interp.dictionary.get(&name) {
        if existing.is_builtin {
            return Err(AjisaiError::from(format!("Cannot redefine builtin fairy: {}", name)));
        }
    }

    // 依存関係チェック（保護された妖精の確認）
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

    // ワード定義を登録（説明なしの従来版）
    interp.dictionary.insert(name.clone(), crate::interpreter::WordDefinition {
        tokens,
        is_builtin: false,
        description: None,  // 従来の招待では説明なし
        category: None,
    });

    interp.append_output(&format!("Summoned fairy: {}\n", name));
    Ok(())
}

// 招妖精 - 新しい妖精を招き寄せる（DEF相当）
pub fn op_summon(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::from("招 requires vector and name"));
    }

    let name_val = interp.workspace.pop().unwrap();
    let code_val = interp.workspace.pop().unwrap();

    let name = match name_val.val_type {
        ValueType::String(s) => s.to_uppercase(),
        _ => return Err(AjisaiError::from("招 requires string name")),
    };

    let tokens = match code_val.val_type {
        ValueType::Vector(v) => {
            // Vectorを直接トークンに変換するのではなく、
            // VectorStart + 内容 + VectorEnd の形でトークンを構築
            let mut tokens = vec![Token::VectorStart];
            for value in v {
                tokens.push(interp.value_to_token(value)?);
            }
            tokens.push(Token::VectorEnd);
            tokens
        },
        _ => return Err(AjisaiError::from("招 requires vector")),
    };

    // 既存のワードチェック
    if let Some(existing) = interp.dictionary.get(&name) {
        if existing.is_builtin {
            return Err(AjisaiError::from(format!("Cannot redefine builtin fairy: {}", name)));
        }
    }

    // 依存関係チェック（保護された妖精の確認）
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

    // ワード定義を登録（説明なしの従来版）
    interp.dictionary.insert(name.clone(), crate::interpreter::WordDefinition {
        tokens,
        is_builtin: false,
        description: None,  // 従来の招待では説明なし
        category: None,
    });

    interp.append_output(&format!("Summoned fairy: {}\n", name));
    Ok(())
}

// 払妖精 - 妖精を払い除ける（DEL相当）
pub fn op_dismiss(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match val.val_type {
        ValueType::String(name) => {
            let name = name.to_uppercase();
            
            // 組み込み妖精の保護
            if let Some(def) = interp.dictionary.get(&name) {
                if def.is_builtin {
                    return Err(AjisaiError::from(format!("Cannot dismiss builtin fairy: {}", name)));
                }
            } else {
                return Err(AjisaiError::from(format!("Fairy '{}' not found", name)));
            }
            
            // 依存関係チェック（他の妖精から使用されていないか確認）
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
            
            interp.append_output(&format!("Dismissed fairy: {}\n", name));
            Ok(())
        },
        _ => Err(AjisaiError::type_error("string", "other type")),
    }
}

// 跳妖精 - 妖精交代（条件付きGOTO相当） 
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
        ValueType::String(fairy_name) => {
            // 同一ワード内制限でワード実行
            let current_word = interp.call_stack.last().cloned();
            interp.execute_word_leap(&fairy_name, current_word.as_deref())?;
            Ok(())
        },
        ValueType::Vector(code_vec) => {
            // 直接コードベクトルを実行
            let tokens = interp.vector_to_tokens(code_vec)?;
            interp.execute_tokens(&tokens)?;
            Ok(())
        },
        _ => Err(AjisaiError::type_error("string or vector", "other type")),
    }
}

// 条件付き実行妖精 - 条件が真の場合のみ実行
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
            ValueType::Vector(code_vec) => {
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

// デフォルト値設定妖精 - nil の場合にデフォルト値を使用
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

// NIL判定妖精
pub fn op_nil_check(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    let is_nil = matches!(val.val_type, ValueType::Nil);
    interp.workspace.push(crate::types::Value { 
        val_type: ValueType::Boolean(is_nil) 
    });
    Ok(())
}

// NOT-NIL判定妖精（KNOWN?と同等）
pub fn op_not_nil_check(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    let is_not_nil = !matches!(val.val_type, ValueType::Nil);
    interp.workspace.push(crate::types::Value { 
        val_type: ValueType::Boolean(is_not_nil) 
    });
    Ok(())
}
