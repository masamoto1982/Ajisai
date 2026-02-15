// rust/src/interpreter/vector_ops/structure.rs
//
// ベクタ構造操作: CONCAT, REVERSE, RANGE, REORDER, COLLECT

use crate::interpreter::{Interpreter, OperationTargetMode, ConsumptionMode};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{get_integer_from_value, get_bigint_from_value, normalize_index};
use crate::types::{Value};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};
use super::reconstruct_vector_elements;

/// CONCAT - 複数のベクタを連結する（Form型）
pub fn op_concat(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let (count_i64, count_value_opt) = if let Some(top) = interp.stack.last() {
                if let Ok(count_bigint) = get_bigint_from_value(top) {
                    let count_val = interp.stack.pop().unwrap();
                    if let Some(c) = count_bigint.to_i64() {
                        (c, Some(count_val))
                    } else {
                        interp.stack.push(count_val);
                        return Err(AjisaiError::from("Count is too large"));
                    }
                } else {
                    (2, None)
                }
            } else {
                return Err(AjisaiError::StackUnderflow);
            };

            let abs_count = count_i64.unsigned_abs() as usize;
            let is_reversed = count_i64 < 0;

            if interp.stack.len() < abs_count {
                if let Some(count_val) = count_value_opt {
                    interp.stack.push(count_val);
                }
                return Err(AjisaiError::StackUnderflow);
            }

            let vecs_to_concat: Vec<Value> = if is_keep_mode {
                let stack_len = interp.stack.len();
                interp.stack[stack_len - abs_count..].iter().cloned().collect()
            } else {
                interp.stack.split_off(interp.stack.len() - abs_count)
            };

            let mut ordered = vecs_to_concat;
            if is_reversed {
                ordered.reverse();
            }

            let mut result_vec = Vec::new();
            for val in ordered {
                if val.is_vector() {
                    let v = reconstruct_vector_elements(&val);
                    result_vec.extend(v);
                } else {
                    result_vec.push(val);
                }
            }

            if is_keep_mode {
                if let Some(count_val) = count_value_opt {
                    interp.stack.push(count_val);
                }
            }
            interp.stack.push(Value::from_vector(result_vec));
            Ok(())
        }
        OperationTargetMode::Stack => {
            let (count_i64, count_value_opt) = if let Some(top) = interp.stack.last() {
                if let Ok(count_bigint) = get_bigint_from_value(top) {
                    let count_val = interp.stack.pop().unwrap();
                    if let Some(c) = count_bigint.to_i64() {
                        (c, Some(count_val))
                    } else {
                        interp.stack.push(count_val);
                        return Err(AjisaiError::from("Count is too large"));
                    }
                } else {
                    (interp.stack.len() as i64, None)
                }
            } else {
                return Err(AjisaiError::StackUnderflow);
            };

            let abs_count = count_i64.unsigned_abs() as usize;
            let is_reversed = count_i64 < 0;

            if interp.stack.len() < abs_count {
                if let Some(count_val) = count_value_opt {
                    interp.stack.push(count_val);
                }
                return Err(AjisaiError::StackUnderflow);
            }

            let vecs_to_concat: Vec<Value> = if is_keep_mode {
                let stack_len = interp.stack.len();
                interp.stack[stack_len - abs_count..].iter().cloned().collect()
            } else {
                interp.stack.split_off(interp.stack.len() - abs_count)
            };

            let mut ordered = vecs_to_concat;
            if is_reversed {
                ordered.reverse();
            }

            let mut result_vec = Vec::new();
            for val in ordered {
                if val.is_vector() {
                    let v = reconstruct_vector_elements(&val);
                    result_vec.extend(v);
                } else {
                    result_vec.push(val);
                }
            }

            if is_keep_mode {
                if let Some(count_val) = count_value_opt {
                    interp.stack.push(count_val);
                }
            }
            interp.stack.push(Value::from_vector(result_vec));
            Ok(())
        }
    }
}

/// REVERSE - 要素の順序を反転する（Form型）
///
/// 【消費モード】
/// - Consume（デフォルト）: 対象ベクタを消費し、反転結果を返す
/// - Keep（,,）: 対象ベクタを保持し、反転結果を追加する
pub fn op_reverse(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let val = if is_keep_mode {
                interp.stack.last().cloned().ok_or(AjisaiError::StackUnderflow)?
            } else {
                interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
            };

            if val.is_vector() {
                let mut v = reconstruct_vector_elements(&val);
                if !interp.disable_no_change_check {
                    if v.len() < 2 {
                        if !is_keep_mode {
                            interp.stack.push(Value::from_vector(v));
                        }
                        return Err(AjisaiError::NoChange { word: "REVERSE".into() });
                    }
                    let original_v = v.clone();
                    v.reverse();
                    if v == original_v {
                        if !is_keep_mode {
                            interp.stack.push(Value::from_vector(original_v));
                        }
                        return Err(AjisaiError::NoChange { word: "REVERSE".into() });
                    }
                    interp.stack.push(Value::from_vector(v));
                } else {
                    v.reverse();
                    interp.stack.push(Value::from_vector(v));
                }
                Ok(())
            } else {
                if !is_keep_mode {
                    interp.stack.push(val);
                }
                Err(AjisaiError::structure_error("vector", "other format"))
            }
        }
        OperationTargetMode::Stack => {
            if is_keep_mode {
                // Keep mode: preserve original, add reversed elements
                let original = interp.stack.clone();
                if !interp.disable_no_change_check {
                    if original.len() < 2 {
                        return Err(AjisaiError::NoChange { word: "REVERSE".into() });
                    }
                    let mut reversed = original.clone();
                    reversed.reverse();
                    if reversed == original {
                        return Err(AjisaiError::NoChange { word: "REVERSE".into() });
                    }
                    interp.stack.extend(reversed);
                } else {
                    let mut reversed = original.clone();
                    reversed.reverse();
                    interp.stack.extend(reversed);
                }
            } else {
                if !interp.disable_no_change_check {
                    if interp.stack.len() < 2 {
                        return Err(AjisaiError::NoChange { word: "REVERSE".into() });
                    }
                    let original_stack = interp.stack.clone();
                    interp.stack.reverse();
                    if interp.stack == original_stack {
                        interp.stack = original_stack;
                        return Err(AjisaiError::NoChange { word: "REVERSE".into() });
                    }
                } else {
                    interp.stack.reverse();
                }
            }
            Ok(())
        }
    }
}

/// RANGE - 数値範囲を生成する（Form型）
pub fn op_range(interp: &mut Interpreter) -> Result<()> {
    // 引数ベクタを取得
    let args_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 引数ベクタから start, end, step を抽出
    let (start, end, step) = if args_val.is_vector() {
        let v = reconstruct_vector_elements(&args_val);
        if v.len() < 2 || v.len() > 3 {
            interp.stack.push(args_val);
            return Err(AjisaiError::from("RANGE requires [start end] or [start end step]"));
        }

        let start = match get_bigint_from_value(&v[0]) {
            Ok(bi) => match bi.to_i64() {
                Some(i) => i,
                None => {
                    interp.stack.push(args_val);
                    return Err(AjisaiError::from("RANGE start is too large"));
                }
            },
            Err(_) => {
                interp.stack.push(args_val);
                return Err(AjisaiError::from("RANGE start must be an integer"));
            }
        };

        let end = match get_bigint_from_value(&v[1]) {
            Ok(bi) => match bi.to_i64() {
                Some(i) => i,
                None => {
                    interp.stack.push(args_val);
                    return Err(AjisaiError::from("RANGE end is too large"));
                }
            },
            Err(_) => {
                interp.stack.push(args_val);
                return Err(AjisaiError::from("RANGE end must be an integer"));
            }
        };

        let step = if v.len() == 3 {
            match get_bigint_from_value(&v[2]) {
                Ok(bi) => match bi.to_i64() {
                    Some(i) => i,
                    None => {
                        interp.stack.push(args_val);
                        return Err(AjisaiError::from("RANGE step is too large"));
                    }
                },
                Err(_) => {
                    interp.stack.push(args_val);
                    return Err(AjisaiError::from("RANGE step must be an integer"));
                }
            }
        } else {
            if start <= end { 1 } else { -1 }
        };

        (start, end, step)
    } else {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("RANGE requires [start end] or [start end step]"));
    };

    if step == 0 {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("RANGE step cannot be 0"));
    }

    if (start < end && step < 0) || (start > end && step > 0) {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("RANGE would create an infinite sequence (check start, end, and step values)"));
    }

    let mut range_vec = Vec::new();
    let mut current = start;

    if step > 0 {
        while current <= end {
            range_vec.push(Value::from_fraction(Fraction::new(BigInt::from(current), BigInt::one())));
            current += step;
        }
    } else {
        while current >= end {
            range_vec.push(Value::from_fraction(Fraction::new(BigInt::from(current), BigInt::one())));
            current += step;
        }
    }

    interp.stack.push(Value::from_vector(range_vec));

    Ok(())
}

/// REORDER - インデックスリストで要素を並べ替える（Form型）
///
/// 【消費モード】
/// - Consume（デフォルト）: 対象ベクタと引数を消費し、並べ替え結果を返す
/// - Keep（,,）: 対象ベクタと引数を保持し、並べ替え結果を追加する
pub fn op_reorder(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    // インデックスリストをポップ
    let indices_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // インデックスリストを抽出
    let indices = if indices_val.is_vector() {
        let v = reconstruct_vector_elements(&indices_val);
        if v.is_empty() {
            interp.stack.push(indices_val);
            return Err(AjisaiError::from("REORDER requires non-empty index list"));
        }

        let mut indices = Vec::with_capacity(v.len());
        for elem in &v {
            match get_integer_from_value(elem) {
                Ok(i) => indices.push(i),
                Err(_) => {
                    interp.stack.push(indices_val);
                    return Err(AjisaiError::from("REORDER indices must be integers"));
                }
            }
        }
        indices
    } else {
        match get_integer_from_value(&indices_val) {
            Ok(i) => vec![i],
            Err(_) => {
                interp.stack.push(indices_val);
                return Err(AjisaiError::from("REORDER requires index list"));
            }
        }
    };

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let target_val = if is_keep_mode {
                interp.stack.last().cloned().ok_or_else(|| {
                    interp.stack.push(indices_val.clone());
                    AjisaiError::StackUnderflow
                })?
            } else {
                interp.stack.pop().ok_or_else(|| {
                    interp.stack.push(indices_val.clone());
                    AjisaiError::StackUnderflow
                })?
            };

            if target_val.is_vector() {
                let elements = reconstruct_vector_elements(&target_val);
                let len = elements.len();

                if len == 0 {
                    if !is_keep_mode {
                        interp.stack.push(target_val);
                    }
                    interp.stack.push(indices_val);
                    return Err(AjisaiError::from("REORDER: target vector is empty"));
                }

                let mut result = Vec::with_capacity(indices.len());
                for &idx in &indices {
                    let actual = match normalize_index(idx, len) {
                        Some(i) => i,
                        None => {
                            if !is_keep_mode {
                                interp.stack.push(target_val);
                            }
                            interp.stack.push(indices_val);
                            return Err(AjisaiError::IndexOutOfBounds { index: idx, length: len });
                        }
                    };
                    result.push(elements[actual].clone());
                }

                if is_keep_mode {
                    interp.stack.push(indices_val);
                }
                if result.is_empty() {
                    interp.stack.push(Value::nil());
                } else {
                    interp.stack.push(Value::from_vector(result));
                }
                Ok(())
            } else {
                if !is_keep_mode {
                    interp.stack.push(target_val);
                }
                interp.stack.push(indices_val);
                Err(AjisaiError::structure_error("vector", "other format"))
            }
        }
        OperationTargetMode::Stack => {
            let len = interp.stack.len();

            if len == 0 {
                interp.stack.push(indices_val);
                return Err(AjisaiError::from("REORDER: stack is empty"));
            }

            let mut result = Vec::with_capacity(indices.len());
            for &idx in &indices {
                let actual = match normalize_index(idx, len) {
                    Some(i) => i,
                    None => {
                        interp.stack.push(indices_val);
                        return Err(AjisaiError::IndexOutOfBounds { index: idx, length: len });
                    }
                };
                result.push(interp.stack[actual].clone());
            }

            if !is_keep_mode {
                interp.stack.clear();
            }
            interp.stack.extend(result);
            Ok(())
        }
    }
}

/// COLLECT - スタックからN個の値を収集してベクタを作成する
pub fn op_collect(interp: &mut Interpreter) -> Result<()> {
    let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let count_bigint = match get_bigint_from_value(&count_val) {
        Ok(bi) => bi,
        Err(_) => {
            interp.stack.push(count_val);
            return Err(AjisaiError::structure_error("integer", "other format"));
        }
    };

    let count: usize = match count_bigint.to_usize() {
        Some(c) if c > 0 => c,
        _ => {
            interp.stack.push(count_val);
            return Err(AjisaiError::from("COLLECT count must be a positive integer"));
        }
    };

    if interp.stack.len() < count {
        interp.stack.push(count_val);
        return Err(AjisaiError::StackUnderflow);
    }

    let collected: Vec<Value> = interp.stack.split_off(interp.stack.len() - count);

    interp.stack.push(Value::from_vector(collected));
    Ok(())
}
