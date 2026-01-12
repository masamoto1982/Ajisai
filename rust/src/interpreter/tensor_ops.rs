//! 行列演算ワード
//!
//! 統一分数アーキテクチャ版
//! すべての値は Vec<Fraction> として表現される。
//! 形状情報は shape フィールドで管理される。

use crate::error::{AjisaiError, Result};
use crate::interpreter::{Interpreter, OperationTarget};
use crate::interpreter::helpers::wrap_number;
use crate::types::{Value, DisplayHint, MAX_VISIBLE_DIMENSIONS};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_traits::{One, Zero};

// ============================================================================
// ヘルパー関数
// ============================================================================

/// 値がベクタ（複数要素または shape あり）かチェック
fn is_vector_value(val: &Value) -> bool {
    val.data.len() > 1 || !val.shape.is_empty()
}

/// 値が数値（単一要素）かチェック
fn is_number_value(val: &Value) -> bool {
    val.data.len() == 1 && val.shape.is_empty()
}

/// 形状を推論する（統一分数アーキテクチャ版）
fn infer_shape_from_value(val: &Value) -> Vec<usize> {
    if val.data.is_empty() {
        return vec![];
    }

    // shapeが設定されていればそれを使用
    if !val.shape.is_empty() {
        return val.shape.clone();
    }

    // 単一要素ならスカラー（空の形状）
    if val.data.len() == 1 {
        return vec![];
    }

    // 複数要素なら1次元配列
    vec![val.data.len()]
}

/// 値からスカラー数値を抽出
fn extract_scalar(val: &Value) -> Option<&Fraction> {
    if val.data.len() == 1 {
        Some(&val.data[0])
    } else {
        None
    }
}

// ============================================================================
// 形状操作ワード
// ============================================================================

/// SHAPE - ベクタの形状を取得
///
/// 使用法:
///   [ 1 2 3 ] SHAPE           → [ 1 2 3 ] [ 3 ]
///   [ { 1 2 } { 3 4 } ] SHAPE → [ { 1 2 } { 3 4 } ] [ 2 2 ]
///
/// 形状は1次元Vectorとして返される
pub fn op_shape(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("SHAPE does not support Stack (..) mode"));
    }

    let val = interp.stack.last().ok_or(AjisaiError::StackUnderflow)?;

    // NILの場合
    if val.is_nil() {
        return Err(AjisaiError::from("SHAPE requires vector, got NIL"));
    }

    // ベクタの場合
    if is_vector_value(val) {
        let shape_vec = infer_shape_from_value(val);

        let shape_values: Vec<Value> = shape_vec
            .iter()
            .map(|&n| Value::from_number(Fraction::new(BigInt::from(n as i64), BigInt::one())))
            .collect();

        interp.stack.push(Value::from_vector(shape_values));
        return Ok(());
    }

    // スカラーの場合はエラー
    Err(AjisaiError::from("SHAPE requires vector, got scalar"))
}

/// RANK - ベクタの次元数を取得
///
/// 使用法:
///   [ 1 2 3 ] RANK           → [ 1 2 3 ] [ 1 ]
///   [ { 1 2 } { 3 4 } ] RANK → [ { 1 2 } { 3 4 } ] [ 2 ]
pub fn op_rank(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("RANK does not support Stack (..) mode"));
    }

    let val = interp.stack.last().ok_or(AjisaiError::StackUnderflow)?;

    // NILの場合
    if val.is_nil() {
        return Err(AjisaiError::from("RANK requires vector, got NIL"));
    }

    // ベクタの場合
    if is_vector_value(val) {
        let shape = infer_shape_from_value(val);
        let r = shape.len();
        let rank_frac = Fraction::new(BigInt::from(r as i64), BigInt::one());
        interp.stack.push(wrap_number(rank_frac));
        return Ok(());
    }

    // スカラーの場合はエラー
    Err(AjisaiError::from("RANK requires vector, got scalar"))
}

/// RESHAPE - ベクタの形状を変更
///
/// 使用法:
///   [ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE → [ { 1 2 3 } { 4 5 6 } ]
///   [ 1 2 3 4 5 6 ] [ 3 2 ] RESHAPE → [ { 1 2 } { 3 4 } { 5 6 } ]
///
/// 注意: 3次元までに制限されています
pub fn op_reshape(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("RESHAPE does not support Stack (..) mode"));
    }

    let shape_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let data_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 形状をベクタから抽出
    if !is_vector_value(&shape_val) && !shape_val.is_nil() {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
        return Err(AjisaiError::from("RESHAPE requires shape as vector"));
    }

    // 形状配列を構築
    let dim_count = shape_val.data.len();
    if dim_count > MAX_VISIBLE_DIMENSIONS {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
        return Err(AjisaiError::from(format!(
            "Dimension limit exceeded: Ajisai supports up to 3 visible dimensions (plus dimension 0: the stack). Nesting depth {} exceeds the limit.",
            dim_count
        )));
    }

    let mut new_shape = Vec::with_capacity(dim_count);
    for f in &shape_val.data {
        let dim = match f.as_usize() {
            Some(d) => d,
            None => {
                interp.stack.push(data_val);
                interp.stack.push(shape_val);
                return Err(AjisaiError::from("Shape dimensions must be positive integers"));
            }
        };
        new_shape.push(dim);
    }

    // データをベクタから抽出
    if data_val.is_nil() {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
        return Err(AjisaiError::from("RESHAPE requires data as vector"));
    }

    // サイズチェック
    let required_size: usize = new_shape.iter().product();
    let data_len = data_val.data.len();
    if data_len != required_size {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
        return Err(AjisaiError::from(format!(
            "RESHAPE failed: data length {} doesn't match shape {:?} (requires {})",
            data_len, new_shape, required_size
        )));
    }

    // 新しい値を作成
    let result = Value {
        data: data_val.data.clone(),
        display_hint: data_val.display_hint,
        shape: new_shape,
    };

    interp.stack.push(result);
    Ok(())
}

/// TRANSPOSE - 2次元ベクタの転置
///
/// 使用法:
///   [ { 1 2 3 } { 4 5 6 } ] TRANSPOSE → [ { 1 4 } { 2 5 } { 3 6 } ]
pub fn op_transpose(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("TRANSPOSE does not support Stack (..) mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // NILの場合
    if val.is_nil() {
        interp.stack.push(val);
        return Err(AjisaiError::from("TRANSPOSE requires vector"));
    }

    // 形状を取得
    let shape = infer_shape_from_value(&val);
    if shape.len() != 2 {
        interp.stack.push(val);
        return Err(AjisaiError::from("TRANSPOSE requires 2D vector"));
    }

    let rows = shape[0];
    let cols = shape[1];

    // 転置を実行
    let mut transposed_data = Vec::with_capacity(val.data.len());
    for j in 0..cols {
        for i in 0..rows {
            transposed_data.push(val.data[i * cols + j].clone());
        }
    }

    let result = Value {
        data: transposed_data,
        display_hint: val.display_hint,
        shape: vec![cols, rows],
    };

    interp.stack.push(result);
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

    // NILの場合
    if val.is_nil() {
        interp.stack.push(val);
        return Err(AjisaiError::from(format!("{} requires number or vector", op_name)));
    }

    // 単一数値の場合
    if is_number_value(&val) {
        let result = op(&val.data[0]);
        interp.stack.push(wrap_number(result));
        return Ok(());
    }

    // ベクタの場合
    if is_vector_value(&val) {
        let result_data: Vec<Fraction> = val.data.iter().map(|f| op(f)).collect();
        let result = Value {
            data: result_data,
            display_hint: val.display_hint,
            shape: val.shape.clone(),
        };
        interp.stack.push(result);
        return Ok(());
    }

    interp.stack.push(val);
    Err(AjisaiError::from(format!("{} requires number or vector", op_name)))
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

    // NILチェック
    if a_val.is_nil() || b_val.is_nil() {
        interp.stack.push(a_val);
        interp.stack.push(b_val);
        return Err(AjisaiError::from("MOD requires vectors or numbers"));
    }

    // ブロードキャスト対応の剰余演算
    let result = apply_binary_broadcast(&a_val, &b_val, |x, y| {
        if y.numerator.is_zero() {
            Err(AjisaiError::from("Modulo by zero"))
        } else {
            Ok(x.modulo(y))
        }
    });

    match result {
        Ok(r) => {
            interp.stack.push(r);
            Ok(())
        }
        Err(e) => {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
            Err(e)
        }
    }
}

/// ブロードキャスト付き二項演算
fn apply_binary_broadcast<F>(a: &Value, b: &Value, op: F) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    let a_len = a.data.len();
    let b_len = b.data.len();

    let mut result_data = Vec::new();

    if a_len > 1 && b_len == 1 {
        // aがベクタ、bがスカラー
        let scalar = &b.data[0];
        for elem in &a.data {
            result_data.push(op(elem, scalar)?);
        }
        Ok(Value {
            data: result_data,
            display_hint: DisplayHint::Number,
            shape: a.shape.clone(),
        })
    } else if a_len == 1 && b_len > 1 {
        // aがスカラー、bがベクタ
        let scalar = &a.data[0];
        for elem in &b.data {
            result_data.push(op(scalar, elem)?);
        }
        Ok(Value {
            data: result_data,
            display_hint: DisplayHint::Number,
            shape: b.shape.clone(),
        })
    } else if a_len == b_len {
        // 同じ長さ
        for (elem_a, elem_b) in a.data.iter().zip(b.data.iter()) {
            result_data.push(op(elem_a, elem_b)?);
        }
        Ok(Value {
            data: result_data,
            display_hint: DisplayHint::Number,
            shape: a.shape.clone(),
        })
    } else {
        Err(AjisaiError::from(format!(
            "Cannot broadcast shapes [{} elements] and [{} elements]",
            a_len, b_len
        )))
    }
}

// ============================================================================
// 生成関数
// ============================================================================

/// FILL - 任意値埋めベクタ生成
///
/// 使用法:
///   [ 2 3 5 ] FILL → [ { 5 5 5 } { 5 5 5 } ]
///   [ 3 1/2 ] FILL → [ 1/2 1/2 1/2 ]
///
/// 引数ベクタの最後の要素が埋める値、それより前が形状
/// 注意: 3次元までに制限されています
pub fn op_fill(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("FILL does not support Stack (..) mode"));
    }

    // 引数ベクタ [ shape... value ] を取得
    let args_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // NILチェック
    if args_val.is_nil() {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("FILL requires [shape... value] vector"));
    }

    // 最低2要素必要
    if args_val.data.len() < 2 {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("FILL requires [shape... value] (at least 2 elements)"));
    }

    // 最後の要素が埋める値
    let fill_value = args_val.data.last().unwrap().clone();

    // それより前の要素が形状
    let shape_len = args_val.data.len() - 1;
    if shape_len > MAX_VISIBLE_DIMENSIONS {
        interp.stack.push(args_val);
        return Err(AjisaiError::from(format!(
            "Dimension limit exceeded: Ajisai supports up to 3 visible dimensions (plus dimension 0: the stack). Nesting depth {} exceeds the limit.",
            shape_len
        )));
    }

    let mut shape = Vec::with_capacity(shape_len);
    for i in 0..shape_len {
        let dim = match args_val.data[i].as_usize() {
            Some(d) if d > 0 => d,
            _ => {
                interp.stack.push(args_val);
                return Err(AjisaiError::from("Shape dimensions must be positive integers"));
            }
        };
        shape.push(dim);
    }

    // データを生成
    let total_size: usize = shape.iter().product();
    let data: Vec<Fraction> = (0..total_size).map(|_| fill_value.clone()).collect();

    let result = Value {
        data,
        display_hint: DisplayHint::Number,
        shape,
    };

    interp.stack.push(result);
    Ok(())
}
