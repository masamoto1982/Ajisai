//! 行列演算ワード
//!
//! Vector指向型システムでの行列・配列操作を提供
//! ブロードキャスト、形状操作、数学関数などを実装

use crate::error::{AjisaiError, Result};
use crate::interpreter::{Interpreter, OperationTarget};
use crate::interpreter::helpers::wrap_number;
use crate::types::{Value, ValueType, infer_shape, flatten_numbers};
use crate::types::tensor::{transpose, reshape, rank};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_traits::{One, Zero};

// ============================================================================
// 形状操作ワード
// ============================================================================

/// SHAPE - ベクタの形状を取得
///
/// 使用法:
///   [ 1 2 3 ] SHAPE           → [ 1 2 3 ] [ 3 ]
///   [ [ 1 2 ] [ 3 4 ] ] SHAPE → [ [ 1 2 ] [ 3 4 ] ] [ 2 2 ]
///
/// 形状は1次元Vectorとして返される
pub fn op_shape(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("SHAPE does not support Stack (..) mode"));
    }

    let val = interp.stack.last().ok_or(AjisaiError::StackUnderflow)?;

    let shape_vec = match &val.val_type {
        ValueType::Vector(v) => {
            infer_shape(v).map_err(|e| AjisaiError::from(format!("Failed to get shape: {}", e)))?
        }
        _ => {
            return Err(AjisaiError::from(format!(
                "SHAPE requires vector, got {}",
                val.val_type
            )));
        }
    };

    let shape_values: Vec<Value> = shape_vec
        .iter()
        .map(|&n| Value::from_number(Fraction::new(BigInt::from(n as i64), BigInt::one())))
        .collect();

    interp.stack.push(Value::from_vector(shape_values));
    Ok(())
}

/// RANK - ベクタの次元数を取得
///
/// 使用法:
///   [ 1 2 3 ] RANK           → [ 1 2 3 ] [ 1 ]
///   [ [ 1 2 ] [ 3 4 ] ] RANK → [ [ 1 2 ] [ 3 4 ] ] [ 2 ]
pub fn op_rank(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("RANK does not support Stack (..) mode"));
    }

    let val = interp.stack.last().ok_or(AjisaiError::StackUnderflow)?;

    let r = match &val.val_type {
        ValueType::Vector(v) => {
            rank(v).map_err(|e| AjisaiError::from(format!("Failed to get rank: {}", e)))?
        }
        _ => {
            return Err(AjisaiError::from(format!(
                "RANK requires vector, got {}",
                val.val_type
            )));
        }
    };

    let rank_frac = Fraction::new(BigInt::from(r as i64), BigInt::one());
    interp.stack.push(wrap_number(rank_frac));
    Ok(())
}

/// RESHAPE - ベクタの形状を変更
///
/// 使用法:
///   [ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE → [ [ 1 2 3 ] [ 4 5 6 ] ]
///   [ 1 2 3 4 5 6 ] [ 3 2 ] RESHAPE → [ [ 1 2 ] [ 3 4 ] [ 5 6 ] ]
pub fn op_reshape(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("RESHAPE does not support Stack (..) mode"));
    }

    let shape_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let data_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 形状をベクタから抽出
    let new_shape: Vec<usize> = match &shape_val.val_type {
        ValueType::Vector(v) => {
            let mut shape = Vec::with_capacity(v.len());
            for elem in v {
                if let ValueType::Number(n) = &elem.val_type {
                    let dim = n.as_usize()
                        .ok_or_else(|| AjisaiError::from("Shape dimensions must be positive integers"))?;
                    shape.push(dim);
                } else {
                    interp.stack.push(data_val);
                    interp.stack.push(shape_val);
                    return Err(AjisaiError::from("Shape must contain only numbers"));
                }
            }
            shape
        }
        _ => {
            interp.stack.push(data_val);
            interp.stack.push(shape_val);
            return Err(AjisaiError::from("RESHAPE requires shape as vector"));
        }
    };

    // データをベクタから抽出
    let data_vec = match &data_val.val_type {
        ValueType::Vector(v) => v,
        _ => {
            interp.stack.push(data_val);
            interp.stack.push(shape_val);
            return Err(AjisaiError::from("RESHAPE requires data as vector"));
        }
    };

    let result = reshape(data_vec, &new_shape)
        .map_err(|e| AjisaiError::from(format!("RESHAPE failed: {}", e)))?;
    interp.stack.push(Value::from_vector(result));
    Ok(())
}

/// TRANSPOSE - 2次元ベクタの転置
///
/// 使用法:
///   [ [ 1 2 3 ] [ 4 5 6 ] ] TRANSPOSE → [ [ 1 4 ] [ 2 5 ] [ 3 6 ] ]
pub fn op_transpose(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("TRANSPOSE does not support Stack (..) mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let data_vec = match &val.val_type {
        ValueType::Vector(v) => v,
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("TRANSPOSE requires vector"));
        }
    };

    let result = transpose(data_vec)
        .map_err(|e| {
            interp.stack.push(val.clone());
            AjisaiError::from(format!("TRANSPOSE failed: {}", e))
        })?;
    interp.stack.push(Value::from_vector(result));
    Ok(())
}

// ============================================================================
// 基本数学関数
// ============================================================================

/// 単項演算のヘルパー関数
fn unary_math_op<F>(interp: &mut Interpreter, op: F, op_name: &str) -> Result<()>
where
    F: Fn(&Fraction) -> Fraction,
{
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from(format!("{} does not support Stack (..) mode", op_name)));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    match &val.val_type {
        ValueType::Vector(v) => {
            let result = apply_unary_to_vector(v, &op)?;
            interp.stack.push(Value::from_vector(result));
            Ok(())
        }
        _ => {
            interp.stack.push(val);
            Err(AjisaiError::from(format!("{} requires vector", op_name)))
        }
    }
}

/// ベクタに単項演算を再帰的に適用
fn apply_unary_to_vector<F>(values: &[Value], op: &F) -> Result<Vec<Value>>
where
    F: Fn(&Fraction) -> Fraction,
{
    let mut result = Vec::with_capacity(values.len());
    for val in values {
        match &val.val_type {
            ValueType::Number(n) => {
                result.push(Value::from_number(op(n)));
            }
            ValueType::Vector(inner) => {
                let inner_result = apply_unary_to_vector(inner, op)?;
                result.push(Value::from_vector(inner_result));
            }
            _ => {
                return Err(AjisaiError::from("Cannot apply math operation to non-numeric value"));
            }
        }
    }
    Ok(result)
}

/// FLOOR - 切り捨て（負の無限大方向）
///
/// 使用法:
///   [ 7/3 ] FLOOR → [ 2 ]      # 7/3 = 2.333... → 2
///   [ -7/3 ] FLOOR → [ -3 ]    # -7/3 = -2.333... → -3
///   [ 5 ] FLOOR → [ 5 ]        # 整数はそのまま
pub fn op_floor(interp: &mut Interpreter) -> Result<()> {
    unary_math_op(interp, |f| f.floor(), "FLOOR")
}

/// CEIL - 切り上げ（正の無限大方向）
///
/// 使用法:
///   [ 7/3 ] CEIL → [ 3 ]       # 7/3 = 2.333... → 3
///   [ -7/3 ] CEIL → [ -2 ]     # -7/3 = -2.333... → -2
///   [ 5 ] CEIL → [ 5 ]         # 整数はそのまま
pub fn op_ceil(interp: &mut Interpreter) -> Result<()> {
    unary_math_op(interp, |f| f.ceil(), "CEIL")
}

/// ROUND - 四捨五入
///
/// 使用法:
///   [ 7/3 ] ROUND → [ 2 ]      # 2.333... → 2
///   [ 5/2 ] ROUND → [ 3 ]      # 2.5 → 3（0から遠い方向）
///   [ -5/2 ] ROUND → [ -3 ]    # -2.5 → -3（0から遠い方向）
pub fn op_round(interp: &mut Interpreter) -> Result<()> {
    unary_math_op(interp, |f| f.round(), "ROUND")
}

/// MOD - 剰余（数学的剰余: a mod b = a - b * floor(a/b)）
///
/// 使用法:
///   [ 7 ] [ 3 ] MOD → [ 1 ]
///   [ -7 ] [ 3 ] MOD → [ 2 ]   # 数学的剰余
///   [ 7 8 9 ] [ 3 ] MOD → [ 1 2 0 ]  # ブロードキャスト
pub fn op_mod(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("MOD does not support Stack (..) mode"));
    }

    let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let a_vec = match &a_val.val_type {
        ValueType::Vector(v) => v,
        _ => {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
            return Err(AjisaiError::from("MOD requires vectors"));
        }
    };

    let b_vec = match &b_val.val_type {
        ValueType::Vector(v) => v,
        _ => {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
            return Err(AjisaiError::from("MOD requires vectors"));
        }
    };

    // ブロードキャスト対応の剰余演算
    let result = apply_binary_broadcast(a_vec, b_vec, |x, y| {
        if y.numerator.is_zero() {
            Err(AjisaiError::from("Modulo by zero"))
        } else {
            Ok(x.modulo(y))
        }
    })?;

    interp.stack.push(Value::from_vector(result));
    Ok(())
}

/// ブロードキャスト付き二項演算
fn apply_binary_broadcast<F>(a: &[Value], b: &[Value], op: F) -> Result<Vec<Value>>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    let a_len = a.len();
    let b_len = b.len();

    let mut result = Vec::new();

    if a_len > 1 && b_len == 1 {
        // aがベクタ、bがスカラー
        let scalar = extract_single_number(&b[0])?;
        for elem in a {
            result.push(apply_binary_element(elem, &scalar, op)?);
        }
    } else if a_len == 1 && b_len > 1 {
        // aがスカラー、bがベクタ
        let scalar = extract_single_number(&a[0])?;
        for elem in b {
            let elem_num = extract_single_number(elem)?;
            result.push(Value::from_number(op(&scalar, &elem_num)?));
        }
    } else if a_len == b_len {
        // 同じ長さ
        for (elem_a, elem_b) in a.iter().zip(b.iter()) {
            let a_num = extract_single_number(elem_a)?;
            let b_num = extract_single_number(elem_b)?;
            result.push(Value::from_number(op(&a_num, &b_num)?));
        }
    } else {
        return Err(AjisaiError::from(format!(
            "Cannot broadcast shapes [{} elements] and [{} elements]",
            a_len, b_len
        )));
    }

    Ok(result)
}

fn extract_single_number(val: &Value) -> Result<Fraction> {
    match &val.val_type {
        ValueType::Number(n) => Ok(n.clone()),
        _ => Err(AjisaiError::from("Expected number")),
    }
}

fn apply_binary_element<F>(elem: &Value, scalar: &Fraction, op: F) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    match &elem.val_type {
        ValueType::Number(n) => Ok(Value::from_number(op(n, scalar)?)),
        ValueType::Vector(inner) => {
            let result: Result<Vec<Value>> = inner
                .iter()
                .map(|e| apply_binary_element(e, scalar, op))
                .collect();
            Ok(Value::from_vector(result?))
        }
        _ => Err(AjisaiError::from("Expected number or vector")),
    }
}

// ============================================================================
// 生成関数
// ============================================================================

/// FILL - 任意値埋めベクタ生成
///
/// 使用法:
///   [ 2 3 ] [ 5 ] FILL → [ [ 5 5 5 ] [ 5 5 5 ] ]
///   [ 3 ] [ 1/2 ] FILL → [ 1/2 1/2 1/2 ]
pub fn op_fill(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("FILL does not support Stack (..) mode"));
    }

    let value_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let shape_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 形状を抽出
    let shape: Vec<usize> = match &shape_val.val_type {
        ValueType::Vector(v) => {
            let mut s = Vec::with_capacity(v.len());
            for elem in v {
                if let ValueType::Number(n) = &elem.val_type {
                    let dim = n.as_usize()
                        .ok_or_else(|| AjisaiError::from("Shape dimensions must be positive integers"))?;
                    s.push(dim);
                } else {
                    interp.stack.push(shape_val);
                    interp.stack.push(value_val);
                    return Err(AjisaiError::from("Shape must contain only numbers"));
                }
            }
            s
        }
        _ => {
            interp.stack.push(shape_val);
            interp.stack.push(value_val);
            return Err(AjisaiError::from("FILL requires shape as vector"));
        }
    };

    // 埋める値を抽出
    let fill_value = match &value_val.val_type {
        ValueType::Vector(v) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) => n.clone(),
                _ => {
                    interp.stack.push(shape_val);
                    interp.stack.push(value_val);
                    return Err(AjisaiError::from("FILL value must be a number"));
                }
            }
        }
        _ => {
            interp.stack.push(shape_val);
            interp.stack.push(value_val);
            return Err(AjisaiError::from("FILL value must be a single-element vector"));
        }
    };

    if shape.is_empty() {
        return Err(AjisaiError::from("FILL requires non-empty shape"));
    }

    let result = build_filled_vector(&shape, &fill_value);
    interp.stack.push(result);
    Ok(())
}

/// 形状に基づいて値で埋めたネスト済みVectorを構築
fn build_filled_vector(shape: &[usize], value: &Fraction) -> Value {
    if shape.len() == 1 {
        let values: Vec<Value> = (0..shape[0])
            .map(|_| Value::from_number(value.clone()))
            .collect();
        Value::from_vector(values)
    } else {
        let outer_size = shape[0];
        let inner_shape = &shape[1..];
        let values: Vec<Value> = (0..outer_size)
            .map(|_| build_filled_vector(inner_shape, value))
            .collect();
        Value::from_vector(values)
    }
}
