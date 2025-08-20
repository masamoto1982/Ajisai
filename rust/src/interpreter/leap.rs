// rust/src/interpreter/leap.rs (デバッグ版)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType};

pub fn op_leap(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let word_name_val = interp.stack.pop().unwrap();
    let condition_val = interp.stack.pop().unwrap();
    
    let word_name = match word_name_val.val_type {
        ValueType::String(s) => s,
        _ => return Err(AjisaiError::type_error("string", "other type")),
    };
    
    let should_leap = match condition_val.val_type {
        ValueType::Boolean(b) => b,
        ValueType::Nil => false,
        _ => true,
    };
    
    if should_leap {
        // 現在実行中のワード名を取得
        let current_word = interp.call_stack.last().cloned();
        
        // デバッグ出力
        interp.append_output(&format!("DEBUG: LEAP from {:?} to {}\n", current_word, word_name));
        
        // 同一ワード内制限付きでワードを実行
        interp.execute_word_leap(&word_name, current_word.as_deref())?;
    }
    
    Ok(())
}
