//! 行列演算ユーティリティ
//!
//! Vectorベースのデータに対する行列演算を提供する。
//! 内部的にはVectorを使用し、形状情報は演算時に動的に計算する。

use crate::types::fraction::Fraction;
use crate::types::{Value, ValueType};
use num_bigint::BigInt;
use num_traits::{One, Zero};

/// ブロードキャスト可能な形状を計算
///
/// NumPyスタイルのブロードキャスティングルールに従う
pub fn broadcast_shapes(shape_a: &[usize], shape_b: &[usize]) -> Result<Vec<usize>, String> {
    let max_rank = shape_a.len().max(shape_b.len());
    let mut result = vec![0; max_rank];

    for i in 0..max_rank {
        let dim_a = if i < shape_a.len() {
            shape_a[shape_a.len() - 1 - i]
        } else {
            1
        };
        let dim_b = if i < shape_b.len() {
            shape_b[shape_b.len() - 1 - i]
        } else {
            1
        };

        if dim_a == dim_b {
            result[max_rank - 1 - i] = dim_a;
        } else if dim_a == 1 {
            result[max_rank - 1 - i] = dim_b;
        } else if dim_b == 1 {
            result[max_rank - 1 - i] = dim_a;
        } else {
            return Err(format!(
                "Cannot broadcast shapes {:?} and {:?}: dimension {} ({} vs {})",
                shape_a, shape_b, i, dim_a, dim_b
            ));
        }
    }

    Ok(result)
}

/// Vectorから形状を推論する
pub fn infer_shape(values: &[Value]) -> Result<Vec<usize>, String> {
    crate::types::infer_shape(values)
}

/// 矩形かどうかを検証
pub fn is_rectangular(values: &[Value]) -> bool {
    crate::types::is_rectangular(values)
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

/// ゼロで埋めたVectorを生成
pub fn zeros(shape: &[usize]) -> Value {
    let zero = Fraction::new(BigInt::zero(), BigInt::one());
    build_nested_vector_with_value(shape, &zero)
}

/// 1で埋めたVectorを生成
pub fn ones(shape: &[usize]) -> Value {
    let one = Fraction::new(BigInt::one(), BigInt::one());
    build_nested_vector_with_value(shape, &one)
}

/// 連番を生成（IOTA相当）
pub fn iota(n: usize) -> Value {
    let values: Vec<Value> = (0..n)
        .map(|i| Value::from_number(Fraction::new(BigInt::from(i), BigInt::one())))
        .collect();
    Value::from_vector(values)
}

/// 指定した値で形状を埋めたVectorを構築
fn build_nested_vector_with_value(shape: &[usize], value: &Fraction) -> Value {
    if shape.is_empty() {
        return Value::from_number(value.clone());
    }

    if shape.len() == 1 {
        let values: Vec<Value> = (0..shape[0])
            .map(|_| Value::from_number(value.clone()))
            .collect();
        return Value::from_vector(values);
    }

    let outer_size = shape[0];
    let inner_shape = &shape[1..];

    let values: Vec<Value> = (0..outer_size)
        .map(|_| build_nested_vector_with_value(inner_shape, value))
        .collect();

    Value::from_vector(values)
}

/// ブロードキャスト付き二項演算を実行
pub fn broadcast_binary_op<F>(
    values_a: &[Value],
    values_b: &[Value],
    op: F,
) -> Result<Vec<Value>, String>
where
    F: Fn(&Fraction, &Fraction) -> Fraction + Copy,
{
    let shape_a = infer_shape(values_a)?;
    let shape_b = infer_shape(values_b)?;
    let result_shape = broadcast_shapes(&shape_a, &shape_b)?;

    // 数値を平坦化して取得
    let data_a = flatten_to_numbers(values_a)?;
    let data_b = flatten_to_numbers(values_b)?;

    // ブロードキャスト演算
    let result_size: usize = result_shape.iter().product();
    let mut result_data = Vec::with_capacity(result_size);

    for i in 0..result_size {
        let idx_a = compute_broadcast_index(i, &result_shape, &shape_a);
        let idx_b = compute_broadcast_index(i, &result_shape, &shape_b);
        result_data.push(op(&data_a[idx_a], &data_b[idx_b]));
    }

    // 結果を形状に従って再構築
    build_nested_from_data(&result_shape, &result_data)
}

/// 単項演算を実行
pub fn unary_op<F>(values: &[Value], op: F) -> Result<Vec<Value>, String>
where
    F: Fn(&Fraction) -> Fraction,
{
    let shape = infer_shape(values)?;
    let data = flatten_to_numbers(values)?;
    let result_data: Vec<Fraction> = data.iter().map(op).collect();
    build_nested_from_data(&shape, &result_data)
}

/// ブロードキャストインデックスを計算
fn compute_broadcast_index(flat_index: usize, result_shape: &[usize], source_shape: &[usize]) -> usize {
    if source_shape.is_empty() || source_shape.iter().product::<usize>() == 1 {
        return 0;
    }

    let mut result_indices = vec![0; result_shape.len()];
    let mut remaining = flat_index;
    for i in (0..result_shape.len()).rev() {
        result_indices[i] = remaining % result_shape[i];
        remaining /= result_shape[i];
    }

    // ソース形状へのマッピング
    let rank_diff = result_shape.len() - source_shape.len();
    let mut source_index = 0;
    let mut stride = 1;

    for i in (0..source_shape.len()).rev() {
        let result_idx = result_indices[i + rank_diff];
        let source_dim = source_shape[i];
        let idx = if source_dim == 1 { 0 } else { result_idx };
        source_index += idx * stride;
        stride *= source_dim;
    }

    source_index
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

/// 内積演算
pub fn inner_product(values_a: &[Value], values_b: &[Value]) -> Result<Fraction, String> {
    let data_a = flatten_to_numbers(values_a)?;
    let data_b = flatten_to_numbers(values_b)?;

    if data_a.len() != data_b.len() {
        return Err(format!(
            "Inner product requires same length vectors: {} vs {}",
            data_a.len(), data_b.len()
        ));
    }

    let mut sum = Fraction::new(BigInt::zero(), BigInt::one());
    for (a, b) in data_a.iter().zip(data_b.iter()) {
        sum = sum.add(&a.mul(b));
    }

    Ok(sum)
}

/// 行列積
pub fn matmul(values_a: &[Value], values_b: &[Value]) -> Result<Vec<Value>, String> {
    let shape_a = infer_shape(values_a)?;
    let shape_b = infer_shape(values_b)?;

    if shape_a.len() != 2 || shape_b.len() != 2 {
        return Err("MATMUL requires 2D matrices".to_string());
    }

    let (m, k1) = (shape_a[0], shape_a[1]);
    let (k2, n) = (shape_b[0], shape_b[1]);

    if k1 != k2 {
        return Err(format!(
            "Matrix dimensions incompatible for multiplication: {:?} x {:?}",
            shape_a, shape_b
        ));
    }

    let data_a = flatten_to_numbers(values_a)?;
    let data_b = flatten_to_numbers(values_b)?;

    let mut result_data = Vec::with_capacity(m * n);
    for i in 0..m {
        for j in 0..n {
            let mut sum = Fraction::new(BigInt::zero(), BigInt::one());
            for k in 0..k1 {
                let a_val = &data_a[i * k1 + k];
                let b_val = &data_b[k * n + j];
                sum = sum.add(&a_val.mul(b_val));
            }
            result_data.push(sum);
        }
    }

    build_nested_from_data(&[m, n], &result_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frac(n: i64) -> Fraction {
        Fraction::new(BigInt::from(n), BigInt::from(1))
    }

    #[test]
    fn test_broadcast_shapes() {
        assert_eq!(broadcast_shapes(&[3], &[3]).unwrap(), vec![3]);
        assert_eq!(broadcast_shapes(&[1], &[3]).unwrap(), vec![3]);
        assert_eq!(broadcast_shapes(&[3, 1], &[1, 4]).unwrap(), vec![3, 4]);
        assert_eq!(broadcast_shapes(&[2, 3], &[3]).unwrap(), vec![2, 3]);
        assert!(broadcast_shapes(&[2], &[3]).is_err());
    }

    #[test]
    fn test_zeros_ones() {
        let z = zeros(&[2, 3]);
        if let ValueType::Vector(v) = z.val_type {
            assert_eq!(v.len(), 2);
        }

        let o = ones(&[3]);
        if let ValueType::Vector(v) = o.val_type {
            assert_eq!(v.len(), 3);
        }
    }

    #[test]
    fn test_iota() {
        let result = iota(5);
        if let ValueType::Vector(v) = result.val_type {
            assert_eq!(v.len(), 5);
            if let ValueType::Number(ref n) = v[0].val_type {
                assert_eq!(n.numerator, BigInt::from(0));
            }
            if let ValueType::Number(ref n) = v[4].val_type {
                assert_eq!(n.numerator, BigInt::from(4));
            }
        }
    }
}
