// rust/src/interpreter/vector_ops.rs (完全修正版)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, Fraction};

// ========== 位置指定操作（0オリジン）==========

// 摘妖精 - 0オリジンの位置指定取得
pub fn op_get(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::from("摘 requires vector and index"));
    }
    
    let index_val = interp.workspace.pop().unwrap();
    let target_val = interp.workspace.pop().unwrap();
    
    let index = match index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(AjisaiError::type_error("integer index", "other type")),
    };
    
    match target_val.val_type {
        ValueType::Vector(v) => {
            if v.is_empty() {
                return Err(AjisaiError::IndexOutOfBounds {
                    index,
                    length: v.len(),
                });
            }
            
            let actual_index = if index < 0 {
                let pos = v.len() as i64 + index;
                if pos < 0 {
                    return Err(AjisaiError::IndexOutOfBounds {
                        index,
                        length: v.len(),
                    });
                }
                pos as usize
            } else {
                index as usize
            };
            
            if actual_index < v.len() {
                interp.workspace.push(v[actual_index].clone());
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

// 挿妖精 - 0オリジンの位置指定挿入
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
                let pos = v.len() as i64 + index + 1;
                pos.max(0) as usize
            } else {
                (index as usize).min(v.len())
            };
            
            v.insert(insert_index, element);
            interp.workspace.push(Value { val_type: ValueType::Vector(v) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 換妖精 - 0オリジンの位置指定上書き（新しいベクターのみ返す）
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
                let pos = v.len() as i64 + index;
                if pos < 0 {
                    return Err(AjisaiError::IndexOutOfBounds {
                        index,
                        length: v.len(),
                    });
                }
                pos as usize
            } else {
                index as usize
            };
            
            if actual_index < v.len() {
                // 置換実行（古い値は破棄）
                v[actual_index] = new_element;
                
                // 新しいベクターのみを返す
                interp.workspace.push(Value { val_type: ValueType::Vector(v) });
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

// 削妖精 - 0オリジンの位置指定削除（新しいベクターのみ返す）
pub fn op_remove(interp: &mut Interpreter) -> Result<()> {
    match interp.workspace.len() {
        0 => Err(AjisaiError::WorkspaceUnderflow),
        1 => {
            // 単一値の場合は破棄
            interp.workspace.pop().unwrap();
            Ok(())
        },
        _ => {
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
                interp.workspace.pop().unwrap(); // インデックスを削除
                let vector_val = interp.workspace.pop().unwrap(); // ベクターを削除
                
                match vector_val.val_type {
                    ValueType::Vector(mut v) => {
                        let actual_index = if index < 0 {
                            let pos = v.len() as i64 + index;
                            if pos < 0 {
                                return Err(AjisaiError::IndexOutOfBounds {
                                    index,
                                    length: v.len(),
                                });
                            }
                            pos as usize
                        } else {
                            index as usize
                        };
                        
                        if actual_index < v.len() {
                            // 削除実行（削除された値は破棄）
                            v.remove(actual_index);
                            
                            // 新しいベクターのみを返す
                            interp.workspace.push(Value { val_type: ValueType::Vector(v) });
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
                // インデックス指定なしの場合は単純に値を破棄
                interp.workspace.pop().unwrap();
                Ok(())
            }
        }
    }
}

// ========== 量指定操作（1オリジン）==========

// 数妖精 - 要素数取得
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

// 取妖精 - 1オリジンの量指定取得（修正版）
pub fn op_take(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let count_val = interp.workspace.pop().unwrap();
    let vector_val = interp.workspace.pop().unwrap();
    
    let count = match count_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(AjisaiError::type_error("integer count", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(v) => {
            let result = if count < 0 {
                // 負の値：末尾からN個
                let abs_count = (-count) as usize;
                if abs_count >= v.len() {
                    v.clone() // 全要素
                } else {
                    v[v.len() - abs_count..].to_vec()
                }
            } else if count == 0 {
                // 0個は空Vector
                vec![]
            } else {
                // 正の値：先頭からN個
                let take_count = (count as usize).min(v.len());
                v[..take_count].to_vec()
            };
            
            interp.workspace.push(Value { val_type: ValueType::Vector(result) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 捨妖精 - 1オリジンの量指定破棄（修正版）
pub fn op_drop(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let count_val = interp.workspace.pop().unwrap();
    let vector_val = interp.workspace.pop().unwrap();
    
    let count = match count_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(AjisaiError::type_error("integer count", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(v) => {
            let result = if count < 0 {
                // 負の値：末尾からN個を捨てる
                let abs_count = (-count) as usize;
                if abs_count >= v.len() {
                    vec![] // 全て捨てる
                } else {
                    v[..v.len() - abs_count].to_vec()
                }
            } else if count == 0 {
                // 0個捨てる = 元のまま
                v
            } else {
                // 正の値：先頭からN個を捨てる
                let drop_count = (count as usize).min(v.len());
                v[drop_count..].to_vec()
            };
            
            interp.workspace.push(Value { val_type: ValueType::Vector(result) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 重妖精 - 1オリジンの回数指定重複
pub fn op_repeat(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let times_val = interp.workspace.pop().unwrap();
    let elem_val = interp.workspace.pop().unwrap();
    
    let times = match times_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(AjisaiError::type_error("integer times", "other type")),
    };
    
    if times < 0 {
        return Err(AjisaiError::from("Repeat times must be non-negative"));
    }
    
    match elem_val.val_type {
        ValueType::Vector(v) => {
            // Vectorの場合は繰り返し結合
            let mut result = Vec::new();
            for _ in 0..times {
                result.extend(v.iter().cloned());
            }
            interp.workspace.push(Value { val_type: ValueType::Vector(result) });
        },
        _ => {
            // その他の値の場合はN個のVectorを作成
            let mut result = Vec::new();
            for _ in 0..times {
                result.push(elem_val.clone());
            }
            interp.workspace.push(Value { val_type: ValueType::Vector(result) });
        }
    }
    Ok(())
}

// 分妖精 - 1オリジンのサイズ指定分割（既存維持）
pub fn op_split(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let (vector_val, sizes) = extract_vector_and_sizes(interp)?;
    
    match vector_val.val_type {
        ValueType::Vector(v) => {
            split_by_sizes(&v, &sizes, interp)
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 結妖精 - Vector結合
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

// ========== ヘルパー関数 ==========

fn extract_vector_and_sizes(interp: &mut Interpreter) -> Result<(Value, Vec<i64>)> {
    let mut sizes = Vec::new();
    let mut temp_values = Vec::new();
    
    while let Some(val) = interp.workspace.pop() {
        match &val.val_type {
            ValueType::Number(n) if n.denominator == 1 => {
                if n.numerator <= 0 {
                    // 値を戻してからエラー
                    temp_values.push(val);
                    for v in temp_values.into_iter().rev() {
                        interp.workspace.push(v);
                    }
                    return Err(AjisaiError::from("Split size must be positive"));
                }
                sizes.push(n.numerator);
                temp_values.push(val);
            },
            ValueType::Vector(_) => {
                sizes.reverse();
                return Ok((val, sizes));
            },
            _ => {
                temp_values.push(val);
                for v in temp_values.into_iter().rev() {
                    interp.workspace.push(v);
                }
                return Err(AjisaiError::from("分 requires vector and positive integers"));
            }
        }
    }
    
    for v in temp_values.into_iter().rev() {
        interp.workspace.push(v);
    }
    Err(AjisaiError::from("分 requires vector"))
}

fn split_by_sizes(v: &[Value], sizes: &[i64], interp: &mut Interpreter) -> Result<()> {
    let total_size: i64 = sizes.iter().sum();
    if total_size != v.len() as i64 {
        return Err(AjisaiError::from(format!(
            "Split sizes sum to {} but vector has {} elements",
            total_size, v.len()
        )));
    }
    
    let mut start = 0;
    let mut results = Vec::new();
    
    for &size in sizes {
        let end = start + size as usize;
        if end > v.len() {
            return Err(AjisaiError::from("Invalid split sizes"));
        }
        
        results.push(Value { 
            val_type: ValueType::Vector(v[start..end].to_vec()) 
        });
        start = end;
    }
    
    // 結果を順番にプッシュ
    for result in results {
        interp.workspace.push(result);
    }
    
    Ok(())
}
