// rust/src/interpreter/control.rs (ビルドエラー修正版)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType, Token, Value, BracketType};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct ConditionalLine {
    pub condition: Option<Vec<Token>>,  // None = デフォルト行
    pub action: Vec<Token>,
}

#[derive(Debug, Clone)]
pub struct RepeatDefinition {
    pub repeat_count: Option<i64>,  // None = 無制限（危険なので使わない）
    pub conditional_lines: Vec<ConditionalLine>,
}

// REPEAT構文の解析とトークン生成
pub fn create_repeat_execution_tokens(repeat_count: Option<i64>, lines: &[Vec<Token>]) -> Result<Vec<Token>> {
    let conditional_lines = parse_conditional_lines(lines)?;
    
    if conditional_lines.is_empty() {
        return Err(AjisaiError::from("No lines found"));
    }
    
    // デフォルト行（条件なし行）の存在チェック
    let has_default = conditional_lines.iter().any(|line| line.condition.is_none());
    if !has_default {
        return Err(AjisaiError::from("Default line (line without condition) is required for safety"));
    }
    
    // REPEAT実行用トークンを生成
    Ok(build_repeat_execution_tokens(repeat_count, &conditional_lines))
}

fn build_repeat_execution_tokens(repeat_count: Option<i64>, lines: &[ConditionalLine]) -> Vec<Token> {
    let mut result = Vec::new();
    
    // 回数制限を設定（デフォルトは1回）
    let count = repeat_count.unwrap_or(1);
    result.push(Token::Number(count, 1));
    
    // 条件行を順番に処理
    for line in lines {
        if let Some(condition) = &line.condition {
            // 条件付き行: [ condition action ]
            result.push(Token::VectorStart(BracketType::Square));
            result.extend(condition.iter().cloned());
            result.extend(line.action.iter().cloned());
            result.push(Token::VectorEnd(BracketType::Square));
        } else {
            // デフォルト行: [ action ]
            result.push(Token::VectorStart(BracketType::Square));
            result.extend(line.action.iter().cloned());
            result.push(Token::VectorEnd(BracketType::Square));
        }
    }
    
    // REPEAT実行ワードを追加
    result.push(Token::Symbol("EXECUTE_REPEAT".to_string()));
    result
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
        if tokens.is_empty() {
            return Err(AjisaiError::from("Empty default line"));
        }
        
        Ok(ConditionalLine {
            condition: None,
            action: tokens.to_vec(),
        })
    }
}

// EXECUTE_REPEAT - REPEAT構文の実行エンジン（簡易版）
pub fn op_execute_repeat(interp: &mut Interpreter) -> Result<()> {
    // より簡単な実装：スタックから情報を取得して実行
    let mut action_vectors = Vec::new();
    
    // 回数制限を取得
    let repeat_count_val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    let repeat_count = match repeat_count_val.val_type {
        ValueType::Vector(ref v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) if n.denominator == 1 => n.numerator,
                _ => return Err(AjisaiError::type_error("integer repeat count", "other type")),
            }
        },
        _ => return Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    };
    
    if repeat_count < 0 {
        return Err(AjisaiError::from("Repeat count must be non-negative"));
    }
    
    // すべてのアクションベクターを収集（逆順で取得）
    while let Some(val) = interp.workspace.pop() {
        match val.val_type {
            ValueType::Vector(action_values, _) => {
                action_vectors.push(action_values);
            },
            _ => {
                // Vector以外が来た場合、処理を終了
                interp.workspace.push(val); // 戻す
                break;
            }
        }
    }
    
    // 取得順序を反転（最初に積まれたものが最初に処理されるように）
    action_vectors.reverse();
    
    if action_vectors.is_empty() {
        return Err(AjisaiError::from("No action vectors found"));
    }
    
    // 最後のアクションがデフォルト行（条件なし）
    let default_action = action_vectors.pop().unwrap();
    
    // 残りが条件付きアクション
    let conditions_and_actions = action_vectors;
    
    // REPEAT実行ループ
    for _iteration in 0..repeat_count {
        let mut executed = false;
        
        // 各条件を順番にチェック
        for condition_action in &conditions_and_actions {
            if condition_action.len() < 2 {
                continue; // 条件とアクションの両方が必要
            }
            
            // 条件部分とアクション部分を分離（簡単な実装：半分ずつ）
            let mid = condition_action.len() / 2;
            
            // 条件部分を評価
            let condition_values = &condition_action[..mid];
            let action_values = &condition_action[mid..];
            
            // 条件をトークンに変換して評価
            let condition_tokens = values_to_tokens(condition_values)?;
            let condition_result = evaluate_condition(interp, &condition_tokens)?;
            
            fn is_truthy(value: &Value) -> bool {
    match &value.val_type {
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
    }
}
        }
        
        if !executed {
            // どの条件も満たさない場合、デフォルト行を実行
            let default_tokens = values_to_tokens(&default_action)?;
            execute_action_tokens(interp, &default_tokens)?;
            break; // デフォルト行実行後は終了
        }
    }
    
    Ok(())
}

fn values_to_tokens(values: &[Value]) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    for value in values {
        tokens.push(value_to_token(value.clone())?);
    }
    Ok(tokens)
}

fn evaluate_condition(interp: &mut Interpreter, condition_tokens: &[Token]) -> Result<Value> {
    // 現在のワークスペースを保存
    let saved_workspace = interp.workspace.clone();
    
    // 条件を実行
    interp.execute_tokens(condition_tokens)?;
    
    // 結果を取得
    let result = if interp.workspace.is_empty() {
        Value { val_type: ValueType::Boolean(false) }
    } else {
        interp.workspace.pop().unwrap()
    };
    
    // ワークスペースを復元
    interp.workspace = saved_workspace;
    
    Ok(result)
}

fn execute_action_tokens(interp: &mut Interpreter, action_tokens: &[Token]) -> Result<()> {
    interp.execute_tokens(action_tokens)
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

fn value_to_token(value: Value) -> Result<Token> {
    match value.val_type {
        ValueType::Number(frac) => Ok(Token::Number(frac.numerator, frac.denominator)),
        ValueType::String(s) => Ok(Token::String(s)),
        ValueType::Boolean(b) => Ok(Token::Boolean(b)),
        ValueType::Symbol(s) => Ok(Token::Symbol(s)),
        ValueType::Nil => Ok(Token::Nil),
        ValueType::Vector(_, _) => {
            Err(AjisaiError::from("Cannot convert vector to token directly"))
        },
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


fn is_truthy(value: &Value) -> bool {
    match &value.val_type {
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
    }
}

fn vector_to_tokens(values: Vec<Value>) -> Result<Vec<Token>> {
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
    Ok(tokens)
}

// DEF - 新しいワードを定義する
pub fn op_def(interp: &mut Interpreter) -> Result<()> {
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
