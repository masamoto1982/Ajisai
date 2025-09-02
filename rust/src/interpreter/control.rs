// rust/src/interpreter/control.rs (事前評価方式実装)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType, Token, Value};

#[derive(Debug, Clone)]
pub struct ConditionalBlock {
    pub condition: Option<Vec<Token>>,  // None = デフォルト
    pub action: Vec<Token>,
}

// 暗黙GOTO - ワード定義時に条件分岐処理を適用
pub fn apply_implicit_goto(tokens: &[Token]) -> Result<Vec<Token>> {
    let blocks = parse_conditional_blocks(tokens)?;
    
    if blocks.is_empty() {
        return Ok(tokens.to_vec());
    }
    
    if blocks.len() == 1 && blocks[0].condition.is_none() {
        // 単一ブロックで条件なし = そのまま実行
        return Ok(blocks[0].action.clone());
    }
    
    // 複数ブロックまたは条件付きブロック = 条件分岐ワードを生成
    Ok(create_conditional_word_tokens(blocks))
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

fn create_conditional_word_tokens(blocks: Vec<ConditionalBlock>) -> Vec<Token> {
    let mut result = Vec::new();
    
    // 条件分岐データをトークン形式で埋め込み
    result.push(Token::Symbol("EXECUTE_CONDITIONS".to_string()));
    
    // ブロック数
    result.push(Token::Number(blocks.len() as i64, 1));
    
    // 各ブロックのデータ
    for block in blocks {
        if let Some(condition) = block.condition {
            // 条件あり
            result.push(Token::Number(1, 1)); // フラグ: 条件あり
            result.push(Token::Number(condition.len() as i64, 1));
            result.extend(condition);
            result.push(Token::Number(block.action.len() as i64, 1));
            result.extend(block.action);
        } else {
            // デフォルトブロック
            result.push(Token::Number(0, 1)); // フラグ: 条件なし
            result.push(Token::Number(block.action.len() as i64, 1));
            result.extend(block.action);
        }
    }
    
    result
}

// EXECUTE_CONDITIONS - 事前評価方式の条件分岐実行
pub fn op_execute_conditions(interp: &mut Interpreter) -> Result<()> {
    // ブロック数を取得
    let block_count_val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    let block_count = match block_count_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator as usize,
        _ => return Err(AjisaiError::from("Invalid block count")),
    };
    
    // 各ブロックを順次評価
    for _ in 0..block_count {
        let has_condition_val = interp.workspace.pop()
            .ok_or(AjisaiError::WorkspaceUnderflow)?;
        
        let has_condition = match has_condition_val.val_type {
            ValueType::Number(n) if n.denominator == 1 => n.numerator != 0,
            _ => return Err(AjisaiError::from("Invalid condition flag")),
        };
        
        if has_condition {
            // 条件ありブロック
            let condition_len_val = interp.workspace.pop()
                .ok_or(AjisaiError::WorkspaceUnderflow)?;
            
            let condition_len = match condition_len_val.val_type {
                ValueType::Number(n) if n.denominator == 1 => n.numerator as usize,
                _ => return Err(AjisaiError::from("Invalid condition length")),
            };
            
            // 条件トークンを取得
            let mut condition_tokens = Vec::new();
            for _ in 0..condition_len {
                let token_val = interp.workspace.pop()
                    .ok_or(AjisaiError::WorkspaceUnderflow)?;
                condition_tokens.push(value_to_token(&token_val)?);
            }
            condition_tokens.reverse(); // スタックなので逆順
            
            let action_len_val = interp.workspace.pop()
                .ok_or(AjisaiError::WorkspaceUnderflow)?;
            
            let action_len = match action_len_val.val_type {
                ValueType::Number(n) if n.denominator == 1 => n.numerator as usize,
                _ => return Err(AjisaiError::from("Invalid action length")),
            };
            
            // アクショントークンを取得
            let mut action_tokens = Vec::new();
            for _ in 0..action_len {
                let token_val = interp.workspace.pop()
                    .ok_or(AjisaiError::WorkspaceUnderflow)?;
                action_tokens.push(value_to_token(&token_val)?);
            }
            action_tokens.reverse(); // スタックなので逆順
            
            // 条件を評価
            let original_len = interp.workspace.len();
            interp.execute_tokens(&condition_tokens)?;
            
            // 条件の結果を取得
            if interp.workspace.len() > original_len {
                let condition_result = interp.workspace.pop().unwrap();
                
                if is_truthy(&condition_result) {
                    // 条件が真なら処理を実行して終了
                    interp.execute_tokens(&action_tokens)?;
                    return Ok(());
                }
            }
            
        } else {
            // デフォルトブロック
            let action_len_val = interp.workspace.pop()
                .ok_or(AjisaiError::WorkspaceUnderflow)?;
            
            let action_len = match action_len_val.val_type {
                ValueType::Number(n) if n.denominator == 1 => n.numerator as usize,
                _ => return Err(AjisaiError::from("Invalid action length")),
            };
            
            // アクショントークンを取得
            let mut action_tokens = Vec::new();
            for _ in 0..action_len {
                let token_val = interp.workspace.pop()
                    .ok_or(AjisaiError::WorkspaceUnderflow)?;
                action_tokens.push(value_to_token(&token_val)?);
            }
            action_tokens.reverse(); // スタックなので逆順
            
            // デフォルト処理を実行して終了
            interp.execute_tokens(&action_tokens)?;
            return Ok(());
        }
    }
    
    // すべての条件が偽の場合は何もしない
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
