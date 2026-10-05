//! A run of a dense tensor's lanes as a tensor of its own: a row of a
//! higher-rank tensor, copied from the columns directly. A child module of
//! `tensor_storage` so it reads the absence map as that module does.

use super::{Column, DenseTensor};

impl DenseTensor {
    /// Lanes `start..start + len` as a tensor of `shape` of their own — a row
    /// of a higher-rank tensor, re-indexed from its first lane.
    ///
    /// The same tensor `from_fractions_with_absences` builds from those lanes
    /// read one by one (`fraction_or_nil`): the pairs as stored, an absent
    /// lane as the `(0, 0)` its NIL reads as, purity recomputed over the
    /// slice, and the reasons of the absent lanes inside it moved with them.
    pub fn lanes(&self, start: usize, len: usize, shape: Vec<usize>) -> Self {
        let end = start + len;
        let mut numerators = Column::with_capacity(len);
        let mut denominators = Column::with_capacity(len);
        let mut is_pure_integer = true;
        for (&n, &d) in self.numerators[start..end]
            .iter()
            .zip(&self.denominators[start..end])
        {
            numerators.push(if d == 0 { 0 } else { n });
            denominators.push(d);
            is_pure_integer &= d == 1;
        }
        let absences = self
            .absences
            .range(start..end)
            .filter(|(index, _)| !self.is_valid(**index))
            .map(|(index, metadata)| (index - start, metadata.clone()))
            .collect();
        Self::from_columns(numerators, denominators, shape, is_pure_integer, absences)
    }
}
