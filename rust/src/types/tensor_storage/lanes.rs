//! A run of a dense tensor's lanes as a tensor of its own: a row of a
//! higher-rank tensor, copied from the columns directly.

use super::{Column, DenseTensor};

impl DenseTensor {
    /// Lanes `start..start + len` as a tensor of `shape` of their own — a row
    /// of a higher-rank tensor, re-indexed from its first lane.
    ///
    /// The same tensor `from_fractions` builds from those lanes read one by
    /// one: the pairs as stored, purity recomputed over the slice.
    pub fn lanes(&self, start: usize, len: usize, shape: &[usize]) -> Self {
        let end = start + len;
        let mut numerators = Column::with_capacity(len);
        let mut denominators = Column::with_capacity(len);
        let mut is_pure_integer = true;
        for (&n, &d) in self.numerators[start..end]
            .iter()
            .zip(&self.denominators[start..end])
        {
            numerators.push(n);
            denominators.push(d);
            is_pure_integer &= d == 1;
        }
        Self::from_columns(numerators, denominators, shape, is_pure_integer)
    }

    /// `self`'s lanes followed by `other`'s, as a tensor of `shape` — `CONCAT`
    /// of two dense operands whose rows agree in shape.
    ///
    /// The columns are appended as they are stored, and purity holds when it
    /// held for both.
    pub fn concatenated(&self, other: &Self, shape: Vec<usize>) -> Self {
        let offset = self.len();
        let mut numerators = Column::with_capacity(offset + other.len());
        numerators.extend_from_slice(&self.numerators);
        numerators.extend_from_slice(&other.numerators);
        let mut denominators = Column::with_capacity(offset + other.len());
        denominators.extend_from_slice(&self.denominators);
        denominators.extend_from_slice(&other.denominators);
        Self::from_columns(
            numerators,
            denominators,
            shape,
            self.is_pure_integer && other.is_pure_integer,
        )
    }
}
