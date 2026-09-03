//! Dense and sparse numeric tensor storage.
//!
//! Invariant: storage validity, shape, and density are representation concerns;
//! semantic interpretation remains owned by `Value`.

use super::fraction::Fraction;

/// A dense numeric tensor in struct-of-arrays form.
///
/// Absence is recorded once, as the denominator-0 sentinel [`Fraction::nil`]
/// stores. There used to be a second record beside it — a `valid_mask` bitmap
/// with one bit per lane — and the two were never cross-checked: every
/// production write path stored the sentinel and left the mask fully set,
/// while every read trusted the mask alone and handed the 0 denominator
/// straight to `Fraction::new`, which panicked. Nothing ever cleared a mask
/// bit outside the tests. One fact, one place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenseTensor {
    pub numerators: Vec<i64>,
    pub denominators: Vec<i64>,
    pub shape: Vec<usize>,
    pub is_pure_integer: bool,
}

impl DenseTensor {
    pub fn from_fractions(data: Vec<Fraction>, shape: Vec<usize>) -> Option<Self> {
        let expected_len = if shape.is_empty() {
            0
        } else {
            shape.iter().product()
        };
        if expected_len != data.len() {
            return None;
        }

        let mut numerators = Vec::with_capacity(data.len());
        let mut denominators = Vec::with_capacity(data.len());
        let mut is_pure_integer = true;
        for fraction in data {
            let (numerator, denominator) = fraction.extract_i64_pair()?;
            numerators.push(numerator);
            denominators.push(denominator);
            is_pure_integer &= denominator == 1;
        }

        Some(Self {
            numerators,
            denominators,
            shape,
            is_pure_integer,
        })
    }

    /// Build a 1-D pure-integer dense tensor directly from `i64` numerators,
    /// without routing through `Fraction`. Every lane is valid and the
    /// denominator is implicitly `1`. This is the SoA fast-path constructor
    /// the integer SIMD lane uses for its output, avoiding the
    /// `Vec<i64> → Vec<Fraction> → re-densify` round-trip (handoff 手1).
    pub fn from_integers(numerators: Vec<i64>) -> Self {
        let len = numerators.len();
        let denominators = vec![1; len];
        Self {
            numerators,
            denominators,
            shape: vec![len],
            is_pure_integer: true,
        }
    }

    pub fn len(&self) -> usize {
        self.numerators.len()
    }

    pub fn is_empty(&self) -> bool {
        self.numerators.is_empty()
    }

    /// `true` when every lane holds a present value — i.e. there are no `nil`
    /// holes. One linear scan for the absence sentinel; the integer SIMD fast
    /// path uses it to confirm density before borrowing the buffers.
    pub fn all_lanes_valid(&self) -> bool {
        !self.denominators.contains(&0)
    }

    pub fn iter(&self) -> impl Iterator<Item = Fraction> + '_ {
        (0..self.len()).map(|index| self.fraction_or_nil(index))
    }

    /// The present value at `index`, or `None` when the lane is absent.
    ///
    /// Never hands a 0 denominator to [`Fraction::new`]: an absent lane is
    /// screened out by [`Self::is_valid`] first. `Fraction::new` keeps its
    /// panic on purpose — constructing a 0-denominator rational anywhere else
    /// is a bug, and silencing it here would turn an absent lane into a
    /// plausible-looking number instead.
    pub fn get_small_fraction(&self, index: usize) -> Option<Fraction> {
        if !self.is_valid(index) {
            return None;
        }
        Some(Fraction::new(
            self.numerators[index].into(),
            self.denominators[index].into(),
        ))
    }

    pub fn fraction_or_nil(&self, index: usize) -> Fraction {
        self.get_small_fraction(index).unwrap_or_else(Fraction::nil)
    }

    pub fn to_fractions(&self) -> Vec<Fraction> {
        self.iter().collect()
    }

    /// `true` when lane `index` holds a present value.
    ///
    /// A denominator of 0 is [`Fraction::nil`] — the absence sentinel every
    /// write path stores — not a rational, so the lane is absent.
    pub fn is_valid(&self, index: usize) -> bool {
        matches!(self.denominators.get(index), Some(denominator) if *denominator != 0)
    }

    pub fn zero_count(&self) -> usize {
        (0..self.len())
            .filter(|&index| self.is_valid(index) && self.numerators[index] == 0)
            .count()
    }

    pub fn nonzero_count(&self) -> usize {
        (0..self.len())
            .filter(|&index| self.is_valid(index) && self.numerators[index] != 0)
            .count()
    }

    pub fn density(&self) -> f64 {
        if self.is_empty() {
            return 0.0;
        }
        self.nonzero_count() as f64 / self.len() as f64
    }

    pub fn is_sparse_candidate(&self) -> bool {
        const MIN_LEN: usize = 64;
        const MAX_DENSITY: f64 = 0.25;

        self.len() >= MIN_LEN && self.density() <= MAX_DENSITY
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// The sparse form of a dense tensor: only the non-zero lanes are stored, and
/// every unstored lane is the number zero.
///
/// It carries no absence record at all, and needs none: [`Self::from_dense`]
/// refuses a tensor with any absent lane, so "not stored" here means zero and
/// never means NIL. Conflating the two is what the dropped `valid_mask` made
/// possible — a NIL lane has numerator 0, so it looked exactly like a zero to
/// the densifier.
pub struct SparseTensor {
    pub indices: Vec<usize>,
    pub numerators: Vec<i64>,
    pub denominators: Vec<i64>,
    pub shape: Vec<usize>,
    pub len: usize,
    pub is_pure_integer: bool,
}

impl SparseTensor {
    pub fn from_dense(dense: &DenseTensor) -> Option<Self> {
        let expected_len = if dense.shape.is_empty() {
            dense.len()
        } else {
            dense.shape.iter().product()
        };
        if expected_len != dense.len() {
            return None;
        }
        if (0..dense.len()).any(|index| !dense.is_valid(index)) {
            return None;
        }

        let nonzero_count = dense.nonzero_count();
        let mut indices = Vec::with_capacity(nonzero_count);
        let mut numerators = Vec::with_capacity(nonzero_count);
        let mut denominators = Vec::with_capacity(nonzero_count);

        for index in 0..dense.len() {
            if dense.numerators[index] != 0 {
                indices.push(index);
                numerators.push(dense.numerators[index]);
                denominators.push(dense.denominators[index]);
            }
        }

        Some(Self {
            indices,
            numerators,
            denominators,
            shape: dense.shape.clone(),
            len: dense.len(),
            is_pure_integer: dense.is_pure_integer,
        })
    }

    pub fn to_dense(&self) -> DenseTensor {
        let mut numerators = vec![0; self.len];
        let mut denominators = vec![1; self.len];
        for (entry, &index) in self.indices.iter().enumerate() {
            if index < self.len {
                numerators[index] = self.numerators[entry];
                denominators[index] = self.denominators[entry];
            }
        }
        DenseTensor {
            numerators,
            denominators,
            shape: self.shape.clone(),
            is_pure_integer: self.is_pure_integer,
        }
    }

    pub fn get_small_fraction(&self, index: usize) -> Option<Fraction> {
        if index >= self.len || !self.is_valid(index) {
            return None;
        }
        let entry = self.indices.binary_search(&index).ok()?;
        // `from_dense` refuses a tensor with an absent lane, so no stored entry
        // can carry the 0-denominator sentinel. Screened anyway: this is the
        // one call that could hand a 0 denominator to `Fraction::new`, and its
        // panic is deliberate.
        if self.denominators[entry] == 0 {
            return None;
        }
        Some(Fraction::new(
            self.numerators[entry].into(),
            self.denominators[entry].into(),
        ))
    }

    pub fn fraction_or_zero(&self, index: usize) -> Fraction {
        self.get_small_fraction(index)
            .unwrap_or_else(|| Fraction::new(0.into(), 1.into()))
    }

    pub fn nonzero_count(&self) -> usize {
        self.indices.len()
    }

    pub fn density(&self) -> f64 {
        if self.len == 0 {
            return 0.0;
        }
        self.nonzero_count() as f64 / self.len as f64
    }

    /// `true` for every lane in range: a sparse tensor holds no absent lanes
    /// (see the type's own note), so being addressable is being present.
    pub fn is_valid(&self, index: usize) -> bool {
        index < self.len
    }
}

#[cfg(test)]
mod sparse_tensor_tests {
    use super::{DenseTensor, SparseTensor};
    use crate::types::fraction::Fraction;

    fn dense_from_i64(values: &[i64], shape: Vec<usize>) -> DenseTensor {
        DenseTensor::from_fractions(values.iter().copied().map(Fraction::from).collect(), shape)
            .expect("small dense tensor should build")
    }

    #[test]
    fn dense_tensor_sparse_density_counts_zero_and_nonzero_lanes() {
        let all_zero = dense_from_i64(&vec![0; 64], vec![64]);
        assert_eq!(all_zero.zero_count(), 64);
        assert_eq!(all_zero.nonzero_count(), 0);
        assert_eq!(all_zero.density(), 0.0);
        assert!(all_zero.is_sparse_candidate());

        let all_nonzero = dense_from_i64(&vec![1; 64], vec![64]);
        assert_eq!(all_nonzero.zero_count(), 0);
        assert_eq!(all_nonzero.nonzero_count(), 64);
        assert_eq!(all_nonzero.density(), 1.0);
        assert!(!all_nonzero.is_sparse_candidate());

        let mixed = dense_from_i64(&[0, 7, 0, -3], vec![4]);
        assert_eq!(mixed.zero_count(), 2);
        assert_eq!(mixed.nonzero_count(), 2);
        assert_eq!(mixed.density(), 0.5);
        assert!(!mixed.is_sparse_candidate());
    }

    #[test]
    fn dense_tensor_sparse_density_does_not_count_absent_lanes_as_zero() {
        // An absent lane has numerator 0, exactly like a zero lane, so it is
        // the denominator that tells them apart. A density that counted the
        // two alike would offer a NIL-holding tensor to the sparse form, which
        // stores no absence and would silently read those lanes back as 0.
        let dense = DenseTensor::from_fractions(
            vec![
                Fraction::nil(),
                Fraction::nil(),
                Fraction::from(0_i64),
                Fraction::from(9_i64),
            ],
            vec![4],
        )
        .expect("small fractions admit dense representation");
        assert!(!dense.is_valid(0));
        assert!(dense.is_valid(2), "a zero lane is present, not absent");
        assert_eq!(dense.zero_count(), 1);
        assert_eq!(dense.nonzero_count(), 1);
        assert_eq!(dense.density(), 0.25);
        assert!(SparseTensor::from_dense(&dense).is_none());
    }

    #[test]
    fn sparse_tensor_round_trips_dense_values_and_shape() {
        let dense = dense_from_i64(&[0, 0, 3, 0, -4, 0], vec![2, 3]);
        let sparse =
            SparseTensor::from_dense(&dense).expect("all-valid dense tensor is sparseable");
        assert_eq!(sparse.shape, vec![2, 3]);
        assert_eq!(sparse.len, 6);
        assert_eq!(sparse.indices, vec![2, 4]);
        assert_eq!(sparse.nonzero_count(), 2);
        assert!(sparse.indices.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(sparse.fraction_or_zero(0), Fraction::from(0_i64));
        assert_eq!(sparse.get_small_fraction(2), Some(Fraction::from(3_i64)));
        assert_eq!(sparse.to_dense(), dense);
    }

    #[test]
    fn sparse_tensor_accepts_all_zero_dense_tensor() {
        let dense = dense_from_i64(&vec![0; 64], vec![8, 8]);
        let sparse =
            SparseTensor::from_dense(&dense).expect("all-zero all-valid tensor is sparseable");
        assert!(sparse.indices.is_empty());
        assert_eq!(sparse.nonzero_count(), 0);
        assert_eq!(sparse.density(), 0.0);
        assert_eq!(sparse.to_dense(), dense);
    }
}
