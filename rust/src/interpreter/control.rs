// rust/src/interpreter/control.rs (NOP追加、EVAL削除)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType, Token, Value};

#[derive(Debug, Clone)]
struct CodeBlock {
    condition: Option<Value>,  // None の場合はDEFAULT
    tokens: Vec<Token>,
}

// GOTO - 条件に基づくコードブロック実行
pub fn op_goto(interp: &mut Interpreter) -> Result<()> {
    let blocks = parse_code_blocks_from_stack(interp)?;
    
    if blocks.is_empty() {
        return Err(AjisaiError::from("GOTO requires code blocks"));
    }
    
    // 条件を順次評価
    for block in blocks {
        if let Some(condition) = block.condition {
            if evaluate_condition(interp, &condition)? {
                // 条件が真の場合、このブロックを実行
                return interp.execute_tokens(&block.tokens);
            }
        } else {
            // DEFAULT ブロック（条件がない）を実行
            return interp.execute_tokens(&block.tokens);
        }
    }
    
    // どの条件も満たさない場合は何もしない
    Ok(())
}

fn parse_code_blocks_from_stack(interp: &mut Interpreter) -> Result<Vec<CodeBlock>> {
    let mut blocks = Vec::new();
    let mut temp_tokens = Vec::new();
    
    // スタックから全てのトークンを取得（逆順）
    while let Some(value) = interp.workspace.pop() {
        temp_tokens.push(value);
    }
    
    // 正順に戻す
    temp_tokens.reverse();
    
    // トークンを解析してコードブロックを構築
    let mut i = 0;
    while i < temp_tokens.len() {
        if let ValueType::Symbol(s) = &temp_tokens[i].val_type {
            if s == ">" {
                // コードブロック開始
                let block = parse_single_code_block(&temp_tokens[i..], &mut i)?;
                blocks.push(block);
                continue;
            }
        }
        
        // 条件値の場合、次のコードブロックの条件として保持
        if i + 1 < temp_tokens.len() {
            if let ValueType::Symbol(s) = &temp_tokens[i + 1].val_type {
                if s == ">" {
                    // 条件をスタックに戻しておく（次の処理で使用）
                    interp.workspace.push(temp_tokens[i].clone());
                    i += 1;
                    continue;
                }
            }
        }
        
        i += 1;
    }
    
    Ok(blocks)
}

fn parse_single_code_block(tokens: &[Value], index: &mut usize) -> Result<CodeBlock> {
    if tokens.is_empty() {
        return Err(AjisaiError::from("Empty code block"));
    }
    
    // > 記号をスキップ
    *index += 1;
    
    let mut condition = None;
    let mut code_tokens = Vec::new();
    
    // 条件または DEFAULT をチェック
    if *index < tokens.len() {
        match &tokens[*index].val_type {
            ValueType::Symbol(s) if s == "CODE" => {
                // > CODE パターン（条件なし）
                *index += 1;
            },
            ValueType::Symbol(s) if s == "DEFAULT" => {
                // > DEFAULT CODE パターン
                *index += 1;
                if *index < tokens.len() {
                    if let ValueType::Symbol(code) = &tokens[*index].val_type {
                        if code == "CODE" {
                            *index += 1;
                        }
                    }
                }
            },
            _ => {
                // > condition CODE パターン
                condition = Some(tokens[*index].clone());
                *index += 1;
                
                // CODE キーワードをチェック
                if *index < tokens.len() {
                    if let ValueType::Symbol(s) = &tokens[*index].val_type {
                        if s == "CODE" {
                            *index += 1;
                        }
                    }
                }
            }
        }
    }
    
    // CODE 後のトークンを収集（次の > まで）
    while *index < tokens.len() {
        if let ValueType::Symbol(s) = &tokens[*index].val_type {
            if s == ">" || s == "GOTO" {
                break;
            }
        }
        
        code_tokens.push(value_to_token(&tokens[*index])?);
        *index += 1;
    }
    
    Ok(CodeBlock {
        condition,
        tokens: code_tokens,
    })
}

fn value_to_token(value: &Value) -> Result<Token> {
    match &value.val_type {
        ValueType::Number(frac) => Ok(Token::Number(frac.numerator, frac.denominator)),
        ValueType::String(s) => Ok(Token::String(s.clone())),
        ValueType::Boolean(b) => Ok(Token::Boolean(*b)),
        ValueType::Symbol(s) => Ok(Token::Symbol(s.clone())),
        ValueType::Nil => Ok(Token::Nil),
        _ => Err(AjisaiError::from("Cannot convert value to token")),
    }
}

fn evaluate_condition(interp: &mut Interpreter, condition: &Value) -> Result<bool> {
    match &condition.val_type {
        ValueType::Boolean(b) => Ok(*b),
        ValueType::Nil => Ok(false),
        ValueType::Number(n) => Ok(n.numerator != 0),
        ValueType::String(s) => Ok(!s.is_empty()),
        ValueType::Vector(v, _) => Ok(!v.is_empty()),
        ValueType::Symbol(_) => {
            // シンボルは評価して結果を条件とする
            interp.workspace.push(condition.clone());
            // シンボルの実行は複雑なので、一旦true固定
            Ok(true)
        },
    }
}

// NOP - 何もしない（EVALの代わり）
pub fn op_nop(_interp: &mut Interpreter) -> Result<()> {
    // 何もしない
    Ok(())
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
            let mut tokens = vec![Token::VectorStart(bracket_type.clone())];
            for value in v {
                tokens.push(value_to_token(&value)?);
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
    for token in &tokens {
        if let Token::Symbol(sym) = token {
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
