// rust/src/interpreter/vector_ops.rs (統合版)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, Fraction};

// 摘妖精 - 文脈判定による取得操作
pub fn op_get(interp: &mut Interpreter) -> Result<()> {
    match interp.workspace.len() {
        0 => Err(AjisaiError::WorkspaceUnderflow),
        1 => {
            // Vector単体の場合はエラー（インデックスが必要）
            Err(AjisaiError::from("摘 requires index for element access"))
        },
        _ => {
            // スタック上位2つを確認してインデックス付きアクセス
            let index_val = interp.workspace.pop().unwrap();
            let target_val = interp.workspace.pop().unwrap();
            
            let index = match index_val.val_type {
                ValueType::Number(n) if n.denominator == 1 => n.numerator,
                _ => return Err(AjisaiError::type_error("integer index", "other type")),
            };
            
            match target_val.val_type {
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
                _ => Err(AjisaiError::type_error("vector", "other type")),
            }
        }
    }
}

// 数妖精 - 文脈判定による計数操作
pub fn op_length(interp: &mut Interpreter) -> Result<()> {
    let target_val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match target_val.val_type {
        ValueType::Vector(v) => {
            interp.workspace.push(Value { 
                val_type: ValueType::Number(Fraction::new(v.len() as i64, 1))
            });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 挿妖精 - 文脈判定による挿入操作
pub fn op_insert(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 3 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let element = interp.workspace.pop().unwrap();
    let index_val = interp.workspace.pop().unwrap();
    let vector_val = interp.workspace.pop().unwrap();
    
    let index = match index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(AjisaiError::type_error("integer", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(mut v) => {
            let insert_index = if index < 0 {
                0
            } else if index as usize > v.len() {
                v.len()
            } else {
                index as usize
            };
            
            v.insert(insert_index, element);
            interp.workspace.push(Value { val_type: ValueType::Vector(v) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 換妖精 - 文脈判定による置換操作
pub fn op_replace(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 3 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let new_element = interp.workspace.pop().unwrap();
    let index_val = interp.workspace.pop().unwrap();
    let vector_val = interp.workspace.pop().unwrap();
    
    let index = match index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(AjisaiError::type_error("integer", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(mut v) => {
            let actual_index = if index < 0 {
                v.len() as i64 + index
            } else {
                index
            };
            
            if actual_index >= 0 && (actual_index as usize) < v.len() {
                let old_element = std::mem::replace(&mut v[actual_index as usize], new_element);
                interp.workspace.push(Value { val_type: ValueType::Vector(v) });
                interp.workspace.push(old_element);
                Ok(())
            } else {
                Err(AjisaiError::IndexOutOfBounds {
                    index,
                    length: v.len(),
                })
            }
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 削妖精 - 文脈判定による削除操作
pub fn op_remove(interp: &mut Interpreter) -> Result<()> {
    match interp.workspace.len() {
        0 => Err(AjisaiError::WorkspaceUnderflow),
        1 => {
            // 単一値の場合は破棄
            interp.workspace.pop().unwrap();
            Ok(())
        },
        _ => {
            // 2つ以上ある場合、パターンマッチングのための値を先に取得
            let should_remove_by_index = {
                let top = &interp.workspace[interp.workspace.len() - 1];
                let second = &interp.workspace[interp.workspace.len() - 2];
                
                match (&top.val_type, &second.val_type) {
                    (ValueType::Number(n), ValueType::Vector(_)) if n.denominator == 1 => {
                        Some(n.numerator)
                    },
                    _ => None,
                }
            };
            
            if let Some(index) = should_remove_by_index {
                // インデックス付きVector削除
                interp.workspace.pop().unwrap(); // indexを破棄
                let vector_val = interp.workspace.pop().unwrap();
                
                match vector_val.val_type {
                    ValueType::Vector(mut v) => {
                        let actual_index = if index < 0 {
                            v.len() as i64 + index
                        } else {
                            index
                        };
                        
                        if actual_index >= 0 && (actual_index as usize) < v.len() {
                            let removed = v.remove(actual_index as usize);
                            interp.workspace.push(Value { val_type: ValueType::Vector(v) });
                            interp.workspace.push(removed);
                            Ok(())
                        } else {
                            Err(AjisaiError::IndexOutOfBounds {
                                index,
                                length: v.len(),
                            })
                        }
                    },
                    _ => Err(AjisaiError::type_error("vector", "other type")),
                }
            } else {
                // パターンに該当しない場合は破棄
                interp.workspace.pop().unwrap();
                Ok(())
            }
        }
    }
}

// 結妖精 - Vector結合操作
pub fn op_concat(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let vec2_val = interp.workspace.pop().unwrap();
    let vec1_val = interp.workspace.pop().unwrap();
    
    match (vec1_val.val_type, vec2_val.val_type) {
        (ValueType::Vector(mut v1), ValueType::Vector(v2)) => {
            v1.extend(v2);
            interp.workspace.push(Value { val_type: ValueType::Vector(v1) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector vector", "other types")),
    }
}

// 分妖精 - Vector分離操作
pub fn op_split(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let index_val = interp.workspace.pop().unwrap();
    let vector_val = interp.workspace.pop().unwrap();
    
    let index = match index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(AjisaiError::type_error("integer", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(v) => {
            let split_index = if index < 0 {
                (v.len() as i64 + index).max(0) as usize
            } else {
                (index as usize).min(v.len())
            };
            
            let (left, right) = v.split_at(split_index);
            interp.workspace.push(Value { val_type: ValueType::Vector(left.to_vec()) });
            interp.workspace.push(Value { val_type: ValueType::Vector(right.to_vec()) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}
