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
// 基本数学関数
// ============================================================================

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

// ============================================================================
// テンソル生成関数（Phase 2）
// ============================================================================

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
}
