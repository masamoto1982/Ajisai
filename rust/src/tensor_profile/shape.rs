use std::fmt;

/// Host/device allocation ceilings used before a tensor buffer is allocated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TensorMemoryBudget {
    pub max_elements: usize,
    pub max_bytes: usize,
}

impl TensorMemoryBudget {
    pub const fn new(max_elements: usize, max_bytes: usize) -> Self {
        Self {
            max_elements,
            max_bytes,
        }
    }
}

/// A shape whose element count and row-major strides cannot overflow `usize`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedShape {
    dimensions: Vec<usize>,
    element_count: usize,
    row_major_strides: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShapeError {
    ElementCountOverflow,
    ByteCountOverflow,
    ElementBudgetExceeded { requested: usize, limit: usize },
    ByteBudgetExceeded { requested: usize, limit: usize },
}

impl fmt::Display for ShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ElementCountOverflow => {
                write!(f, "tensor element count overflows the host index space")
            }
            Self::ByteCountOverflow => {
                write!(f, "tensor byte count overflows the host index space")
            }
            Self::ElementBudgetExceeded { requested, limit } => write!(
                f,
                "tensor requires {requested} elements, exceeding the limit of {limit}"
            ),
            Self::ByteBudgetExceeded { requested, limit } => write!(
                f,
                "tensor requires {requested} bytes, exceeding the limit of {limit}"
            ),
        }
    }
}

impl std::error::Error for ShapeError {}

impl CheckedShape {
    /// Validate dimensions, strides, element count, and allocation bytes as a
    /// single operation. Rank-zero is one scalar element; a zero dimension is
    /// an empty tensor.
    pub fn new(
        dimensions: Vec<usize>,
        element_bytes: usize,
        budget: TensorMemoryBudget,
    ) -> Result<Self, ShapeError> {
        let element_count = dimensions
            .iter()
            .try_fold(1usize, |count, dimension| count.checked_mul(*dimension))
            .ok_or(ShapeError::ElementCountOverflow)?;
        if element_count > budget.max_elements {
            return Err(ShapeError::ElementBudgetExceeded {
                requested: element_count,
                limit: budget.max_elements,
            });
        }
        let bytes = element_count
            .checked_mul(element_bytes)
            .ok_or(ShapeError::ByteCountOverflow)?;
        if bytes > budget.max_bytes {
            return Err(ShapeError::ByteBudgetExceeded {
                requested: bytes,
                limit: budget.max_bytes,
            });
        }

        let mut row_major_strides = vec![0; dimensions.len()];
        let mut stride = 1usize;
        for (axis, dimension) in dimensions.iter().enumerate().rev() {
            row_major_strides[axis] = stride;
            stride = stride
                .checked_mul(*dimension)
                .ok_or(ShapeError::ElementCountOverflow)?;
        }
        Ok(Self {
            dimensions,
            element_count,
            row_major_strides,
        })
    }

    pub fn dimensions(&self) -> &[usize] {
        &self.dimensions
    }

    pub const fn element_count(&self) -> usize {
        self.element_count
    }

    pub fn row_major_strides(&self) -> &[usize] {
        &self.row_major_strides
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPEN: TensorMemoryBudget = TensorMemoryBudget::new(usize::MAX, usize::MAX);

    #[test]
    fn scalar_and_zero_dimension_have_defined_counts() {
        assert_eq!(
            CheckedShape::new(vec![], 4, OPEN).unwrap().element_count(),
            1
        );
        assert_eq!(
            CheckedShape::new(vec![2, 0, 3], 4, OPEN)
                .unwrap()
                .element_count(),
            0
        );
    }

    #[test]
    fn computes_row_major_strides_once() {
        let shape = CheckedShape::new(vec![2, 3, 4], 4, OPEN).unwrap();
        assert_eq!(shape.row_major_strides(), &[12, 4, 1]);
    }

    #[test]
    fn rejects_product_overflow_before_allocation() {
        assert_eq!(
            CheckedShape::new(vec![usize::MAX, 2], 4, OPEN),
            Err(ShapeError::ElementCountOverflow)
        );
    }

    #[test]
    fn enforces_byte_budget_independently_of_element_budget() {
        let budget = TensorMemoryBudget::new(8, 16);
        assert_eq!(
            CheckedShape::new(vec![8], 4, budget),
            Err(ShapeError::ByteBudgetExceeded {
                requested: 32,
                limit: 16
            })
        );
    }
}
