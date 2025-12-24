//! 行列演算ユーティリティ
//!
//! Vectorベースのデータに対する行列演算を提供する。
//! 内部的にはVectorを使用し、形状情報は演算時に動的に計算する。

use crate::types::fraction::Fraction;
use crate::types::{Value, ValueType};

/// Vectorから形状を推論する
fn infer_shape(values: &[Value]) -> Result<Vec<usize>, String> {
    crate::types::infer_shape(values)
}

/// 行列の転置（2次元Vectorに対して）
pub fn transpose(values: &[Value]) -> Result<Vec<Value>, String> {
    if values.is_empty() {
        return Ok(vec![]);
    }

    // まず形状を確認
    let shape = infer_shape(values)?;
    if shape.len() != 2 {
        return Err(format!("TRANSPOSE requires 2D array, got shape {:?}", shape));
    }

    let rows = shape[0];
    let cols = shape[1];

    // 2次元配列からデータを取り出す
    let mut result = Vec::with_capacity(cols);
    for j in 0..cols {
        let mut new_row = Vec::with_capacity(rows);
        for i in 0..rows {
            if let ValueType::Vector(ref row) = values[i].val_type {
                new_row.push(row[j].clone());
            } else {
                return Err("Expected vector of vectors for transpose".to_string());
            }
        }
        result.push(Value::from_vector(new_row));
    }

    Ok(result)
}

/// Vectorを数値配列に平坦化
fn flatten_to_numbers(values: &[Value]) -> Result<Vec<Fraction>, String> {
    crate::types::flatten_numbers(values)
}

/// 形状とデータからネストされたVectorを構築
fn build_nested_from_data(shape: &[usize], data: &[Fraction]) -> Result<Vec<Value>, String> {
    if shape.is_empty() {
        if data.len() != 1 {
            return Err("Scalar requires exactly one data element".to_string());
        }
        return Ok(vec![Value::from_number(data[0].clone())]);
    }

    if shape.len() == 1 {
        let values: Vec<Value> = data.iter()
            .map(|f| Value::from_number(f.clone()))
            .collect();
        return Ok(values);
    }

    let outer_size = shape[0];
    let inner_shape = &shape[1..];
    let inner_size: usize = inner_shape.iter().product();

    let mut values = Vec::with_capacity(outer_size);
    for i in 0..outer_size {
        let start = i * inner_size;
        let inner_data = &data[start..start + inner_size];
        let inner_values = build_nested_from_data(inner_shape, inner_data)?;
        values.push(Value::from_vector(inner_values));
    }

    Ok(values)
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
pub fn rank(values: &[Value]) -> Result<usize, String> {
    let shape = infer_shape(values)?;
    // 空配列の場合は 1次元
    if shape == vec![0] {
        Ok(1)
    } else {
        Ok(shape.len())
    }
}
