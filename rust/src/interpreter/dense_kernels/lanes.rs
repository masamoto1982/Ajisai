//! The operands and the result of a column kernel: an operand's lanes read
//! straight off a Tensor's columns or splatted from a scalar, and the result
//! columns. A child module of `dense_kernels` so the kernels themselves read
//! as the laws they compute.

use crate::types::{DenseTensor, Value, ValueData};
use std::sync::Arc;

/// An operand's lanes: a Tensor's columns, or one rational for every lane.
///
/// The columns are held as plain slices, taken off the Tensor once: indexing
/// the `SmallVec` columns themselves re-tests whether they are inline on every
/// lane, which kept the kernels from vectorising and cost more than the
/// arithmetic.
#[derive(Clone, Copy)]
pub(super) enum Lanes<'a> {
    Columns {
        nums: &'a [i64],
        dens: &'a [i64],
        /// Every lane is an integer, so the integer kernels apply to the
        /// numerators alone.
        integer: bool,
    },
    Splat(i64, i64),
}

impl Lanes<'_> {
    pub(super) fn of(value: &Value) -> Option<Lanes<'_>> {
        match &value.data {
            ValueData::Tensor { data, shape } if shape.len() == 1 && !data.is_empty() => {
                Some(Lanes::Columns {
                    nums: &data.numerators,
                    dens: &data.denominators,
                    integer: data.is_pure_integer,
                })
            }
            ValueData::Scalar(f) if value.absence.is_none() => {
                let (n, d) = f.extract_i64_pair()?;
                Some(Lanes::Splat(n, d))
            }
            _ => None,
        }
    }

    pub(super) fn len(self) -> Option<usize> {
        match self {
            Lanes::Columns { nums, .. } => Some(nums.len()),
            Lanes::Splat(..) => None,
        }
    }

    pub(super) fn integer(self) -> bool {
        match self {
            Lanes::Columns { integer, .. } => integer,
            Lanes::Splat(_, d) => d == 1,
        }
    }

    #[inline(always)]
    pub(super) fn at(self, i: usize) -> (i64, i64) {
        match self {
            Lanes::Columns { nums, dens, .. } => (nums[i], dens[i]),
            Lanes::Splat(n, d) => (n, d),
        }
    }

    #[inline(always)]
    pub(super) fn num(self, i: usize) -> i64 {
        match self {
            Lanes::Columns { nums, .. } => nums[i],
            Lanes::Splat(n, _) => n,
        }
    }
}

/// The lane count two operands pair over: a Tensor's length against a
/// scalar, or the shared length of two Tensors. Anything else — two scalars,
/// two lengths — is the general route's.
pub(super) fn paired(a: Lanes, b: Lanes) -> Option<usize> {
    match (a.len(), b.len()) {
        (Some(n), None) | (None, Some(n)) => Some(n),
        (Some(n), Some(m)) if n == m => Some(n),
        _ => None,
    }
}

/// A kernel's result columns, built as plain `Vec`s (a push into a
/// `SmallVec` re-tests whether it has spilled) and handed to the Tensor
/// without a copy.
pub(super) struct Out {
    pub(super) nums: Vec<i64>,
    pub(super) dens: Vec<i64>,
    integer: bool,
}

impl Out {
    pub(super) fn with_capacity(n: usize) -> Self {
        Self {
            nums: Vec::with_capacity(n),
            dens: Vec::with_capacity(n),
            integer: true,
        }
    }

    /// Integer lanes already computed.
    pub(super) fn integers(nums: Vec<i64>) -> Self {
        let n = nums.len();
        Self {
            nums,
            dens: vec![1; n],
            integer: true,
        }
    }

    /// Columns already computed, in lowest terms.
    pub(super) fn columns(nums: Vec<i64>, dens: Vec<i64>) -> Self {
        let integer = dens.iter().all(|&d| d == 1);
        Self {
            nums,
            dens,
            integer,
        }
    }

    #[inline(always)]
    pub(super) fn push(&mut self, num: i64, den: i64) {
        self.nums.push(num);
        self.dens.push(den);
        self.integer &= den == 1;
    }

    pub(super) fn into_value(self) -> Value {
        let shape = vec![self.nums.len()];
        let tensor = DenseTensor::from_columns(self.nums, self.dens, shape.clone(), self.integer);
        Value::new(
            ValueData::Tensor {
                data: Arc::new(tensor),
                shape: Arc::new(shape),
            },
            None,
        )
    }
}
