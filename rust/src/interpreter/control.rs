// rust/src/interpreter/control.rs (暗黙のGOTO削除版)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType, Token, Value, BracketType};
use std::collections::HashSet;

// IF_SELECT - 条件に基づいてアクションを選択実行
pub fn op_if_select(interp: &mut Interpreter) -> Result<()> {
    interp.append_output("*** IF_SELECT CALLED ***\n");
    
    if interp.workspace.len() < 3 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let false_action = interp.workspace.pop().unwrap();
    let true_action = interp.workspace.pop().unwrap();
    let condition = interp.workspace.pop().unwrap();
    
    // デバッグ出力を追加
    interp.append_output(&format!("DEBUG: IF_SELECT condition: {:?}\n", condition));
    interp.append_output(&format!("DEBUG: true_action: {:?}\n", true_action));
    interp.append_output(&format!("DEBUG: false_action: {:?}\n", false_action));
    
    let condition_is_true = is_truthy(&condition);
    interp.append_output(&format!("DEBUG: is_truthy result: {}\n", condition_is_true));
    
    let selected_action = if condition_is_true {
        interp.append_output("DEBUG: Selecting true_action\n");
        true_action
    } else {
        interp.append_output("DEBUG: Selecting false_action\n");
        false_action
    };
    
    // 選択されたアクションを実行
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
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("*** vector_to_tokens CALLED ***"));
    
    let mut tokens = Vec::new();
    for value in values {
        match value.val_type {
            ValueType::Vector(inner_values, bracket_type) => {
                tokens.push(Token::VectorStart(bracket_type.clone()));
                let inner_tokens = vector_to_tokens(inner_values)?;
                tokens.extend(inner_tokens);
                tokens.push(Token::VectorEnd(bracket_type));
            },
            _ => {
                tokens.push(value_to_token(value)?);
            }
        }
    }
    
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("vector_to_tokens result: {:?}", tokens)));
    Ok(tokens)
}

fn is_truthy(value: &Value) -> bool {
    let result = match &value.val_type {
        ValueType::Boolean(b) => *b,
        ValueType::Nil => false,
        ValueType::Number(n) => n.numerator != 0,
        ValueType::String(s) => !s.is_empty(),
        ValueType::Vector(v, _) => {
            // 単一要素Vectorの場合、中身の値で判定
            if v.len() == 1 {
                is_truthy(&v[0])  // 再帰的に中身を評価
            } else {
                !v.is_empty()     // 複数要素の場合は空/非空で判定
            }
        },
        ValueType::Symbol(_) => true,
    };
    
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("is_truthy({:?}) = {}", value, result)));
    result
}

fn value_to_token(value: Value) -> Result<Token> {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("*** value_to_token: {:?} ***", value)));
    
    let result = match value.val_type {
        ValueType::Number(frac) => Ok(Token::Number(frac.numerator, frac.denominator)),
        ValueType::String(s) => Ok(Token::String(s)),
        ValueType::Boolean(b) => Ok(Token::Boolean(b)),
        ValueType::Symbol(s) => Ok(Token::Symbol(s)),
        ValueType::Nil => Ok(Token::Nil),
        ValueType::Vector(_, _) => {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("ERROR: Cannot convert vector to token"));
            Err(AjisaiError::from("Vector should be handled by vector_to_tokens function"))
        },
    };
    
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("value_to_token result: {:?}", result)));
    result
}

// DEF - 新しいワードを定義する
pub fn op_def(interp: &mut Interpreter) -> Result<()> {
    interp.append_output("*** DEF CALLED ***\n");
    
    let workspace_len = interp.workspace.len();
    
    // 最低2つ（本体ベクトル + 名前）は必要
    if workspace_len < 2 {
        return Err(AjisaiError::from("DEF requires at least vector and name"));
    }
    
    // パターン判定: 3つある場合は説明付き、2つの場合は説明なし
    let (code_val, name_val, description) = if workspace_len >= 3 {
        let desc_or_name = interp.workspace.pop().unwrap();
        let name_or_code = interp.workspace.pop().unwrap();
        let code_or_other = interp.workspace.pop().unwrap();
        
        match (&code_or_other.val_type, &name_or_code.val_type, &desc_or_name.val_type) {
            (ValueType::Vector(_, _), ValueType::String(_), ValueType::String(desc)) => {
                (code_or_other, name_or_code, Some(desc.clone()))
            },
            (ValueType::Vector(_, _), ValueType::String(_), _) => {
                interp.workspace.push(desc_or_name);
                (code_or_other, name_or_code, None)
            },
            _ => {
                interp.workspace.push(code_or_other);
                interp.workspace.push(name_or_code);
                (desc_or_name, interp.workspace.pop().unwrap(), None)
            }
        }
    } else {
        let name_val = interp.workspace.pop().unwrap();
        let code_val = interp.workspace.pop().unwrap();
        (code_val, name_val, None)
    };

    let name = match name_val.val_type {
        ValueType::String(s) => s.to_uppercase(),
        _ => return Err(AjisaiError::from("DEF requires string name")),
    };

    let (original_tokens, final_description) = match code_val.val_type {
        ValueType::Vector(v, _) => {
            let mut tokens = Vec::new();
            let mut function_comments = Vec::new();
            
            if let Some(desc) = description {
                function_comments.push(desc);
            }
            
            for value in v {
                tokens.push(value_to_token(value)?);
            }
            
            let final_description = if !function_comments.is_empty() {
                Some(function_comments.join(" "))
            } else {
                None
            };
            
            (tokens, final_description)
        },
        _ => return Err(AjisaiError::from("DEF requires vector")),
    };

    // 既存のワードチェック
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
    for token in &original_tokens {
        if let Token::Symbol(sym) = token {
            if interp.dictionary.contains_key(sym) && !interp.is_builtin_word(sym) {
                interp.dependencies.entry(sym.clone())
                    .or_insert_with(HashSet::new)
                    .insert(name.clone());
            }
        }
    }

    let description_clone = final_description.clone();

    interp.dictionary.insert(name.clone(), crate::interpreter::WordDefinition {
        tokens: original_tokens,
        is_builtin: false,
        description: final_description,
        category: None,
    });

    if let Some(desc) = &description_clone {
        interp.append_output(&format!("Defined word: {} ({})\n", name, desc));
    } else {
        interp.append_output(&format!("Defined word: {}\n", name));
    }
    Ok(())
}

// DEL - ワードを削除する
pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    interp.append_output("*** DEL CALLED ***\n");
    
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match val.val_type {
        ValueType::String(name) => {
            let name = name.to_uppercase();
            
            if let Some(def) = interp.dictionary.get(&name) {
                if def.is_builtin {
                    return Err(AjisaiError::from(format!("Cannot delete builtin word: {}", name)));
                }
            } else {
                return Err(AjisaiError::from(format!("Word '{}' not found", name)));
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
            
            interp.append_output(&format!("Deleted word: {}\n", name));
            Ok(())
        },
        _ => Err(AjisaiError::type_error("string", "other type")),
    }
}
