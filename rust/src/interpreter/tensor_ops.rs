//! 行列演算ワード
//!
//! 統一Value宇宙アーキテクチャ版
//! すべての値は再帰的ValueData構造で表現される。

use crate::error::{AjisaiError, Result};
use crate::interpreter::{Interpreter, OperationTarget};
use crate::interpreter::helpers::wrap_number;
use crate::types::{Value, ValueData, DisplayHint, MAX_VISIBLE_DIMENSIONS};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_traits::{One, Zero};

// ============================================================================
// ヘルパー関数
// ============================================================================

/// 値がベクタ（複数要素）かチェック
fn is_vector_value(val: &Value) -> bool {
    val.is_vector()
}

/// 値が数値（スカラー）かチェック
fn is_number_value(val: &Value) -> bool {
    val.is_scalar()
}

/// 形状を推論する（統一Value宇宙アーキテクチャ版）
fn infer_shape_from_value(val: &Value) -> Vec<usize> {
    val.shape()
}

/// 値からスカラー数値を抽出
#[allow(dead_code)]
fn extract_scalar(val: &Value) -> Option<&Fraction> {
    val.as_scalar()
}

/// ベクタの要素を再構築する
fn reconstruct_vector_elements(val: &Value) -> Vec<Value> {
    match &val.data {
        ValueData::Vector(children) => children.clone(),
        ValueData::Scalar(_) => vec![val.clone()],
        ValueData::Nil => vec![],
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
    let shape_elements = reconstruct_vector_elements(&shape_val);
    let dim_count = shape_elements.len();
    if dim_count > MAX_VISIBLE_DIMENSIONS {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
        return Err(AjisaiError::from(format!(
            "Dimension limit exceeded: Ajisai supports up to 3 visible dimensions (plus dimension 0: the stack). Nesting depth {} exceeds the limit.",
            dim_count
        )));
    }

    let mut new_shape = Vec::with_capacity(dim_count);
    for elem in &shape_elements {
        let dim = match elem.as_scalar().and_then(|f| f.as_usize()) {
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

    // 平坦化されたデータを取得
    let data_fractions = data_val.flatten_fractions();

    // サイズチェック
    let required_size: usize = new_shape.iter().product();
    let data_len = data_fractions.len();
    if data_len != required_size {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
        return Err(AjisaiError::from(format!(
            "RESHAPE failed: data length {} doesn't match shape {:?} (requires {})",
            data_len, new_shape, required_size
        )));
    }

    // 新しい値を作成（再帰的構造を構築）
    let result = build_nested_value(&data_fractions, &new_shape, data_val.display_hint);

    interp.stack.push(result);
    Ok(())
}

/// 平坦化されたデータから再帰的Value構造を構築
fn build_nested_value(data: &[Fraction], shape: &[usize], hint: DisplayHint) -> Value {
    if shape.is_empty() {
        // スカラー
        if data.len() == 1 {
            return Value {
                data: ValueData::Scalar(data[0].clone()),
                display_hint: hint,
                audio_hint: None,
            };
        }
        // データが複数ある場合はベクタ
        let children: Vec<Value> = data.iter()
            .map(|f| Value::from_fraction(f.clone()))
            .collect();
        return Value::from_children(children);
    }

    if shape.len() == 1 {
        // 1次元ベクタ
        let children: Vec<Value> = data.iter()
            .map(|f| Value::from_fraction(f.clone()))
            .collect();
        return Value {
            data: ValueData::Vector(children),
            display_hint: hint,
            audio_hint: None,
        };
    }

    // 多次元: 最外層の次元でチャンクに分割し、再帰的に構築
    let outer_size = shape[0];
    let inner_shape = &shape[1..];
    let inner_size: usize = inner_shape.iter().product();

    let children: Vec<Value> = (0..outer_size)
        .map(|i| {
            let start = i * inner_size;
            let end = start + inner_size;
            build_nested_value(&data[start..end], inner_shape, hint)
        })
        .collect();

    Value {
        data: ValueData::Vector(children),
        display_hint: hint,
        audio_hint: None,
    }
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

    // 平坦化されたデータを取得
    let data = val.flatten_fractions();

    // 転置を実行
    let mut transposed_data = Vec::with_capacity(data.len());
    for j in 0..cols {
        for i in 0..rows {
            transposed_data.push(data[i * cols + j].clone());
        }
    }

    // 新しい形状で再構築
    let result = build_nested_value(&transposed_data, &[cols, rows], val.display_hint);

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
        if let Some(f) = val.as_scalar() {
            let result = op(f);
            interp.stack.push(wrap_number(result));
            return Ok(());
        }
    }

    // ベクタの場合
    if is_vector_value(&val) {
        let result = apply_unary_to_value(&val, &op);
        interp.stack.push(result);
        return Ok(());
    }

    interp.stack.push(val);
    Err(AjisaiError::from(format!("{} requires number or vector", op_name)))
}

/// 再帰的に単項演算を適用
fn apply_unary_to_value<F>(val: &Value, op: &F) -> Value
where
    F: Fn(&Fraction) -> Fraction,
{
    match &val.data {
        ValueData::Scalar(f) => Value {
            data: ValueData::Scalar(op(f)),
            display_hint: val.display_hint,
            audio_hint: val.audio_hint.clone(),
        },
        ValueData::Vector(children) => {
            let new_children: Vec<Value> = children.iter()
                .map(|c| apply_unary_to_value(c, op))
                .collect();
            Value {
                data: ValueData::Vector(new_children),
                display_hint: val.display_hint,
                audio_hint: val.audio_hint.clone(),
            }
        }
        ValueData::Nil => val.clone(),
    }
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
    // 両方スカラーの場合
    if let (Some(fa), Some(fb)) = (a.as_scalar(), b.as_scalar()) {
        return Ok(Value::from_fraction(op(fa, fb)?));
    }

    // 一方がスカラー、他方がベクタの場合
    if a.is_scalar() && b.is_vector() {
        if let Some(scalar) = a.as_scalar() {
            return apply_scalar_to_vector(scalar, b, |s, x| op(s, x));
        }
    }

    if a.is_vector() && b.is_scalar() {
        if let Some(scalar) = b.as_scalar() {
            return apply_vector_to_scalar(a, scalar, |x, s| op(x, s));
        }
    }

    // 両方ベクタの場合
    if a.is_vector() && b.is_vector() {
        return apply_vector_to_vector(a, b, op);
    }

    Err(AjisaiError::from("Cannot broadcast values"))
}

/// スカラーをベクタの各要素に適用
fn apply_scalar_to_vector<F>(scalar: &Fraction, vec: &Value, op: F) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    match &vec.data {
        ValueData::Vector(children) => {
            let new_children: Result<Vec<Value>> = children.iter()
                .map(|c| apply_scalar_to_value(scalar, c, op))
                .collect();
            Ok(Value {
                data: ValueData::Vector(new_children?),
                display_hint: DisplayHint::Number,
                audio_hint: None,
            })
        }
        _ => Err(AjisaiError::from("Expected vector")),
    }
}

fn apply_scalar_to_value<F>(scalar: &Fraction, val: &Value, op: F) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    match &val.data {
        ValueData::Scalar(f) => Ok(Value::from_fraction(op(scalar, f)?)),
        ValueData::Vector(children) => {
            let new_children: Result<Vec<Value>> = children.iter()
                .map(|c| apply_scalar_to_value(scalar, c, op))
                .collect();
            Ok(Value {
                data: ValueData::Vector(new_children?),
                display_hint: val.display_hint,
                audio_hint: val.audio_hint.clone(),
            })
        }
        ValueData::Nil => Ok(val.clone()),
    }
}

/// ベクタの各要素にスカラーを適用
fn apply_vector_to_scalar<F>(vec: &Value, scalar: &Fraction, op: F) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    match &vec.data {
        ValueData::Vector(children) => {
            let new_children: Result<Vec<Value>> = children.iter()
                .map(|c| apply_value_to_scalar(c, scalar, op))
                .collect();
            Ok(Value {
                data: ValueData::Vector(new_children?),
                display_hint: DisplayHint::Number,
                audio_hint: None,
            })
        }
        _ => Err(AjisaiError::from("Expected vector")),
    }
}

fn apply_value_to_scalar<F>(val: &Value, scalar: &Fraction, op: F) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    match &val.data {
        ValueData::Scalar(f) => Ok(Value::from_fraction(op(f, scalar)?)),
        ValueData::Vector(children) => {
            let new_children: Result<Vec<Value>> = children.iter()
                .map(|c| apply_value_to_scalar(c, scalar, op))
                .collect();
            Ok(Value {
                data: ValueData::Vector(new_children?),
                display_hint: val.display_hint,
                audio_hint: val.audio_hint.clone(),
            })
        }
        ValueData::Nil => Ok(val.clone()),
    }
}

/// 2つのベクタを要素ごとに適用
fn apply_vector_to_vector<F>(a: &Value, b: &Value, op: F) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    match (&a.data, &b.data) {
        (ValueData::Vector(ca), ValueData::Vector(cb)) => {
            if ca.len() != cb.len() {
                return Err(AjisaiError::from(format!(
                    "Cannot broadcast shapes [{} elements] and [{} elements]",
                    ca.len(), cb.len()
                )));
            }
            let new_children: Result<Vec<Value>> = ca.iter().zip(cb.iter())
                .map(|(a, b)| apply_values(a, b, op))
                .collect();
            Ok(Value {
                data: ValueData::Vector(new_children?),
                display_hint: DisplayHint::Number,
                audio_hint: None,
            })
        }
        _ => Err(AjisaiError::from("Expected vectors")),
    }
}

fn apply_values<F>(a: &Value, b: &Value, op: F) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    match (&a.data, &b.data) {
        (ValueData::Scalar(fa), ValueData::Scalar(fb)) => {
            Ok(Value::from_fraction(op(fa, fb)?))
        }
        (ValueData::Vector(ca), ValueData::Vector(cb)) => {
            if ca.len() != cb.len() {
                return Err(AjisaiError::from("Shape mismatch"));
            }
            let new_children: Result<Vec<Value>> = ca.iter().zip(cb.iter())
                .map(|(a, b)| apply_values(a, b, op))
                .collect();
            Ok(Value {
                data: ValueData::Vector(new_children?),
                display_hint: a.display_hint,
                audio_hint: a.audio_hint.clone(),
            })
        }
        (ValueData::Scalar(fa), ValueData::Vector(cb)) => {
            let new_children: Result<Vec<Value>> = cb.iter()
                .map(|c| apply_scalar_to_value(fa, c, op))
                .collect();
            Ok(Value {
                data: ValueData::Vector(new_children?),
                display_hint: b.display_hint,
                audio_hint: b.audio_hint.clone(),
            })
        }
        (ValueData::Vector(ca), ValueData::Scalar(fb)) => {
            let new_children: Result<Vec<Value>> = ca.iter()
                .map(|c| apply_value_to_scalar(c, fb, op))
                .collect();
            Ok(Value {
                data: ValueData::Vector(new_children?),
                display_hint: a.display_hint,
                audio_hint: a.audio_hint.clone(),
            })
        }
        _ => Err(AjisaiError::from("Cannot apply operation")),
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

    // 要素を取得
    let elements = reconstruct_vector_elements(&args_val);

    // 最低2要素必要
    if elements.len() < 2 {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("FILL requires [shape... value] (at least 2 elements)"));
    }

    // 最後の要素が埋める値
    let fill_value = match elements.last().and_then(|v| v.as_scalar()) {
        Some(f) => f.clone(),
        None => {
            interp.stack.push(args_val);
            return Err(AjisaiError::from("FILL value must be a scalar"));
        }
    };

    // それより前の要素が形状
    let shape_len = elements.len() - 1;
    if shape_len > MAX_VISIBLE_DIMENSIONS {
        interp.stack.push(args_val);
        return Err(AjisaiError::from(format!(
            "Dimension limit exceeded: Ajisai supports up to 3 visible dimensions (plus dimension 0: the stack). Nesting depth {} exceeds the limit.",
            shape_len
        )));
    }

    let mut shape = Vec::with_capacity(shape_len);
    for i in 0..shape_len {
        let dim = match elements[i].as_scalar().and_then(|f| f.as_usize()) {
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

    // 再帰的構造を構築
    let result = build_nested_value(&data, &shape, DisplayHint::Number);

    interp.stack.push(result);
    Ok(())
}
