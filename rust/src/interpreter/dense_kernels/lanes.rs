//! The operands and the result of a column kernel: an operand's lanes read
//! straight off a Tensor's columns or splatted from a scalar, and the result
//! columns. A child module of `dense_kernels` so the kernels themselves read
//! as the laws they compute.

use crate::types::Column;
use crate::types::{DenseTensor, Value, ValueData};
use std::sync::Arc;

/// An operand's lanes: a Tensor's columns, or one rational for every lane.
#[derive(Clone, Copy)]
pub(super) enum Lanes<'a> {
    Columns {
        tensor: &'a DenseTensor,
        /// Every lane is an integer, so the integer kernels apply to the
        /// numerators alone.
        integer: bool,
        /// Every lane is a rational: no lane is one of the three points over
        /// zero, whose pair laws are the `Fraction`'s own.
        finite: bool,
    },
    Splat(i64, i64),
}

impl Lanes<'_> {
    pub(super) fn of(value: &Value) -> Option<Lanes<'_>> {
        match &value.data {
            ValueData::Tensor { data, shape } if shape.len() == 1 && !data.is_empty() => {
                Some(Lanes::Columns {
                    tensor: data,
                    integer: data.is_pure_integer,
                    finite: data.all_finite(),
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
            Lanes::Columns { tensor, .. } => Some(tensor.len()),
            Lanes::Splat(..) => None,
        }
    }

    pub(super) fn integer(self) -> bool {
        match self {
            Lanes::Columns { integer, .. } => integer,
            Lanes::Splat(_, d) => d == 1,
        }
    }

    /// Whether every lane is a rational.
    pub(super) fn finite(self) -> bool {
        match self {
            Lanes::Columns { finite, .. } => finite,
            Lanes::Splat(_, d) => d != 0,
        }
    }

    #[inline(always)]
    pub(super) fn at(self, i: usize) -> (i64, i64) {
        match self {
            Lanes::Columns { tensor, .. } => (tensor.numerators[i], tensor.denominators[i]),
            Lanes::Splat(n, d) => (n, d),
        }
    }

    #[inline(always)]
    pub(super) fn num(self, i: usize) -> i64 {
        match self {
            Lanes::Columns { tensor, .. } => tensor.numerators[i],
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

/// A kernel's result columns.
pub(super) struct Out {
    pub(super) nums: Column,
    pub(super) dens: Column,
    integer: bool,
}

impl Out {
    pub(super) fn with_capacity(n: usize) -> Self {
        Self {
            nums: Column::with_capacity(n),
            dens: Column::with_capacity(n),
            integer: true,
        }
    }

    /// Integer lanes already computed.
    pub(super) fn integers(nums: Column) -> Self {
        let n = nums.len();
        Self {
            nums,
            dens: smallvec::smallvec![1; n],
            integer: true,
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
