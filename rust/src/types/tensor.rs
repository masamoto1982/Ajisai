//! 行列演算ユーティリティ
//!
//! 統一分数アーキテクチャ版
//!
//! Vectorベースのデータに対する行列演算を提供する。
//! 新しいアーキテクチャでは、すべてのデータは Vec<Fraction> として表現される。

use crate::types::fraction::Fraction;
use crate::types::{Value, DisplayHint};

/// Valueの配列から形状を推論する
///
/// 統一分数アーキテクチャでは、Valueは常にフラットな分数配列。
/// このバージョンでは、形状は単にデータの長さとして扱う。
pub fn infer_shape(values: &[Value]) -> Result<Vec<usize>, String> {
    if values.is_empty() {
        return Ok(vec![0]);
    }

    // 統一分数アーキテクチャでは、各Valueのdataを連結したものが全体のデータ
    let total_elements: usize = values.iter().map(|v| v.data.len()).sum();
    Ok(vec![total_elements])
}

/// 行列の転置（2次元Vectorに対して）
///
/// 統一分数アーキテクチャでは、転置は行列構造を前提とする。
/// 入力は [rows * cols] の形状を持つ必要がある。
pub fn transpose(values: &[Value], rows: usize, cols: usize) -> Result<Vec<Value>, String> {
    if values.is_empty() {
        return Ok(vec![]);
    }

    // すべてのデータを収集
    let data: Vec<Fraction> = values.iter()
        .flat_map(|v| v.data.iter().cloned())
        .collect();

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

    Ok(vec![Value {
        data: result_data,
        display_hint: DisplayHint::Number,
    }])
}

/// Vectorを数値配列に平坦化
pub fn flatten_to_numbers(values: &[Value]) -> Result<Vec<Fraction>, String> {
    let data: Vec<Fraction> = values.iter()
        .flat_map(|v| v.data.iter().cloned())
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

    // 統一分数アーキテクチャでは、すべてのデータはフラットな配列として格納
    let expected_size: usize = shape.iter().product();
    if data.len() != expected_size {
        return Err(format!(
            "Data size {} doesn't match shape {:?} (size {})",
            data.len(), shape, expected_size
        ));
    }

    Ok(vec![Value {
        data: data.to_vec(),
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
/// 統一分数アーキテクチャでは、すべてのデータは1次元配列として格納される。
/// 形状情報は別途管理する必要がある。
pub fn rank(values: &[Value]) -> Result<usize, String> {
    let shape = infer_shape(values)?;
    if shape == vec![0] {
        Ok(1)
    } else {
        Ok(shape.len())
    }
}
