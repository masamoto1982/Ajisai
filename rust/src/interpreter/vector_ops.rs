use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, Fraction};

// 既存の操作
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
                Err(AjisaiError::from("頭: 空のベクトルです"))
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
                Err(AjisaiError::from("尾: 空のベクトルです"))
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

// 新機能: UNCONS（離）
pub fn op_uncons(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match val.val_type {
        ValueType::Vector(v) => {
            if v.is_empty() {
                return Err(AjisaiError::from("離: 空のベクトルです"));
            }
            interp.workspace.push(v[0].clone());  // 先頭要素
            interp.workspace.push(Value { 
                val_type: ValueType::Vector(v[1..].to_vec()) 
            });  // 残り
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

// 新機能: REMOVE_LAST（除）
pub fn op_remove_last(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match val.val_type {
        ValueType::Vector(mut v) => {
            if v.is_empty() {
                return Err(AjisaiError::from("除: 空のベクトルです"));
            }
            let last_elem = v.pop().unwrap();
            interp.workspace.push(Value { val_type: ValueType::Vector(v) });  // 残り
            interp.workspace.push(last_elem);  // 除去した要素
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 新機能: CLONE（複）
pub fn op_clone(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.last()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    interp.workspace.push(val.clone());
    Ok(())
}

// 新機能: SELECT（選）
pub fn op_select(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 3 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let condition = interp.workspace.pop().unwrap();
    let b = interp.workspace.pop().unwrap();
    let a = interp.workspace.pop().unwrap();
    
    let result = match condition.val_type {
        ValueType::Boolean(true) => a,
        ValueType::Boolean(false) => b,
        ValueType::Nil => b,
        _ => a,  // nilでもfalseでもない場合はtrueとして扱う
    };
    
    interp.workspace.push(result);
    Ok(())
}

// 新機能: COUNT（数）- ワークスペースまたはベクトルの要素数
pub fn op_count(interp: &mut Interpreter) -> Result<()> {
    if let Some(val) = interp.workspace.last() {
        match &val.val_type {
            ValueType::Vector(v) => {
                // ベクトルの要素数
                let count = Value { 
                    val_type: ValueType::Number(Fraction::new(v.len() as i64, 1)) 
                };
                interp.workspace.push(count);
                Ok(())
            },
            _ => {
                // ワークスペース全体の要素数
                let count = Value { 
                    val_type: ValueType::Number(Fraction::new(interp.workspace.len() as i64, 1)) 
                };
                interp.workspace.push(count);
                Ok(())
            }
        }
    } else {
        // 空のワークスペース
        interp.workspace.push(Value { 
            val_type: ValueType::Number(Fraction::new(0, 1)) 
        });
        Ok(())
    }
}

// 新機能: AT（在）- 位置アクセス
pub fn op_at(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let target = interp.workspace.pop().unwrap();
    let index_val = interp.workspace.pop().unwrap();
    
    let index = match index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(AjisaiError::type_error("integer", "other type")),
    };
    
    match target.val_type {
        ValueType::Vector(v) => {
            let actual_index = if index < 0 { 
                v.len() as i64 + index 
            } else { 
                index 
            };
            
            if actual_index >= 0 && (actual_index as usize) < v.len() {
                interp.workspace.push(v[actual_index as usize].clone());
                Ok(())
            } else {
                Err(AjisaiError::IndexOutOfBounds {
                    index,
                    length: v.len(),
                })
            }
        },
        _ => {
            // ワークスペースへのアクセス（PICKと同様）
            if index >= 0 && (index as usize) < interp.workspace.len() {
                let item = interp.workspace[interp.workspace.len() - 1 - index as usize].clone();
                interp.workspace.push(item);
                Ok(())
            } else {
                Err(AjisaiError::IndexOutOfBounds {
                    index,
                    length: interp.workspace.len(),
                })
            }
        }
    }
}

// 新機能: DO（行）- 統一実行
pub fn op_do(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match val.val_type {
        ValueType::Vector(v) => {
            // ベクトルをコードとして実行
            let tokens = interp.vector_to_tokens(v)?;
            interp.execute_tokens(&tokens)
        },
        _ => {
            // 値を出力
            interp.append_output(&format!("{}", val));
            Ok(())
        }
    }
}
