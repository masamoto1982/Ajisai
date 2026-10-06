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

impl DenseTensor {
    /// `self`'s lanes followed by `other`'s, as a tensor of `shape` — `CONCAT`
    /// of two dense operands whose rows agree in shape.
    ///
    /// The columns are appended as they are stored; purity holds when it held
    /// for both; and the reasons of `other`'s absent lanes move up by
    /// `self.len()`, which is the whole of the re-indexing, the reverse of what
    /// [`Self::lanes`] does to a slice.
    pub fn concatenated(&self, other: &Self, shape: Vec<usize>) -> Self {
        let offset = self.len();
        let mut numerators = Column::with_capacity(offset + other.len());
        numerators.extend_from_slice(&self.numerators);
        numerators.extend_from_slice(&other.numerators);
        let mut denominators = Column::with_capacity(offset + other.len());
        denominators.extend_from_slice(&self.denominators);
        denominators.extend_from_slice(&other.denominators);
        let absences = self
            .absences()
            .map(|(index, metadata)| (index, metadata.clone()))
            .chain(
                other
                    .absences()
                    .map(|(index, metadata)| (index + offset, metadata.clone())),
            )
            .collect();
        Self::from_columns(
            numerators,
            denominators,
            shape,
            self.is_pure_integer && other.is_pure_integer,
            absences,
        )
    }
}
