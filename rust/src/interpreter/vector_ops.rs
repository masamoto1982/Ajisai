// rust/src/interpreter/vector_ops.rs

use crate::interpreter::{Interpreter, error::{LPLError, Result}};
use crate::types::{Value, ValueType, Fraction};

// 頁司書 - 書籍の特定ページを取得（1オリジン）
pub fn op_page(interp: &mut Interpreter) -> Result<()> {
    if interp.bookshelf.len() < 2 {
        return Err(LPLError::BookshelfUnderflow);
    }
    
    let index_val = interp.bookshelf.pop().unwrap();
    let vector_val = interp.bookshelf.pop().unwrap();
    
    let user_index = match index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(LPLError::type_error("integer", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(v) => {
            // 1オリジン → 0オリジン変換
            let internal_index = if user_index < 0 {
                v.len() as i64 + user_index + 1  // 負のインデックス調整
            } else {
                user_index - 1  // 1オリジン → 0オリジン
            };
            
            if internal_index >= 0 && (internal_index as usize) < v.len() {
                interp.bookshelf.push(v[internal_index as usize].clone());
                Ok(())
            } else {
                Err(LPLError::IndexOutOfBounds {
                    index: user_index,
                    length: v.len(),
                })
            }
        },
        _ => Err(LPLError::type_error("vector", "other type")),
    }
}

// 頁数司書 - 書籍の総ページ数を取得
pub fn op_page_count(interp: &mut Interpreter) -> Result<()> {
    let vector_val = interp.bookshelf.pop()
        .ok_or(LPLError::BookshelfUnderflow)?;
    
    match vector_val.val_type {
        ValueType::Vector(v) => {
            interp.bookshelf.push(Value { 
                val_type: ValueType::Number(Fraction::new(v.len() as i64, 1))
            });
            Ok(())
        },
        _ => Err(LPLError::type_error("vector", "other type")),
    }
}

// 冊司書 - 書架から特定の冊（書籍）を取得（1オリジン）
pub fn op_book(interp: &mut Interpreter) -> Result<()> {
    if interp.bookshelf.len() < 2 {
        return Err(LPLError::BookshelfUnderflow);
    }
    
    let index_val = interp.bookshelf.pop().unwrap();
    let vector_val = interp.bookshelf.pop().unwrap();
    
    let user_index = match index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(LPLError::type_error("integer", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(v) => {
            // 1オリジン → 0オリジン変換
            let internal_index = if user_index < 0 {
                v.len() as i64 + user_index + 1  // 負のインデックス調整
            } else {
                user_index - 1  // 1オリジン → 0オリジン
            };
            
            if internal_index >= 0 && (internal_index as usize) < v.len() {
                interp.bookshelf.push(v[internal_index as usize].clone());
                Ok(())
            } else {
                Err(LPLError::IndexOutOfBounds {
                    index: user_index,
                    length: v.len(),
                })
            }
        },
        _ => Err(LPLError::type_error("vector", "other type")),
    }
}

// 挿入司書 - 指定位置にページを挿入（1オリジン）
pub fn op_insert(interp: &mut Interpreter) -> Result<()> {
    if interp.bookshelf.len() < 3 {
        return Err(LPLError::BookshelfUnderflow);
    }
    
    let element = interp.bookshelf.pop().unwrap();
    let index_val = interp.bookshelf.pop().unwrap();
    let vector_val = interp.bookshelf.pop().unwrap();
    
    let user_index = match index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(LPLError::type_error("integer", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(mut v) => {
            // 1オリジン → 0オリジン変換、境界チェック
            let internal_index = if user_index <= 0 {
                0
            } else if user_index as usize > v.len() + 1 {
                v.len()
            } else {
                user_index as usize - 1  // 1オリジン → 0オリジン
            };
            
            v.insert(internal_index, element);
            interp.bookshelf.push(Value { val_type: ValueType::Vector(v) });
            Ok(())
        },
        _ => Err(LPLError::type_error("vector", "other type")),
    }
}

// 置換司書 - 指定位置のページを置換（1オリジン）
pub fn op_replace(interp: &mut Interpreter) -> Result<()> {
    if interp.bookshelf.len() < 3 {
        return Err(LPLError::BookshelfUnderflow);
    }
    
    let new_element = interp.bookshelf.pop().unwrap();
    let index_val = interp.bookshelf.pop().unwrap();
    let vector_val = interp.bookshelf.pop().unwrap();
    
    let user_index = match index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(LPLError::type_error("integer", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(mut v) => {
            // 1オリジン → 0オリジン変換
            let internal_index = if user_index < 0 {
                v.len() as i64 + user_index + 1  // 負のインデックス調整
            } else {
                user_index - 1  // 1オリジン → 0オリジン
            };
            
            if internal_index >= 0 && (internal_index as usize) < v.len() {
                let old_element = std::mem::replace(&mut v[internal_index as usize], new_element);
                interp.bookshelf.push(Value { val_type: ValueType::Vector(v) });
                interp.bookshelf.push(old_element);
                Ok(())
            } else {
                Err(LPLError::IndexOutOfBounds {
                    index: user_index,
                    length: v.len(),
                })
            }
        },
        _ => Err(LPLError::type_error("vector", "other type")),
    }
}

// 削除司書 - 指定位置のページを削除、または要素全体を削除（1オリジン）
pub fn op_delete(interp: &mut Interpreter) -> Result<()> {
    if interp.bookshelf.is_empty() {
        return Err(LPLError::BookshelfUnderflow);
    }
    
    // 引数が1つの場合：要素全体を削除（DROP相当）
    if interp.bookshelf.len() == 1 {
        interp.bookshelf.pop();
        return Ok(());
    }
    
    // 引数が2つの場合：インデックス指定削除
    if interp.bookshelf.len() >= 2 {
        let index_val = interp.bookshelf.pop().unwrap();
        let vector_val = interp.bookshelf.pop().unwrap();
        
        let user_index = match index_val.val_type {
            ValueType::Number(n) if n.denominator == 1 => n.numerator,
            _ => return Err(LPLError::type_error("integer", "other type")),
        };
        
        match vector_val.val_type {
            ValueType::Vector(mut v) => {
                // 1オリジン → 0オリジン変換
                let internal_index = if user_index < 0 {
                    v.len() as i64 + user_index + 1  // 負のインデックス調整
                } else {
                    user_index - 1  // 1オリジン → 0オリジン
                };
                
                if internal_index >= 0 && (internal_index as usize) < v.len() {
                    let removed = v.remove(internal_index as usize);
                    interp.bookshelf.push(Value { val_type: ValueType::Vector(v) });
                    interp.bookshelf.push(removed);
                    Ok(())
                } else {
                    Err(LPLError::IndexOutOfBounds {
                        index: user_index,
                        length: v.len(),
                    })
                }
            },
            _ => Err(LPLError::type_error("vector", "other type")),
        }
    } else {
        Err(LPLError::BookshelfUnderflow)
    }
}

// 合併司書 - 2つの書籍を結合
pub fn op_merge(interp: &mut Interpreter) -> Result<()> {
    if interp.bookshelf.len() < 2 {
        return Err(LPLError::BookshelfUnderflow);
    }
    
    let vec2_val = interp.bookshelf.pop().unwrap();
    let vec1_val = interp.bookshelf.pop().unwrap();
    
    match (vec1_val.val_type, vec2_val.val_type) {
        (ValueType::Vector(mut v1), ValueType::Vector(v2)) => {
            v1.extend(v2);
            interp.bookshelf.push(Value { val_type: ValueType::Vector(v1) });
            Ok(())
        },
        _ => Err(LPLError::type_error("vector vector", "other types")),
    }
}

// 分離司書 - 書籍を2つに分ける（1オリジン）
pub fn op_split(interp: &mut Interpreter) -> Result<()> {
    if interp.bookshelf.len() < 2 {
        return Err(LPLError::BookshelfUnderflow);
    }
    
    let index_val = interp.bookshelf.pop().unwrap();
    let vector_val = interp.bookshelf.pop().unwrap();
    
    let user_index = match index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(LPLError::type_error("integer", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(v) => {
            // 1オリジン → 0オリジン変換、境界調整
            let internal_index = if user_index < 0 {
                (v.len() as i64 + user_index + 1).max(0) as usize
            } else {
                ((user_index - 1) as usize).min(v.len())  // 1オリジン → 0オリジン
            };
            
            let (left, right) = v.split_at(internal_index);
            interp.bookshelf.push(Value { val_type: ValueType::Vector(left.to_vec()) });
            interp.bookshelf.push(Value { val_type: ValueType::Vector(right.to_vec()) });
            Ok(())
        },
        _ => Err(LPLError::type_error("vector", "other type")),
    }
}

// 待機司書 - 何もしない（pass文）
pub fn op_wait(_interp: &mut Interpreter) -> Result<()> {
    // 何もしない
    Ok(())
}

// 複製司書 - 書籍を複製
pub fn op_duplicate(interp: &mut Interpreter) -> Result<()> {
    let val = interp.bookshelf.last()
        .ok_or(LPLError::BookshelfUnderflow)?;
    
    interp.bookshelf.push(val.clone());
    Ok(())
}
