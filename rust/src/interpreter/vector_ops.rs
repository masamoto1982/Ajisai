// rust/src/interpreter/vector_ops.rs (ビルドエラー修正版)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, Fraction};

// 頁司書 - 書籍の特定ページを取得
pub fn op_page(interp: &mut Interpreter) -> Result<()> {
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

// 頁数司書 - 書籍の総ページ数を取得
pub fn op_page_count(interp: &mut Interpreter) -> Result<()> {
    let vector_val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match vector_val.val_type {
        ValueType::Vector(v) => {
            interp.workspace.push(Value { 
                val_type: ValueType::Number(Fraction::new(v.len() as i64, 1))
            });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 冊司書 - 書籍コレクション（2次元配列）の特定の冊を取得
pub fn op_book(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let index_val = interp.workspace.pop().unwrap();
    let collection_val = interp.workspace.pop().unwrap();
    
    let index = match index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(AjisaiError::type_error("integer", "other type")),
    };
    
    match &collection_val.val_type {  // 参照を使用
        ValueType::Vector(collection) => {
            let actual_index = if index < 0 {
                collection.len() as i64 + index
            } else {
                index
            };
            
            if actual_index >= 0 && (actual_index as usize) < collection.len() {
                // 選択された冊をワークスペースに配置
                interp.workspace.push(collection[actual_index as usize].clone());
                Ok(())
            } else {
                Err(AjisaiError::IndexOutOfBounds {
                    index,
                    length: collection.len(),
                })
            }
        },
        _ => Err(AjisaiError::type_error("vector (book collection)", "other type")),
    }
}

// 冊数司書 - 書籍コレクションの総冊数を取得
pub fn op_book_count(interp: &mut Interpreter) -> Result<()> {
    let collection_val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match collection_val.val_type {
        ValueType::Vector(collection) => {
            interp.workspace.push(Value { 
                val_type: ValueType::Number(Fraction::new(collection.len() as i64, 1))
            });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector (book collection)", "other type")),
    }
}

// 挿入司書 - 指定位置にページを挿入
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

// 置換司書 - 指定位置のページを置換
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

// 削除司書 - 指定位置のページを削除
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

// 合併司書 - 2つの書籍を結合
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

// 分離司書 - 書籍を2つに分ける
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

// 待機司書 - 何もしない（pass文）
pub fn op_wait(_interp: &mut Interpreter) -> Result<()> {
    // 何もしない
    Ok(())
}

// 複製司書 - 書籍を複製
pub fn op_duplicate(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.last()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    interp.workspace.push(val.clone());
    Ok(())
}

// 破棄司書 - 書籍を破棄
pub fn op_discard(interp: &mut Interpreter) -> Result<()> {
    interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    Ok(())
}

// 統合版削除司書 - 削除と破棄の機能統合（シンプル版）
pub fn op_unified_delete(interp: &mut Interpreter) -> Result<()> {
    match interp.workspace.len() {
        0 => Err(AjisaiError::WorkspaceUnderflow),
        1 => {
            // スタックに1つだけ値がある場合は破棄
            interp.workspace.pop().unwrap();
            Ok(())
        },
        _ => {
            // スタックに2つ以上値がある場合、上位2つを見て判定
            let top = &interp.workspace[interp.workspace.len() - 1];
            let second = &interp.workspace[interp.workspace.len() - 2];
            
            // 上位がindex（整数）、次がvectorの場合はベクトルから削除
            if let (ValueType::Number(n), ValueType::Vector(_)) = (&top.val_type, &second.val_type) {
                if n.denominator == 1 {
                    // ベクトルから削除
                    return op_delete(interp);
                }
            }
            
            // どのパターンにも該当しなければ破棄
            interp.workspace.pop().unwrap();
            Ok(())
        }
    }
}

// 冊操作用の補助関数群

// 冊全体削除司書 - 2次元配列から特定の冊を削除  
pub fn op_book_delete(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let book_index_val = interp.workspace.pop().unwrap();
    let collection_val = interp.workspace.pop().unwrap();
    
    let book_index = match book_index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(AjisaiError::type_error("integer", "book index")),
    };
    
    match collection_val.val_type {
        ValueType::Vector(mut collection) => {
            let actual_index = if book_index < 0 {
                collection.len() as i64 + book_index
            } else {
                book_index
            };
            
            if actual_index >= 0 && (actual_index as usize) < collection.len() {
                let removed_book = collection.remove(actual_index as usize);
                interp.workspace.push(Value { val_type: ValueType::Vector(collection) });
                interp.workspace.push(removed_book);
                Ok(())
            } else {
                Err(AjisaiError::IndexOutOfBounds {
                    index: book_index,
                    length: collection.len(),
                })
            }
        },
        _ => Err(AjisaiError::type_error("vector (book collection)", "other type")),
    }
}

// 冊置換司書 - 2次元配列内の特定の冊を置換
pub fn op_book_replace(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 3 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let new_book = interp.workspace.pop().unwrap();
    let book_index_val = interp.workspace.pop().unwrap();
    let collection_val = interp.workspace.pop().unwrap();
    
    let book_index = match book_index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(AjisaiError::type_error("integer", "book index")),
    };
    
    match collection_val.val_type {
        ValueType::Vector(mut collection) => {
            let actual_index = if book_index < 0 {
                collection.len() as i64 + book_index
            } else {
                book_index
            };
            
            if actual_index >= 0 && (actual_index as usize) < collection.len() {
                let old_book = std::mem::replace(&mut collection[actual_index as usize], new_book);
                interp.workspace.push(Value { val_type: ValueType::Vector(collection) });
                interp.workspace.push(old_book);
                Ok(())
            } else {
                Err(AjisaiError::IndexOutOfBounds {
                    index: book_index,
                    length: collection.len(),
                })
            }
        },
        _ => Err(AjisaiError::type_error("vector (book collection)", "other type")),
    }
}

// 冊挿入司書 - 2次元配列に新しい冊を挿入
pub fn op_book_insert(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 3 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let new_book = interp.workspace.pop().unwrap();
    let book_index_val = interp.workspace.pop().unwrap();
    let collection_val = interp.workspace.pop().unwrap();
    
    let book_index = match book_index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(AjisaiError::type_error("integer", "book index")),
    };
    
    match collection_val.val_type {
        ValueType::Vector(mut collection) => {
            let insert_index = if book_index < 0 {
                0
            } else if book_index as usize > collection.len() {
                collection.len()
            } else {
                book_index as usize
            };
            
            collection.insert(insert_index, new_book);
            interp.workspace.push(Value { val_type: ValueType::Vector(collection) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector (book collection)", "other type")),
    }
}
