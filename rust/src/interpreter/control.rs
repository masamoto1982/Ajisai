// rust/src/interpreter/control.rs (ビルドエラー修正版)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType, Token, Value, BracketType}; // BracketType追加
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct ConditionalLine {
    pub condition: Option<Vec<Token>>,  // None = デフォルト行
    pub action: Vec<Token>,
}

// 条件分岐実行用のトークンを生成（新しい実装）
pub fn create_conditional_execution_tokens(lines: &[Vec<Token>]) -> Result<Vec<Token>> {
    let conditional_lines = parse_conditional_lines(lines)?;
    
    if conditional_lines.is_empty() {
        return Err(AjisaiError::from("No conditional lines found"));
    }
    
    // 再帰的に条件分岐を構築
    Ok(build_nested_conditions(&conditional_lines))
}

fn build_nested_conditions(lines: &[ConditionalLine]) -> Vec<Token> {
    if lines.is_empty() {
        return Vec::new();
    }
    
    if lines.len() == 1 {
        // 最後の行（デフォルト行またはただ一つの条件行）
        let line = &lines[0];
        if let Some(condition) = &line.condition {
            // 単一条件の場合
            let mut result = Vec::new();
            result.extend(condition.iter().cloned());
            result.push(Token::VectorStart(BracketType::Square));
            result.extend(line.action.iter().cloned());
            result.push(Token::VectorEnd(BracketType::Square));
            result.push(Token::VectorStart(BracketType::Square));
            // 空のデフォルトアクション
            result.push(Token::VectorEnd(BracketType::Square));
            result.push(Token::Symbol("IF_SELECT".to_string()));
            return result;
        } else {
            // デフォルト行のみ
            return line.action.clone();
        }
    }
    
    // 複数行の場合：最初の条件 + 残りを再帰処理
    let first_line = &lines[0];
    let remaining_lines = &lines[1..];
    
    if let Some(condition) = &first_line.condition {
        let mut result = Vec::new();
        
        // 最初の条件
        result.extend(condition.iter().cloned());
        
        // 真の場合のアクション
        result.push(Token::VectorStart(BracketType::Square));
        result.extend(first_line.action.iter().cloned());
        result.push(Token::VectorEnd(BracketType::Square));
        
        // 偽の場合のアクション（残りの条件を再帰処理）
        result.push(Token::VectorStart(BracketType::Square));
        let nested = build_nested_conditions(remaining_lines);
        result.extend(nested);
        result.push(Token::VectorEnd(BracketType::Square));
        
        result.push(Token::Symbol("IF_SELECT".to_string()));
        return result;
    } else {
        // 条件がない場合（デフォルト行）
        return first_line.action.clone();
    }
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

// IF_SELECT - 条件に基づいてアクションを選択実行
pub fn op_if_select(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 3 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let false_action = interp.workspace.pop().unwrap();
    let true_action = interp.workspace.pop().unwrap();
    let condition = interp.workspace.pop().unwrap();
    
    let selected_action = if is_truthy(&condition) {
        true_action
    } else {
        false_action
    };
    
    // 選択されたアクション（Vector）を実行
    if let ValueType::Vector(action_values, _) = selected_action.val_type {
        let tokens = vector_to_tokens(action_values)?;
        interp.execute_tokens(&tokens)?;
    }
    
    Ok(())
}

// CONDITIONAL_BRANCH - シンプルな条件分岐実行（後方互換性のため残す）
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
        ValueType::Vector(_, _) => {
            // ベクター型は直接トークンに変換できない
            Err(AjisaiError::from("Cannot convert vector to token - vectors should be handled differently"))
        },
        // 他の型があれば追加
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
                    .or_insert_with(HashSet::new)
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
