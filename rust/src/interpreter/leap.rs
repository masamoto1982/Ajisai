// rust/src/interpreter/leap.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType};

pub fn op_leap(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let target_val = interp.workspace.pop().unwrap();
    let condition_val = interp.workspace.pop().unwrap();
    
    let should_leap = match condition_val.val_type {
        ValueType::Boolean(b) => b,
        ValueType::Nil => false,
        _ => true,
    };
    
    if should_leap {
        match target_val.val_type {
            ValueType::String(word_name) => {
                // 現在実行中のワード名を取得
                let current_word = interp.call_stack.last().cloned();
                
                // 同一ワード内制限付きでワードを実行
                interp.execute_word_leap(&word_name, current_word.as_deref())?;
            },
            ValueType::Vector(code_vec) => {
                // 直接コードベクトルを実行（新機能）
                let tokens = interp.vector_to_tokens(code_vec)?;
                interp.execute_tokens(&tokens)?;
            },
            _ => return Err(AjisaiError::type_error("string or vector", "other type")),
        }
    }
    
    Ok(())
}
