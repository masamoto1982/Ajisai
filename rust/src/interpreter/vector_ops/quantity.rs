// rust/src/interpreter/vector_ops/quantity.rs
//
// 量指定操作（1オリジン）: LENGTH, TAKE, SPLIT

use super::extract_vector_elements;
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{extract_bigint_from_value, extract_integer_from_value, create_number_value};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::Value;
use num_traits::ToPrimitive;

fn compute_take_bounds(len: usize, count: i64, target: &str) -> Result<(usize, usize)> {
    if count < 0 {
        let take = (-count) as usize;
        if take > len {
            return Err(AjisaiError::from(format!(
                "Take count exceeds {} length",
                target
            )));
        }
        return Ok((len - take, len));
    }

    let take = count as usize;
    if take > len {
        return Err(AjisaiError::from(format!(
            "Take count exceeds {} length",
            target
        )));
    }
    Ok((0, take))
}

/// LENGTH - 要素数を取得する（Form型）
///
/// 【消費モード】
/// - Consume（デフォルト）: 対象ベクタを消費し、要素数を返す
/// - Keep（,,）: 対象ベクタを保持し、要素数を追加する
pub fn op_length(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    // In gui_mode, LENGTH always preserves the source vector (Form型)
    let preserve_source = interp.gui_mode || is_keep_mode;

    let len = match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            if preserve_source {
                let target_val = interp.stack.last().ok_or(AjisaiError::StackUnderflow)?;

                if target_val.is_nil() {
                    0
                } else if target_val.is_vector() {
                    extract_vector_elements(target_val).len()
                } else {
                    return Err(AjisaiError::create_structure_error("vector", "other format"));
                }
            } else {
                let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

                if target_val.is_nil() {
                    0
                } else if target_val.is_vector() {
                    extract_vector_elements(&target_val).len()
                } else {
                    interp.stack.push(target_val);
                    return Err(AjisaiError::create_structure_error("vector", "other format"));
                }
            }
        }
        OperationTargetMode::Stack => {
            if preserve_source {
                interp.stack.len()
            } else {
                let len = interp.stack.len();
                interp.stack.clear();
                len
            }
        }
    };
    let len_frac = Fraction::from(len as i64);
    interp.stack.push(create_number_value(len_frac));
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
    let count = match extract_integer_from_value(&count_val) {
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

            if !vector_val.is_vector() {
                if !is_keep_mode {
                    interp.stack.push(vector_val);
                }
                interp.stack.push(count_val);
                return Err(AjisaiError::create_structure_error("vector", "other format"));
            }

            let elements = extract_vector_elements(&vector_val);
            let (start, end) = match compute_take_bounds(elements.len(), count, "vector") {
                Ok(bounds) => bounds,
                Err(error) => {
                    if !is_keep_mode {
                        interp.stack.push(vector_val);
                    }
                    interp.stack.push(count_val);
                    return Err(error);
                }
            };
            let result = elements[start..end].to_vec();

            if is_keep_mode {
                interp.stack.push(count_val);
            }
            if result.is_empty() {
                interp.stack.push(Value::nil());
            } else {
                interp.stack.push(Value::from_vector(result));
            }
            Ok(())
        }
        OperationTargetMode::Stack => {
            let len = interp.stack.len();
            let (start, end) = match compute_take_bounds(len, count, "stack") {
                Ok(bounds) => bounds,
                Err(error) => {
                    interp.stack.push(count_val);
                    return Err(error);
                }
            };

            if is_keep_mode {
                let taken: Vec<Value> = interp.stack[start..end].to_vec();
                interp.stack.extend(taken);
            } else if count < 0 {
                interp.stack = interp.stack.split_off(start);
            } else {
                interp.stack.truncate(end);
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
        let n = args_val.len();
        if n == 0 {
            interp.stack.push(args_val);
            return Err(AjisaiError::from("SPLIT requires at least one size"));
        }

        let mut sizes = Vec::with_capacity(n);
        for i in 0..n {
            match extract_bigint_from_value(args_val.get_child(i).unwrap()) {
                Ok(bi) => match bi.to_usize() {
                    Some(s) => sizes.push(s),
                    None => {
                        interp.stack.push(args_val);
                        return Err(AjisaiError::from("Split size is too large"));
                    }
                },
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
                let elements = extract_vector_elements(&vector_val);
                let total_size: usize = sizes.iter().sum();
                if total_size > elements.len() {
                    if !is_keep_mode {
                        interp.stack.push(vector_val);
                    }
                    interp.stack.push(args_val);
                    return Err(AjisaiError::from("Split sizes sum exceeds vector length"));
                }

                let mut current_pos = 0;
                let mut result_vectors = Vec::new();

                for &size in &sizes {
                    let chunk = elements[current_pos..current_pos + size].to_vec();
                    result_vectors.push(Value::from_vector(chunk));
                    current_pos += size;
                }
                if current_pos < elements.len() {
                    let chunk = elements[current_pos..].to_vec();
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
                Err(AjisaiError::create_structure_error("vector", "other format"))
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
