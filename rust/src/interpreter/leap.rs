// rust/src/interpreter/leap.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType};

pub fn op_leap(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.is_empty() {
        return Err(AjisaiError::StackUnderflow);
    }
    
    // スタックサイズで無条件/条件付きを判定
    let (should_leap, offset) = if interp.stack.len() == 1 {
        // 無条件ジャンプ: offset LEAP
        let offset_val = interp.stack.pop().unwrap();
        let offset = match offset_val.val_type {
            ValueType::Number(n) => {
                if n.denominator != 1 {
                    return Err(AjisaiError::from("LEAP requires integer offset"));
                }
                n.numerator
            },
            _ => return Err(AjisaiError::type_error("number", "other type")),
        };
        (true, offset)
    } else {
        // 条件付きジャンプ: condition offset LEAP
        let offset_val = interp.stack.pop().unwrap();
        let condition_val = interp.stack.pop().unwrap();
        
        let offset = match offset_val.val_type {
            ValueType::Number(n) => {
                if n.denominator != 1 {
                    return Err(AjisaiError::from("LEAP requires integer offset"));
                }
                n.numerator
            },
            _ => return Err(AjisaiError::type_error("number", "other type")),
        };
        
        let should_leap = match condition_val.val_type {
            ValueType::Boolean(b) => b,
            ValueType::Nil => false,
            _ => true, // その他の値は真として扱う
        };
        
        (should_leap, offset)
    };
    
    if should_leap {
        let new_pc = if offset >= 0 {
            interp.pc.saturating_add(offset as usize)
        } else {
            interp.pc.saturating_sub((-offset) as usize)
        };
        
        if new_pc < interp.program.len() {
            interp.pc = new_pc.saturating_sub(1); // execute_token末尾の+1を相殺
        } else {
            interp.pc = interp.program.len(); // プログラム終了
        }
    }
    
    Ok(())
}
