use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType};

pub fn op_def(_interp: &mut Interpreter) -> Result<()> {
    // DEFは行末での特殊な構文として処理されるため、
    // 通常の実行フローでここに到達した場合はエラー
    Err(AjisaiError::from("DEF must be used at the end of a line with a string name: <words> \"NAME\" DEF"))
}

// 条件選択
pub fn op_if_select(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 3 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let false_val = interp.stack.pop().unwrap();
    let true_val = interp.stack.pop().unwrap();
    let condition = interp.stack.pop().unwrap();
    
    let result = apply_if_select(&condition, &true_val, &false_val);
    
    interp.stack.push(result);
    Ok(())
}

// 再帰的なヘルパー関数
fn apply_if_select(condition: &Value, true_val: &Value, false_val: &Value) -> Value {
    match &condition.val_type {
        ValueType::Boolean(b) => {
            if *b { true_val.clone() } else { false_val.clone() }
        },
        ValueType::Nil => false_val.clone(),
        ValueType::Vector(v) => {
            // ベクトルの各要素に再帰的に適用
            let results: Vec<Value> = v.iter().map(|elem| {
                apply_if_select(elem, true_val, false_val)
            }).collect();
            Value { val_type: ValueType::Vector(results) }
        },
        _ => condition.clone(),  // その他の型はそのまま返す
    }
}

// 新規追加: WHEN（条件付き実行）
pub fn op_when(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let condition = interp.stack.pop().unwrap();
    let value = interp.stack.pop().unwrap();
    
    match condition.val_type {
        ValueType::Boolean(true) => {
            interp.stack.push(value);
        },
        ValueType::Boolean(false) | ValueType::Nil => {
            // 何もプッシュしない
        },
        _ => return Err(AjisaiError::type_error("boolean or nil", "other type")),
    }
    
    Ok(())
}

pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    match val.val_type {
        ValueType::String(name) => {
            let name = name.to_uppercase();
            
            // ワードが存在するかチェック
            if !interp.dictionary.contains_key(&name) {
                return Err(AjisaiError::from(format!("Word not found: {}", name)));
            }
            
            // ビルトインワードは削除不可
            if let Some(def) = interp.dictionary.get(&name) {
                if def.is_builtin {
                    return Err(AjisaiError::from(format!("Cannot delete builtin word: {}", name)));
                }
            }
            
            // 依存関係チェック（このワードを使っている他のワードがあるか）
            if let Some(dependents) = interp.dependencies.get(&name) {
                if !dependents.is_empty() {
                    let dependent_list: Vec<String> = dependents.iter().cloned().collect();
                    return Err(AjisaiError::ProtectedWord {
                        name: name.clone(),
                        dependents: dependent_list,
                    });
                }
            }
            
            // このワードが依存している他のワードから、依存関係を削除
            if let Some(def) = interp.dictionary.get(&name) {
                for token in &def.tokens {
                    if let crate::types::Token::Symbol(dep_name) = token {
                        // 正しい構文に修正
                        if let Some(deps) = interp.dependencies.get_mut(dep_name) {
                            deps.remove(&name);
                        }
                    }
                }
            }
            
            // ワードを削除
            interp.dictionary.remove(&name);
            interp.dependencies.remove(&name);
            interp.word_properties.remove(&name);
            
            interp.append_output(&format!("Deleted: {}\n", name));
            Ok(())
        },
        ValueType::Symbol(name) => {
            // シンボルの場合も処理（後方互換性のため）
            let name = name.to_uppercase();
            
            if !interp.dictionary.contains_key(&name) {
                return Err(AjisaiError::from(format!("Word not found: {}", name)));
            }
            
            if let Some(def) = interp.dictionary.get(&name) {
                if def.is_builtin {
                    return Err(AjisaiError::from(format!("Cannot delete builtin word: {}", name)));
                }
            }
            
            if let Some(dependents) = interp.dependencies.get(&name) {
                if !dependents.is_empty() {
                    let dependent_list: Vec<String> = dependents.iter().cloned().collect();
                    return Err(AjisaiError::ProtectedWord {
                        name: name.clone(),
                        dependents: dependent_list,
                    });
                }
            }
            
            // このワードが依存している他のワードから、依存関係を削除
            if let Some(def) = interp.dictionary.get(&name) {
                for token in &def.tokens {
                    if let crate::types::Token::Symbol(dep_name) = token {
                        // 正しい構文に修正
                        if let Some(deps) = interp.dependencies.get_mut(dep_name) {
                            deps.remove(&name);
                        }
                    }
                }
            }
            
            interp.dictionary.remove(&name);
            interp.dependencies.remove(&name);
            interp.word_properties.remove(&name);
            
            interp.append_output(&format!("Deleted: {}\n", name));
            Ok(())
        },
        _ => Err(AjisaiError::type_error("string or symbol", "other type")),
    }
}
