// rust/src/interpreter/dictionary.rs

use crate::interpreter::{Interpreter, WordDefinition};
use crate::interpreter::error::{AjisaiError, Result};
use crate::types::{Token, ValueType, ExecutionLine, Value}; // ★ BracketType を削除
use std::collections::HashSet;
use std::fmt::Write; // for write!

// === 新しいヘルパー関数 ===

/// Value を Token に変換する
/// (シグネチャを &Value に変更し、Result<T> 構文を修正)
fn value_to_token(val: &Value) -> Result<Token> {
    match &val.val_type {
        ValueType::Number(_) => {
            // Value の Display impl が "1/2" や "1" の形式で出力するので、それを利用
            Ok(Token::Number(val.to_string()))
        },
        ValueType::String(s) => Ok(Token::String(s.clone())),
        ValueType::Boolean(b) => Ok(Token::Boolean(*b)),
        ValueType::Symbol(s) => Ok(Token::Symbol(s.clone())),
        ValueType::Nil => Ok(Token::Nil),
        ValueType::Vector(_, _) => Err(AjisaiError::from("Cannot convert nested vector root to single token")),
    }
}

/// Vec<Value> を Vec<Token> に再帰的に変換する
/// (Result<T> 構文を修正し、value_to_token に &Value を渡すよう修正)
fn values_to_tokens(values: &[Value]) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    for val in values {
        match &val.val_type {
            ValueType::Vector(v, bt) => {
                // ネストしたベクタ
                tokens.push(Token::VectorStart(bt.clone()));
                tokens.extend(values_to_tokens(v)?); // 再帰呼び出し
                tokens.push(Token::VectorEnd(bt.clone()));
            },
            _ => {
                // 他のプリミティブ型
                tokens.push(value_to_token(val)?);
            }
        }
    }
    Ok(tokens)
}


// === op_def のロジックを修正 ===
pub fn op_def(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let mut description: Option<String> = None;
    let name_str: String; // mut を削除

    // トップが文字列かチェック
    let val1 = interp.stack.pop().unwrap(); // トップをポップ
    
    if let ValueType::String(s1) = val1.val_type {
        // s1 は 'NAME' か 'DESCRIPTION' のどちらか
        
        // 2番目も文字列かチェック
        if let Some(val2) = interp.stack.last() {
             if let ValueType::String(s2) = &val2.val_type {
                // スタック: [ ... , [ DEF ], 'NAME', 'DESCRIPTION' ]
                // val1 = 'DESCRIPTION', val2 = 'NAME'
                description = Some(s1);
                name_str = s2.clone();
                interp.stack.pop(); // 'NAME' をポップ
             } else {
                // スタック: [ ... , [ DEF ], 'NAME' ]
                // val1 = 'NAME', val2 = [ DEF ]
                name_str = s1;
             }
        } else {
             // スタック: [ [ DEF ], 'NAME' ]
             name_str = s1;
        }
    } else {
        // トップが文字列でない (構文エラー)
        interp.stack.push(val1); // ポップした値を戻す
        return Err(AjisaiError::type_error("string 'name' or 'description'", "other type"));
    }

    // 3. [ 処理内容 ] (ベクタ) をポップ
    let def_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    
    // 定義本体 (Vec<Value>) を取得
    let definition_values = match def_val.val_type {
        ValueType::Vector(vec, _) => vec,
        _ => return Err(AjisaiError::type_error("vector (quotation)", "other type")),
    };

    // Vec<Value> を Vec<Token> に変換
    let tokens = values_to_tokens(&definition_values)?;
    
    // 内部定義関数を呼び出し
    op_def_inner(interp, &name_str, &tokens, description)
}


pub(crate) fn op_def_inner(interp: &mut Interpreter, name: &str, tokens: &[Token], description: Option<String>) -> Result<()> {
    let upper_name = name.to_uppercase();
    
    // デバッグバッファに書き込む
    writeln!(interp.debug_buffer, "[DEBUG] Defining word '{}' with tokens: {:?}", upper_name, tokens).unwrap();

    if let Some(old_def) = interp.dictionary.get(&upper_name) {
        for dep_name in &old_def.dependencies {
            if let Some(dependents) = interp.dependents.get_mut(dep_name) {
                dependents.remove(&upper_name);
            }
        }
    }

    // トークンを LineBreak で分割する
    let mut lines = Vec::new();
    let mut current_line = Vec::new();
    
    for token in tokens {
        if let Token::LineBreak = token {
            // ★ 行分割ロジック修正：改行トークンは ExecutionLine に含めない
            if !current_line.is_empty() {
                lines.push(ExecutionLine { body_tokens: current_line });
                current_line = Vec::new();
            }
            // 空行も lines には追加しない
        } else {
            current_line.push(token.clone());
        }
    }
    
    if !current_line.is_empty() {
        lines.push(ExecutionLine { body_tokens: current_line });
    }

    // ★ 空の定義 [ ] DEF の場合は lines が空になるが、それでOK
    // if lines.is_empty() {
    //      lines.push(ExecutionLine { body_tokens: vec![] });
    // }

    
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
    
    // 通常のアウトプットバッファに書き込む
    interp.output_buffer.push_str(&format!("Defined word: {}\n", name));
    Ok(())
}

pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    // DELは 'NAME' を期待する
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
    // LOOKUP (?) は 'NAME' を期待する
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
            // get_word_definition_tokens は '1 2 +\n3 +' のような文字列を返す
            let definition = interp.get_word_definition_tokens(&upper_name).unwrap_or_default();
            
            // 説明文を取得。ない場合は空文字列をデフォルトにする
            let desc = def.description.as_deref().unwrap_or(""); 
            
            // [ {body} ] の形式で復元
            let full_definition = format!("[ {} ] '{}' '{}' DEF", definition, name_str, desc);
            interp.definition_to_load = Some(full_definition);
        }
        Ok(())
    } else {
        Err(AjisaiError::UnknownWord(name_str))
    }
}
