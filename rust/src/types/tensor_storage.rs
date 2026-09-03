//! Dense and sparse numeric tensor storage.
//!
//! Invariant: storage validity, shape, and density are representation concerns;
//! semantic interpretation remains owned by `Value`.

use super::fraction::Fraction;

#[derive(Debug, Clone, Eq)]
pub struct DenseTensor {
    pub numerators: Vec<i64>,
    pub denominators: Vec<i64>,
    pub valid_mask: Vec<u64>,
    pub shape: Vec<usize>,
    pub is_pure_integer: bool,
}

impl PartialEq for DenseTensor {
    fn eq(&self, other: &Self) -> bool {
        self.numerators == other.numerators
            && self.denominators == other.denominators
            && self.valid_mask == other.valid_mask
            && self.shape == other.shape
            && self.is_pure_integer == other.is_pure_integer
    }
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

        let valid_mask_len = numerators.len().div_ceil(64);
        let mut valid_mask = vec![u64::MAX; valid_mask_len];
        if let Some(last) = valid_mask.last_mut() {
            let live_bits = numerators.len() % 64;
            if live_bits != 0 {
                *last = (1u64 << live_bits) - 1;
            }
        }

        Some(Self {
            numerators,
            denominators,
            valid_mask,
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
        let valid_mask_len = len.div_ceil(64);
        let mut valid_mask = vec![u64::MAX; valid_mask_len];
        if let Some(last) = valid_mask.last_mut() {
            let live_bits = len % 64;
            if live_bits != 0 {
                *last = (1u64 << live_bits) - 1;
            }
        }
        Self {
            numerators,
            denominators,
            valid_mask,
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

    /// `true` when every lane (`0..len`) is valid — i.e. there are no `nil`
    /// holes. Screens the bitmask word-at-a-time first (O(len/64)), then
    /// confirms no lane carries the denominator-0 absence sentinel; see
    /// [`Self::is_valid`] for why both records have to agree.
    pub fn all_lanes_valid(&self) -> bool {
        if self.denominators.iter().any(|denominator| *denominator == 0) {
            return false;
        }
        let len = self.len();
        let full_words = len / 64;
        for word in self.valid_mask.iter().take(full_words) {
            if *word != u64::MAX {
                return false;
            }
        }
        let remainder = len % 64;
        if remainder != 0 {
            let expected = (1u64 << remainder) - 1;
            match self.valid_mask.get(full_words) {
                Some(word) => return *word == expected,
                None => return false,
            }
        }
        true
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

    pub fn clear_valid(&mut self, index: usize) {
        if index < self.len() {
            self.valid_mask[index / 64] &= !(1u64 << (index % 64));
        }
    }

    /// `true` when lane `index` holds a present value.
    ///
    /// Absence is recorded twice in this struct: as the denominator-0 sentinel
    /// that [`Fraction::nil`] stores, and as a cleared `valid_mask` bit. Every
    /// production write path (`ValueData::Nil => buf.push(Fraction::nil())` in
    /// `value_children.rs`) uses the sentinel and leaves the mask fully set, so
    /// a reader that trusts the mask alone reads an absent lane as a present
    /// `n/0`. Both records are consulted here, and a lane counts as present
    /// only when they agree.
    pub fn is_valid(&self, index: usize) -> bool {
        if index >= self.len() {
            return false;
        }
        if self.denominators.get(index).copied() == Some(0) {
            return false;
        }
        let Some(word) = self.valid_mask.get(index / 64) else {
            return false;
        };
        ((word >> (index % 64)) & 1) == 1
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
pub struct SparseTensor {
    pub indices: Vec<usize>,
    pub numerators: Vec<i64>,
    pub denominators: Vec<i64>,
    pub valid_mask: Vec<u64>,
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

        let valid_mask_len = dense.len().div_ceil(64);
        let mut valid_mask = vec![u64::MAX; valid_mask_len];
        if let Some(last) = valid_mask.last_mut() {
            let live_bits = dense.len() % 64;
            if live_bits != 0 {
                *last = (1u64 << live_bits) - 1;
            }
        }

        Some(Self {
            indices,
            numerators,
            denominators,
            valid_mask,
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
            valid_mask: self.valid_mask.clone(),
            shape: self.shape.clone(),
            is_pure_integer: self.is_pure_integer,
        }
    }

    pub fn get_small_fraction(&self, index: usize) -> Option<Fraction> {
        if index >= self.len || !self.is_valid(index) {
            return None;
        }
        let entry = self.indices.binary_search(&index).ok()?;
        // Same sentinel screen as `DenseTensor::get_small_fraction`: a stored
        // entry with a 0 denominator is an absent lane, not a rational.
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

    pub fn is_valid(&self, index: usize) -> bool {
        if index >= self.len {
            return false;
        }
        let Some(word) = self.valid_mask.get(index / 64) else {
            return false;
        };
        ((word >> (index % 64)) & 1) == 1
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
    fn dense_tensor_sparse_density_does_not_count_invalid_lanes_as_zero() {
        let mut dense = dense_from_i64(&[0, 5, 0, 9], vec![4]);
        dense.clear_valid(0);
        dense.clear_valid(1);
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
