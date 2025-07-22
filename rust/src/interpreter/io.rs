use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::ValueType;

pub fn op_dot(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    interp.append_output(&format!("{} ", val));
    Ok(())
}

pub fn op_print(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.last()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    interp.append_output(&format!("{} ", val));
    Ok(())
}

pub fn op_cr(interp: &mut Interpreter) -> Result<()> {
    interp.append_output("\n");
    Ok(())
}

pub fn op_space(interp: &mut Interpreter) -> Result<()> {
    interp.append_output(" ");
    Ok(())
}

pub fn op_spaces(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    match val.val_type {
        ValueType::Number(n) => {
            if n.denominator == 1 && n.numerator >= 0 {
                interp.append_output(&" ".repeat(n.numerator as usize));
                Ok(())
            } else {
                Err(AjisaiError::from("SPACES requires a non-negative integer"))
            }
        },
        _ => Err(AjisaiError::type_error("number", "other type")),
    }
}

pub fn op_emit(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    match val.val_type {
        ValueType::Number(n) => {
            if n.denominator == 1 && n.numerator >= 0 && n.numerator <= 255 {
                interp.append_output(&(n.numerator as u8 as char).to_string());
                Ok(())
            } else {
                Err(AjisaiError::from("EMIT requires an integer between 0 and 255"))
            }
        },
        _ => Err(AjisaiError::type_error("number", "other type")),
    }
}


// ワイルドカード関数（簡易実装）
pub fn op_match(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let pattern = interp.stack.pop().unwrap();
    let value = interp.stack.pop().unwrap();
    
    match (value.val_type, pattern.val_type) {
        (ValueType::String(s), ValueType::String(p)) => {
            let result = wildcard_match(&s, &p);
            interp.stack.push(crate::types::Value { 
                val_type: ValueType::Boolean(result) 
            });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("two strings", "other types")),
    }
}

pub fn op_wildcard(_interp: &mut Interpreter) -> Result<()> {
    // パターンをそのまま使うので、特に処理は不要
    Ok(())
}

// ヘルパー関数
fn wildcard_match(text: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return text.is_empty();
    }
    
    if !pattern.contains('*') && !pattern.contains('?') {
        return text == pattern;
    }
    
    // 簡易実装
    let pattern_without_wildcards = pattern.replace("*", "").replace("?", "");
    text.contains(&pattern_without_wildcards)
}