// rust/src/interpreter/vector_ops.rs (純粋Vector操作言語版)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, Fraction, BracketType};

// ========== 位置指定操作（0オリジン）==========

// GET - 0オリジンの位置指定取得（旧NTH）
pub fn op_get(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::from("GET requires vector and index"));
    }
    
    let index_val = interp.workspace.pop().unwrap();
    let target_val = interp.workspace.pop().unwrap();
    
    // インデックスを取得（単一要素Vectorから）
    let index = match target_val.val_type {
        ValueType::Vector(ref v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) if n.denominator == 1 => n.numerator,
                _ => return Err(AjisaiError::type_error("integer index", "other type")),
            }
        },
        _ => return Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    };
    
    match index_val.val_type {
        ValueType::Vector(v, bracket_type) => {
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
                // 取得した要素を単一要素Vectorでラップ
                let result = Value {
                    val_type: ValueType::Vector(vec![v[actual_index].clone()], bracket_type)
                };
                interp.workspace.push(result);
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

// INSERT - 0オリジンの位置指定挿入
pub fn op_insert(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 3 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let element = interp.workspace.pop().unwrap();
    let index_val = interp.workspace.pop().unwrap();
    let vector_val = interp.workspace.pop().unwrap();
    
    let index = match index_val.val_type {
        ValueType::Vector(ref v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) if n.denominator == 1 => n.numerator,
                _ => return Err(AjisaiError::type_error("integer", "other type")),
            }
        },
        _ => return Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    };
    
    // 挿入する要素をVector内の値として取得
    let insert_element = match element.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => v[0].clone(),
        _ => return Err(AjisaiError::type_error("single-element vector", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(mut v, bracket_type) => {
            let insert_index = if index < 0 {
                let pos = v.len() as i64 + index + 1;
                pos.max(0) as usize
            } else {
                (index as usize).min(v.len())
            };
            
            v.insert(insert_index, insert_element);
            interp.workspace.push(Value { val_type: ValueType::Vector(v, bracket_type) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// REPLACE - 0オリジンの位置指定上書き
pub fn op_replace(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 3 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let new_element = interp.workspace.pop().unwrap();
    let index_val = interp.workspace.pop().unwrap();
    let vector_val = interp.workspace.pop().unwrap();
    
    let index = match index_val.val_type {
        ValueType::Vector(ref v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) if n.denominator == 1 => n.numerator,
                _ => return Err(AjisaiError::type_error("integer", "other type")),
            }
        },
        _ => return Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    };
    
    let replace_element = match new_element.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => v[0].clone(),
        _ => return Err(AjisaiError::type_error("single-element vector", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(mut v, bracket_type) => {
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
                v[actual_index] = replace_element;
                interp.workspace.push(Value { val_type: ValueType::Vector(v, bracket_type) });
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

// REMOVE - 0オリジンの位置指定削除
pub fn op_remove(interp: &mut Interpreter) -> Result<()> {
    match interp.workspace.len() {
        0 => Err(AjisaiError::WorkspaceUnderflow),
        1 => {
            interp.workspace.pop().unwrap();
            Ok(())
        },
        _ => {
            let index_val = interp.workspace.pop().unwrap();
            let vector_val = interp.workspace.pop().unwrap();
            
            let index = match index_val.val_type {
                ValueType::Vector(ref v, _) if v.len() == 1 => {
                    match &v[0].val_type {
                        ValueType::Number(n) if n.denominator == 1 => n.numerator,
                        _ => return Err(AjisaiError::type_error("integer", "other type")),
                    }
                },
                _ => return Err(AjisaiError::type_error("single-element vector with integer", "other type")),
            };
            
            match vector_val.val_type {
                ValueType::Vector(mut v, bracket_type) => {
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
                        v.remove(actual_index);
                        interp.workspace.push(Value { val_type: ValueType::Vector(v, bracket_type) });
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

// ========== 量指定操作（1オリジン）==========

// LENGTH - 要素数取得
pub fn op_length(interp: &mut Interpreter) -> Result<()> {
    let target_val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match target_val.val_type {
        ValueType::Vector(v, _) => {
            let length_wrapped = Value {
                val_type: ValueType::Vector(
                    vec![Value { 
                        val_type: ValueType::Number(Fraction::new(v.len() as i64, 1))
                    }],
                    BracketType::Square
                )
            };
            interp.workspace.push(length_wrapped);
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// TAKE - 1オリジンの量指定取得
pub fn op_take(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let count_val = interp.workspace.pop().unwrap();
    let vector_val = interp.workspace.pop().unwrap();
    
    let count = match count_val.val_type {
        ValueType::Vector(ref v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) if n.denominator == 1 => n.numerator,
                _ => return Err(AjisaiError::type_error("integer count", "other type")),
            }
        },
        _ => return Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(v, bracket_type) => {
            let result = if count < 0 {
                let abs_count = (-count) as usize;
                if abs_count > v.len() {
                    return Err(AjisaiError::from(format!(
                        "Cannot take {} elements from vector of length {}",
                        abs_count, v.len()
                    )));
                }
                v[v.len() - abs_count..].to_vec()
            } else if count == 0 {
                vec![]
            } else {
                let take_count = count as usize;
                if take_count > v.len() {
                    return Err(AjisaiError::from(format!(
                        "Cannot take {} elements from vector of length {}",
                        take_count, v.len()
                    )));
                }
                v[..take_count].to_vec()
            };
            
            interp.workspace.push(Value { val_type: ValueType::Vector(result, bracket_type) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// DROP - 1オリジンの量指定破棄（Vector操作版）
pub fn op_drop_vector(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let count_val = interp.workspace.pop().unwrap();
    let vector_val = interp.workspace.pop().unwrap();
    
    let count = match count_val.val_type {
        ValueType::Vector(ref v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) if n.denominator == 1 => n.numerator,
                _ => return Err(AjisaiError::type_error("integer count", "other type")),
            }
        },
        _ => return Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    };
    
    match vector_val.val_type {
        ValueType::Vector(v, bracket_type) => {
            let result = if count < 0 {
                let abs_count = (-count) as usize;
                if abs_count > v.len() {
                    return Err(AjisaiError::from(format!(
                        "Cannot drop {} elements from vector of length {}",
                        abs_count, v.len()
                    )));
                }
                v[..v.len() - abs_count].to_vec()
            } else if count == 0 {
                v
            } else {
                let drop_count = count as usize;
                if drop_count > v.len() {
                    return Err(AjisaiError::from(format!(
                        "Cannot drop {} elements from vector of length {}",
                        drop_count, v.len()
                    )));
                }
                v[drop_count..].to_vec()
            };
            
            interp.workspace.push(Value { val_type: ValueType::Vector(result, bracket_type) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// REPEAT - 1オリジンの回数指定重複
pub fn op_repeat(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let times_val = interp.workspace.pop().unwrap();
    let elem_val = interp.workspace.pop().unwrap();
    
    let times = match times_val.val_type {
        ValueType::Vector(ref v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) if n.denominator == 1 => n.numerator,
                _ => return Err(AjisaiError::type_error("integer times", "other type")),
            }
        },
        _ => return Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    };
    
    if times < 0 {
        return Err(AjisaiError::from("Repeat times must be non-negative"));
    }
    
    match elem_val.val_type {
        ValueType::Vector(v, bracket_type) => {
            if v.len() == 1 {
                // 単一要素Vectorの場合は、個別に複数回ワークスペースに配置
                for _ in 0..times {
                    interp.workspace.push(Value { val_type: ValueType::Vector(v.clone(), bracket_type.clone()) });
                }
            } else {
                // 複数要素Vectorの場合は繰り返し結合してVectorとして返す
                let mut result = Vec::new();
                for _ in 0..times {
                    result.extend(v.iter().cloned());
                }
                interp.workspace.push(Value { val_type: ValueType::Vector(result, bracket_type) });
            }
        },
        _ => return Err(AjisaiError::type_error("vector", "other type")),
    }
    Ok(())
}

// SPLIT - 1オリジンのサイズ指定分割
pub fn op_split(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let (vector_val, sizes) = extract_vector_and_sizes(interp)?;
    
    match vector_val.val_type {
        ValueType::Vector(v, bracket_type) => {
            split_by_sizes(&v, &sizes, interp, bracket_type)
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// ========== ワークスペース操作（Vector操作として実装）==========

// DUP - ワークスペース最上位要素を複製
pub fn op_dup_workspace(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.is_empty() {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let last_element = interp.workspace.last().unwrap().clone();
    interp.workspace.push(last_element);
    Ok(())
}

// SWAP - ワークスペース最上位2要素を交換
pub fn op_swap_workspace(interp: &mut Interpreter) -> Result<()> {
    let len = interp.workspace.len();
    if len < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    interp.workspace.swap(len - 1, len - 2);
    Ok(())
}

// ROT - ワークスペース最上位3要素を回転 (a b c → b c a)
pub fn op_rot_workspace(interp: &mut Interpreter) -> Result<()> {
    let len = interp.workspace.len();
    if len < 3 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    let third = interp.workspace.remove(len - 3);
    interp.workspace.push(third);
    Ok(())
}

// ========== Vector構造操作 ==========

// CONCAT - Vector結合
pub fn op_concat(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let vec2_val = interp.workspace.pop().unwrap();
    let vec1_val = interp.workspace.pop().unwrap();
    
    match (vec1_val.val_type, vec2_val.val_type) {
        (ValueType::Vector(mut v1, bracket_type1), ValueType::Vector(v2, _)) => {
            v1.extend(v2);
            interp.workspace.push(Value { val_type: ValueType::Vector(v1, bracket_type1) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector vector", "other types")),
    }
}

// REVERSE - Vector要素順序反転
pub fn op_reverse(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match val.val_type {
        ValueType::Vector(mut v, bracket_type) => {
            v.reverse();
            interp.workspace.push(Value { val_type: ValueType::Vector(v, bracket_type) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// ========== ヘルパー関数 ==========

fn extract_vector_and_sizes(interp: &mut Interpreter) -> Result<(Value, Vec<i64>)> {
    let mut sizes = Vec::new();
    let mut temp_values = Vec::new();
    
    while let Some(val) = interp.workspace.pop() {
        match &val.val_type {
            ValueType::Vector(v, _) if v.len() == 1 => {
                match &v[0].val_type {
                    ValueType::Number(n) if n.denominator == 1 => {
                        if n.numerator <= 0 {
                            temp_values.push(val);
                            for v in temp_values.into_iter().rev() {
                                interp.workspace.push(v);
                            }
                            return Err(AjisaiError::from("Split size must be positive"));
                        }
                        sizes.push(n.numerator);
                        temp_values.push(val);
                    },
                    _ => {
                        // ベクトルが見つかった
                        sizes.reverse();
                        return Ok((val, sizes));
                    }
                }
            },
            ValueType::Vector(_, _) => {
                // 複数要素ベクトルが見つかった
                sizes.reverse();
                return Ok((val, sizes));
            },
            _ => {
                temp_values.push(val);
                for v in temp_values.into_iter().rev() {
                    interp.workspace.push(v);
                }
                return Err(AjisaiError::from("SPLIT requires vector and positive integers"));
            }
        }
    }
    
    for v in temp_values.into_iter().rev() {
        interp.workspace.push(v);
    }
    Err(AjisaiError::from("SPLIT requires vector"))
}

fn split_by_sizes(v: &[Value], sizes: &[i64], interp: &mut Interpreter, bracket_type: BracketType) -> Result<()> {
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
            val_type: ValueType::Vector(v[start..end].to_vec(), bracket_type.clone()) 
        });
        start = end;
    }
    
    for result in results {
        interp.workspace.push(result);
    }
    
    Ok(())
}
