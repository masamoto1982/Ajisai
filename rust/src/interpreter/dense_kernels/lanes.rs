//! The operands and the result of a column kernel: an operand's lanes read
//! straight off a Tensor's columns or splatted from a scalar, and the result
//! columns with the absent lanes the two absence laws leave in them. A child
//! module of `dense_kernels` so the kernels themselves read as the laws they
//! compute.

use crate::interpreter::arithmetic_meter::division_by_zero_absence;
use crate::semantic::AbsenceMetadata;
use crate::types::Column;
use crate::types::{DenseTensor, Value, ValueData};
use std::collections::BTreeMap;
use std::sync::Arc;

/// An operand's lanes: a Tensor's columns, or one rational for every lane.
#[derive(Clone, Copy)]
pub(super) enum Lanes<'a> {
    Columns {
        tensor: &'a DenseTensor,
        /// Every present lane is an integer, so the integer kernels apply to
        /// the numerators alone.
        integer: bool,
        /// Some lane is absent: a denominator of 0, the dividend it holds in
        /// the numerator, and its reason in the Tensor's absence map.
        absent: bool,
    },
    Splat(i64, i64),
}

impl Lanes<'_> {
    pub(super) fn of(value: &Value) -> Option<Lanes<'_>> {
        match &value.data {
            ValueData::Tensor { data, shape } if shape.len() == 1 && !data.is_empty() => {
                // A pure-integer Tensor has no absent lane — every
                // constructor reads an absent lane as not an integer — so
                // the sentinel scan is spent only where it can find one.
                let absent = !data.is_pure_integer && !data.all_lanes_valid();
                let integer = data.is_pure_integer
                    || (absent && data.denominators.iter().all(|&d| d == 1 || d == 0));
                Some(Lanes::Columns {
                    tensor: data,
                    integer,
                    absent,
                })
            }
            ValueData::Scalar(f) if value.absence.is_none() && !f.is_nil() => {
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

    pub(super) fn absent(self) -> bool {
        match self {
            Lanes::Columns { absent, .. } => absent,
            Lanes::Splat(..) => false,
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

    /// The denominator of lane `i`: 0 exactly when the lane is absent.
    #[inline(always)]
    pub(super) fn den(self, i: usize) -> i64 {
        match self {
            Lanes::Columns { tensor, .. } => tensor.denominators[i],
            Lanes::Splat(_, d) => d,
        }
    }

    /// Why lane `i` is absent, as the general route reads it off a lane
    /// (`Value::from_dense_lane`): the map's entry, or a reasonless absence
    /// for a lane the Tensor was never told a reason for.
    pub(super) fn absence(self, i: usize) -> AbsenceMetadata {
        match self {
            Lanes::Columns { tensor, .. } => tensor
                .absence_at(i)
                .cloned()
                .unwrap_or_else(AbsenceMetadata::with_reasonless_unknown),
            Lanes::Splat(..) => AbsenceMetadata::with_reasonless_unknown(),
        }
    }

    /// Absent lane `i` as the `Value` the general route materializes: the
    /// pair it holds and its reason, carried whole.
    pub(super) fn absent_value(self, i: usize) -> Value {
        match self {
            Lanes::Columns { tensor, .. } => Value::from_dense_lane(tensor, i),
            Lanes::Splat(..) => Value::nil_with_absence(self.absence(i)),
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

/// A kernel's result columns, with the absent lanes and the reason for each.
pub(super) struct Out {
    pub(super) nums: Column,
    pub(super) dens: Column,
    integer: bool,
    pub(super) absences: BTreeMap<usize, AbsenceMetadata>,
}

impl Out {
    pub(super) fn with_capacity(n: usize) -> Self {
        Self {
            nums: Column::with_capacity(n),
            dens: Column::with_capacity(n),
            integer: true,
            absences: BTreeMap::new(),
        }
    }

    /// Integer lanes already computed, every lane present until
    /// `carry_absent_lanes` says otherwise.
    pub(super) fn integers(nums: Column) -> Self {
        let n = nums.len();
        Self {
            nums,
            dens: smallvec::smallvec![1; n],
            integer: true,
            absences: BTreeMap::new(),
        }
    }

    #[inline(always)]
    pub(super) fn push(&mut self, num: i64, den: i64) {
        self.nums.push(num);
        self.dens.push(den);
        self.integer &= den == 1;
    }

    /// Lane `index` is absent, divided by zero: `dividend` over the zero in
    /// the columns (`Fraction::over_zero`, lane for lane), the reason in the
    /// map. An absent lane is not an integer lane, so the Tensor is not
    /// pure-integer — the same reading `from_vector_promoted` gives the
    /// general route's result.
    pub(super) fn project(&mut self, index: usize, dividend: i64) {
        self.nums.push(dividend);
        self.dens.push(0);
        self.integer = false;
        self.absences.insert(index, division_by_zero_absence());
    }

    /// Lane `index` is absent because `from`'s lane is: that lane's pair in
    /// the columns, and `from`'s reason carried over, not minted again.
    pub(super) fn carry(&mut self, index: usize, from: Lanes) {
        self.nums.push(from.num(index));
        self.dens.push(0);
        self.integer = false;
        self.absences.insert(index, from.absence(index));
    }

    pub(super) fn into_value(self) -> Value {
        let shape = vec![self.nums.len()];
        let tensor = DenseTensor::from_columns(
            self.nums,
            self.dens,
            shape.clone(),
            self.integer,
            self.absences,
        );
        Value::new(
            ValueData::Tensor {
                data: Arc::new(tensor),
                shape: Arc::new(shape),
            },
            None,
        )
    }
}

/// The passthrough law over lanes already computed as if every lane were
/// present: each absent operand lane becomes an absent result lane carrying
/// that operand's pair and reason, `a`'s lanes before `b`'s so the leftmost
/// absent operand wins (LANG.FAILURE.PASSTHROUGH). A denominator scan per
/// absent operand, and one map entry per absent lane.
pub(super) fn carry_absent_lanes(out: &mut Out, a: Lanes, b: Lanes) {
    for operand in [a, b] {
        let Lanes::Columns {
            tensor,
            absent: true,
            ..
        } = operand
        else {
            continue;
        };
        for (index, &den) in tensor.denominators.iter().enumerate() {
            if den == 0 && out.dens[index] != 0 {
                out.nums[index] = tensor.numerators[index];
                out.dens[index] = 0;
                out.integer = false;
                out.absences.insert(index, operand.absence(index));
            }
        }
    }
}
