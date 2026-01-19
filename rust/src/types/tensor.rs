//! 行列演算ユーティリティ
//!
//! 統一Value宇宙アーキテクチャ版
//!
//! 再帰的Value構造に対する行列演算を提供する。
//!
//! 注意: これらの関数は将来の行列演算ワード（TRANSPOSE, RESHAPE等）で使用予定。
//! 現時点では未使用だが、アーキテクチャの一部として保持している。

#![allow(dead_code)]

use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData, DisplayHint};

/// Valueの配列から形状を推論する
///
/// 新しいアーキテクチャでは、Valueの形状はそのまま構造から推論できる。
pub fn infer_shape(values: &[Value]) -> Result<Vec<usize>, String> {
    if values.is_empty() {
        return Ok(vec![0]);
    }

    // 各Valueのサイズを合計
    let total_elements: usize = values.iter().map(|v| v.len()).sum();
    Ok(vec![total_elements])
}

/// 行列の転置（2次元Vectorに対して）
///
/// 入力は [rows * cols] の形状を持つ必要がある。
pub fn transpose(values: &[Value], rows: usize, cols: usize) -> Result<Vec<Value>, String> {
    if values.is_empty() {
        return Ok(vec![]);
    }

    // すべてのデータを収集（平坦化）
    let data = flatten_to_numbers(values)?;

    let expected_size = rows * cols;
    if data.len() != expected_size {
        return Err(format!(
            "TRANSPOSE: data size {} doesn't match rows {} * cols {} = {}",
            data.len(), rows, cols, expected_size
        ));
    }

    // 転置を計算
    let mut result_data = Vec::with_capacity(expected_size);
    for j in 0..cols {
        for i in 0..rows {
            result_data.push(data[i * cols + j].clone());
        }
    }

    // 新しいValue構造で返す
    let children: Vec<Value> = result_data.into_iter()
        .map(Value::from_fraction)
        .collect();

    Ok(vec![Value {
        data: ValueData::Vector(children),
        display_hint: DisplayHint::Number,
    }])
}

/// Vectorを数値配列に平坦化
pub fn flatten_to_numbers(values: &[Value]) -> Result<Vec<Fraction>, String> {
    let data: Vec<Fraction> = values.iter()
        .flat_map(|v| v.flatten_fractions())
        .collect();
    Ok(data)
}

/// 形状とデータからValueを構築
pub fn build_nested_from_data(shape: &[usize], data: &[Fraction]) -> Result<Vec<Value>, String> {
    if shape.is_empty() {
        if data.len() != 1 {
            return Err("Scalar requires exactly one data element".to_string());
        }
        return Ok(vec![Value::from_fraction(data[0].clone())]);
    }

    // 期待されるサイズをチェック
    let expected_size: usize = shape.iter().product();
    if data.len() != expected_size {
        return Err(format!(
            "Data size {} doesn't match shape {:?} (size {})",
            data.len(), shape, expected_size
        ));
    }

    // 新しいValue構造で返す
    let children: Vec<Value> = data.iter()
        .map(|f| Value::from_fraction(f.clone()))
        .collect();

    Ok(vec![Value {
        data: ValueData::Vector(children),
        display_hint: DisplayHint::Number,
    }])
}

/// Reshape操作
pub fn reshape(values: &[Value], new_shape: &[usize]) -> Result<Vec<Value>, String> {
    let data = flatten_to_numbers(values)?;
    let expected_size: usize = new_shape.iter().product();

    if data.len() != expected_size {
        return Err(format!(
            "Cannot reshape: data size {} doesn't match new shape {:?} (size {})",
            data.len(), new_shape, expected_size
        ));
    }

    build_nested_from_data(new_shape, &data)
}

/// Rank（次元数）を取得
///
/// 新しいアーキテクチャでは、ネストの深さがランクとなる。
pub fn rank(values: &[Value]) -> Result<usize, String> {
    let shape = infer_shape(values)?;
    if shape == vec![0] {
        Ok(1)
    } else {
        Ok(shape.len())
    }
}
