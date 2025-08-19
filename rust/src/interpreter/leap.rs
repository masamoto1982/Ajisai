// rust/src/interpreter/leap.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType};

pub fn op_leap(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let label_val = interp.stack.pop().unwrap();
    let condition_val = interp.stack.pop().unwrap();
    
    let label = match label_val.val_type {
        ValueType::String(s) => s,
        _ => return Err(AjisaiError::type_error("string", "other type")),
    };
    
    let should_leap = match condition_val.val_type {
        ValueType::Boolean(b) => b,
        ValueType::Nil => false,
        _ => true, // その他の値は真として扱う
    };
    
    if should_leap {
        if let Some(&target_pc) = interp.labels.get(&label) {
            interp.pc = target_pc.saturating_sub(1); // execute_token末尾の+1を相殺
        } else {
            return Err(AjisaiError::from(format!("Unknown label: {}", label)));
        }
    }
    
    Ok(())
}
