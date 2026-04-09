// rust/src/interpreter/vector_ops/position.rs
//
// 位置指定操作（0オリジン）: GET, INSERT, REPLACE, REMOVE

use super::extract_vector_elements;
use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{extract_integer_from_value, normalize_index};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::Value;

fn pop_index_operand(interp: &mut Interpreter) -> Result<(Value, i64)> {
    let index_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index = match extract_integer_from_value(&index_val) {
        Ok(value) => value,
        Err(error) => {
            interp.stack.push(index_val);
            return Err(error);
        }
    };
    Ok((index_val, index))
}

fn acquire_stacktop_target(
    interp: &mut Interpreter,
    arg_to_restore: &Value,
    preserve_source: bool,
) -> Result<Value> {
    if preserve_source {
        return interp.stack.last().cloned().ok_or_else(|| {
            interp.stack.push(arg_to_restore.clone());
            AjisaiError::StackUnderflow
        });
    }

    interp.stack.pop().ok_or_else(|| {
        interp.stack.push(arg_to_restore.clone());
        AjisaiError::StackUnderflow
    })
}

fn with_stacktop_vector_target<R, F>(
    interp: &mut Interpreter,
    arg_to_restore: &Value,
    preserve_source: bool,
    action: F,
) -> Result<R>
where
    F: FnOnce(&Value) -> Result<R>,
{
    let target_val = acquire_stacktop_target(interp, arg_to_restore, preserve_source)?;
    if !target_val.is_vector() {
        if !preserve_source {
            interp.stack.push(target_val);
        }
        interp.stack.push(arg_to_restore.clone());
        return Err(AjisaiError::create_structure_error(
            "vector",
            "other format",
        ));
    }

    match action(&target_val) {
        Ok(result) => Ok(result),
        Err(error) => {
            if !preserve_source {
                interp.stack.push(target_val);
            }
            interp.stack.push(arg_to_restore.clone());
            Err(error)
        }
    }
}

fn parse_index_element_args(word: &str, args_val: &Value) -> Result<(i64, Value)> {
    if !args_val.is_vector() || args_val.len() != 2 {
        return Err(AjisaiError::from(format!(
            "{} requires [index element]",
            word
        )));
    }

    let index = extract_integer_from_value(args_val.get_child(0).unwrap())
        .map_err(|_| AjisaiError::from(format!("{} index must be an integer", word)))?;
    let element = args_val.get_child(1).unwrap().clone();
    Ok((index, element))
}

/// GET - 指定位置の要素を取得する（Form型）
///
/// 【消費モード】
/// - Consume（デフォルト）: 対象ベクタを消費し、取得した要素を返す
/// - Keep（,,）: 対象ベクタを保持し、取得した要素を追加する
pub fn op_get(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let (index_val, index) = pop_index_operand(interp)?;

    // In gui_mode, GET always preserves the source vector (Form型)
    let preserve_source = interp.gui_mode || is_keep_mode;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let result_elem =
                with_stacktop_vector_target(interp, &index_val, preserve_source, |target_val| {
                    let len = target_val.len();
                    if len == 0 {
                        return Err(AjisaiError::IndexOutOfBounds { index, length: 0 });
                    }

                    let actual_index = normalize_index(index, len)
                        .ok_or(AjisaiError::IndexOutOfBounds { index, length: len })?;
                    Ok(target_val.get_child(actual_index).unwrap().clone())
                })?;

            if is_keep_mode {
                interp.stack.push(index_val);
            }
            interp.stack.push(result_elem);
            Ok(())
        }
        OperationTargetMode::Stack => {
            let stack_len = interp.stack.len();
            if stack_len == 0 {
                interp.stack.push(index_val);
                return Err(AjisaiError::IndexOutOfBounds { index, length: 0 });
            }

            let actual_index = match normalize_index(index, stack_len) {
                Some(idx) => idx,
                None => {
                    interp.stack.push(index_val);
                    return Err(AjisaiError::IndexOutOfBounds {
                        index,
                        length: stack_len,
                    });
                }
            };

            let result_elem = interp.stack[actual_index].clone();
            if !preserve_source {
                // Consume mode: スタック全体を消費し、取得した要素のみを残す
                interp.stack.clear();
            }
            interp.stack.push(result_elem);
            Ok(())
        }
    }
}

/// INSERT - 指定位置に要素を挿入する（Form型）
///
/// 【消費モード】
/// - Consume（デフォルト）: 対象ベクタと引数を消費し、挿入結果を返す
/// - Keep（,,）: 対象ベクタと引数を保持し、挿入結果を追加する
pub fn op_insert(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    // 引数ベクタ [index element] を取得
    let args_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 引数から index と element を抽出
    let (index, element) = match parse_index_element_args("INSERT", &args_val) {
        Ok(parsed) => parsed,
        Err(error) => {
            interp.stack.push(args_val);
            return Err(error);
        }
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let inserted =
                with_stacktop_vector_target(interp, &args_val, is_keep_mode, |vector_val| {
                    let mut values = extract_vector_elements(vector_val).to_vec();
                    let len = values.len() as i64;
                    let insert_index = if index < 0 {
                        (len + index).max(0) as usize
                    } else {
                        (index as usize).min(values.len())
                    };

                    values.insert(insert_index, element.clone());
                    Ok(Value::from_vector(values))
                })?;

            if is_keep_mode {
                interp.stack.push(args_val);
            }
            interp.stack.push(inserted);
            Ok(())
        }
        OperationTargetMode::Stack => {
            let len = interp.stack.len() as i64;
            let insert_index = if index < 0 {
                (len + index).max(0) as usize
            } else {
                (index as usize).min(interp.stack.len())
            };
            interp.stack.insert(insert_index, element);
            Ok(())
        }
    }
}

/// REPLACE - 指定位置の要素を置き換える（Form型）
///
/// 【消費モード】
/// - Consume（デフォルト）: 対象ベクタと引数を消費し、置換結果を返す
/// - Keep（,,）: 対象ベクタと引数を保持し、置換結果を追加する
pub fn op_replace(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    // 引数ベクタ [index new_element] を取得
    let args_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let (index, new_element) = match parse_index_element_args("REPLACE", &args_val) {
        Ok(parsed) => parsed,
        Err(error) => {
            interp.stack.push(args_val);
            return Err(error);
        }
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let replaced =
                with_stacktop_vector_target(interp, &args_val, is_keep_mode, |vector_val| {
                    let mut values = extract_vector_elements(vector_val).to_vec();
                    let len = values.len();
                    let actual_index = normalize_index(index, len)
                        .ok_or(AjisaiError::IndexOutOfBounds { index, length: len })?;

                    values[actual_index] = new_element.clone();
                    Ok(Value::from_vector(values))
                })?;

            if is_keep_mode {
                interp.stack.push(args_val);
            }
            interp.stack.push(replaced);
            Ok(())
        }
        OperationTargetMode::Stack => {
            let len = interp.stack.len();
            let actual_index = match normalize_index(index, len) {
                Some(idx) => idx,
                None => {
                    interp.stack.push(args_val);
                    return Err(AjisaiError::IndexOutOfBounds { index, length: len });
                }
            };

            if is_keep_mode {
                let original_stack = interp.stack.clone();
                interp.stack[actual_index] = new_element;
                // In Keep mode for stack, we already modified in-place.
                // The spec for Stack+Keep says "preserve originals and add result".
                // For REPLACE in stack mode, the stack IS the result.
                // Keep original + push modified doesn't quite make sense for stack mutations.
                // We follow existing behavior.
                let _ = original_stack; // Keep mode doesn't fundamentally change stack-mode REPLACE
            } else {
                interp.stack[actual_index] = new_element;
            }
            Ok(())
        }
    }
}

/// REMOVE - 指定位置の要素を削除する（Form型）
///
/// 【消費モード】
/// - Consume（デフォルト）: 対象ベクタを消費し、削除結果を返す
/// - Keep（,,）: 対象ベクタを保持し、削除結果を追加する
pub fn op_remove(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let (index_val, index) = pop_index_operand(interp)?;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let removed =
                with_stacktop_vector_target(interp, &index_val, is_keep_mode, |vector_val| {
                    let mut values = extract_vector_elements(vector_val).to_vec();
                    let len = values.len();
                    let actual_index = normalize_index(index, len)
                        .ok_or(AjisaiError::IndexOutOfBounds { index, length: len })?;

                    values.remove(actual_index);
                    if values.is_empty() {
                        return Ok(Value::nil());
                    }
                    Ok(Value::from_vector(values))
                })?;

            if is_keep_mode {
                interp.stack.push(index_val);
            }
            interp.stack.push(removed);
            Ok(())
        }
        OperationTargetMode::Stack => {
            let len = interp.stack.len();
            let actual_index = match normalize_index(index, len) {
                Some(idx) => idx,
                None => {
                    interp.stack.push(index_val);
                    return Err(AjisaiError::IndexOutOfBounds { index, length: len });
                }
            };

            interp.stack.remove(actual_index);
            Ok(())
        }
    }
}
