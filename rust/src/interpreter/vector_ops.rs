use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, Fraction};

pub fn op_length(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match val.val_type {
        ValueType::Vector(v) => {
            interp.workspace.push(Value { 
                val_type: ValueType::Number(Fraction::new(v.len() as i64, 1)) 
            });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

pub fn op_head(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match val.val_type {
        ValueType::Vector(v) => {
            if let Some(first) = v.first() {
                interp.workspace.push(first.clone());
                Ok(())
            } else {
                Err(AjisaiError::from("HEAD on empty vector"))
            }
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

pub fn op_tail(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match val.val_type {
        ValueType::Vector(v) => {
            if v.is_empty() {
                Err(AjisaiError::from("TAIL on empty vector"))
            } else {
                interp.workspace.push(Value { 
                    val_type: ValueType::Vector(v[1..].to_vec()) 
                });
                Ok(())
            }
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

pub fn op_cons(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let vec_val = interp.workspace.pop().unwrap();
    let elem = interp.workspace.pop().unwrap();
    
    match vec_val.val_type {
        ValueType::Vector(mut v) => {
            v.insert(0, elem);
            interp.workspace.push(Value { val_type: ValueType::Vector(v) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

pub fn op_append(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let elem = interp.workspace.pop().unwrap();
    let vec_val = interp.workspace.pop().unwrap();
    
    match vec_val.val_type {
        ValueType::Vector(mut v) => {
            v.push(elem);
            interp.workspace.push(Value { val_type: ValueType::Vector(v) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

pub fn op_reverse(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match val.val_type {
        ValueType::Vector(mut v) => {
            v.reverse();
            interp.workspace.push(Value { val_type: ValueType::Vector(v) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

pub fn op_nth(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let vec_val = interp.workspace.pop().unwrap();
    let index_val = interp.workspace.pop().unwrap();
    
    match (index_val.val_type, vec_val.val_type) {
        (ValueType::Number(n), ValueType::Vector(v)) => {
            if n.denominator != 1 {
                return Err(AjisaiError::from("NTH requires an integer index"));
            }
            
            let index = if n.numerator < 0 { 
                v.len() as i64 + n.numerator 
            } else { 
                n.numerator 
            };
            
            if index >= 0 && (index as usize) < v.len() {
                interp.workspace.push(v[index as usize].clone());
                Ok(())
            } else {
                Err(AjisaiError::IndexOutOfBounds {
                    index: n.numerator,
                    length: v.len(),
                })
            }
        },
        _ => Err(AjisaiError::type_error("number and vector", "other types")),
    }
}

pub fn op_uncons(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match val.val_type {
        ValueType::Vector(v) => {
            if v.is_empty() {
                return Err(AjisaiError::from("UNCONS on empty vector"));
            }
            interp.workspace.push(v[0].clone());
            interp.workspace.push(Value { 
                val_type: ValueType::Vector(v[1..].to_vec()) 
            });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

pub fn op_empty(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match val.val_type {
        ValueType::Vector(v) => {
            interp.workspace.push(Value { 
                val_type: ValueType::Boolean(v.is_empty()) 
            });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 新機能：ベクトルをコードとして実行
pub fn op_exec(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match val.val_type {
        ValueType::Vector(v) => {
            let tokens = interp.vector_to_tokens(v)?;
            interp.execute_tokens(&tokens)
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}
