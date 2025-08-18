// rust/src/interpreter/goto.rs (新規作成)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType};

pub fn op_goto(interp: &mut Interpreter) -> Result<()> {
    let label_val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    let label = match label_val.val_type {
        ValueType::String(s) => s,
        _ => return Err(AjisaiError::type_error("string", "other type")),
    };
    
    if let Some(&target_pc) = interp.labels.get(&label) {
        interp.pc = target_pc;
        Ok(())
    } else {
        Err(AjisaiError::from(format!("Unknown label: {}", label)))
    }
}

pub fn op_jump_if(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let offset_val = interp.stack.pop().unwrap();
    let condition_val = interp.stack.pop().unwrap();
    
    let offset = match offset_val.val_type {
        ValueType::Number(n) => {
            if n.denominator != 1 {
                return Err(AjisaiError::from("J requires integer offset"));
            }
            n.numerator
        },
        _ => return Err(AjisaiError::type_error("number", "other type")),
    };
    
    let should_jump = match condition_val.val_type {
        ValueType::Boolean(b) => b,
        ValueType::Nil => false,
        _ => true,
    };
    
    if should_jump {
        let new_pc = if offset >= 0 {
            interp.pc.saturating_add(offset as usize)
        } else {
            interp.pc.saturating_sub((-offset) as usize)
        };
        
        if new_pc < interp.program.len() {
            interp.pc = new_pc;
        } else {
            interp.pc = interp.program.len(); // プログラム終了
        }
    }
    
    Ok(())
}
