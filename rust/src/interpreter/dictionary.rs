// rust/src/interpreter/dictionary.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Token, ExecutionLine, ValueType, WordDefinition, Value}; // <--- Value を追加
use std::collections::HashSet;
use crate::tokenizer; // <--- 追加

pub fn op_def(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.is_empty() { return Err(AjisaiError::StackUnderflow); }

    let mut description: Option<String> = None;
    let name_val: Value;
    let body_val: Value;

    // --- 修正 (E0502対策) ---
    // 先にスタックトップの値の型をチェックし、
    // descriptionの可能性のある文字列を先にクロ―ンする。
    // これにより、この後の .pop() との借用競合を防ぐ。
    let top_is_string = if let Some(top_val) = interp.stack.last() {
        if let ValueType::String(s) = &top_val.val_type {
            description = Some(s.clone()); // 文字列を先にクロ―ン
            true
        } else {
            false
        }
    } else {
        return Err(AjisaiError::StackUnderflow); // stack is empty (if check)
    };
    // -----------------------

    if top_is_string {
        // --- シナリオ 1: [ 'body' ] 'name' 'comment' DEF ---
        if interp.stack.len() < 3 {
            return Err(AjisaiError::from("DEF with comment requires [ 'body' ], 'name', and 'comment'"));
        }
        
        let _comment_val = interp.stack.pop().unwrap(); // 'comment' をポップ (警告対応)
        name_val = interp.stack.pop().unwrap(); // 'name' をポップ
        body_val = interp.stack.pop().unwrap(); // [ 'body' ] をポップ
    
    } else {
        // --- シナリオ 2: [ 'body' ] 'name' DEF ---
        if interp.stack.len() < 2 {
            return Err(AjisaiError::from("DEF requires [ 'body' ] and 'name'"));
        }
        
        name_val = interp.stack.pop().unwrap(); // 'name' をポップ
        body_val = interp.stack.pop().unwrap(); // [ 'body' ] をポップ
        description = None;
    }

    // name_val ('name') が String型であることを確認
    let name_str = if let ValueType::String(s) = &name_val.val_type {
        s.clone()
    } else {
        // --- 修正 (E0382対策) ---
        // 1. エラーメッセージ用の型情報を先に取得
        let got_type = name_val.val_type.to_string();
        // 2. スタックを元に戻す (ムーブ発生)
        interp.stack.push(body_val);
        interp.stack.push(name_val);
        if let Some(desc_str) = description {
            interp.stack.push(Value { val_type: ValueType::String(desc_str) });
        }
        // 3. 取得済みの型情報でエラーを返す
        return Err(AjisaiError::type_error("string for word name", got_type.as_str()));
        // -----------------------
    };
    
    // body_val ([ 'body' ]) が [ String ] 型であることを確認
    let body_str = if let ValueType::Vector(v, _) = &body_val.val_type {
         if v.len() == 1 {
            if let ValueType::String(s) = &v[0].val_type {
                s.clone()
            } else {
                // --- 修正 (E0505対策) ---
                // 1. エラーメッセージ用の型情報を先に取得
                let got_type = v[0].val_type.to_string();
                // 2. スタックを元に戻す (ムーブ発生)
                interp.stack.push(body_val);
                interp.stack.push(name_val);
                if let Some(desc_str) = description {
                    interp.stack.push(Value { val_type: ValueType::String(desc_str) });
                }
                // 3. 取得済みの型情報でエラーを返す
                return Err(AjisaiError::type_error("string for word body", got_type.as_str()));
                // -----------------------
            }
        } else {
            // --- 修正 (E0382/E0505同様) ---
            interp.stack.push(body_val);
            interp.stack.push(name_val);
            if let Some(desc_str) = description {
                interp.stack.push(Value { val_type: ValueType::String(desc_str) });
            }
            return Err(AjisaiError::type_error("single-element vector", "multi-element vector"));
            // -----------------------
        }
    } else {
        // --- 修正 (E0382対策) ---
        // 1. エラーメッセージ用の型情報を先に取得
        let got_type = body_val.val_type.to_string();
        // 2. スタックを元に戻す (ムーブ発生)
        interp.stack.push(body_val);
        interp.stack.push(name_val);
        if let Some(desc_str) = description {
            interp.stack.push(Value { val_type: ValueType::String(desc_str) });
        }
        // 3. 取得済みの型情報でエラーを返す
        return Err(AjisaiError::type_error("vector for word body", got_type.as_str()));
        // -----------------------
    };

    // 処理内容の文字列をトークン化
    let custom_word_names: HashSet<String> = interp.dictionary.iter()
        .filter(|(_, def)| !def.is_builtin)
        .map(|(name, _)| name.clone())
        .collect();
    
    let tokens = tokenizer::tokenize_with_custom_words(&body_str, &custom_word_names)
        .map_err(|e| AjisaiError::from(format!("Tokenization error in DEF: {}", e)))?;

    // 内部定義関数を呼び出し
    op_def_inner(interp, &name_str, &tokens, description)
}


pub(crate) fn op_def_inner(interp: &mut Interpreter, name: &str, tokens: &[Token], description: Option<String>) -> Result<()> {
    let upper_name = name.to_uppercase();
    interp.output_buffer.push_str(&format!("[DEBUG] Defining word '{}'\n", upper_name));

    if let Some(old_def) = interp.dictionary.get(&upper_name) {
        for dep_name in &old_def.dependencies {
            if let Some(dependents) = interp.dependents.get_mut(dep_name) {
                dependents.remove(&upper_name);
            }
        }
    }

    let lines = parse_definition_body(tokens)?;
    
    let mut new_dependencies = HashSet::new();
    for line in &lines {
        for token in line.body_tokens.iter() {
            if let Token::Symbol(s) = token {
                let upper_s = s.to_uppercase();
                if interp.dictionary.contains_key(&upper_s) && !interp.dictionary.get(&upper_s).unwrap().is_builtin {
                    new_dependencies.insert(upper_s);
                }
            }
        }
    }
    
    for dep_name in &new_dependencies {
        interp.dependents.entry(dep_name.clone()).or_default().insert(upper_name.clone());
    }
    
    let new_def = WordDefinition {
        lines,
        is_builtin: false,
        description,
        dependencies: new_dependencies,
        original_source: None,
    };
    
    interp.dictionary.insert(upper_name.clone(), new_def);
    interp.output_buffer.push_str(&format!("Defined word: {}\n", name));
    Ok(())
}

fn parse_definition_body(tokens: &[Token]) -> Result<Vec<ExecutionLine>> {
    let mut lines = Vec::new();
    let mut current_line_tokens = Vec::new();
    
    for token in tokens {
        match token {
            Token::LineBreak => {
                if !current_line_tokens.is_empty() {
                    let execution_line = parse_single_execution_line(&current_line_tokens)?;
                    lines.push(execution_line);
                    current_line_tokens.clear();
                }
            },
            _ => {
                current_line_tokens.push(token.clone());
            }
        }
    }
    
    if !current_line_tokens.is_empty() {
        let execution_line = parse_single_execution_line(&current_line_tokens)?;
        lines.push(execution_line);
    }
    
    if lines.is_empty() {
        return Err(AjisaiError::from("Word definition cannot be empty"));
    }
    
    Ok(lines)
}

fn parse_single_execution_line(tokens: &[Token]) -> Result<ExecutionLine> {
    Ok(ExecutionLine {
        body_tokens: tokens.to_vec(),
    })
}

pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    // 変更：DELは 'NAME' を期待する
    let val = interp.stack.last().ok_or(AjisaiError::StackUnderflow)?;
    
    let name = match &val.val_type {
        ValueType::String(s) => s.clone(),
        _ => return Err(AjisaiError::type_error("string 'name'", "other type")),
    };

    let upper_name = name.to_uppercase();

    if let Some(removed_def) = interp.dictionary.remove(&upper_name) {
        for dep_name in &removed_def.dependencies {
            if let Some(dependents) = interp.dependents.get_mut(dep_name) {
                dependents.remove(&upper_name);
            }
        }
        interp.dependents.remove(&upper_name);
        
        interp.stack.pop(); // 'NAME' をポップ
        interp.output_buffer.push_str(&format!("Deleted word: {}\n", name));
        Ok(())
    } else {
        Err(AjisaiError::UnknownWord(upper_name))
    }
}

pub fn op_lookup(interp: &mut Interpreter) -> Result<()> {
    // 変更：LOOKUP (?) は 'NAME' を期待する
    let name_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let name_str = if let ValueType::String(s) = name_val.val_type {
        s.clone()
    } else {
        return Err(AjisaiError::type_error("string 'name'", name_val.val_type.to_string().as_str()));
    };

    let upper_name = name_str.to_uppercase();
    
    if let Some(def) = interp.dictionary.get(&upper_name) {
        if def.is_builtin {
            let detailed_info = crate::builtins::get_builtin_detail(&upper_name);
            interp.definition_to_load = Some(detailed_info);
            return Ok(());
        }
        
        if let Some(original_source) = &def.original_source {
            interp.definition_to_load = Some(original_source.clone());
        } else {
            let definition = interp.get_word_definition_tokens(&upper_name).unwrap_or_default();
            let full_definition = if definition.is_empty() {
                format!("[ '' ] '{}' DEF", name_str) // 空の定義
            } else {
                if let Some(desc) = &def.description {
                    format!("[ '{}' ] '{}' '{}' DEF", definition, name_str, desc)
                } else {
                    format!("[ '{}' ] '{}' DEF", definition, name_str)
                }
            };
            interp.definition_to_load = Some(full_definition);
        }
        Ok(())
    } else {
        Err(AjisaiError::UnknownWord(name_str))
    }
}
