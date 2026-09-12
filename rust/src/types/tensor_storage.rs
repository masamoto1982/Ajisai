//! Dense and sparse numeric tensor storage.
//!
//! Invariant: storage validity, shape, and density are representation concerns;
//! semantic interpretation remains owned by `Value` — with one exception this
//! module states rather than hides. A lane can be *absent*, and under
//! LANG.VALUES.NIL the reason for an absence is its entire observable
//! content, so a store that records absence without the reason for it does
//! not store the value it was given. [`DenseTensor::absences`] is where that
//! reason lives.

use std::collections::BTreeMap;

use super::fraction::Fraction;
use crate::error::NilReason;
use crate::semantic::AbsenceMetadata;

/// A dense numeric tensor in struct-of-arrays form.
///
/// *Whether* a lane is absent is recorded once, as the denominator-0 sentinel
/// [`Fraction::nil`] stores. There used to be a second record beside it — a
/// `valid_mask` bitmap with one bit per lane — and the two were never
/// cross-checked: every production write path stored the sentinel and left the
/// mask fully set, while every read trusted the mask alone and handed the 0
/// denominator straight to `Fraction::new`, which panicked. Nothing ever
/// cleared a mask bit outside the tests. One fact, one place.
///
/// *Why* a lane is absent is a different fact, and `absences` is its one
/// place. It is not a second account of presence: it is keyed by lane index,
/// it is read only through [`Self::absence_at`], which screens through
/// [`Self::is_valid`] first, and a constructor drops any entry for a lane the
/// sentinel calls present. So the `valid_mask` failure mode — two records of
/// the same fact, drifting apart — cannot recur here; there is no second
/// record of that fact to drift.
#[derive(Debug, Clone, Eq)]
pub struct DenseTensor {
    pub numerators: Vec<i64>,
    pub denominators: Vec<i64>,
    pub shape: Vec<usize>,
    pub is_pure_integer: bool,
    /// Why each absent lane is absent, keyed by lane index.
    ///
    /// Sparse deliberately: only absent lanes appear, so the overwhelmingly
    /// common all-present tensor carries an empty map and pays nothing for a
    /// facility it does not use. An absent lane is a failure, and failures are
    /// rare in a buffer that may hold a million numbers.
    ///
    /// Private, because the screening in [`Self::absence_at`] is the whole
    /// guarantee: a caller reading this field directly could observe a reason
    /// for a lane that holds a number.
    absences: BTreeMap<usize, AbsenceMetadata>,
}

/// Lanes, shape, and — for each absent lane — its *reason*, and nothing more
/// of its provenance.
///
/// [`AbsenceMetadata`] also carries an origin, a recoverability and a debug
/// diagnosis. Comparing those would make two dense tensors disagree where the
/// same two values held as nested `Vector`s agree, because `Value`'s own
/// equality is `data == data && nil_reason == nil_reason`: LANG.VALUES.NIL
/// makes the reason the observable content of an absence and leaves the rest
/// as provenance, and the persistence codec says the same in its own words.
/// One value identity, whichever representation holds it.
impl PartialEq for DenseTensor {
    fn eq(&self, other: &Self) -> bool {
        if self.numerators != other.numerators
            || self.denominators != other.denominators
            || self.shape != other.shape
            || self.is_pure_integer != other.is_pure_integer
        {
            return false;
        }
        // The denominators matched, so the two agree on *which* lanes are
        // absent; only the reasons are left to compare, and only there.
        (0..self.len())
            .filter(|index| !self.is_valid(*index))
            .all(|index| self.lane_reason(index) == other.lane_reason(index))
    }
}

impl DenseTensor {
    /// The reason lane `index` is absent, or `None` when it holds a number
    /// *or* is absent for a reason this tensor was never told. The identity of
    /// an absence, as [`PartialEq`] and the `Value` hash both read it.
    pub fn lane_reason(&self, index: usize) -> Option<NilReason> {
        self.absence_at(index).and_then(|metadata| metadata.reason)
    }

    /// Assemble a tensor from already-separated columns.
    ///
    /// The single place a `DenseTensor` is built field-by-field, so the
    /// absence map can be reconciled against the sentinel in one place:
    /// an entry naming a lane that holds a number is dropped, because the
    /// sentinel is the authority on presence and a reason for a present lane
    /// is not a fact about this tensor.
    pub fn from_columns(
        numerators: Vec<i64>,
        denominators: Vec<i64>,
        shape: Vec<usize>,
        is_pure_integer: bool,
        absences: BTreeMap<usize, AbsenceMetadata>,
    ) -> Self {
        let absences = absences
            .into_iter()
            .filter(|(index, _)| matches!(denominators.get(*index), Some(0)))
            .collect();
        Self {
            numerators,
            denominators,
            shape,
            is_pure_integer,
            absences,
        }
    }

    /// Build from rationals alone, which carry absence but no reason for it.
    ///
    /// Every lane this makes absent is therefore *reasonless* — see
    /// [`Self::absence_at`] for what that means when the lane is read back.
    /// Prefer [`Self::from_lane_values`] wherever the caller holds `Value`
    /// lanes, which do carry the reason.
    pub fn from_fractions(data: Vec<Fraction>, shape: Vec<usize>) -> Option<Self> {
        Self::from_fractions_with_absences(data, shape, BTreeMap::new())
    }

    /// [`Self::from_fractions`] plus the reason for each absent lane.
    pub fn from_fractions_with_absences(
        data: Vec<Fraction>,
        shape: Vec<usize>,
        absences: BTreeMap<usize, AbsenceMetadata>,
    ) -> Option<Self> {
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

        Some(Self::from_columns(
            numerators,
            denominators,
            shape,
            is_pure_integer,
            absences,
        ))
    }

    /// The reason lane `index` is absent, or `None` when it holds a number.
    ///
    /// Screened through [`Self::is_valid`] first, so a present lane never
    /// reports a reason however the map was built. `Some(metadata)` whose
    /// `reason` is itself `None` is the honest answer for a lane that was
    /// made absent by a rational alone: absent, for a reason this tensor was
    /// never told.
    pub fn absence_at(&self, index: usize) -> Option<&AbsenceMetadata> {
        if self.is_valid(index) {
            return None;
        }
        self.absences.get(&index)
    }

    /// Every recorded absence, in lane order — for the boundaries that must
    /// carry the whole tensor across (persistence, the value arena).
    pub fn absences(&self) -> impl Iterator<Item = (usize, &AbsenceMetadata)> {
        self.absences
            .iter()
            .filter(|(index, _)| !self.is_valid(**index))
            .map(|(index, metadata)| (*index, metadata))
    }

    /// This flat buffer with its lanes in reverse order, columns and all.
    ///
    /// Rearranging lanes is a representation concern, so it happens here rather
    /// than by unpacking the tensor into boxed `Value`s, reversing those, and
    /// re-densifying: two columns of `i64` reverse in place, and the only fact
    /// that has to move with them is *why* an absent lane is absent — lane
    /// `index` becomes lane `len - 1 - index`, which is the whole of the
    /// remapping. Whether a lane is absent travels with the denominator
    /// sentinel, as it does everywhere else, so there is no second record to
    /// keep in step.
    ///
    /// Flat buffers only: the caller checks rank, because reversing a rank-2
    /// tensor reverses its *rows*, and a row is a stride rather than a lane.
    pub fn reversed_lanes(&self) -> Self {
        let len = self.len();
        let mut numerators = self.numerators.clone();
        numerators.reverse();
        let mut denominators = self.denominators.clone();
        denominators.reverse();
        let absences = self
            .absences()
            .map(|(index, metadata)| (len - 1 - index, metadata.clone()))
            .collect();
        Self::from_columns(
            numerators,
            denominators,
            self.shape.clone(),
            self.is_pure_integer,
            absences,
        )
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
            absences: BTreeMap::new(),
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
        // A sparse tensor holds no absent lanes (`from_dense` refuses one), so
        // there is no absence to restore here — every unstored lane is zero.
        DenseTensor::from_columns(
            numerators,
            denominators,
            self.shape.clone(),
            self.is_pure_integer,
            BTreeMap::new(),
        )
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
