use super::{CheckedShape, ShapeError, TensorMemoryBudget};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DType {
    F32,
    F64,
}

impl DType {
    pub const fn element_bytes(self) -> usize {
        match self {
            Self::F32 => 4,
            Self::F64 => 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TensorData {
    F32(Vec<f32>),
    F64(Vec<f64>),
}

impl TensorData {
    fn dtype(&self) -> DType {
        match self {
            Self::F32(_) => DType::F32,
            Self::F64(_) => DType::F64,
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::F32(values) => values.len(),
            Self::F64(values) => values.len(),
        }
    }
}

/// An immutable, explicitly approximate Tensor Profile value.
#[derive(Debug, Clone, PartialEq)]
pub struct Tensor {
    dtype: DType,
    shape: CheckedShape,
    data: TensorData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TensorError {
    Shape(ShapeError),
    ElementCountMismatch { expected: usize, actual: usize },
}

impl fmt::Display for TensorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shape(error) => error.fmt(f),
            Self::ElementCountMismatch { expected, actual } => {
                write!(
                    f,
                    "tensor shape requires {expected} elements, received {actual}"
                )
            }
        }
    }
}

impl std::error::Error for TensorError {}

impl From<ShapeError> for TensorError {
    fn from(value: ShapeError) -> Self {
        Self::Shape(value)
    }
}

impl Tensor {
    pub fn new(
        dimensions: Vec<usize>,
        data: TensorData,
        budget: TensorMemoryBudget,
    ) -> Result<Self, TensorError> {
        let dtype = data.dtype();
        let shape = CheckedShape::new(dimensions, dtype.element_bytes(), budget)?;
        if shape.element_count() != data.len() {
            return Err(TensorError::ElementCountMismatch {
                expected: shape.element_count(),
                actual: data.len(),
            });
        }
        Ok(Self { dtype, shape, data })
    }

    pub const fn dtype(&self) -> DType {
        self.dtype
    }

    pub const fn shape(&self) -> &CheckedShape {
        &self.shape
    }

    pub const fn data(&self) -> &TensorData {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPEN: TensorMemoryBudget = TensorMemoryBudget::new(usize::MAX, usize::MAX);

    #[test]
    fn constructs_f32_tensor_without_entering_core_value_domain() {
        let tensor =
            Tensor::new(vec![2, 2], TensorData::F32(vec![1.0, 2.0, 3.0, 4.0]), OPEN).unwrap();
        assert_eq!(tensor.dtype(), DType::F32);
        assert_eq!(tensor.shape().dimensions(), &[2, 2]);
    }

    #[test]
    fn rejects_shape_and_buffer_disagreement() {
        assert_eq!(
            Tensor::new(vec![2, 2], TensorData::F64(vec![1.0]), OPEN),
            Err(TensorError::ElementCountMismatch {
                expected: 4,
                actual: 1
            })
        );
    }

    #[test]
    fn nan_remains_tensor_data() {
        let tensor = Tensor::new(vec![], TensorData::F32(vec![f32::NAN]), OPEN).unwrap();
        let TensorData::F32(values) = tensor.data() else {
            panic!("dtype changed")
        };
        assert!(values[0].is_nan());
    }
}
