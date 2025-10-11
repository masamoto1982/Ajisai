// rust/src/interpreter/eval.rs

use crate::interpreter::{Interpreter, OperationTarget, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType};
use std::collections::HashSet;
use num_bigint::BigInt;
use num_traits::One;

pub fn op_eval(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => eval_stacktop(interp),
        OperationTarget::Stack => eval_stack(interp),
    }
}

fn eval_stacktop(interp: &mut Interpreter) -> Result<()> {
    // スタックトップのベクトルを取得
    let code_vec = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    
    // ベクトル内の要素を連結してコード文字列に変換
    let code_string = vector_to_code_string(&code_vec)?;
    
    // 現在のスタック状態を保存
    let saved_stack = interp.stack.clone();
    
    // 評価用の新しいスタックで実行
    interp.stack.clear();
    
    // コードをトークナイズして実行
    execute_code(interp, &code_string)?;
    
    // 結果を取得
    let result = interp.stack.clone();
    
    // 元のスタックを復元して結果を追加
    interp.stack = saved_stack;
    interp.stack.extend(result);
    
    Ok(())
}

fn eval_stack(interp: &mut Interpreter) -> Result<()> {
    // スタック上の全ベクトルを連結
    let all_vecs = std::mem::take(&mut interp.stack);
    
    let code_parts: Result<Vec<String>> = all_vecs.iter()
        .map(vector_to_code_string)
        .collect();
    
    let code_string = code_parts?.join(" ");
    
    // 実行（スタックは既にクリア済み）
    execute_code(interp, &code_string)?;
    
    Ok(())
}

fn vector_to_code_string(vec: &Value) -> Result<String> {
    match &vec.val_type {
        ValueType::Vector(elements, _) => {
            let parts: Result<Vec<String>> = elements.iter()
                .map(|elem| match &elem.val_type {
                    ValueType::String(s) => Ok(s.clone()),
                    ValueType::Number(n) => {
                        if n.denominator == BigInt::one() {
                            Ok(n.numerator.to_string())
                        } else {
                            Ok(format!("{}/{}", n.numerator, n.denominator))
                        }
                    },
                    ValueType::Symbol(s) => Ok(s.clone()),
                    ValueType::Boolean(b) => Ok(if *b { "TRUE".to_string() } else { "FALSE".to_string() }),
                    ValueType::Nil => Ok("NIL".to_string()),
                    ValueType::Vector(_, bracket_type) => {
                        // ネストしたベクトルはそのまま文字列化
                        Ok(format!("{}", elem))
                    },
                    _ => Err(AjisaiError::from("EVAL cannot convert this element type to code")),
                })
                .collect();
            
            Ok(parts?.join(" "))
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

fn execute_code(interp: &mut Interpreter, code: &str) -> Result<()> {
    // DEF処理
    if code.contains(" DEF") {
        return crate::interpreter::control::parse_multiple_word_definitions(interp, code);
    }
    
    // カスタムワード名を取得
    let custom_word_names: HashSet<String> = interp.dictionary.iter()
        .filter(|(_, def)| !def.is_builtin)
        .map(|(name, _)| name.clone())
        .collect();
    
    // トークナイズ
    let tokens = crate::tokenizer::tokenize_with_custom_words(code, &custom_word_names)
        .map_err(|e| AjisaiError::from(format!("EVAL tokenization error: {}", e)))?;
    
    // 実行
    interp.execute_tokens_sync(&tokens)
}
