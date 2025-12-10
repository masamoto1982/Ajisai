//! テンソル演算とブロードキャスト機能
//!
//! NumPy/APL準拠のブロードキャスト規則に基づくテンソル演算を提供

use crate::error::{AjisaiError, Result};
use crate::types::tensor::Tensor;
use crate::types::fraction::Fraction;
use num_traits::Zero;

/// 2つの形状からブロードキャスト後の形状を計算
///
/// NumPy/APL準拠のブロードキャスト規則：
/// 1. 形状の比較は右から行う
/// 2. 各次元は以下の場合に互換：
///    - サイズが同じ
///    - どちらかが1
/// 3. 足りない次元は左に1を追加して補う
pub fn broadcast_shapes(a: &[usize], b: &[usize]) -> Result<Vec<usize>> {
    let max_rank = a.len().max(b.len());
    let mut result = Vec::with_capacity(max_rank);

    // 右から比較
    for i in 0..max_rank {
        let a_dim = if i < a.len() { a[a.len() - 1 - i] } else { 1 };
        let b_dim = if i < b.len() { b[b.len() - 1 - i] } else { 1 };

        if a_dim == b_dim {
            result.push(a_dim);
        } else if a_dim == 1 {
            result.push(b_dim);
        } else if b_dim == 1 {
            result.push(a_dim);
        } else {
            return Err(AjisaiError::from(format!(
                "Cannot broadcast shapes {:?} and {:?}: dimension mismatch at axis {} ({} vs {})",
                a, b, max_rank - 1 - i, a_dim, b_dim
            )));
        }
    }

    result.reverse();
    Ok(result)
}

/// 多次元インデックスを1次元インデックスに変換
fn unravel_index(flat_index: usize, shape: &[usize]) -> Vec<usize> {
    let mut indices = Vec::with_capacity(shape.len());
    let mut remaining = flat_index;

    for &dim in shape.iter().rev() {
        indices.push(remaining % dim);
        remaining /= dim;
    }

    indices.reverse();
    indices
}

/// ブロードキャストされたインデックスを元の形状のインデックスに変換
fn broadcast_index(result_idx: &[usize], original_shape: &[usize]) -> Vec<usize> {
    let rank_diff = result_idx.len().saturating_sub(original_shape.len());
    let mut idx = Vec::with_capacity(original_shape.len());

    for (i, &dim) in original_shape.iter().enumerate() {
        let result_dim_idx = if i + rank_diff < result_idx.len() {
            result_idx[rank_diff + i]
        } else {
            0
        };
        // サイズが1の次元は常にインデックス0
        idx.push(if dim == 1 { 0 } else { result_dim_idx });
    }

    idx
}

/// ブロードキャスト付き二項演算
///
/// 2つのテンソルに対してブロードキャスト規則を適用しながら演算を実行
pub fn broadcast_binary_op<F>(
    a: &Tensor,
    b: &Tensor,
    op: F,
    op_name: &str,
) -> Result<Tensor>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction>,
{
    // 1. ブロードキャスト後の形状を計算
    let result_shape = broadcast_shapes(a.shape(), b.shape())?;

    // 2. 各要素に対して演算を適用
    let result_size: usize = result_shape.iter().product();
    let mut result_data = Vec::with_capacity(result_size);

    for i in 0..result_size {
        let idx = unravel_index(i, &result_shape);
        let a_idx = broadcast_index(&idx, a.shape());
        let b_idx = broadcast_index(&idx, b.shape());

        let a_val = a.get(&a_idx).map_err(|e| AjisaiError::from(format!(
            "{} failed at index {:?}: {}",
            op_name, a_idx, e
        )))?;
        let b_val = b.get(&b_idx).map_err(|e| AjisaiError::from(format!(
            "{} failed at index {:?}: {}",
            op_name, b_idx, e
        )))?;

        result_data.push(op(a_val, b_val).map_err(|e| AjisaiError::from(format!(
            "{} operation failed: {}",
            op_name, e
        )))?);
    }

    Tensor::new(result_shape, result_data)
        .map_err(|e| AjisaiError::from(format!("{} result construction failed: {}", op_name, e)))
}

// ============================================================================
// テンソル形状操作ワード
// ============================================================================

use crate::interpreter::{Interpreter, OperationTarget};
use crate::types::{Value, ValueType};
use num_bigint::BigInt;

/// SHAPE - テンソルの形状を取得
///
/// 使用法:
///   [ 1 2 3 ] SHAPE           → [ 1 2 3 ] [ 3 ]
///   [ [ 1 2 ] [ 3 4 ] ] SHAPE → [ [ 1 2 ] [ 3 4 ] ] [ 2 2 ]
///
/// 形状は1次元テンソルとして返される
pub fn op_shape(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("SHAPE does not support Stack (..) mode"));
    }

    let val = interp.stack.last().ok_or(AjisaiError::StackUnderflow)?;

    let shape_vec = match &val.val_type {
        ValueType::Tensor(t) => t.shape().to_vec(),
        ValueType::Vector(v) => {
            // Vectorの場合は変換してから形状を取得
            let tensor = Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to get shape: {}", e)))?;
            tensor.shape().to_vec()
        }
        _ => {
            return Err(AjisaiError::from(format!(
                "SHAPE requires tensor or vector, got {}",
                val.val_type
            )));
        }
    };

    let shape_data: Vec<Fraction> = shape_vec
        .iter()
        .map(|&n| Fraction::new(BigInt::from(n as i64), BigInt::from(1)))
        .collect();

    let shape_tensor = Tensor::vector(shape_data);
    interp.stack.push(Value::from_tensor(shape_tensor));
    Ok(())
}

/// RANK - テンソルの次元数を取得
///
/// 使用法:
///   [ 1 2 3 ] RANK           → [ 1 2 3 ] [ 1 ]
///   [ [ 1 2 ] [ 3 4 ] ] RANK → [ [ 1 2 ] [ 3 4 ] ] [ 2 ]
pub fn op_rank(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("RANK does not support Stack (..) mode"));
    }

    let val = interp.stack.last().ok_or(AjisaiError::StackUnderflow)?;

    let rank = match &val.val_type {
        ValueType::Tensor(t) => t.rank(),
        ValueType::Vector(v) => {
            let tensor = Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to get rank: {}", e)))?;
            tensor.rank()
        }
        _ => {
            return Err(AjisaiError::from(format!(
                "RANK requires tensor or vector, got {}",
                val.val_type
            )));
        }
    };

    let rank_frac = Fraction::new(BigInt::from(rank as i64), BigInt::from(1));
    let rank_tensor = Tensor::vector(vec![rank_frac]);
    interp.stack.push(Value::from_tensor(rank_tensor));
    Ok(())
}

/// RESHAPE - テンソルの形状を変更
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

    // 形状をテンソルとして取得
    let shape_tensor = match &shape_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert shape: {}", e)))?
        }
        _ => {
            interp.stack.push(data_val);
            interp.stack.push(shape_val);
            return Err(AjisaiError::from("RESHAPE requires shape as tensor or vector"));
        }
    };

    // データをテンソルとして取得
    let data_tensor = match &data_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert data: {}", e)))?
        }
        _ => {
            interp.stack.push(data_val);
            interp.stack.push(shape_val);
            return Err(AjisaiError::from("RESHAPE requires data as tensor or vector"));
        }
    };

    // 形状を整数ベクタとして取得
    let new_shape: Result<Vec<usize>> = shape_tensor
        .data()
        .iter()
        .map(|f| {
            f.as_usize()
                .ok_or_else(|| AjisaiError::from("Shape dimensions must be positive integers"))
        })
        .collect();
    let new_shape = new_shape?;

    let result = data_tensor.reshape(new_shape)?;
    interp.stack.push(Value::from_tensor(result));
    Ok(())
}

/// TRANSPOSE - 2次元テンソルの転置
///
/// 使用法:
///   [ [ 1 2 3 ] [ 4 5 6 ] ] TRANSPOSE → [ [ 1 4 ] [ 2 5 ] [ 3 6 ] ]
pub fn op_transpose(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("TRANSPOSE does not support Stack (..) mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert to tensor: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("TRANSPOSE requires tensor or vector"));
        }
    };

    let result = tensor.transpose()?;
    interp.stack.push(Value::from_tensor(result));
    Ok(())
}

// ============================================================================
// 集約関数（Aggregate Functions）
// ============================================================================

/// SUM - テンソルの全要素の総和
///
/// 使用法:
///   [ 1 2 3 4 5 ] SUM → [ 15 ]
///   [ [ 1 2 3 ] [ 4 5 6 ] ] SUM → [ 21 ]
pub fn op_sum(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("SUM does not support Stack (..) mode yet"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("SUM requires tensor or vector"));
        }
    };

    if tensor.data().is_empty() {
        // 空テンソルの総和は0
        let zero = Fraction::new(BigInt::from(0), BigInt::from(1));
        interp.stack.push(Value::from_tensor(Tensor::scalar(zero)));
        return Ok(());
    }

    let sum = tensor.data().iter().fold(
        Fraction::new(BigInt::from(0), BigInt::from(1)),
        |acc, x| acc.add(x)
    );

    interp.stack.push(Value::from_tensor(Tensor::scalar(sum)));
    Ok(())
}

/// MAX - テンソルの最大値
///
/// 使用法:
///   [ 3 1 4 1 5 9 2 6 ] MAX → [ 9 ]
pub fn op_max(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("MAX does not support Stack (..) mode yet"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("MAX requires tensor or vector"));
        }
    };

    if tensor.data().is_empty() {
        interp.stack.push(val);
        return Err(AjisaiError::from("MAX requires non-empty tensor"));
    }

    let max_val = tensor.data().iter().max().unwrap().clone();
    interp.stack.push(Value::from_tensor(Tensor::scalar(max_val)));
    Ok(())
}

/// MIN - テンソルの最小値
///
/// 使用法:
///   [ 3 1 4 1 5 9 2 6 ] MIN → [ 1 ]
pub fn op_min(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("MIN does not support Stack (..) mode yet"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("MIN requires tensor or vector"));
        }
    };

    if tensor.data().is_empty() {
        interp.stack.push(val);
        return Err(AjisaiError::from("MIN requires non-empty tensor"));
    }

    let min_val = tensor.data().iter().min().unwrap().clone();
    interp.stack.push(Value::from_tensor(Tensor::scalar(min_val)));
    Ok(())
}

/// MEAN - テンソルの平均値
///
/// 使用法:
///   [ 1 2 3 4 5 ] MEAN → [ 3 ]
///   [ [ 10 20 ] [ 30 40 ] ] MEAN → [ 25 ]
pub fn op_mean(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("MEAN does not support Stack (..) mode yet"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("MEAN requires tensor or vector"));
        }
    };

    if tensor.data().is_empty() {
        interp.stack.push(val);
        return Err(AjisaiError::from("MEAN requires non-empty tensor"));
    }

    let sum = tensor.data().iter().fold(
        Fraction::new(BigInt::from(0), BigInt::from(1)),
        |acc, x| acc.add(x)
    );
    let count = Fraction::new(BigInt::from(tensor.size() as i64), BigInt::from(1));
    let mean = sum.div(&count);

    interp.stack.push(Value::from_tensor(Tensor::scalar(mean)));
    Ok(())
}

/// PRODUCT - テンソルの全要素の積
///
/// 使用法:
///   [ 1 2 3 4 5 ] PRODUCT → [ 120 ]
pub fn op_product(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("PRODUCT does not support Stack (..) mode yet"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("PRODUCT requires tensor or vector"));
        }
    };

    if tensor.data().is_empty() {
        // 空テンソルの総積は1（乗法単位元）
        let one = Fraction::new(BigInt::from(1), BigInt::from(1));
        interp.stack.push(Value::from_tensor(Tensor::scalar(one)));
        return Ok(());
    }

    let product = tensor.data().iter().fold(
        Fraction::new(BigInt::from(1), BigInt::from(1)),
        |acc, x| acc.mul(x)
    );

    interp.stack.push(Value::from_tensor(Tensor::scalar(product)));
    Ok(())
}

// ============================================================================
// 軸指定集約関数（Axis-wise Aggregate Functions）
// ============================================================================

/// SUM-ROWS - 各行の総和（axis=1で集約）
///
/// 使用法:
///   [ [ 1 2 3 ] [ 4 5 6 ] ] SUM-ROWS → [ 6 15 ]
pub fn op_sum_rows(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("SUM-ROWS does not support Stack (..) mode yet"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("SUM-ROWS requires tensor or vector"));
        }
    };

    if tensor.rank() != 2 {
        interp.stack.push(val);
        return Err(AjisaiError::from("SUM-ROWS requires 2-dimensional tensor (matrix)"));
    }

    let rows = tensor.shape()[0];
    let cols = tensor.shape()[1];
    let mut result_data = Vec::with_capacity(rows);

    for i in 0..rows {
        let mut row_sum = Fraction::new(BigInt::from(0), BigInt::from(1));
        for j in 0..cols {
            row_sum = row_sum.add(&tensor.data()[i * cols + j]);
        }
        result_data.push(row_sum);
    }

    interp.stack.push(Value::from_tensor(Tensor::vector(result_data)));
    Ok(())
}

/// SUM-COLS - 各列の総和（axis=0で集約）
///
/// 使用法:
///   [ [ 1 2 3 ] [ 4 5 6 ] ] SUM-COLS → [ 5 7 9 ]
pub fn op_sum_cols(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("SUM-COLS does not support Stack (..) mode yet"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("SUM-COLS requires tensor or vector"));
        }
    };

    if tensor.rank() != 2 {
        interp.stack.push(val);
        return Err(AjisaiError::from("SUM-COLS requires 2-dimensional tensor (matrix)"));
    }

    let rows = tensor.shape()[0];
    let cols = tensor.shape()[1];
    let mut result_data = Vec::with_capacity(cols);

    for j in 0..cols {
        let mut col_sum = Fraction::new(BigInt::from(0), BigInt::from(1));
        for i in 0..rows {
            col_sum = col_sum.add(&tensor.data()[i * cols + j]);
        }
        result_data.push(col_sum);
    }

    interp.stack.push(Value::from_tensor(Tensor::vector(result_data)));
    Ok(())
}

/// MEAN-ROWS - 各行の平均
///
/// 使用法:
///   [ [ 1 2 3 ] [ 4 5 6 ] ] MEAN-ROWS → [ 2 5 ]
pub fn op_mean_rows(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("MEAN-ROWS does not support Stack (..) mode yet"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("MEAN-ROWS requires tensor or vector"));
        }
    };

    if tensor.rank() != 2 {
        interp.stack.push(val);
        return Err(AjisaiError::from("MEAN-ROWS requires 2-dimensional tensor (matrix)"));
    }

    let rows = tensor.shape()[0];
    let cols = tensor.shape()[1];
    let count = Fraction::new(BigInt::from(cols as i64), BigInt::from(1));
    let mut result_data = Vec::with_capacity(rows);

    for i in 0..rows {
        let mut row_sum = Fraction::new(BigInt::from(0), BigInt::from(1));
        for j in 0..cols {
            row_sum = row_sum.add(&tensor.data()[i * cols + j]);
        }
        result_data.push(row_sum.div(&count));
    }

    interp.stack.push(Value::from_tensor(Tensor::vector(result_data)));
    Ok(())
}

/// MEAN-COLS - 各列の平均
///
/// 使用法:
///   [ [ 1 2 3 ] [ 4 5 6 ] ] MEAN-COLS → [ 5/2 7/2 9/2 ]
pub fn op_mean_cols(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("MEAN-COLS does not support Stack (..) mode yet"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("MEAN-COLS requires tensor or vector"));
        }
    };

    if tensor.rank() != 2 {
        interp.stack.push(val);
        return Err(AjisaiError::from("MEAN-COLS requires 2-dimensional tensor (matrix)"));
    }

    let rows = tensor.shape()[0];
    let cols = tensor.shape()[1];
    let count = Fraction::new(BigInt::from(rows as i64), BigInt::from(1));
    let mut result_data = Vec::with_capacity(cols);

    for j in 0..cols {
        let mut col_sum = Fraction::new(BigInt::from(0), BigInt::from(1));
        for i in 0..rows {
            col_sum = col_sum.add(&tensor.data()[i * cols + j]);
        }
        result_data.push(col_sum.div(&count));
    }

    interp.stack.push(Value::from_tensor(Tensor::vector(result_data)));
    Ok(())
}

/// MAX-ROWS - 各行の最大値
///
/// 使用法:
///   [ [ 1 5 3 ] [ 4 2 6 ] ] MAX-ROWS → [ 5 6 ]
pub fn op_max_rows(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("MAX-ROWS does not support Stack (..) mode yet"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("MAX-ROWS requires tensor or vector"));
        }
    };

    if tensor.rank() != 2 {
        interp.stack.push(val);
        return Err(AjisaiError::from("MAX-ROWS requires 2-dimensional tensor (matrix)"));
    }

    let rows = tensor.shape()[0];
    let cols = tensor.shape()[1];
    let mut result_data = Vec::with_capacity(rows);

    for i in 0..rows {
        let row_start = i * cols;
        let row_max = tensor.data()[row_start..row_start + cols]
            .iter()
            .max()
            .unwrap()
            .clone();
        result_data.push(row_max);
    }

    interp.stack.push(Value::from_tensor(Tensor::vector(result_data)));
    Ok(())
}

/// MAX-COLS - 各列の最大値
///
/// 使用法:
///   [ [ 1 5 3 ] [ 4 2 6 ] ] MAX-COLS → [ 4 5 6 ]
pub fn op_max_cols(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("MAX-COLS does not support Stack (..) mode yet"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("MAX-COLS requires tensor or vector"));
        }
    };

    if tensor.rank() != 2 {
        interp.stack.push(val);
        return Err(AjisaiError::from("MAX-COLS requires 2-dimensional tensor (matrix)"));
    }

    let rows = tensor.shape()[0];
    let cols = tensor.shape()[1];
    let mut result_data = Vec::with_capacity(cols);

    for j in 0..cols {
        let mut col_max = tensor.data()[j].clone();
        for i in 1..rows {
            if tensor.data()[i * cols + j].gt(&col_max) {
                col_max = tensor.data()[i * cols + j].clone();
            }
        }
        result_data.push(col_max);
    }

    interp.stack.push(Value::from_tensor(Tensor::vector(result_data)));
    Ok(())
}

/// MIN-ROWS - 各行の最小値
///
/// 使用法:
///   [ [ 1 5 3 ] [ 4 2 6 ] ] MIN-ROWS → [ 1 2 ]
pub fn op_min_rows(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("MIN-ROWS does not support Stack (..) mode yet"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("MIN-ROWS requires tensor or vector"));
        }
    };

    if tensor.rank() != 2 {
        interp.stack.push(val);
        return Err(AjisaiError::from("MIN-ROWS requires 2-dimensional tensor (matrix)"));
    }

    let rows = tensor.shape()[0];
    let cols = tensor.shape()[1];
    let mut result_data = Vec::with_capacity(rows);

    for i in 0..rows {
        let row_start = i * cols;
        let row_min = tensor.data()[row_start..row_start + cols]
            .iter()
            .min()
            .unwrap()
            .clone();
        result_data.push(row_min);
    }

    interp.stack.push(Value::from_tensor(Tensor::vector(result_data)));
    Ok(())
}

/// MIN-COLS - 各列の最小値
///
/// 使用法:
///   [ [ 1 5 3 ] [ 4 2 6 ] ] MIN-COLS → [ 1 2 3 ]
pub fn op_min_cols(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("MIN-COLS does not support Stack (..) mode yet"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("MIN-COLS requires tensor or vector"));
        }
    };

    if tensor.rank() != 2 {
        interp.stack.push(val);
        return Err(AjisaiError::from("MIN-COLS requires 2-dimensional tensor (matrix)"));
    }

    let rows = tensor.shape()[0];
    let cols = tensor.shape()[1];
    let mut result_data = Vec::with_capacity(cols);

    for j in 0..cols {
        let mut col_min = tensor.data()[j].clone();
        for i in 1..rows {
            if tensor.data()[i * cols + j].lt(&col_min) {
                col_min = tensor.data()[i * cols + j].clone();
            }
        }
        result_data.push(col_min);
    }

    interp.stack.push(Value::from_tensor(Tensor::vector(result_data)));
    Ok(())
}

// ============================================================================
// 行列演算（Matrix Operations）
// ============================================================================

/// DOT - 内積
///
/// 使用法:
///   [ 1 2 3 ] [ 4 5 6 ] DOT → [ 32 ]  (1*4 + 2*5 + 3*6)
pub fn op_dot(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("DOT does not support Stack (..) mode yet"));
    }

    let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let a = match &a_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
            return Err(AjisaiError::from("DOT requires tensor or vector"));
        }
    };

    let b = match &b_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
            return Err(AjisaiError::from("DOT requires tensor or vector"));
        }
    };

    if a.rank() != 1 || b.rank() != 1 {
        interp.stack.push(a_val);
        interp.stack.push(b_val);
        return Err(AjisaiError::from("DOT requires two 1-dimensional tensors (vectors)"));
    }

    if a.shape()[0] != b.shape()[0] {
        interp.stack.push(a_val);
        interp.stack.push(b_val);
        return Err(AjisaiError::from(format!(
            "DOT requires vectors of same length, got {} and {}",
            a.shape()[0], b.shape()[0]
        )));
    }

    let dot_product = a.data().iter()
        .zip(b.data().iter())
        .fold(
            Fraction::new(BigInt::from(0), BigInt::from(1)),
            |acc, (x, y)| acc.add(&x.mul(y))
        );

    interp.stack.push(Value::from_tensor(Tensor::scalar(dot_product)));
    Ok(())
}

/// MATMUL - 行列積
///
/// 使用法:
///   [ [ 1 2 ] [ 3 4 ] ] [ [ 5 6 ] [ 7 8 ] ] MATMUL → [ [ 19 22 ] [ 43 50 ] ]
///   [ [ 1 2 ] [ 3 4 ] ] [ 5 6 ] MATMUL → [ 17 39 ]  (行列×ベクタ)
pub fn op_matmul(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("MATMUL does not support Stack (..) mode yet"));
    }

    let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let a = match &a_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
            return Err(AjisaiError::from("MATMUL requires tensor or vector"));
        }
    };

    let b = match &b_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
            return Err(AjisaiError::from("MATMUL requires tensor or vector"));
        }
    };

    // Case 1: 行列×行列 (2D × 2D)
    if a.rank() == 2 && b.rank() == 2 {
        let m = a.shape()[0];
        let n = a.shape()[1];
        let p = b.shape()[1];

        if n != b.shape()[0] {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
            return Err(AjisaiError::from(format!(
                "MATMUL: incompatible shapes [{}, {}] and [{}, {}]",
                m, n, b.shape()[0], p
            )));
        }

        let mut result_data = Vec::with_capacity(m * p);
        for i in 0..m {
            for k in 0..p {
                let mut sum = Fraction::new(BigInt::from(0), BigInt::from(1));
                for j in 0..n {
                    sum = sum.add(&a.data()[i * n + j].mul(&b.data()[j * p + k]));
                }
                result_data.push(sum);
            }
        }

        let result = Tensor::new(vec![m, p], result_data)?;
        interp.stack.push(Value::from_tensor(result));
        return Ok(());
    }

    // Case 2: 行列×ベクタ (2D × 1D)
    if a.rank() == 2 && b.rank() == 1 {
        let m = a.shape()[0];
        let n = a.shape()[1];

        if n != b.shape()[0] {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
            return Err(AjisaiError::from(format!(
                "MATMUL: incompatible shapes [{}, {}] and [{}]",
                m, n, b.shape()[0]
            )));
        }

        let mut result_data = Vec::with_capacity(m);
        for i in 0..m {
            let mut sum = Fraction::new(BigInt::from(0), BigInt::from(1));
            for j in 0..n {
                sum = sum.add(&a.data()[i * n + j].mul(&b.data()[j]));
            }
            result_data.push(sum);
        }

        interp.stack.push(Value::from_tensor(Tensor::vector(result_data)));
        return Ok(());
    }

    // Case 3: ベクタ×行列 (1D × 2D)
    if a.rank() == 1 && b.rank() == 2 {
        let n = a.shape()[0];
        let p = b.shape()[1];

        if n != b.shape()[0] {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
            return Err(AjisaiError::from(format!(
                "MATMUL: incompatible shapes [{}] and [{}, {}]",
                n, b.shape()[0], p
            )));
        }

        let mut result_data = Vec::with_capacity(p);
        for k in 0..p {
            let mut sum = Fraction::new(BigInt::from(0), BigInt::from(1));
            for j in 0..n {
                sum = sum.add(&a.data()[j].mul(&b.data()[j * p + k]));
            }
            result_data.push(sum);
        }

        interp.stack.push(Value::from_tensor(Tensor::vector(result_data)));
        return Ok(());
    }

    interp.stack.push(a_val);
    interp.stack.push(b_val);
    Err(AjisaiError::from("MATMUL requires matrices (2D) or matrix and vector (2D and 1D)"))
}

// ============================================================================
// テンソルアクセス関数（Tensor Access Functions）
// ============================================================================

/// ROW - 行の抽出
///
/// 使用法:
///   [ [ 1 2 3 ] [ 4 5 6 ] [ 7 8 9 ] ] [ 1 ] ROW → [ 4 5 6 ]
pub fn op_row(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("ROW does not support Stack (..) mode yet"));
    }

    let idx_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let tensor_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let idx = match &idx_val.val_type {
        ValueType::Tensor(t) if t.is_scalar() => {
            t.as_scalar().unwrap().as_usize()
                .ok_or_else(|| AjisaiError::from("ROW index must be non-negative integer"))?
        }
        ValueType::Tensor(t) if t.rank() == 1 && t.size() == 1 => {
            t.data()[0].as_usize()
                .ok_or_else(|| AjisaiError::from("ROW index must be non-negative integer"))?
        }
        _ => {
            interp.stack.push(tensor_val);
            interp.stack.push(idx_val);
            return Err(AjisaiError::from("ROW requires scalar index"));
        }
    };

    let tensor = match &tensor_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(tensor_val);
            interp.stack.push(idx_val);
            return Err(AjisaiError::from("ROW requires tensor or vector"));
        }
    };

    if tensor.rank() != 2 {
        interp.stack.push(tensor_val);
        interp.stack.push(idx_val);
        return Err(AjisaiError::from("ROW requires 2-dimensional tensor (matrix)"));
    }

    let rows = tensor.shape()[0];
    let cols = tensor.shape()[1];

    if idx >= rows {
        interp.stack.push(tensor_val);
        interp.stack.push(idx_val);
        return Err(AjisaiError::from(format!(
            "ROW index {} out of bounds for matrix with {} rows",
            idx, rows
        )));
    }

    let row_start = idx * cols;
    let row_data: Vec<Fraction> = tensor.data()[row_start..row_start + cols].to_vec();
    interp.stack.push(Value::from_tensor(Tensor::vector(row_data)));
    Ok(())
}

/// COL - 列の抽出
///
/// 使用法:
///   [ [ 1 2 3 ] [ 4 5 6 ] [ 7 8 9 ] ] [ 2 ] COL → [ 3 6 9 ]
pub fn op_col(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("COL does not support Stack (..) mode yet"));
    }

    let idx_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let tensor_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let idx = match &idx_val.val_type {
        ValueType::Tensor(t) if t.is_scalar() => {
            t.as_scalar().unwrap().as_usize()
                .ok_or_else(|| AjisaiError::from("COL index must be non-negative integer"))?
        }
        ValueType::Tensor(t) if t.rank() == 1 && t.size() == 1 => {
            t.data()[0].as_usize()
                .ok_or_else(|| AjisaiError::from("COL index must be non-negative integer"))?
        }
        _ => {
            interp.stack.push(tensor_val);
            interp.stack.push(idx_val);
            return Err(AjisaiError::from("COL requires scalar index"));
        }
    };

    let tensor = match &tensor_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(tensor_val);
            interp.stack.push(idx_val);
            return Err(AjisaiError::from("COL requires tensor or vector"));
        }
    };

    if tensor.rank() != 2 {
        interp.stack.push(tensor_val);
        interp.stack.push(idx_val);
        return Err(AjisaiError::from("COL requires 2-dimensional tensor (matrix)"));
    }

    let rows = tensor.shape()[0];
    let cols = tensor.shape()[1];

    if idx >= cols {
        interp.stack.push(tensor_val);
        interp.stack.push(idx_val);
        return Err(AjisaiError::from(format!(
            "COL index {} out of bounds for matrix with {} columns",
            idx, cols
        )));
    }

    let mut col_data = Vec::with_capacity(rows);
    for i in 0..rows {
        col_data.push(tensor.data()[i * cols + idx].clone());
    }
    interp.stack.push(Value::from_tensor(Tensor::vector(col_data)));
    Ok(())
}

/// DIAG - 対角成分の抽出
///
/// 使用法:
///   [ [ 1 2 3 ] [ 4 5 6 ] [ 7 8 9 ] ] DIAG → [ 1 5 9 ]
pub fn op_diag(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("DIAG does not support Stack (..) mode yet"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("DIAG requires tensor or vector"));
        }
    };

    if tensor.rank() != 2 {
        interp.stack.push(val);
        return Err(AjisaiError::from("DIAG requires 2-dimensional tensor (matrix)"));
    }

    let rows = tensor.shape()[0];
    let cols = tensor.shape()[1];
    let diag_len = rows.min(cols);
    let mut diag_data = Vec::with_capacity(diag_len);

    for i in 0..diag_len {
        diag_data.push(tensor.data()[i * cols + i].clone());
    }

    interp.stack.push(Value::from_tensor(Tensor::vector(diag_data)));
    Ok(())
}

// ============================================================================
// 基本数学関数（Phase 1）
// ============================================================================

/// ABS - 絶対値
///
/// 使用法:
///   [ -5 ] ABS → [ 5 ]
///   [ -3 7 -2 ] ABS → [ 3 7 2 ]
///   [ [ -1 2 ] [ -3 4 ] ] ABS → [ [ 1 2 ] [ 3 4 ] ]
pub fn op_abs(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("ABS does not support Stack (..) mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("ABS requires tensor or vector"));
        }
    };

    let result_data: Vec<Fraction> = tensor.data().iter().map(|f| f.abs()).collect();
    let result = Tensor::new(tensor.shape().to_vec(), result_data)?;
    interp.stack.push(Value::from_tensor(result));
    Ok(())
}

/// NEG - 符号反転
///
/// 使用法:
///   [ 5 ] NEG → [ -5 ]
///   [ -3 7 -2 ] NEG → [ 3 -7 2 ]
pub fn op_neg(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("NEG does not support Stack (..) mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("NEG requires tensor or vector"));
        }
    };

    let result_data: Vec<Fraction> = tensor.data().iter().map(|f| f.neg()).collect();
    let result = Tensor::new(tensor.shape().to_vec(), result_data)?;
    interp.stack.push(Value::from_tensor(result));
    Ok(())
}

/// SIGN - 符号取得
///
/// 使用法:
///   [ -5 ] SIGN → [ -1 ]
///   [ 0 ] SIGN → [ 0 ]
///   [ 7 ] SIGN → [ 1 ]
///   [ -3 0 5 ] SIGN → [ -1 0 1 ]
pub fn op_sign(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("SIGN does not support Stack (..) mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("SIGN requires tensor or vector"));
        }
    };

    let result_data: Vec<Fraction> = tensor.data().iter().map(|f| f.sign()).collect();
    let result = Tensor::new(tensor.shape().to_vec(), result_data)?;
    interp.stack.push(Value::from_tensor(result));
    Ok(())
}

/// FLOOR - 切り捨て（負の無限大方向）
///
/// 使用法:
///   [ 7/3 ] FLOOR → [ 2 ]      # 7/3 = 2.333... → 2
///   [ -7/3 ] FLOOR → [ -3 ]    # -7/3 = -2.333... → -3
///   [ 5 ] FLOOR → [ 5 ]        # 整数はそのまま
pub fn op_floor(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("FLOOR does not support Stack (..) mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("FLOOR requires tensor or vector"));
        }
    };

    let result_data: Vec<Fraction> = tensor.data().iter().map(|f| f.floor()).collect();
    let result = Tensor::new(tensor.shape().to_vec(), result_data)?;
    interp.stack.push(Value::from_tensor(result));
    Ok(())
}

/// CEIL - 切り上げ（正の無限大方向）
///
/// 使用法:
///   [ 7/3 ] CEIL → [ 3 ]       # 7/3 = 2.333... → 3
///   [ -7/3 ] CEIL → [ -2 ]     # -7/3 = -2.333... → -2
///   [ 5 ] CEIL → [ 5 ]         # 整数はそのまま
pub fn op_ceil(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("CEIL does not support Stack (..) mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("CEIL requires tensor or vector"));
        }
    };

    let result_data: Vec<Fraction> = tensor.data().iter().map(|f| f.ceil()).collect();
    let result = Tensor::new(tensor.shape().to_vec(), result_data)?;
    interp.stack.push(Value::from_tensor(result));
    Ok(())
}

/// ROUND - 四捨五入
///
/// 使用法:
///   [ 7/3 ] ROUND → [ 2 ]      # 2.333... → 2
///   [ 5/2 ] ROUND → [ 3 ]      # 2.5 → 3（0から遠い方向）
///   [ -5/2 ] ROUND → [ -3 ]    # -2.5 → -3（0から遠い方向）
pub fn op_round(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("ROUND does not support Stack (..) mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("ROUND requires tensor or vector"));
        }
    };

    let result_data: Vec<Fraction> = tensor.data().iter().map(|f| f.round()).collect();
    let result = Tensor::new(tensor.shape().to_vec(), result_data)?;
    interp.stack.push(Value::from_tensor(result));
    Ok(())
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

    let a = match &a_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
            return Err(AjisaiError::from("MOD requires tensor or vector"));
        }
    };

    let b = match &b_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
            return Err(AjisaiError::from("MOD requires tensor or vector"));
        }
    };

    let result = broadcast_binary_op(&a, &b, |x, y| {
        if y.numerator.is_zero() {
            Err(AjisaiError::from("Modulo by zero"))
        } else {
            Ok(x.modulo(y))
        }
    }, "MOD")?;

    interp.stack.push(Value::from_tensor(result));
    Ok(())
}

/// POW - べき乗（整数指数のみ）
///
/// 使用法:
///   [ 2 ] [ 3 ] POW → [ 8 ]
///   [ 3 ] [ -2 ] POW → [ 1/9 ]
///   [ 2 3 ] [ 2 ] POW → [ 4 9 ]  # ブロードキャスト
pub fn op_pow(interp: &mut Interpreter) -> Result<()> {
    use num_traits::ToPrimitive;

    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("POW does not support Stack (..) mode"));
    }

    let exp_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let base_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let base = match &base_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(base_val);
            interp.stack.push(exp_val);
            return Err(AjisaiError::from("POW requires tensor or vector"));
        }
    };

    let exp = match &exp_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(base_val);
            interp.stack.push(exp_val);
            return Err(AjisaiError::from("POW requires tensor or vector"));
        }
    };

    let result = broadcast_binary_op(&base, &exp, |b, e| {
        // 指数が整数であることを確認
        if !e.is_exact_integer() {
            return Err(AjisaiError::from("POW exponent must be an integer (rational exponents not supported)"));
        }
        let exp_int = e.numerator.to_i64()
            .ok_or_else(|| AjisaiError::from("POW exponent too large"))?;
        Ok(b.pow(exp_int))
    }, "POW")?;

    interp.stack.push(Value::from_tensor(result));
    Ok(())
}

// ============================================================================
// テンソル生成関数（Phase 2）
// ============================================================================

/// ZEROS - ゼロ埋めテンソル生成
///
/// 使用法:
///   [ 3 ] ZEROS → [ 0 0 0 ]
///   [ 2 3 ] ZEROS → [ [ 0 0 0 ] [ 0 0 0 ] ]
///   [ 2 2 2 ] ZEROS → [ [ [ 0 0 ] [ 0 0 ] ] [ [ 0 0 ] [ 0 0 ] ] ]
pub fn op_zeros(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("ZEROS does not support Stack (..) mode"));
    }

    let shape_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let shape_tensor = match &shape_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert shape: {}", e)))?
        }
        _ => {
            interp.stack.push(shape_val);
            return Err(AjisaiError::from("ZEROS requires tensor for shape"));
        }
    };

    let shape: Result<Vec<usize>> = shape_tensor
        .data()
        .iter()
        .map(|f| {
            f.as_usize()
                .ok_or_else(|| AjisaiError::from("Shape dimensions must be positive integers"))
        })
        .collect();
    let shape = shape?;

    if shape.is_empty() {
        interp.stack.push(shape_val);
        return Err(AjisaiError::from("ZEROS requires non-empty shape"));
    }

    let size: usize = shape.iter().product();
    let zero = Fraction::new(BigInt::from(0), BigInt::from(1));
    let data: Vec<Fraction> = vec![zero; size];

    let result = Tensor::new(shape, data)?;
    interp.stack.push(Value::from_tensor(result));
    Ok(())
}

/// ONES - 1埋めテンソル生成
///
/// 使用法:
///   [ 3 ] ONES → [ 1 1 1 ]
///   [ 2 3 ] ONES → [ [ 1 1 1 ] [ 1 1 1 ] ]
pub fn op_ones(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("ONES does not support Stack (..) mode"));
    }

    let shape_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let shape_tensor = match &shape_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert shape: {}", e)))?
        }
        _ => {
            interp.stack.push(shape_val);
            return Err(AjisaiError::from("ONES requires tensor for shape"));
        }
    };

    let shape: Result<Vec<usize>> = shape_tensor
        .data()
        .iter()
        .map(|f| {
            f.as_usize()
                .ok_or_else(|| AjisaiError::from("Shape dimensions must be positive integers"))
        })
        .collect();
    let shape = shape?;

    if shape.is_empty() {
        interp.stack.push(shape_val);
        return Err(AjisaiError::from("ONES requires non-empty shape"));
    }

    let size: usize = shape.iter().product();
    let one = Fraction::new(BigInt::from(1), BigInt::from(1));
    let data: Vec<Fraction> = vec![one; size];

    let result = Tensor::new(shape, data)?;
    interp.stack.push(Value::from_tensor(result));
    Ok(())
}

/// FILL - 任意値埋めテンソル生成
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

    let shape_tensor = match &shape_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert shape: {}", e)))?
        }
        _ => {
            interp.stack.push(shape_val);
            interp.stack.push(value_val);
            return Err(AjisaiError::from("FILL requires tensor for shape"));
        }
    };

    let fill_value = match &value_val.val_type {
        ValueType::Tensor(t) => {
            if t.is_scalar() {
                t.as_scalar().unwrap().clone()
            } else if t.size() == 1 {
                t.data()[0].clone()
            } else {
                interp.stack.push(shape_val);
                interp.stack.push(value_val);
                return Err(AjisaiError::from("FILL value must be a scalar"));
            }
        }
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
            return Err(AjisaiError::from("FILL value must be a scalar tensor"));
        }
    };

    let shape: Result<Vec<usize>> = shape_tensor
        .data()
        .iter()
        .map(|f| {
            f.as_usize()
                .ok_or_else(|| AjisaiError::from("Shape dimensions must be positive integers"))
        })
        .collect();
    let shape = shape?;

    if shape.is_empty() {
        return Err(AjisaiError::from("FILL requires non-empty shape"));
    }

    let size: usize = shape.iter().product();
    let data: Vec<Fraction> = vec![fill_value; size];

    let result = Tensor::new(shape, data)?;
    interp.stack.push(Value::from_tensor(result));
    Ok(())
}

/// EYE - 単位行列生成
///
/// 使用法:
///   [ 3 ] EYE → [ [ 1 0 0 ] [ 0 1 0 ] [ 0 0 1 ] ]
///   [ 2 ] EYE → [ [ 1 0 ] [ 0 1 ] ]
pub fn op_eye(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("EYE does not support Stack (..) mode"));
    }

    let size_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let n = match &size_val.val_type {
        ValueType::Tensor(t) => {
            if t.is_scalar() {
                t.as_scalar().unwrap().as_usize()
                    .ok_or_else(|| AjisaiError::from("EYE size must be a positive integer"))?
            } else if t.size() == 1 {
                t.data()[0].as_usize()
                    .ok_or_else(|| AjisaiError::from("EYE size must be a positive integer"))?
            } else {
                interp.stack.push(size_val);
                return Err(AjisaiError::from("EYE requires scalar size"));
            }
        }
        ValueType::Vector(v) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(num) => num.as_usize()
                    .ok_or_else(|| AjisaiError::from("EYE size must be a positive integer"))?,
                _ => {
                    interp.stack.push(size_val);
                    return Err(AjisaiError::from("EYE size must be a number"));
                }
            }
        }
        _ => {
            interp.stack.push(size_val);
            return Err(AjisaiError::from("EYE requires scalar size"));
        }
    };

    if n == 0 {
        return Err(AjisaiError::from("EYE size must be positive"));
    }

    let zero = Fraction::new(BigInt::from(0), BigInt::from(1));
    let one = Fraction::new(BigInt::from(1), BigInt::from(1));
    let mut data = Vec::with_capacity(n * n);

    for i in 0..n {
        for j in 0..n {
            if i == j {
                data.push(one.clone());
            } else {
                data.push(zero.clone());
            }
        }
    }

    let result = Tensor::new(vec![n, n], data)?;
    interp.stack.push(Value::from_tensor(result));
    Ok(())
}

/// IOTA - 連番テンソル生成
///
/// 使用法:
///   [ 5 ] IOTA → [ 0 1 2 3 4 ]
///   [ 2 3 ] IOTA → [ [ 0 1 2 ] [ 3 4 5 ] ]
///   [ 2 2 2 ] IOTA → [ [ [ 0 1 ] [ 2 3 ] ] [ [ 4 5 ] [ 6 7 ] ] ]
pub fn op_iota(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("IOTA does not support Stack (..) mode"));
    }

    let shape_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let shape_tensor = match &shape_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert shape: {}", e)))?
        }
        _ => {
            interp.stack.push(shape_val);
            return Err(AjisaiError::from("IOTA requires tensor for shape"));
        }
    };

    let shape: Result<Vec<usize>> = shape_tensor
        .data()
        .iter()
        .map(|f| {
            f.as_usize()
                .ok_or_else(|| AjisaiError::from("Shape dimensions must be positive integers"))
        })
        .collect();
    let shape = shape?;

    if shape.is_empty() {
        interp.stack.push(shape_val);
        return Err(AjisaiError::from("IOTA requires non-empty shape"));
    }

    let size: usize = shape.iter().product();
    let data: Vec<Fraction> = (0..size)
        .map(|i| Fraction::new(BigInt::from(i as i64), BigInt::from(1)))
        .collect();

    let result = Tensor::new(shape, data)?;
    interp.stack.push(Value::from_tensor(result));
    Ok(())
}

/// LINSPACE - 等間隔数列生成
///
/// 使用法:
///   [ 0 ] [ 10 ] [ 5 ] LINSPACE → [ 0 5/2 5 15/2 10 ]  # 0から10まで5点
///   [ 1 ] [ 2 ] [ 3 ] LINSPACE → [ 1 3/2 2 ]           # 1から2まで3点
pub fn op_linspace(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("LINSPACE does not support Stack (..) mode"));
    }

    let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let end_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let start_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // countを取得
    let count = match &count_val.val_type {
        ValueType::Tensor(t) => {
            if t.is_scalar() {
                t.as_scalar().unwrap().as_usize()
                    .ok_or_else(|| AjisaiError::from("LINSPACE count must be a positive integer"))?
            } else if t.size() == 1 {
                t.data()[0].as_usize()
                    .ok_or_else(|| AjisaiError::from("LINSPACE count must be a positive integer"))?
            } else {
                interp.stack.push(start_val);
                interp.stack.push(end_val);
                interp.stack.push(count_val);
                return Err(AjisaiError::from("LINSPACE count must be a scalar"));
            }
        }
        _ => {
            interp.stack.push(start_val);
            interp.stack.push(end_val);
            interp.stack.push(count_val);
            return Err(AjisaiError::from("LINSPACE count must be a scalar tensor"));
        }
    };

    if count < 2 {
        interp.stack.push(start_val);
        interp.stack.push(end_val);
        interp.stack.push(count_val);
        return Err(AjisaiError::from("LINSPACE count must be at least 2"));
    }

    // startとendを取得
    let start = match &start_val.val_type {
        ValueType::Tensor(t) => {
            if t.is_scalar() {
                t.as_scalar().unwrap().clone()
            } else if t.size() == 1 {
                t.data()[0].clone()
            } else {
                interp.stack.push(start_val);
                interp.stack.push(end_val);
                interp.stack.push(count_val);
                return Err(AjisaiError::from("LINSPACE start must be a scalar"));
            }
        }
        _ => {
            interp.stack.push(start_val);
            interp.stack.push(end_val);
            interp.stack.push(count_val);
            return Err(AjisaiError::from("LINSPACE start must be a scalar tensor"));
        }
    };

    let end = match &end_val.val_type {
        ValueType::Tensor(t) => {
            if t.is_scalar() {
                t.as_scalar().unwrap().clone()
            } else if t.size() == 1 {
                t.data()[0].clone()
            } else {
                interp.stack.push(start_val);
                interp.stack.push(end_val);
                interp.stack.push(count_val);
                return Err(AjisaiError::from("LINSPACE end must be a scalar"));
            }
        }
        _ => {
            interp.stack.push(start_val);
            interp.stack.push(end_val);
            interp.stack.push(count_val);
            return Err(AjisaiError::from("LINSPACE end must be a scalar tensor"));
        }
    };

    // step = (end - start) / (count - 1)
    let count_minus_one = Fraction::new(BigInt::from((count - 1) as i64), BigInt::from(1));
    let step = end.sub(&start).div(&count_minus_one);

    let data: Vec<Fraction> = (0..count)
        .map(|i| {
            let i_frac = Fraction::new(BigInt::from(i as i64), BigInt::from(1));
            start.add(&step.mul(&i_frac))
        })
        .collect();

    let result = Tensor::vector(data);
    interp.stack.push(Value::from_tensor(result));
    Ok(())
}

// ============================================================================
// 軸指定演算（Phase 3）
// ============================================================================

/// ALONG - 汎用軸指定リダクション
///
/// 使用法:
///   [ [ 1 2 3 ] [ 4 5 6 ] ] '+' [ 0 ] ALONG → [ 5 7 9 ]   # 軸0で集約（列方向の合計）
///   [ [ 1 2 3 ] [ 4 5 6 ] ] '+' [ 1 ] ALONG → [ 6 15 ]    # 軸1で集約（行方向の合計）
///   [ [ 1 2 3 ] [ 4 5 6 ] ] '*' [ 0 ] ALONG → [ 4 10 18 ] # 軸0で積
pub fn op_along(interp: &mut Interpreter) -> Result<()> {
    use num_traits::ToPrimitive;

    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("ALONG does not support Stack (..) mode"));
    }

    let axis_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let op_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let tensor_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 軸を取得
    let axis = match &axis_val.val_type {
        ValueType::Tensor(t) => {
            if t.is_scalar() {
                t.as_scalar().unwrap().as_usize()
                    .ok_or_else(|| AjisaiError::from("ALONG axis must be a non-negative integer"))?
            } else if t.size() == 1 {
                t.data()[0].as_usize()
                    .ok_or_else(|| AjisaiError::from("ALONG axis must be a non-negative integer"))?
            } else {
                interp.stack.push(tensor_val);
                interp.stack.push(op_val);
                interp.stack.push(axis_val);
                return Err(AjisaiError::from("ALONG axis must be a scalar"));
            }
        }
        _ => {
            interp.stack.push(tensor_val);
            interp.stack.push(op_val);
            interp.stack.push(axis_val);
            return Err(AjisaiError::from("ALONG axis must be a scalar tensor"));
        }
    };

    // 演算ワード名を取得
    let op_name = match &op_val.val_type {
        ValueType::String(s) => s.clone(),
        ValueType::Vector(v) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::String(s) => s.clone(),
                _ => {
                    interp.stack.push(tensor_val);
                    interp.stack.push(op_val);
                    interp.stack.push(axis_val);
                    return Err(AjisaiError::from("ALONG operation must be a string"));
                }
            }
        }
        _ => {
            interp.stack.push(tensor_val);
            interp.stack.push(op_val);
            interp.stack.push(axis_val);
            return Err(AjisaiError::from("ALONG operation must be a string"));
        }
    };

    // テンソルを取得
    let tensor = match &tensor_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(tensor_val);
            interp.stack.push(op_val);
            interp.stack.push(axis_val);
            return Err(AjisaiError::from("ALONG requires tensor or vector"));
        }
    };

    if axis >= tensor.rank() {
        interp.stack.push(tensor_val);
        interp.stack.push(op_val);
        interp.stack.push(axis_val);
        return Err(AjisaiError::from(format!(
            "ALONG axis {} out of bounds for tensor of rank {}",
            axis, tensor.rank()
        )));
    }

    // 演算関数を取得
    let op_fn: Box<dyn Fn(&Fraction, &Fraction) -> Fraction> = match op_name.to_uppercase().as_str() {
        "+" => Box::new(|a, b| a.add(b)),
        "-" => Box::new(|a, b| a.sub(b)),
        "*" => Box::new(|a, b| a.mul(b)),
        "/" => Box::new(|a, b| a.div(b)),
        "MAX2" => Box::new(|a, b| if a.gt(b) { a.clone() } else { b.clone() }),
        "MIN2" => Box::new(|a, b| if a.lt(b) { a.clone() } else { b.clone() }),
        _ => {
            interp.stack.push(tensor_val);
            interp.stack.push(op_val);
            interp.stack.push(axis_val);
            return Err(AjisaiError::from(format!(
                "ALONG: unsupported operation '{}'. Supported: +, -, *, /, MAX2, MIN2",
                op_name
            )));
        }
    };

    // 軸に沿ってリダクション
    let shape = tensor.shape();
    let axis_size = shape[axis];

    if axis_size == 0 {
        return Err(AjisaiError::from("Cannot reduce along empty axis"));
    }

    // 結果の形状を計算（軸を削除）
    let result_shape: Vec<usize> = shape.iter().enumerate()
        .filter(|(i, _)| *i != axis)
        .map(|(_, &s)| s)
        .collect();

    if result_shape.is_empty() {
        // スカラー結果
        let mut acc = tensor.data()[0].clone();
        for i in 1..tensor.data().len() {
            acc = op_fn(&acc, &tensor.data()[i]);
        }
        interp.stack.push(Value::from_tensor(Tensor::scalar(acc)));
        return Ok(());
    }

    let result_size: usize = result_shape.iter().product();
    let mut result_data = Vec::with_capacity(result_size);

    // 各結果要素のインデックスを計算
    for result_idx in 0..result_size {
        // 結果インデックスを多次元に変換
        let mut result_multi_idx = vec![0usize; result_shape.len()];
        let mut temp = result_idx;
        for i in (0..result_shape.len()).rev() {
            result_multi_idx[i] = temp % result_shape[i];
            temp /= result_shape[i];
        }

        // 初期値を取得
        let mut source_idx = Vec::with_capacity(shape.len());
        let mut r_idx = 0;
        for s_idx in 0..shape.len() {
            if s_idx == axis {
                source_idx.push(0);
            } else {
                source_idx.push(result_multi_idx[r_idx]);
                r_idx += 1;
            }
        }

        let init_val = tensor.get(&source_idx)?;
        let mut acc = init_val.clone();

        // 軸に沿って畳み込み
        for k in 1..axis_size {
            source_idx[axis] = k;
            let val = tensor.get(&source_idx)?;
            acc = op_fn(&acc, val);
        }

        result_data.push(acc);
    }

    let result = Tensor::new(result_shape, result_data)?;
    interp.stack.push(Value::from_tensor(result));
    Ok(())
}

// ============================================================================
// 外積（Phase 4）
// ============================================================================

/// OUTER - 外積演算
///
/// 使用法:
///   [ 1 2 3 ] [ 4 5 ] '*' OUTER → [ [ 4 5 ] [ 8 10 ] [ 12 15 ] ]
///   [ 1 2 ] [ 1 2 3 ] '+' OUTER → [ [ 2 3 4 ] [ 3 4 5 ] ]
pub fn op_outer(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("OUTER does not support Stack (..) mode"));
    }

    let op_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 演算ワード名を取得
    let op_name = match &op_val.val_type {
        ValueType::String(s) => s.clone(),
        ValueType::Vector(v) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::String(s) => s.clone(),
                _ => {
                    interp.stack.push(a_val);
                    interp.stack.push(b_val);
                    interp.stack.push(op_val);
                    return Err(AjisaiError::from("OUTER operation must be a string"));
                }
            }
        }
        _ => {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
            interp.stack.push(op_val);
            return Err(AjisaiError::from("OUTER operation must be a string"));
        }
    };

    // テンソルを取得
    let a = match &a_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
            interp.stack.push(op_val);
            return Err(AjisaiError::from("OUTER requires tensor or vector"));
        }
    };

    let b = match &b_val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
            interp.stack.push(op_val);
            return Err(AjisaiError::from("OUTER requires tensor or vector"));
        }
    };

    // 両方が1次元であることを確認
    if a.rank() != 1 || b.rank() != 1 {
        interp.stack.push(a_val);
        interp.stack.push(b_val);
        interp.stack.push(op_val);
        return Err(AjisaiError::from("OUTER requires two 1-dimensional tensors (vectors)"));
    }

    // 演算関数を取得
    let op_fn: Box<dyn Fn(&Fraction, &Fraction) -> Fraction> = match op_name.to_uppercase().as_str() {
        "+" => Box::new(|a, b| a.add(b)),
        "-" => Box::new(|a, b| a.sub(b)),
        "*" => Box::new(|a, b| a.mul(b)),
        "/" => Box::new(|a, b| a.div(b)),
        _ => {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
            interp.stack.push(op_val);
            return Err(AjisaiError::from(format!(
                "OUTER: unsupported operation '{}'. Supported: +, -, *, /",
                op_name
            )));
        }
    };

    let a_len = a.shape()[0];
    let b_len = b.shape()[0];
    let mut result_data = Vec::with_capacity(a_len * b_len);

    for i in 0..a_len {
        for j in 0..b_len {
            result_data.push(op_fn(&a.data()[i], &b.data()[j]));
        }
    }

    let result = Tensor::new(vec![a_len, b_len], result_data)?;
    interp.stack.push(Value::from_tensor(result));
    Ok(())
}

// ============================================================================
// 統計関数（Phase 5）
// ============================================================================

/// VAR - 分散（母分散）
///
/// 使用法:
///   [ 1 2 3 4 5 ] VAR → [ 2 ]  # Σ(x-μ)²/n
pub fn op_var(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("VAR does not support Stack (..) mode yet"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("VAR requires tensor or vector"));
        }
    };

    if tensor.data().is_empty() {
        interp.stack.push(val);
        return Err(AjisaiError::from("VAR requires non-empty tensor"));
    }

    let n = tensor.size();
    let n_frac = Fraction::new(BigInt::from(n as i64), BigInt::from(1));

    // 平均を計算
    let sum = tensor.data().iter().fold(
        Fraction::new(BigInt::from(0), BigInt::from(1)),
        |acc, x| acc.add(x)
    );
    let mean = sum.div(&n_frac);

    // 分散を計算: Σ(x-μ)²/n
    let var_sum = tensor.data().iter().fold(
        Fraction::new(BigInt::from(0), BigInt::from(1)),
        |acc, x| {
            let diff = x.sub(&mean);
            acc.add(&diff.mul(&diff))
        }
    );
    let variance = var_sum.div(&n_frac);

    interp.stack.push(Value::from_tensor(Tensor::scalar(variance)));
    Ok(())
}

/// MEDIAN - 中央値
///
/// 使用法:
///   [ 1 2 3 4 5 ] MEDIAN → [ 3 ]
///   [ 1 2 3 4 ] MEDIAN → [ 5/2 ]  # 偶数個の場合は中央2値の平均
pub fn op_median(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target == OperationTarget::Stack {
        return Err(AjisaiError::from("MEDIAN does not support Stack (..) mode yet"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tensor = match &val.val_type {
        ValueType::Tensor(t) => t.clone(),
        ValueType::Vector(v) => {
            Value::vector_to_tensor(v)
                .map_err(|e| AjisaiError::from(format!("Failed to convert: {}", e)))?
        }
        _ => {
            interp.stack.push(val);
            return Err(AjisaiError::from("MEDIAN requires tensor or vector"));
        }
    };

    if tensor.data().is_empty() {
        interp.stack.push(val);
        return Err(AjisaiError::from("MEDIAN requires non-empty tensor"));
    }

    // データをソート
    let mut sorted: Vec<Fraction> = tensor.data().to_vec();
    sorted.sort();

    let n = sorted.len();
    let median = if n % 2 == 1 {
        // 奇数個: 中央の値
        sorted[n / 2].clone()
    } else {
        // 偶数個: 中央2値の平均
        let mid1 = &sorted[n / 2 - 1];
        let mid2 = &sorted[n / 2];
        let two = Fraction::new(BigInt::from(2), BigInt::from(1));
        mid1.add(mid2).div(&two)
    };

    interp.stack.push(Value::from_tensor(Tensor::scalar(median)));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;
    use crate::interpreter::Interpreter;

    fn frac(n: i64) -> Fraction {
        Fraction::new(BigInt::from(n), BigInt::from(1))
    }

    #[tokio::test]
    async fn test_tensor_consistency_after_arithmetic_operations() {
        let mut interp = Interpreter::new();

        // 算術演算の結果がTensorであることを確認
        interp.execute("[ 1 2 3 ] [ 4 5 6 ] +").await.unwrap();
        assert!(matches!(
            interp.stack.last().unwrap().val_type,
            ValueType::Tensor(_)
        ), "Addition result should be Tensor");

        interp.stack.clear();

        // 減算もTensorを返す
        interp.execute("[ 10 20 30 ] [ 1 2 3 ] -").await.unwrap();
        assert!(matches!(
            interp.stack.last().unwrap().val_type,
            ValueType::Tensor(_)
        ), "Subtraction result should be Tensor");

        interp.stack.clear();

        // 乗算もTensorを返す
        interp.execute("[ 2 3 4 ] [ 5 6 7 ] *").await.unwrap();
        assert!(matches!(
            interp.stack.last().unwrap().val_type,
            ValueType::Tensor(_)
        ), "Multiplication result should be Tensor");
    }

    #[tokio::test]
    async fn test_tensor_consistency_after_shape_operations() {
        let mut interp = Interpreter::new();

        // SHAPE結果がTensorであることを確認
        interp.execute("[ [ 1 2 ] [ 3 4 ] ] SHAPE").await.unwrap();
        assert!(matches!(
            interp.stack.last().unwrap().val_type,
            ValueType::Tensor(_)
        ), "SHAPE result should be Tensor");

        interp.stack.clear();

        // RANK結果がTensorであることを確認
        interp.execute("[ [ 1 2 ] [ 3 4 ] ] RANK").await.unwrap();
        assert!(matches!(
            interp.stack.last().unwrap().val_type,
            ValueType::Tensor(_)
        ), "RANK result should be Tensor");
    }

    #[tokio::test]
    async fn test_numeric_array_auto_converted_to_tensor() {
        let mut interp = Interpreter::new();

        // 数値のみの配列は自動的にTensorに変換される
        interp.execute("[ 1 2 3 ]").await.unwrap();
        assert!(matches!(
            interp.stack.last().unwrap().val_type,
            ValueType::Tensor(_)
        ), "Numeric array should be auto-converted to Tensor");

        interp.stack.clear();

        // ネストされた数値配列もTensorに変換される
        interp.execute("[ [ 1 2 ] [ 3 4 ] ]").await.unwrap();
        assert!(matches!(
            interp.stack.last().unwrap().val_type,
            ValueType::Tensor(_)
        ), "Nested numeric array should be auto-converted to Tensor");
    }

    #[tokio::test]
    async fn test_mixed_type_array_stays_vector() {
        let mut interp = Interpreter::new();

        // 混合型（数値と文字列）はVectorのまま
        interp.execute("[ 1 'hello' 3 ]").await.unwrap();
        assert!(matches!(
            interp.stack.last().unwrap().val_type,
            ValueType::Vector(_)
        ), "Mixed type array should remain as Vector");

        interp.stack.clear();

        // Boolean配列はVectorのまま
        interp.execute("[ TRUE FALSE TRUE ]").await.unwrap();
        assert!(matches!(
            interp.stack.last().unwrap().val_type,
            ValueType::Vector(_)
        ), "Boolean array should remain as Vector");
    }

    #[test]
    fn test_broadcast_shapes_same() {
        let result = broadcast_shapes(&[2, 3], &[2, 3]).unwrap();
        assert_eq!(result, vec![2, 3]);
    }

    #[test]
    fn test_broadcast_shapes_scalar_vector() {
        // [] と [3] → [3]
        let result = broadcast_shapes(&[], &[3]).unwrap();
        assert_eq!(result, vec![3]);
    }

    #[test]
    fn test_broadcast_shapes_vector_matrix() {
        // [3] と [2, 3] → [2, 3]
        let result = broadcast_shapes(&[3], &[2, 3]).unwrap();
        assert_eq!(result, vec![2, 3]);
    }

    #[test]
    fn test_broadcast_shapes_size_one() {
        // [2, 1] と [1, 3] → [2, 3]
        let result = broadcast_shapes(&[2, 1], &[1, 3]).unwrap();
        assert_eq!(result, vec![2, 3]);
    }

    #[test]
    fn test_broadcast_shapes_incompatible() {
        // [2, 3] と [2, 4] → エラー
        assert!(broadcast_shapes(&[2, 3], &[2, 4]).is_err());
    }

    #[test]
    fn test_broadcast_binary_op_scalar_vector() {
        let a = Tensor::scalar(frac(10));
        let b = Tensor::vector(vec![frac(1), frac(2), frac(3)]);

        let result = broadcast_binary_op(&a, &b, |x, y| {
            Ok(Fraction::new(
                &x.numerator + &y.numerator,
                x.denominator.clone()
            ))
        }, "ADD").unwrap();

        assert_eq!(result.shape(), &[3]);
        assert_eq!(result.get(&[0]).unwrap(), &frac(11));
        assert_eq!(result.get(&[1]).unwrap(), &frac(12));
        assert_eq!(result.get(&[2]).unwrap(), &frac(13));
    }

    #[test]
    fn test_broadcast_binary_op_vector_matrix() {
        // [2, 3] 行列と [3] ベクタのブロードキャスト
        let matrix = Tensor::new(
            vec![2, 3],
            vec![frac(1), frac(2), frac(3), frac(4), frac(5), frac(6)]
        ).unwrap();
        let vector = Tensor::vector(vec![frac(10), frac(20), frac(30)]);

        let result = broadcast_binary_op(&matrix, &vector, |x, y| {
            Ok(Fraction::new(
                &x.numerator + &y.numerator,
                x.denominator.clone()
            ))
        }, "ADD").unwrap();

        assert_eq!(result.shape(), &[2, 3]);
        // 第1行: [1+10, 2+20, 3+30] = [11, 22, 33]
        assert_eq!(result.get(&[0, 0]).unwrap(), &frac(11));
        assert_eq!(result.get(&[0, 1]).unwrap(), &frac(22));
        assert_eq!(result.get(&[0, 2]).unwrap(), &frac(33));
        // 第2行: [4+10, 5+20, 6+30] = [14, 25, 36]
        assert_eq!(result.get(&[1, 0]).unwrap(), &frac(14));
        assert_eq!(result.get(&[1, 1]).unwrap(), &frac(25));
        assert_eq!(result.get(&[1, 2]).unwrap(), &frac(36));
    }

    // ============================================================================
    // 集約関数のテスト
    // ============================================================================

    #[tokio::test]
    async fn test_sum_vector() {
        let mut interp = Interpreter::new();
        interp.execute("[ 1 2 3 4 5 ] SUM").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert!(t.is_scalar());
            assert_eq!(t.as_scalar().unwrap(), &frac(15));
        } else {
            panic!("Expected Tensor");
        }
    }

    #[tokio::test]
    async fn test_sum_matrix() {
        let mut interp = Interpreter::new();
        interp.execute("[ [ 1 2 3 ] [ 4 5 6 ] ] SUM").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert!(t.is_scalar());
            assert_eq!(t.as_scalar().unwrap(), &frac(21));
        } else {
            panic!("Expected Tensor");
        }
    }

    #[tokio::test]
    async fn test_max_vector() {
        let mut interp = Interpreter::new();
        interp.execute("[ 3 1 4 1 5 9 2 6 ] MAX").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert!(t.is_scalar());
            assert_eq!(t.as_scalar().unwrap(), &frac(9));
        } else {
            panic!("Expected Tensor");
        }
    }

    #[tokio::test]
    async fn test_min_vector() {
        let mut interp = Interpreter::new();
        interp.execute("[ 3 1 4 1 5 9 2 6 ] MIN").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert!(t.is_scalar());
            assert_eq!(t.as_scalar().unwrap(), &frac(1));
        } else {
            panic!("Expected Tensor");
        }
    }

    #[tokio::test]
    async fn test_mean_vector() {
        let mut interp = Interpreter::new();
        interp.execute("[ 1 2 3 4 5 ] MEAN").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert!(t.is_scalar());
            assert_eq!(t.as_scalar().unwrap(), &frac(3));
        } else {
            panic!("Expected Tensor");
        }
    }

    #[tokio::test]
    async fn test_mean_matrix() {
        let mut interp = Interpreter::new();
        interp.execute("[ [ 10 20 ] [ 30 40 ] ] MEAN").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert!(t.is_scalar());
            assert_eq!(t.as_scalar().unwrap(), &frac(25));
        } else {
            panic!("Expected Tensor");
        }
    }

    #[tokio::test]
    async fn test_product_vector() {
        let mut interp = Interpreter::new();
        interp.execute("[ 1 2 3 4 5 ] PRODUCT").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert!(t.is_scalar());
            assert_eq!(t.as_scalar().unwrap(), &frac(120));
        } else {
            panic!("Expected Tensor");
        }
    }

    // ============================================================================
    // 軸指定集約関数のテスト
    // ============================================================================

    #[tokio::test]
    async fn test_sum_rows() {
        let mut interp = Interpreter::new();
        interp.execute("[ [ 1 2 3 ] [ 4 5 6 ] ] SUM-ROWS").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert_eq!(t.shape(), &[2]);
            assert_eq!(t.data()[0], frac(6));
            assert_eq!(t.data()[1], frac(15));
        } else {
            panic!("Expected Tensor");
        }
    }

    #[tokio::test]
    async fn test_sum_cols() {
        let mut interp = Interpreter::new();
        interp.execute("[ [ 1 2 3 ] [ 4 5 6 ] ] SUM-COLS").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert_eq!(t.shape(), &[3]);
            assert_eq!(t.data()[0], frac(5));
            assert_eq!(t.data()[1], frac(7));
            assert_eq!(t.data()[2], frac(9));
        } else {
            panic!("Expected Tensor");
        }
    }

    #[tokio::test]
    async fn test_mean_rows() {
        let mut interp = Interpreter::new();
        interp.execute("[ [ 1 2 3 ] [ 4 5 6 ] ] MEAN-ROWS").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert_eq!(t.shape(), &[2]);
            assert_eq!(t.data()[0], frac(2));
            assert_eq!(t.data()[1], frac(5));
        } else {
            panic!("Expected Tensor");
        }
    }

    #[tokio::test]
    async fn test_max_rows() {
        let mut interp = Interpreter::new();
        interp.execute("[ [ 1 5 3 ] [ 4 2 6 ] ] MAX-ROWS").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert_eq!(t.shape(), &[2]);
            assert_eq!(t.data()[0], frac(5));
            assert_eq!(t.data()[1], frac(6));
        } else {
            panic!("Expected Tensor");
        }
    }

    #[tokio::test]
    async fn test_max_cols() {
        let mut interp = Interpreter::new();
        interp.execute("[ [ 1 5 3 ] [ 4 2 6 ] ] MAX-COLS").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert_eq!(t.shape(), &[3]);
            assert_eq!(t.data()[0], frac(4));
            assert_eq!(t.data()[1], frac(5));
            assert_eq!(t.data()[2], frac(6));
        } else {
            panic!("Expected Tensor");
        }
    }

    #[tokio::test]
    async fn test_min_rows() {
        let mut interp = Interpreter::new();
        interp.execute("[ [ 1 5 3 ] [ 4 2 6 ] ] MIN-ROWS").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert_eq!(t.shape(), &[2]);
            assert_eq!(t.data()[0], frac(1));
            assert_eq!(t.data()[1], frac(2));
        } else {
            panic!("Expected Tensor");
        }
    }

    #[tokio::test]
    async fn test_min_cols() {
        let mut interp = Interpreter::new();
        interp.execute("[ [ 1 5 3 ] [ 4 2 6 ] ] MIN-COLS").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert_eq!(t.shape(), &[3]);
            assert_eq!(t.data()[0], frac(1));
            assert_eq!(t.data()[1], frac(2));
            assert_eq!(t.data()[2], frac(3));
        } else {
            panic!("Expected Tensor");
        }
    }

    // ============================================================================
    // 行列演算のテスト
    // ============================================================================

    #[tokio::test]
    async fn test_dot() {
        let mut interp = Interpreter::new();
        interp.execute("[ 1 2 3 ] [ 4 5 6 ] DOT").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert!(t.is_scalar());
            // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
            assert_eq!(t.as_scalar().unwrap(), &frac(32));
        } else {
            panic!("Expected Tensor");
        }
    }

    #[tokio::test]
    async fn test_matmul_matrix_matrix() {
        let mut interp = Interpreter::new();
        interp.execute("[ [ 1 2 ] [ 3 4 ] ] [ [ 5 6 ] [ 7 8 ] ] MATMUL").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert_eq!(t.shape(), &[2, 2]);
            // [[1*5+2*7, 1*6+2*8], [3*5+4*7, 3*6+4*8]]
            // [[5+14, 6+16], [15+28, 18+32]]
            // [[19, 22], [43, 50]]
            assert_eq!(t.get(&[0, 0]).unwrap(), &frac(19));
            assert_eq!(t.get(&[0, 1]).unwrap(), &frac(22));
            assert_eq!(t.get(&[1, 0]).unwrap(), &frac(43));
            assert_eq!(t.get(&[1, 1]).unwrap(), &frac(50));
        } else {
            panic!("Expected Tensor");
        }
    }

    #[tokio::test]
    async fn test_matmul_matrix_vector() {
        let mut interp = Interpreter::new();
        interp.execute("[ [ 1 2 ] [ 3 4 ] ] [ 5 6 ] MATMUL").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert_eq!(t.shape(), &[2]);
            // [1*5+2*6, 3*5+4*6] = [5+12, 15+24] = [17, 39]
            assert_eq!(t.data()[0], frac(17));
            assert_eq!(t.data()[1], frac(39));
        } else {
            panic!("Expected Tensor");
        }
    }

    // ============================================================================
    // テンソルアクセス関数のテスト
    // ============================================================================

    #[tokio::test]
    async fn test_row() {
        let mut interp = Interpreter::new();
        interp.execute("[ [ 1 2 3 ] [ 4 5 6 ] [ 7 8 9 ] ] [ 1 ] ROW").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert_eq!(t.shape(), &[3]);
            assert_eq!(t.data()[0], frac(4));
            assert_eq!(t.data()[1], frac(5));
            assert_eq!(t.data()[2], frac(6));
        } else {
            panic!("Expected Tensor");
        }
    }

    #[tokio::test]
    async fn test_col() {
        let mut interp = Interpreter::new();
        interp.execute("[ [ 1 2 3 ] [ 4 5 6 ] [ 7 8 9 ] ] [ 2 ] COL").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert_eq!(t.shape(), &[3]);
            assert_eq!(t.data()[0], frac(3));
            assert_eq!(t.data()[1], frac(6));
            assert_eq!(t.data()[2], frac(9));
        } else {
            panic!("Expected Tensor");
        }
    }

    #[tokio::test]
    async fn test_diag() {
        let mut interp = Interpreter::new();
        interp.execute("[ [ 1 2 3 ] [ 4 5 6 ] [ 7 8 9 ] ] DIAG").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert_eq!(t.shape(), &[3]);
            assert_eq!(t.data()[0], frac(1));
            assert_eq!(t.data()[1], frac(5));
            assert_eq!(t.data()[2], frac(9));
        } else {
            panic!("Expected Tensor");
        }
    }

    // ============================================================================
    // エッジケースのテスト
    // ============================================================================

    #[tokio::test]
    async fn test_sum_empty_tensor() {
        let mut interp = Interpreter::new();
        // 空テンソルの場合、sumは0を返す
        interp.execute("[ ] SUM").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert!(t.is_scalar());
            assert_eq!(t.as_scalar().unwrap(), &frac(0));
        } else {
            panic!("Expected Tensor");
        }
    }

    #[tokio::test]
    async fn test_product_empty_tensor() {
        let mut interp = Interpreter::new();
        // 空テンソルの場合、productは1を返す（乗法単位元）
        interp.execute("[ ] PRODUCT").await.unwrap();

        let result = interp.stack.last().unwrap();
        if let ValueType::Tensor(t) = &result.val_type {
            assert!(t.is_scalar());
            assert_eq!(t.as_scalar().unwrap(), &frac(1));
        } else {
            panic!("Expected Tensor");
        }
    }

    #[tokio::test]
    async fn test_sum_with_reduce_equivalence() {
        // [ 1 2 3 4 5 ] '+' REDUCE と [ 1 2 3 4 5 ] SUM は同じ結果
        let mut interp1 = Interpreter::new();
        let mut interp2 = Interpreter::new();

        interp1.execute("[ 1 2 3 4 5 ] '+' REDUCE").await.unwrap();
        interp2.execute("[ 1 2 3 4 5 ] SUM").await.unwrap();

        let result1 = &interp1.stack.last().unwrap().val_type;
        let result2 = &interp2.stack.last().unwrap().val_type;

        // REDUCEは1要素のベクタ [15] を返し、SUMはスカラー 15 を返す
        // どちらも同じ値を持っていることを確認
        if let (ValueType::Tensor(t1), ValueType::Tensor(t2)) = (result1, result2) {
            // REDUCE結果: 1要素のベクタ [15] またはスカラー
            // SUM結果: スカラー
            let val1 = if t1.is_scalar() {
                t1.as_scalar().unwrap()
            } else {
                &t1.data()[0]
            };
            let val2 = if t2.is_scalar() {
                t2.as_scalar().unwrap()
            } else {
                &t2.data()[0]
            };
            assert_eq!(val1, val2);
        } else {
            panic!("Expected Tensors");
        }
    }
}
