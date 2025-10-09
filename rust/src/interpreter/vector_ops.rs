// rust/src/interpreter/vector_ops.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, Fraction, BracketType};
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive, Zero};
use std::collections::VecDeque;

// スタックトップから数値の引数を取得する
fn get_index_from_stack(interp: &mut Interpreter) -> Result<Option<BigInt>> {
    if let Some(top) = interp.stack.last() {
        if let ValueType::Vector(v, _) = &top.val_type {
            if v.len() == 1 {
                if let ValueType::Number(n) = &v[0].val_type {
                    if n.denominator == BigInt::one() {
                        let index = n.numerator.clone();
                        interp.stack.pop(); // indexを消費
                        return Ok(Some(index));
                    }
                }
            }
        }
    }
    Ok(None)
}

// ========== Vector/Stack共通操作 ==========

pub fn op_get(interp: &mut Interpreter) -> Result<()> {
    let index_bigint = get_index_from_stack(interp)?.ok_or_else(|| AjisaiError::from("GET requires an index"))?;
    let index = index_bigint.to_i64().ok_or_else(|| AjisaiError::from("Index is too large"))?;

    // スタック操作かVector操作かを判断
    if let Some(target_val) = interp.stack.last() {
        if matches!(target_val.val_type, ValueType::Vector(_, _)) {
            // Vector操作
            let target_val = interp.stack.pop().unwrap();
            let v = match target_val.val_type {
                ValueType::Vector(v, _) => v,
                _ => unreachable!(),
            };
            let len = v.len();
            let actual_index = resolve_index(index, len)?;
            
            let result_element = v.get(actual_index).cloned().ok_or(AjisaiError::IndexOutOfBounds { index, length: len })?;
            interp.stack.push(Value { val_type: ValueType::Vector(vec![result_element], BracketType::Square) });
        } else {
             return Err(AjisaiError::type_error("vector", "other type"));
        }
    } else {
        // スタック操作
        let len = interp.stack.len();
        let actual_index = resolve_index(index, len)?;
        let value_to_get = interp.stack.get(actual_index).cloned().ok_or(AjisaiError::IndexOutOfBounds { index, length: len })?;
        interp.stack.push(value_to_get);
    }
    Ok(())
}

pub fn op_remove(interp: &mut Interpreter) -> Result<()> {
    let index_bigint = get_index_from_stack(interp)?.ok_or_else(|| AjisaiError::from("REMOVE requires an index"))?;
    let index = index_bigint.to_i64().ok_or_else(|| AjisaiError::from("Index is too large"))?;

    if let Some(target_val) = interp.stack.last() {
        if matches!(target_val.val_type, ValueType::Vector(_, _)) {
            // Vector操作
            let mut target_vec = interp.stack.pop().unwrap();
            if let ValueType::Vector(ref mut v, _) = target_vec.val_type {
                let len = v.len();
                let actual_index = resolve_index(index, len)?;
                if actual_index < len {
                    v.remove(actual_index);
                } else {
                    return Err(AjisaiError::IndexOutOfBounds { index, length: len });
                }
            }
            interp.stack.push(target_vec);

        } else {
            return Err(AjisaiError::type_error("vector", "other type"));
        }
    } else {
        // スタック操作
        let len = interp.stack.len();
        let actual_index = resolve_index(index, len)?;
        if actual_index < len {
            interp.stack.remove(actual_index);
        } else {
            return Err(AjisaiError::IndexOutOfBounds { index, length: len });
        }
    }
    Ok(())
}

pub fn op_reverse(interp: &mut Interpreter) -> Result<()> {
    if let Some(count) = get_index_from_stack(interp)? {
        // スタック操作
        let n = count.to_usize().ok_or_else(|| AjisaiError::from("Count is too large"))?;
        if interp.stack.len() < n { return Err(AjisaiError::StackUnderflow); }
        let start_index = interp.stack.len() - n;
        interp.stack[start_index..].reverse();
    } else {
        // Vector操作
        let mut val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        if let ValueType::Vector(ref mut v, _) = val.val_type {
            v.reverse();
        } else {
            return Err(AjisaiError::type_error("vector", "other type"));
        }
        interp.stack.push(val);
    }
    Ok(())
}

pub fn op_concat(interp: &mut Interpreter) -> Result<()> {
    let n = get_index_from_stack(interp)?.map(|c| c.to_usize().unwrap()).unwrap_or(2);
    if interp.stack.len() < n { return Err(AjisaiError::StackUnderflow); }
    
    let mut result_vec = Vec::new();
    let mut first_bracket_type = BracketType::Square;

    let mut vectors_to_concat = interp.stack.split_off(interp.stack.len() - n);

    for (i, val) in vectors_to_concat.drain(..).enumerate() {
        if let ValueType::Vector(mut v, bracket_type) = val.val_type {
            if i == 0 { first_bracket_type = bracket_type; }
            result_vec.append(&mut v);
        } else {
            return Err(AjisaiError::type_error("vector", "other type"));
        }
    }
    interp.stack.push(Value { val_type: ValueType::Vector(result_vec, first_bracket_type) });
    Ok(())
}

// 他のワードも同様に修正...
// ... op_insert, op_replace, op_length, op_take, op_level, op_split ...

// ヘルパー関数: 負のインデックスを解決
fn resolve_index(index: i64, len: usize) -> Result<usize> {
    let ulen = len as i64;
    let actual_index = if index < 0 {
        ulen + index
    } else {
        index
    };
    if actual_index < 0 || actual_index >= ulen {
        Err(AjisaiError::IndexOutOfBounds { index, length: len })
    } else {
        Ok(actual_index as usize)
    }
}

// (変更のない、または削除される関数は省略)
fn get_index_from_value(value: &Value) -> Result<BigInt> {
    match &value.val_type {
        ValueType::Vector(ref v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) if n.denominator == BigInt::one() => Ok(n.numerator.clone()),
                _ => Err(AjisaiError::type_error("integer index", "other type")),
            }
        },
        _ => Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    }
}

pub fn op_insert(interp: &mut Interpreter) -> Result<()> {
    let element = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index = get_index_from_value(&interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?)?
        .to_i64().ok_or_else(|| AjisaiError::from("Index too large"))?;

    if let Some(target_val) = interp.stack.last_mut() {
        if let ValueType::Vector(v, _) = &mut target_val.val_type {
            // Vector操作
             let insert_element = match element.val_type {
                ValueType::Vector(v_el, _) if v_el.len() == 1 => v_el[0].clone(),
                _ => return Err(AjisaiError::type_error("single-element vector for insertion", "other type")),
            };
            let len = v.len() as i64;
            let insert_index = if index < 0 { (len + index + 1).max(0) } else { index.min(len) } as usize;
            v.insert(insert_index, insert_element);
        } else {
            // スタック操作
            let len = interp.stack.len() as i64;
            let insert_index = if index < 0 { (len + index + 1).max(0) } else { index.min(len) } as usize;
            interp.stack.insert(insert_index, element);
        }
    } else {
        // スタックが空の場合のスタック操作
        interp.stack.insert(0, element);
    }
    Ok(())
}

pub fn op_replace(interp: &mut Interpreter) -> Result<()> {
    let new_element = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index = get_index_from_value(&interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?)?
        .to_i64().ok_or_else(|| AjisaiError::from("Index too large"))?;

    if let Some(target_val) = interp.stack.last_mut() {
        if let ValueType::Vector(v, _) = &mut target_val.val_type {
            // Vector操作
            let replace_element = match new_element.val_type {
                ValueType::Vector(v_el, _) if v_el.len() == 1 => v_el[0].clone(),
                _ => return Err(AjisaiError::type_error("single-element vector for replacement", "other type")),
            };
            let len = v.len();
            let actual_index = resolve_index(index, len)?;
            v[actual_index] = replace_element;
        } else {
            // スタック操作
            let len = interp.stack.len();
            let actual_index = resolve_index(index, len)?;
            interp.stack[actual_index] = new_element;
        }
    } else {
        return Err(AjisaiError::StackUnderflow);
    }
    Ok(())
}

pub fn op_length(interp: &mut Interpreter) -> Result<()> {
    if get_index_from_stack(interp)?.is_some() {
        return Err(AjisaiError::from("LENGTH with argument is not supported"));
    }
    let len = if let Some(target_val) = interp.stack.last() {
        match &target_val.val_type {
            ValueType::Vector(v, _) => {
                let val = interp.stack.pop().unwrap(); // drop the vector
                v.len()
            },
            _ => interp.stack.len(), // スタック操作
        }
    } else {
        0 // 空のスタック
    };
    let len_frac = Fraction::new(BigInt::from(len), BigInt::one());
    let val = Value { val_type: ValueType::Vector(vec![Value{ val_type: ValueType::Number(len_frac)}], BracketType::Square) };
    interp.stack.push(val);
    Ok(())
}

pub fn op_take(interp: &mut Interpreter) -> Result<()> {
    let count = get_index_from_stack(interp)?.ok_or_else(|| AjisaiError::from("TAKE requires a count"))?
        .to_i64().ok_or_else(|| AjisaiError::from("Count is too large"))?;

    if let Some(target_val) = interp.stack.last() {
        if let ValueType::Vector(v, bracket_type) = &target_val.val_type {
            // Vector操作
            let v_clone = v.clone();
            let bracket_clone = bracket_type.clone();
            interp.stack.pop(); // drop original
            
            let len = v_clone.len();
            let result_vec = if count >= 0 {
                let take_count = count as usize;
                if take_count > len { return Err(AjisaiError::from("Take count exceeds vector length")); }
                v_clone[..take_count].to_vec()
            } else {
                let take_count = (-count) as usize;
                if take_count > len { return Err(AjisaiError::from("Take count exceeds vector length")); }
                v_clone[len - take_count..].to_vec()
            };
            interp.stack.push(Value { val_type: ValueType::Vector(result_vec, bracket_clone) });

        } else {
             // スタック操作
            let n = count.abs() as usize;
            if interp.stack.len() < n { return Err(AjisaiError::StackUnderflow); }
            let new_vec_elements = if count >= 0 {
                interp.stack.drain(..n).collect()
            } else {
                let len = interp.stack.len();
                interp.stack.drain(len-n..).collect()
            };
            interp.stack.push(Value { val_type: ValueType::Vector(new_vec_elements, BracketType::Square) });
        }
    } else {
        return Err(AjisaiError::StackUnderflow);
    }
    Ok(())
}

pub fn op_split(interp: &mut Interpreter) -> Result<()> {
    // SPLITはVectorをスタックに展開する操作のみに限定
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    match val.val_type {
        ValueType::Vector(v, bracket_type) => {
            for item in v.into_iter() {
                interp.stack.push(Value {
                    val_type: ValueType::Vector(vec![item], bracket_type.clone()),
                });
            }
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

pub fn op_level(interp: &mut Interpreter) -> Result<()> {
    if let Some(count) = get_index_from_stack(interp)? {
        // スタック操作
        let n = count.to_usize().ok_or_else(|| AjisaiError::from("Count is too large"))?;
        if interp.stack.len() < n { return Err(AjisaiError::StackUnderflow); }
        let vectors_to_level = interp.stack.drain(interp.stack.len() - n..).collect::<Vec<_>>();
        for val in vectors_to_level {
            if let ValueType::Vector(v, _) = val.val_type {
                let mut flattened = Vec::new();
                flatten_vector_recursive(v, &mut flattened);
                interp.stack.extend(flattened.into_iter().map(|item| Value { 
                    val_type: ValueType::Vector(vec![item], BracketType::Square)
                }));
            } else {
                return Err(AjisaiError::type_error("vector", "other type"));
            }
        }
    } else {
        // Vector操作
        let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        if let ValueType::Vector(v, bracket_type) = val.val_type {
            let mut flattened = Vec::new();
            flatten_vector_recursive(v, &mut flattened);
            interp.stack.push(Value {
                val_type: ValueType::Vector(flattened, bracket_type),
            });
        } else {
            return Err(AjisaiError::type_error("vector", "other type"));
        }
    }
    Ok(())
}

fn flatten_vector_recursive(vec: Vec<Value>, result: &mut Vec<Value>) {
    for val in vec {
        if let ValueType::Vector(inner_vec, _) = val.val_type {
            flatten_vector_recursive(inner_vec, result);
        } else {
            result.push(val);
        }
    }
}
