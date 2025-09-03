// rust/src/interpreter/control.rs (複数行定義自動判定対応版)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType, Token, Value};

#[derive(Debug, Clone)]
pub struct ConditionalLine {
    pub condition: Option<Vec<Token>>,  // None = デフォルト行
    pub action: Vec<Token>,
}

// 条件分岐実行用のトークンを生成
pub fn create_conditional_execution_tokens(lines: &[Vec<Token>]) -> Result<Vec<Token>> {
    let conditional_lines = parse_conditional_lines(lines)?;
    
    if conditional_lines.is_empty() {
        return Err(AjisaiError::from("No conditional lines found"));
    }
    
    // シンプルな条件分岐実行トークンを生成
    let mut result = Vec::new();
    
    // 各条件行を順次チェックして最初に真になった処理を実行
    for (i, cond_line) in conditional_lines.iter().enumerate() {
        if let Some(condition) = &cond_line.condition {
            // 条件あり：condition実行→判定→真なら action 実行して終了
            result.extend(condition.iter().cloned());
            result.extend(cond_line.action.iter().cloned());
            result.push(Token::Symbol("CONDITIONAL_BRANCH".to_string()));
            result.push(Token::Number(conditional_lines.len() as i64 - i as i64 - 1, 1)); // 残り行数
        } else {
            // デフォルト行：無条件実行
            result.extend(cond_line.action.iter().cloned());
            break;
        }
    }
    
    Ok(result)
}

fn parse_conditional_lines(lines: &[Vec<Token>]) -> Result<Vec<ConditionalLine>> {
    let mut conditional_lines = Vec::new();
    
    for line in lines {
        conditional_lines.push(parse_single_conditional_line(line)?);
    }
    
    Ok(conditional_lines)
}

fn parse_single_conditional_line(tokens: &[Token]) -> Result<ConditionalLine> {
    // コロンで分割
    if let Some(colon_pos) = tokens.iter().position(|t| matches!(t, Token::Colon)) {
        let condition = tokens[..colon_pos].to_vec();
        let action = tokens[colon_pos + 1..].to_vec();
        
        if condition.is_empty() {
            return Err(AjisaiError::from("Empty condition before colon"));
        }
        if action.is_empty() {
            return Err(AjisaiError::from("Empty action after colon"));
        }
        
        Ok(ConditionalLine {
            condition: Some(condition),
            action,
        })
    } else {
        // コロンなし = デフォルト行
        Ok(ConditionalLine {
            condition: None,
            action: tokens.to_vec(),
        })
    }
}

// CONDITIONAL_BRANCH - シンプルな条件分岐実行
pub fn op_conditional_branch(interp: &mut Interpreter) -> Result<()> {
    // スタックから残り分岐数を取得（使用しないが互換性のため）
    let _remaining_branches_val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    // アクション（文字列）を取得
    let action_val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    // 条件（真偽値）を取得
    let condition_val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    if is_truthy(&condition_val) {
        // 条件が真の場合、アクションを実行
        if let ValueType::String(action_str) = action_val.val_type {
            interp.workspace.push(Value {
                val_type: ValueType::String(action_str)
            });
        }
    }
    // 条件が偽の場合は何もしない（次の条件へ）
    
    Ok(())
}

fn is_truthy(value: &Value) -> bool {
    match &value.val_type {
        ValueType::Boolean(b) => *b,
        ValueType::Nil => false,
        ValueType::Number(n) => n.numerator != 0,
        ValueType::String(s) => !s.is_empty(),
        ValueType::Vector(v, _) => !v.is_empty(),
        ValueType::Symbol(_) => true,
    }
}

fn vector_to_tokens(values: Vec<Value>) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    for value in values {
        tokens.push(value_to_token(value)?);
    }
    Ok(tokens)
}

fn value_to_token(value: Value) -> Result<Token> {
    match value.val_type {
        ValueType::Number(frac) => Ok(Token::Number(frac.numerator, frac.denominator)),
        ValueType::String(s) => Ok(Token::String(s)),
        ValueType::Boolean(b) => Ok(Token::Boolean(b)),
        ValueType::Symbol(s) => Ok(Token::Symbol(s)),
        ValueType::Nil => Ok(Token::Nil),
        _ => Err(AjisaiError::from("Cannot convert value to token")),
    }
}

// NOP - 何もしない
pub fn op_nop(_interp: &mut Interpreter) -> Result<()> {
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

    let original_tokens = match code_val.val_type {
        ValueType::Vector(v, _) => {
            let mut tokens = Vec::new();
            for value in v {
                tokens.push(value_to_token(value)?);
            }
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
    for token in &original_tokens {
        if let Token::Symbol(sym) = token {
            if interp.dictionary.contains_key(sym) && !interp.is_builtin_word(sym) {
                interp.dependencies.entry(sym.clone())
                    .or_insert_with(std::collections::HashSet::new)
                    .insert(name.clone());
            }
        }
    }

    interp.dictionary.insert(name.clone(), crate::interpreter::WordDefinition {
        tokens: original_tokens,
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
