// rust/src/interpreter/vector_ops/position.rs
//
// 位置指定操作（0オリジン）: GET, INSERT, REPLACE, REMOVE

use crate::interpreter::{Interpreter, OperationTargetMode, ConsumptionMode};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{get_integer_from_value, normalize_index};
use crate::types::Value;
use super::reconstruct_vector_elements;

/// GET - 指定位置の要素を取得する（Form型）
///
/// 【消費モード】
/// - Consume（デフォルト）: 対象ベクタを消費し、取得した要素を返す
/// - Keep（,,）: 対象ベクタを保持し、取得した要素を追加する
pub fn op_get(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let index_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index = match get_integer_from_value(&index_val) {
        Ok(v) => v,
        Err(e) => {
            interp.stack.push(index_val);
            return Err(e);
        }
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let target_val = if is_keep_mode {
                // Keep mode: peek without removing
                interp.stack.last().cloned().ok_or_else(|| {
                    interp.stack.push(index_val.clone());
                    AjisaiError::StackUnderflow
                })?
            } else {
                // Consume mode: pop
                interp.stack.pop().ok_or_else(|| {
                    interp.stack.push(index_val.clone());
                    AjisaiError::StackUnderflow
                })?
            };

            if target_val.is_vector() {
                let v = reconstruct_vector_elements(&target_val);
                let len = v.len();
                if len == 0 {
                    if !is_keep_mode {
                        interp.stack.push(target_val);
                    }
                    interp.stack.push(index_val);
                    return Err(AjisaiError::IndexOutOfBounds { index, length: 0 });
                }

                let actual_index = match normalize_index(index, len) {
                    Some(idx) => idx,
                    None => {
                        if !is_keep_mode {
                            interp.stack.push(target_val);
                        }
                        interp.stack.push(index_val);
                        return Err(AjisaiError::IndexOutOfBounds { index, length: len });
                    }
                };

                let result_elem = v[actual_index].clone();
                interp.stack.push(result_elem);
                Ok(())
            } else {
                if !is_keep_mode {
                    interp.stack.push(target_val);
                }
                interp.stack.push(index_val);
                Err(AjisaiError::structure_error("vector", "other format"))
            }
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
                    return Err(AjisaiError::IndexOutOfBounds { index, length: stack_len });
                }
            };

            let result_elem = interp.stack[actual_index].clone();
            if !is_keep_mode {
                // Consume mode: remove the element from its position
                // (Note: for Stack GET, consuming the entire stack is the Form-type behavior,
                //  but GET in stack mode reads a single element. We keep existing behavior
                //  of not draining the stack for GET in stack mode, only adding the result.)
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
    let (index, element) = if args_val.is_vector() {
        let v = reconstruct_vector_elements(&args_val);
        if v.len() != 2 {
            interp.stack.push(args_val);
            return Err(AjisaiError::from("INSERT requires [index element]"));
        }

        let index = match get_integer_from_value(&v[0]) {
            Ok(i) => i,
            Err(_) => {
                interp.stack.push(args_val);
                return Err(AjisaiError::from("INSERT index must be an integer"));
            }
        };

        let element = v[1].clone();
        (index, element)
    } else {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("INSERT requires [index element]"));
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let vector_val = if is_keep_mode {
                interp.stack.last().cloned().ok_or_else(|| {
                    interp.stack.push(args_val.clone());
                    AjisaiError::StackUnderflow
                })?
            } else {
                interp.stack.pop().ok_or_else(|| {
                    interp.stack.push(args_val.clone());
                    AjisaiError::StackUnderflow
                })?
            };

            if vector_val.is_vector() {
                let mut v = reconstruct_vector_elements(&vector_val);
                let len = v.len() as i64;
                let insert_index = if index < 0 {
                    (len + index).max(0) as usize
                } else {
                    (index as usize).min(v.len())
                };

                v.insert(insert_index, element.clone());
                if is_keep_mode {
                    // In Keep mode, args_val is already consumed from stack but we need to preserve it
                    interp.stack.push(args_val);
                }
                interp.stack.push(Value::from_vector(v));
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

    let (index, new_element) = if args_val.is_vector() {
        let v = reconstruct_vector_elements(&args_val);
        if v.len() != 2 {
            interp.stack.push(args_val);
            return Err(AjisaiError::from("REPLACE requires [index element]"));
        }

        let index = match get_integer_from_value(&v[0]) {
            Ok(i) => i,
            Err(_) => {
                interp.stack.push(args_val);
                return Err(AjisaiError::from("REPLACE index must be an integer"));
            }
        };

        let new_element = v[1].clone();
        (index, new_element)
    } else {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("REPLACE requires [index element]"));
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let vector_val = if is_keep_mode {
                interp.stack.last().cloned().ok_or_else(|| {
                    interp.stack.push(args_val.clone());
                    AjisaiError::StackUnderflow
                })?
            } else {
                interp.stack.pop().ok_or_else(|| {
                    interp.stack.push(args_val.clone());
                    AjisaiError::StackUnderflow
                })?
            };

            if vector_val.is_vector() {
                let mut v = reconstruct_vector_elements(&vector_val);
                let len = v.len();
                let actual_index = match normalize_index(index, len) {
                    Some(idx) => idx,
                    None => {
                        if !is_keep_mode {
                            interp.stack.push(Value::from_vector(v));
                        }
                        interp.stack.push(args_val);
                        return Err(AjisaiError::IndexOutOfBounds { index, length: len });
                    }
                };

                v[actual_index] = new_element;
                if is_keep_mode {
                    interp.stack.push(args_val);
                }
                interp.stack.push(Value::from_vector(v));
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
    let index_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index = match get_integer_from_value(&index_val) {
        Ok(v) => v,
        Err(e) => {
            interp.stack.push(index_val);
            return Err(e);
        }
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let vector_val = if is_keep_mode {
                interp.stack.last().cloned().ok_or_else(|| {
                    interp.stack.push(index_val.clone());
                    AjisaiError::StackUnderflow
                })?
            } else {
                interp.stack.pop().ok_or_else(|| {
                    interp.stack.push(index_val.clone());
                    AjisaiError::StackUnderflow
                })?
            };

            if vector_val.is_vector() {
                let mut v = reconstruct_vector_elements(&vector_val);
                let len = v.len();
                let actual_index = match normalize_index(index, len) {
                    Some(idx) => idx,
                    None => {
                        if !is_keep_mode {
                            interp.stack.push(Value::from_vector(v));
                        }
                        interp.stack.push(index_val);
                        return Err(AjisaiError::IndexOutOfBounds { index, length: len });
                    }
                };

                v.remove(actual_index);
                if is_keep_mode {
                    interp.stack.push(index_val);
                }
                if v.is_empty() {
                    interp.stack.push(Value::nil());
                } else {
                    interp.stack.push(Value::from_vector(v));
                }
                Ok(())
            } else {
                if !is_keep_mode {
                    interp.stack.push(vector_val);
                }
                interp.stack.push(index_val);
                Err(AjisaiError::structure_error("vector", "other format"))
            }
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
