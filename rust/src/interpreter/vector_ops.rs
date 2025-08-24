// rust/src/interpreter/vector_ops.rs (新しいワード体系に対応)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, Fraction};

// 新しいワード体系の実装

// 頁 - インデックス指定（スタックに指定情報を残す）
pub fn op_page(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let index_val = interp.workspace.pop().unwrap();
    let vector_val = interp.workspace.last().unwrap().clone();
    
    let index = match index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(AjisaiError::type_error("integer", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(ref v) => {
            let actual_index = if index < 0 {
                v.len() as i64 + index
            } else {
                index
            };
            
            if actual_index < 0 || actual_index >= v.len() as i64 {
                return Err(AjisaiError::IndexOutOfBounds {
                    index,
                    length: v.len(),
                });
            }
            
            // インデックス情報をスタックに残す（後続の操作で使用）
            interp.workspace.push(Value { 
                val_type: ValueType::Number(Fraction::new(actual_index, 1))
            });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 頁数 - 長さ指定（スタックに長さ情報を残す）
pub fn op_page_count(interp: &mut Interpreter) -> Result<()> {
    let vector_val = interp.workspace.last()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match vector_val.val_type {
        ValueType::Vector(ref v) => {
            interp.workspace.push(Value { 
                val_type: ValueType::Number(Fraction::new(v.len() as i64, 1))
            });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 取得 - 指定された対象を取得
pub fn op_get(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let target_val = interp.workspace.pop().unwrap();
    let vector_val = interp.workspace.pop().unwrap();
    
    match (&vector_val.val_type, &target_val.val_type) {
        (ValueType::Vector(v), ValueType::Number(n)) if n.denominator == 1 => {
            let index = n.numerator;
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
        _ => Err(AjisaiError::type_error("vector and index", "other types")),
    }
}

// 挿入 - 指定位置に要素を挿入
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

// 置換 - 指定位置の要素を置き換え
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

// 削除 - 指定位置の要素を削除
pub fn op_delete(interp: &mut Interpreter) -> Result<()> {
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
}

// 合併 - 2つのベクトルを結合
pub fn op_merge(interp: &mut Interpreter) -> Result<()> {
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

// 分離 - ベクトルを指定位置で分離
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
