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

pub fn op_clone(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.last()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    interp.workspace.push(val.clone());
    Ok(())
}

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

// ==================== 新機能（13個） ====================

// 1. 結（JOIN/CONCAT）- Vector結合
pub fn op_join(interp: &mut Interpreter) -> Result<()> {
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

// 2. 切（SPLIT）- 指定位置で分割
pub fn op_split(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let index_val = interp.workspace.pop().unwrap();
    let vec_val = interp.workspace.pop().unwrap();
    
    let index = match index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(AjisaiError::type_error("integer", "other type")),
    };
    
    match vec_val.val_type {
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

// 3. 反（REVERSE）- 順序反転
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

// 4. 挿（INSERT）- 指定位置に挿入
pub fn op_insert(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 3 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let elem = interp.workspace.pop().unwrap();
    let index_val = interp.workspace.pop().unwrap();
    let vec_val = interp.workspace.pop().unwrap();
    
    let index = match index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(AjisaiError::type_error("integer", "other type")),
    };
    
    match vec_val.val_type {
        ValueType::Vector(mut v) => {
            let insert_index = if index < 0 {
                0
            } else {
                (index as usize).min(v.len())
            };
            
            v.insert(insert_index, elem);
            interp.workspace.push(Value { val_type: ValueType::Vector(v) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 5. 消（DELETE）- 指定位置削除
pub fn op_delete(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let index_val = interp.workspace.pop().unwrap();
    let vec_val = interp.workspace.pop().unwrap();
    
    let index = match index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(AjisaiError::type_error("integer", "other type")),
    };
    
    match vec_val.val_type {
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

// 6. 探（FIND）- 要素検索
pub fn op_find(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let elem = interp.workspace.pop().unwrap();
    let vec_val = interp.workspace.pop().unwrap();
    
    match vec_val.val_type {
        ValueType::Vector(v) => {
            for (i, item) in v.iter().enumerate() {
                if *item == elem {
                    interp.workspace.push(Value { 
                        val_type: ValueType::Number(Fraction::new(i as i64, 1)) 
                    });
                    return Ok(());
                }
            }
            interp.workspace.push(Value { val_type: ValueType::Nil });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 7. 含（CONTAINS）- 含有チェック
pub fn op_contains(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let elem = interp.workspace.pop().unwrap();
    let vec_val = interp.workspace.pop().unwrap();
    
    match vec_val.val_type {
        ValueType::Vector(v) => {
            let contains = v.iter().any(|item| *item == elem);
            interp.workspace.push(Value { val_type: ValueType::Boolean(contains) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 8. 換（REPLACE）- 要素置換
pub fn op_replace(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 3 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let new_elem = interp.workspace.pop().unwrap();
    let index_val = interp.workspace.pop().unwrap();
    let vec_val = interp.workspace.pop().unwrap();
    
    let index = match index_val.val_type {
        ValueType::Number(n) if n.denominator == 1 => n.numerator,
        _ => return Err(AjisaiError::type_error("integer", "other type")),
    };
    
    match vec_val.val_type {
        ValueType::Vector(mut v) => {
            let actual_index = if index < 0 {
                v.len() as i64 + index
            } else {
                index
            };
            
            if actual_index >= 0 && (actual_index as usize) < v.len() {
                let old_elem = std::mem::replace(&mut v[actual_index as usize], new_elem);
                interp.workspace.push(Value { val_type: ValueType::Vector(v) });
                interp.workspace.push(old_elem);
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

// 9. 抽（FILTER）- 条件抽出
pub fn op_filter(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let predicate_vec = interp.workspace.pop().unwrap();
    let vec_val = interp.workspace.pop().unwrap();
    
    let predicate_tokens = match predicate_vec.val_type {
        ValueType::Vector(v) => interp.vector_to_tokens(v)?,
        _ => return Err(AjisaiError::type_error("vector (predicate)", "other type")),
    };
    
    match vec_val.val_type {
        ValueType::Vector(v) => {
            let mut result = Vec::new();
            
            for item in v {
                // アイテムをスタックにプッシュ
                interp.workspace.push(item.clone());
                
                // 述語を実行
                interp.execute_tokens(&predicate_tokens)?;
                
                // 結果をチェック
                if let Some(result_val) = interp.workspace.pop() {
                    let include = match result_val.val_type {
                        ValueType::Boolean(b) => b,
                        ValueType::Nil => false,
                        _ => true,
                    };
                    
                    if include {
                        result.push(item);
                    }
                }
            }
            
            interp.workspace.push(Value { val_type: ValueType::Vector(result) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 10. 変（MAP）- 要素変換
pub fn op_map(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let transform_vec = interp.workspace.pop().unwrap();
    let vec_val = interp.workspace.pop().unwrap();
    
    let transform_tokens = match transform_vec.val_type {
        ValueType::Vector(v) => interp.vector_to_tokens(v)?,
        _ => return Err(AjisaiError::type_error("vector (transform)", "other type")),
    };
    
    match vec_val.val_type {
        ValueType::Vector(v) => {
            let mut result = Vec::new();
            
            for item in v {
                // アイテムをスタックにプッシュ
                interp.workspace.push(item);
                
                // 変換を実行
                interp.execute_tokens(&transform_tokens)?;
                
                // 結果を取得
                if let Some(transformed) = interp.workspace.pop() {
                    result.push(transformed);
                } else {
                    return Err(AjisaiError::WorkspaceUnderflow);
                }
            }
            
            interp.workspace.push(Value { val_type: ValueType::Vector(result) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 11. 畳（FOLD/REDUCE）- 畳込処理
pub fn op_fold(interp: &mut Interpreter) -> Result<()> {
    if interp.workspace.len() < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    
    let op_vec = interp.workspace.pop().unwrap();
    let vec_val = interp.workspace.pop().unwrap();
    
    let op_tokens = match op_vec.val_type {
        ValueType::Vector(v) => interp.vector_to_tokens(v)?,
        _ => return Err(AjisaiError::type_error("vector (operation)", "other type")),
    };
    
    match vec_val.val_type {
        ValueType::Vector(v) => {
            if v.is_empty() {
                return Err(AjisaiError::from("畳: 空のベクトルです"));
            }
            
            let mut accumulator = v[0].clone();
            
            for item in v.iter().skip(1) {
                // アキュムレータとアイテムをスタックにプッシュ
                interp.workspace.push(accumulator);
                interp.workspace.push(item.clone());
                
                // 演算を実行
                interp.execute_tokens(&op_tokens)?;
                
                // 結果を取得
                if let Some(result) = interp.workspace.pop() {
                    accumulator = result;
                } else {
                    return Err(AjisaiError::WorkspaceUnderflow);
                }
            }
            
            interp.workspace.push(accumulator);
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 12. 並（SORT）- ソート
pub fn op_sort(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match val.val_type {
        ValueType::Vector(mut v) => {
            // 数値のみのソート（他の型の比較は複雑なので数値に限定）
            v.sort_by(|a, b| {
                match (&a.val_type, &b.val_type) {
                    (ValueType::Number(n1), ValueType::Number(n2)) => {
                        let val1 = n1.numerator as f64 / n1.denominator as f64;
                        let val2 = n2.numerator as f64 / n2.denominator as f64;
                        val1.partial_cmp(&val2).unwrap_or(std::cmp::Ordering::Equal)
                    },
                    (ValueType::String(s1), ValueType::String(s2)) => s1.cmp(s2),
                    _ => std::cmp::Ordering::Equal,
                }
            });
            
            interp.workspace.push(Value { val_type: ValueType::Vector(v) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

// 13. 空（EMPTY）- 空判定
pub fn op_empty(interp: &mut Interpreter) -> Result<()> {
    let val = interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match val.val_type {
        ValueType::Vector(v) => {
            interp.workspace.push(Value { val_type: ValueType::Boolean(v.is_empty()) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}
