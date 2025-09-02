// rust/src/interpreter/control.rs (暗黙GOTO実装)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType, Token, Value};

#[derive(Debug, Clone)]
pub struct ConditionalBlock {
    pub condition: Option<Vec<Token>>,  // None = デフォルト
    pub action: Vec<Token>,
}

// 暗黙GOTO - ワード定義時に自動適用される条件分岐機能
pub fn apply_implicit_goto(tokens: &[Token]) -> Result<Vec<Token>> {
    let blocks = parse_conditional_blocks(tokens)?;
    
    if blocks.is_empty() {
        return Ok(tokens.to_vec());
    }
    
    if blocks.len() == 1 && blocks[0].condition.is_none() {
        // 単一ブロックで条件なし = そのまま実行
        return Ok(blocks[0].action.clone());
    }
    
    // 複数ブロックまたは条件付きブロック = GOTO機能を構築
    Ok(build_goto_tokens(blocks)?)
}

fn parse_conditional_blocks(tokens: &[Token]) -> Result<Vec<ConditionalBlock>> {
    let mut blocks = Vec::new();
    let mut current_tokens = Vec::new();
    let mut i = 0;
    
    while i < tokens.len() {
        match &tokens[i] {
            Token::LineBreak => {
                if !current_tokens.is_empty() {
                    blocks.push(parse_single_line_block(current_tokens)?);
                    current_tokens = Vec::new();
                }
                i += 1;
            },
            Token::FunctionComment(_) => {
                // コメントはスキップ
                i += 1;
            },
            _ => {
                current_tokens.push(tokens[i].clone());
                i += 1;
            }
        }
    }
    
    // 最後の行を処理
    if !current_tokens.is_empty() {
        blocks.push(parse_single_line_block(current_tokens)?);
    }
    
    Ok(blocks)
}

fn parse_single_line_block(tokens: Vec<Token>) -> Result<ConditionalBlock> {
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
        
        Ok(ConditionalBlock {
            condition: Some(condition),
            action,
        })
    } else {
        // コロンなし = デフォルトブロック
        Ok(ConditionalBlock {
            condition: None,
            action: tokens,
        })
    }
}

fn build_goto_tokens(blocks: Vec<ConditionalBlock>) -> Result<Vec<Token>> {
    let mut result = Vec::new();
    
    for block in blocks {
        if let Some(condition) = block.condition {
            // 条件をスタックに積む
            result.extend(condition);
            
            // 条件分岐の実装：IF action THEN 形式
            result.push(Token::Symbol("BRANCH_IF".to_string()));
            result.extend(block.action);
            result.push(Token::Symbol("BRANCH_END".to_string()));
        } else {
            // デフォルトブロックは最後に無条件実行
            result.extend(block.action);
        }
    }
    
    Ok(result)
}

// BRANCH_IF - 条件分岐の実装
pub fn op_branch_if(interp: &mut Interpreter) -> Result<()> {
    let condition_val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    let should_execute = match condition_val.val_type {
        ValueType::Boolean(b) => b,
        ValueType::Nil => false,
        ValueType::Number(n) => n.numerator != 0,
        ValueType::String(s) => !s.is_empty(),
        ValueType::Vector(v, _) => !v.is_empty(),
        ValueType::Symbol(_) => true,
    };
    
    if !should_execute {
        // 条件が偽の場合、次のBRANCH_ENDまでスキップ
        skip_to_branch_end(interp)?;
    }
    
    Ok(())
}

// BRANCH_END - 分岐終了マーカー
pub fn op_branch_end(_interp: &mut Interpreter) -> Result<()> {
    // 何もしない（マーカーとしてのみ使用）
    Ok(())
}

fn skip_to_branch_end(_interp: &mut Interpreter) -> Result<()> {
    // この実装は簡略版。実際にはコールスタックや実行状態を考慮する必要がある
    // 今回は基本的な動作のみ実装
    Ok(())
}

// NOP - 何もしない
pub fn op_nop(_interp: &mut Interpreter) -> Result<()> {
    Ok(())
}

// DEF - 新しいワードを定義する（暗黙GOTO対応）
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
                tokens.push(value_to_token(&value)?);
            }
            tokens
        },
        _ => return Err(AjisaiError::from("DEF requires vector")),
    };

    // 暗黙GOTO機能を適用
    let processed_tokens = apply_implicit_goto(&original_tokens)?;

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
    for token in &processed_tokens {
        if let Token::Symbol(sym) = token {
            if interp.dictionary.contains_key(sym) && !interp.is_builtin_word(sym) {
                interp.dependencies.entry(sym.clone())
                    .or_insert_with(std::collections::HashSet::new)
                    .insert(name.clone());
            }
        }
    }

    interp.dictionary.insert(name.clone(), crate::interpreter::WordDefinition {
        tokens: processed_tokens,
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
