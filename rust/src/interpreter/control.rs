use crate::interpreter::{Interpreter, WordDefinition, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, Token};
use std::collections::HashSet;

// 新しいDEF実装（Quotationなしでワード定義）
pub fn op_def(interp: &mut Interpreter, all_tokens: &[Token], description: Option<String>) -> Result<()> {
    // スタックからワード名を取得
    let name_val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    let name = match &name_val.val_type {
        ValueType::String(s) => s.clone(),
        _ => return Err(AjisaiError::type_error("string", "other type")),
    };
    
    // DEFトークンとワード名の位置を特定
    let mut def_index = None;
    let mut name_index = None;
    
    for (i, token) in all_tokens.iter().enumerate() {
        if let Token::Symbol(sym) = token {
            if sym == "DEF" {
                def_index = Some(i);
            }
        }
        if let Token::String(s) = token {
            if s == &name && name_index.is_none() {
                // DEFより前の最後の出現を探す
                if def_index.is_none() || i < def_index.unwrap() {
                    name_index = Some(i);
                }
            }
        }
    }
    
    let def_idx = def_index.ok_or("DEF token not found")?;
    let name_idx = name_index.ok_or("Word name not found before DEF")?;
    
    // ワード名より前のトークンをすべて定義内容とする
    let definition_tokens: Vec<Token> = all_tokens[..name_idx].to_vec();
    
    if definition_tokens.is_empty() {
        return Err(AjisaiError::from("Empty word definition"));
    }
    
    // 既存の保護チェック
    if let Some(existing) = interp.dictionary.get(&name) {
        if existing.is_builtin {
            return Err(AjisaiError::from("Cannot redefine builtin word"));
        }
    }
    
    if let Some(deps) = interp.dependencies.get(&name) {
        if !deps.is_empty() {
            return Err(AjisaiError::ProtectedWord {
                name: name.clone(),
                dependents: deps.iter().cloned().collect(),
            });
        }
    }
    
    // 依存関係の更新
    let old_deps = find_dependencies(&name, &interp.dictionary);
    for dep in &old_deps {
        if let Some(dep_set) = interp.dependencies.get_mut(dep) {
            dep_set.remove(&name);
        }
    }
    
    let new_deps = extract_words(&definition_tokens);
    for dep in &new_deps {
        interp.dependencies.entry(dep.clone())
            .or_insert_with(HashSet::new)
            .insert(name.clone());
    }
    
    // 新しいワードを辞書に登録
    interp.dictionary.insert(name.clone(), WordDefinition {
        tokens: definition_tokens,
        is_builtin: false,
        description,
    });
    
    Ok(())
}

// DEL（ワード削除）
pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    let name_val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    let name = match &name_val.val_type {
        ValueType::String(s) => s.clone(),
        _ => return Err(AjisaiError::type_error("string", "other type")),
    };
    
    if let Some(def) = interp.dictionary.get(&name) {
        if def.is_builtin {
            return Err(AjisaiError::from("Cannot delete builtin word"));
        }
    } else {
        return Err(AjisaiError::UnknownWord(name));
    }
    
    if let Some(deps) = interp.dependencies.get(&name) {
        if !deps.is_empty() {
            return Err(AjisaiError::ProtectedWord {
                name: name.clone(),
                dependents: deps.iter().cloned().collect(),
            });
        }
    }
    
    let removed_def = interp.dictionary.remove(&name).unwrap();
    let deps_to_remove = extract_words(&removed_def.tokens);
    
    for dep in deps_to_remove {
        if let Some(dep_set) = interp.dependencies.get_mut(&dep) {
            dep_set.remove(&name);
        }
    }
    
    Ok(())
}

// IFを条件値のみで動作するように変更
pub fn op_if(interp: &mut Interpreter) -> Result<()> {
    // スタックから条件値を取得
    let cond = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    match &cond.val_type {
        ValueType::Boolean(b) => {
            // 条件値をそのままスタックにプッシュ
            interp.stack.push(Value {
                val_type: ValueType::Boolean(*b),
            });
            Ok(())
        },
        ValueType::Nil => {
            // nilの場合もスタックにプッシュ（三値論理対応）
            interp.stack.push(cond);
            Ok(())
        },
        _ => Err(AjisaiError::type_error("boolean", "other type")),
    }
}

// CALLは削除される可能性があるが、後方互換性のため残す
pub fn op_call(_interp: &mut Interpreter) -> Result<()> {
    Err(AjisaiError::from("CALL is deprecated. Quotations are no longer supported."))
}

fn extract_words(tokens: &[Token]) -> HashSet<String> {
    let mut words = HashSet::new();
    for token in tokens {
        if let Token::Symbol(word) = token {
            words.insert(word.clone());
        }
    }
    words
}

fn find_dependencies(word: &str, dictionary: &std::collections::HashMap<String, WordDefinition>) -> HashSet<String> {
    let mut deps = HashSet::new();
    for (name, def) in dictionary {
        if !def.is_builtin && extract_words(&def.tokens).contains(word) {
            deps.insert(name.clone());
        }
    }
    deps
}
