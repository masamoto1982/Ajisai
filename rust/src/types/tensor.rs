//! テンソル（N次元配列）の実装
//!
//! Ajisaiの次元モデルの中核となるデータ構造。
//! すべての数値データはTensorとして表現される。

use crate::types::fraction::Fraction;
use crate::error::{AjisaiError, Result};
use num_traits::ToPrimitive;

/// テンソル構造体
#[derive(Debug, Clone, PartialEq)]
pub struct Tensor {
    /// 形状: 各次元のサイズ
    /// - [] (空): スカラー（0次元）
    /// - [n]: ベクタ（1次元、長さn）
    /// - [m, n]: 行列（2次元、m行n列）
    shape: Vec<usize>,

    /// データ: 行優先順序（row-major order）で格納
    data: Vec<Fraction>,
}

impl Tensor {
    /// 最大次元数（time, layer, row, col）
    pub const MAX_DIMENSIONS: usize = 4;

    /// 次元数の検証
    fn validate_dimensions(shape: &[usize]) -> Result<()> {
        if shape.len() > Self::MAX_DIMENSIONS {
            return Err(AjisaiError::from(format!(
                "Ajisai supports up to {} dimensions (time, layer, row, col), got {}",
                Self::MAX_DIMENSIONS, shape.len()
            )));
        }
        Ok(())
    }

    /// スカラーを作成
    pub fn scalar(value: Fraction) -> Self {
        Tensor {
            shape: vec![],
            data: vec![value],
        }
    }

    /// 1次元テンソル（ベクタ）を作成
    pub fn vector(data: Vec<Fraction>) -> Self {
        let len = data.len();
        Tensor {
            shape: vec![len],
            data,
        }
    }

    /// 任意の形状でテンソルを作成
    pub fn new(shape: Vec<usize>, data: Vec<Fraction>) -> Result<Self> {
        // 次元数の検証
        Self::validate_dimensions(&shape)?;

        let expected_len: usize = if shape.is_empty() {
            1 // スカラーの場合
        } else {
            shape.iter().product()
        };

        if expected_len == 0 && data.is_empty() {
            // 空テンソル
            Ok(Tensor { shape, data })
        } else if data.len() != expected_len {
            Err(AjisaiError::from(format!(
                "Shape {:?} requires {} elements, but got {}",
                shape, expected_len, data.len()
            )))
        } else {
            Ok(Tensor { shape, data })
        }
    }

    /// 次元数（ランク）を取得
    pub fn rank(&self) -> usize {
        self.shape.len()
    }

    /// 形状を取得
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    /// データへの参照を取得（読み取り専用）
    pub fn data(&self) -> &[Fraction] {
        &self.data
    }

    /// 要素総数を取得
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// スカラーかどうか
    pub fn is_scalar(&self) -> bool {
        self.shape.is_empty()
    }

    /// スカラー値を取得（スカラーの場合のみ）
    pub fn as_scalar(&self) -> Option<&Fraction> {
        if self.is_scalar() {
            self.data.first()
        } else {
            None
        }
    }

    /// スカラー値をusizeとして取得（スカラーの場合のみ）
    pub fn as_scalar_usize(&self) -> Result<usize> {
        let scalar = self.as_scalar()
            .ok_or_else(|| AjisaiError::from("Expected scalar"))?;

        scalar.to_usize()
            .ok_or_else(|| AjisaiError::from("Cannot convert to usize"))
    }

    /// 形状を変更（要素数が一致する必要あり）
    pub fn reshape(&self, new_shape: Vec<usize>) -> Result<Self> {
        // 次元数の検証
        Self::validate_dimensions(&new_shape)?;

        let new_size: usize = if new_shape.is_empty() {
            1
        } else {
            new_shape.iter().product()
        };

        if new_size != self.size() {
            return Err(AjisaiError::from(format!(
                "Cannot reshape: {} elements to shape {:?} ({} elements)",
                self.size(), new_shape, new_size
            )));
        }
        Ok(Tensor {
            shape: new_shape,
            data: self.data.clone(),
        })
    }

    /// 転置（2次元の場合）
    pub fn transpose(&self) -> Result<Self> {
        if self.rank() != 2 {
            return Err(AjisaiError::from(
                "TRANSPOSE requires 2-dimensional tensor"
            ));
        }

        let rows = self.shape[0];
        let cols = self.shape[1];
        let mut new_data = Vec::with_capacity(self.data.len());

        for j in 0..cols {
            for i in 0..rows {
                new_data.push(self.data[i * cols + j].clone());
            }
        }

        Ok(Tensor {
            shape: vec![cols, rows],
            data: new_data,
        })
    }

    /// インデックスアクセス（多次元対応）
    pub fn get(&self, indices: &[usize]) -> Result<&Fraction> {
        if indices.len() != self.rank() {
            return Err(AjisaiError::from(format!(
                "Expected {} indices, got {}",
                self.rank(), indices.len()
            )));
        }

        let flat_index = self.flat_index(indices)?;
        self.data.get(flat_index).ok_or_else(|| {
            AjisaiError::from("Index out of bounds")
        })
    }

    /// 多次元インデックスを1次元インデックスに変換
    fn flat_index(&self, indices: &[usize]) -> Result<usize> {
        let mut flat = 0;
        let mut stride = 1;

        for (i, (&idx, &dim)) in indices.iter().zip(&self.shape).rev().enumerate() {
            if idx >= dim {
                return Err(AjisaiError::from(format!(
                    "Index {} out of bounds for dimension {} (size {})",
                    idx, self.rank() - 1 - i, dim
                )));
            }
            flat += idx * stride;
            stride *= dim;
        }

        Ok(flat)
    }

    /// 1次元に平坦化
    pub fn flatten(&self) -> Self {
        Tensor {
            shape: vec![self.size()],
            data: self.data.clone(),
        }
    }
}

impl Fraction {
    /// Fractionをusizeに変換
    pub fn to_usize(&self) -> Option<usize> {
        if self.denominator == num_bigint::BigInt::from(1) {
            self.numerator.to_usize()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;

    fn frac(n: i64) -> Fraction {
        Fraction::new(BigInt::from(n), BigInt::from(1))
    }

    #[test]
    fn test_scalar_creation() {
        let s = Tensor::scalar(frac(5));
        assert_eq!(s.rank(), 0);
        let empty: &[usize] = &[];
        assert_eq!(s.shape(), empty);
        assert_eq!(s.size(), 1);
        assert!(s.is_scalar());
    }

    #[test]
    fn test_vector_creation() {
        let v = Tensor::vector(vec![frac(1), frac(2), frac(3)]);
        assert_eq!(v.rank(), 1);
        assert_eq!(v.shape(), &[3]);
        assert_eq!(v.size(), 3);
        assert!(!v.is_scalar());
    }

    #[test]
    fn test_matrix_creation() {
        let m = Tensor::new(
            vec![2, 2],
            vec![frac(1), frac(2), frac(3), frac(4)]
        ).unwrap();
        assert_eq!(m.rank(), 2);
        assert_eq!(m.shape(), &[2, 2]);
        assert_eq!(m.size(), 4);
    }

    #[test]
    fn test_reshape() {
        let v = Tensor::vector(vec![frac(1), frac(2), frac(3), frac(4)]);
        let m = v.reshape(vec![2, 2]).unwrap();
        assert_eq!(m.shape(), &[2, 2]);
        assert_eq!(m.size(), 4);
    }

    #[test]
    fn test_reshape_error() {
        let v = Tensor::vector(vec![frac(1), frac(2), frac(3)]);
        assert!(v.reshape(vec![2, 2]).is_err());
    }

    #[test]
    fn test_transpose() {
        let m = Tensor::new(
            vec![2, 3],
            vec![frac(1), frac(2), frac(3), frac(4), frac(5), frac(6)]
        ).unwrap();

        let mt = m.transpose().unwrap();
        assert_eq!(mt.shape(), &[3, 2]);

        // 転置後の値を確認
        assert_eq!(mt.get(&[0, 0]).unwrap(), &frac(1));
        assert_eq!(mt.get(&[0, 1]).unwrap(), &frac(4));
        assert_eq!(mt.get(&[1, 0]).unwrap(), &frac(2));
        assert_eq!(mt.get(&[1, 1]).unwrap(), &frac(5));
    }

    #[test]
    fn test_flatten() {
        let m = Tensor::new(
            vec![2, 3],
            vec![frac(1), frac(2), frac(3), frac(4), frac(5), frac(6)]
        ).unwrap();

        let f = m.flatten();
        assert_eq!(f.shape(), &[6]);
        assert_eq!(f.size(), 6);
    }
}
