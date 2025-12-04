//! テンソル演算とブロードキャスト機能
//!
//! NumPy/APL準拠のブロードキャスト規則に基づくテンソル演算を提供

use crate::error::{AjisaiError, Result};
use crate::types::tensor::Tensor;
use crate::types::fraction::Fraction;

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

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;

    fn frac(n: i64) -> Fraction {
        Fraction::new(BigInt::from(n), BigInt::from(1))
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
