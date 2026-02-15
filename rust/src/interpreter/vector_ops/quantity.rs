// rust/src/interpreter/vector_ops/quantity.rs
//
// 量指定操作（1オリジン）: LENGTH, TAKE, SPLIT

use crate::interpreter::{Interpreter, OperationTargetMode, ConsumptionMode};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{get_integer_from_value, get_bigint_from_value, wrap_number};
use crate::types::{Value};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};
use super::reconstruct_vector_elements;

/// LENGTH - 要素数を取得する（Form型）
///
/// 【消費モード】
/// - Consume（デフォルト）: 対象ベクタを消費し、要素数を返す
/// - Keep（,,）: 対象ベクタを保持し、要素数を追加する
pub fn op_length(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    let len = match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            if is_keep_mode {
                // Keep mode: peek without removing
                let target_val = interp.stack.last().ok_or(AjisaiError::StackUnderflow)?;

                if target_val.is_nil() {
                    0
                } else if target_val.is_vector() {
                    let v = reconstruct_vector_elements(target_val);
                    v.len()
                } else {
                    return Err(AjisaiError::structure_error("vector", "other format"));
                }
            } else {
                // Consume mode: pop
                let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

                if target_val.is_nil() {
                    0
                } else if target_val.is_vector() {
                    let v = reconstruct_vector_elements(&target_val);
                    v.len()
                } else {
                    interp.stack.push(target_val);
                    return Err(AjisaiError::structure_error("vector", "other format"));
                }
            }
        }
        OperationTargetMode::Stack => {
            if is_keep_mode {
                interp.stack.len()
            } else {
                let len = interp.stack.len();
                interp.stack.clear();
                len
            }
        }
    };
    let len_frac = Fraction::new(BigInt::from(len), BigInt::one());
    interp.stack.push(wrap_number(len_frac));
    Ok(())
}

/// TAKE - 先頭または末尾から指定数の要素を取得する（Form型）
///
/// 【消費モード】
/// - Consume（デフォルト）: 対象ベクタと引数を消費し、取得結果を返す
/// - Keep（,,）: 対象ベクタと引数を保持し、取得結果を追加する
pub fn op_take(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let count = match get_integer_from_value(&count_val) {
        Ok(v) => v,
        Err(e) => {
            interp.stack.push(count_val);
            return Err(e);
        }
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let vector_val = if is_keep_mode {
                interp.stack.last().cloned().ok_or_else(|| {
                    interp.stack.push(count_val.clone());
                    AjisaiError::StackUnderflow
                })?
            } else {
                interp.stack.pop().ok_or_else(|| {
                    interp.stack.push(count_val.clone());
                    AjisaiError::StackUnderflow
                })?
            };

            if vector_val.is_vector() {
                let v = reconstruct_vector_elements(&vector_val);
                let len = v.len();
                let result = if count < 0 {
                    let abs_count = (-count) as usize;
                    if abs_count > len {
                        if !is_keep_mode {
                            interp.stack.push(Value::from_vector(v));
                        }
                        interp.stack.push(count_val);
                        return Err(AjisaiError::from("Take count exceeds vector length"));
                    }
                    v[len - abs_count..].to_vec()
                } else {
                    let take_count = count as usize;
                    if take_count > len {
                        if !is_keep_mode {
                            interp.stack.push(Value::from_vector(v));
                        }
                        interp.stack.push(count_val);
                        return Err(AjisaiError::from("Take count exceeds vector length"));
                    }
                    v[..take_count].to_vec()
                };

                if is_keep_mode {
                    interp.stack.push(count_val);
                }
                if result.is_empty() {
                    interp.stack.push(Value::nil());
                } else {
                    interp.stack.push(Value::from_vector(result));
                }
                Ok(())
            } else {
                if !is_keep_mode {
                    interp.stack.push(vector_val);
                }
                interp.stack.push(count_val);
                Err(AjisaiError::structure_error("vector", "other format"))
            }
        }
        OperationTargetMode::Stack => {
            if is_keep_mode {
                // Keep mode: preserve original stack, push taken elements
                let len = interp.stack.len();
                if count < 0 {
                    let abs_count = (-count) as usize;
                    if abs_count > len {
                        interp.stack.push(count_val);
                        return Err(AjisaiError::from("Take count exceeds stack length"));
                    }
                    let taken: Vec<Value> = interp.stack[len - abs_count..].to_vec();
                    interp.stack.extend(taken);
                } else {
                    let take_count = count as usize;
                    if take_count > len {
                        interp.stack.push(count_val);
                        return Err(AjisaiError::from("Take count exceeds stack length"));
                    }
                    let taken: Vec<Value> = interp.stack[..take_count].to_vec();
                    interp.stack.extend(taken);
                }
            } else {
                let len = interp.stack.len();
                if count < 0 {
                    let abs_count = (-count) as usize;
                    if abs_count > len {
                        interp.stack.push(count_val);
                        return Err(AjisaiError::from("Take count exceeds stack length"));
                    }
                    interp.stack = interp.stack.split_off(len - abs_count);
                } else {
                    let take_count = count as usize;
                    if take_count > len {
                        interp.stack.push(count_val);
                        return Err(AjisaiError::from("Take count exceeds stack length"));
                    }
                    interp.stack.truncate(take_count);
                }
            }
            Ok(())
        }
    }
}

/// SPLIT - 指定サイズで分割する（Form型）
///
/// 【消費モード】
/// - Consume（デフォルト）: 対象ベクタと引数を消費し、分割結果を返す
/// - Keep（,,）: 対象ベクタと引数を保持し、分割結果を追加する
pub fn op_split(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    // 引数ベクタ [sizes...] を取得
    let args_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // サイズを抽出
    let sizes: Vec<usize> = if args_val.is_vector() {
        let v = reconstruct_vector_elements(&args_val);
        if v.is_empty() {
            interp.stack.push(args_val);
            return Err(AjisaiError::from("SPLIT requires at least one size"));
        }

        let mut sizes = Vec::with_capacity(v.len());
        for elem in &v {
            match get_bigint_from_value(elem) {
                Ok(bi) => {
                    match bi.to_usize() {
                        Some(s) => sizes.push(s),
                        None => {
                            interp.stack.push(args_val);
                            return Err(AjisaiError::from("Split size is too large"));
                        }
                    }
                }
                Err(_) => {
                    interp.stack.push(args_val);
                    return Err(AjisaiError::from("Split sizes must be integers"));
                }
            }
        }
        sizes
    } else {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("SPLIT requires [sizes...] vector"));
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let vector_val = if is_keep_mode {
                interp.stack.last().cloned().ok_or_else(|| {
                    interp.stack.push(args_val.clone());
                    AjisaiError::from("SPLIT requires a vector to split")
                })?
            } else {
                interp.stack.pop().ok_or_else(|| {
                    interp.stack.push(args_val.clone());
                    AjisaiError::from("SPLIT requires a vector to split")
                })?
            };

            if vector_val.is_vector() {
                let v = reconstruct_vector_elements(&vector_val);
                let total_size: usize = sizes.iter().sum();
                if total_size > v.len() {
                    if !is_keep_mode {
                        interp.stack.push(Value::from_vector(v));
                    }
                    interp.stack.push(args_val);
                    return Err(AjisaiError::from("Split sizes sum exceeds vector length"));
                }

                let mut current_pos = 0;
                let mut result_vectors = Vec::new();

                for &size in &sizes {
                    let chunk = v[current_pos..current_pos + size].to_vec();
                    result_vectors.push(Value::from_vector(chunk));
                    current_pos += size;
                }
                if current_pos < v.len() {
                    let chunk = v[current_pos..].to_vec();
                    result_vectors.push(Value::from_vector(chunk));
                }
                if is_keep_mode {
                    interp.stack.push(args_val);
                }
                interp.stack.extend(result_vectors);
                Ok(())
            } else {
                if !is_keep_mode {
                    interp.stack.push(vector_val);
                }
                interp.stack.push(args_val);
                Err(AjisaiError::structure_error("vector", "other format"))
            }
        }
        OperationTargetMode::Stack => {
            let total_size: usize = sizes.iter().sum();
            if total_size > interp.stack.len() {
                interp.stack.push(args_val);
                return Err(AjisaiError::from("Split sizes sum exceeds stack length"));
            }

            if is_keep_mode {
                // Keep mode: preserve original stack elements, then push split results
                let original_elements: Vec<Value> = interp.stack.iter().cloned().collect();
                let mut result_stack = Vec::new();
                let mut pos = 0;

                for &size in &sizes {
                    let chunk: Vec<Value> = original_elements[pos..pos + size].to_vec();
                    result_stack.push(Value::from_vector(chunk));
                    pos += size;
                }
                if pos < original_elements.len() {
                    result_stack.push(Value::from_vector(original_elements[pos..].to_vec()));
                }
                interp.stack.extend(result_stack);
            } else {
                let mut remaining_stack = interp.stack.split_off(0);
                let mut result_stack = Vec::new();

                for &size in &sizes {
                    let chunk: Vec<Value> = remaining_stack.drain(..size).collect();
                    result_stack.push(Value::from_vector(chunk));
                }
                if !remaining_stack.is_empty() {
                    result_stack.push(Value::from_vector(remaining_stack));
                }
                interp.stack = result_stack;
            }
            Ok(())
        }
    }
}
