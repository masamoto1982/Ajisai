//! The unified numeric-observation primitive (SPEC §4.2).
//!
//! Every Ajisai number is an *observation process*: feeding it water (the
//! single refinement budget) narrows a rational enclosure of the value.
//! The three cost tiers share this one interface —
//!
//! - **Tier 0** (rational, `Fraction`): the enclosure is a point and
//!   `refine` settles immediately, consuming no water.
//! - **Tier 1** (algebraic, `types::exact::algebraic`): sign, floor, and
//!   comparison are decidable, so every observation terminates in finite
//!   water; refinement only speeds them up, never gates correctness.
//! - **Tier 2** (general computable reals): a lazily refined shrinking
//!   enclosure. Only this tier may report `Starved`, the sole source of
//!   the logical `Unknown` (U).
//!
//! Which tier a value flows through is never observable (SPEC §4.8): the
//! contract below fixes identity, display, and serialization to the value
//! itself, not its representation.

use crate::types::fraction::Fraction;

/// Refinement budget for one `Observation::refine` call. Water is the
/// single resource behind what used to be separate comparison and display
/// budgets; the execution step budget (SPEC §5.3) stays separate for now
/// but shares the naming so a future unification stays open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Water(pub u64);

impl Water {
    /// The zero budget: `refine(Water::NONE)` may only report an already
    /// known outcome (`Settled`/`Empty`) or `Starved`, never do work.
    pub const NONE: Water = Water(0);
}

/// A closed rational interval `[lo, hi]` known to contain the observed
/// value. A point (`lo == hi`) is an exact value; endpoints are always
/// non-nil fractions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RatInterval {
    pub lo: Fraction,
    pub hi: Fraction,
}

impl RatInterval {
    /// The point interval `[f, f]`. `f` must be non-nil.
    pub fn point(f: Fraction) -> Self {
        RatInterval { lo: f.clone(), hi: f }
    }

    /// `[lo, hi]`, normalizing a reversed pair so the invariant
    /// `lo <= hi` always holds.
    pub fn new(lo: Fraction, hi: Fraction) -> Self {
        if lo.le(&hi) {
            RatInterval { lo, hi }
        } else {
            RatInterval { lo: hi, hi: lo }
        }
    }

    /// Whether the interval is a single point (the value is exactly known).
    pub fn is_point(&self) -> bool {
        self.lo == self.hi
    }

    /// `hi - lo`.
    pub fn width(&self) -> Fraction {
        self.hi.sub(&self.lo)
    }

    /// Whether `self` is contained in `outer` (both closed). This is the
    /// monotonicity relation `refine` must preserve.
    pub fn is_within(&self, outer: &RatInterval) -> bool {
        outer.lo.le(&self.lo) && self.hi.le(&outer.hi)
    }
}

/// Outcome of one `Observation::refine` step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refine {
    /// The exact value was reached; the observation is finished and
    /// consumes no further water. Tier 0 reports this immediately.
    Settled(Fraction),
    /// The enclosure narrowed but the value has no exact rational form
    /// (an irrational); refining again narrows it further.
    Narrower,
    /// The observation process is permanently empty — the observational
    /// counterpart of NIL (e.g. observing a nil fraction).
    Empty,
    /// The given water was not enough to make progress. Only Tier 2
    /// observations may report this; it is the sole source of the logical
    /// `Unknown` (U).
    Starved,
}

/// A numeric value as an observation process (SPEC §4.2). The contract:
///
/// - **Monotone**: after `refine`, `current_interval` is contained in
///   every interval reported before.
/// - **Convergent**: for Tier ≤ 1 values, finitely much water decides
///   sign, floor, and any comparison; only Tier 2 may starve.
/// - **Deterministic**: equal values fed equal total water report equal
///   interval sequences.
/// - **Unobservable representation** (SPEC §4.8): no tier or internal
///   form shows through identity, display, or serialization.
pub trait Observation {
    /// The tightest enclosure currently known, without consuming water.
    /// `None` when the process is empty (the NIL counterpart).
    fn current_interval(&self) -> Option<RatInterval>;

    /// Spend at most `w` water narrowing the enclosure.
    fn refine(&mut self, w: Water) -> Refine;
}

/// Tier 0: a rational is an already-finished observation. The enclosure
/// is the point interval and `refine` settles at once, for any water
/// including none.
impl Observation for Fraction {
    fn current_interval(&self) -> Option<RatInterval> {
        if self.is_nil() {
            return None;
        }
        Some(RatInterval::point(self.clone()))
    }

    fn refine(&mut self, _w: Water) -> Refine {
        if self.is_nil() {
            return Refine::Empty;
        }
        Refine::Settled(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;

    fn frac(n: i64, d: i64) -> Fraction {
        Fraction::new(BigInt::from(n), BigInt::from(d))
    }

    #[test]
    fn tier0_settles_immediately_without_water() {
        let mut f = frac(3, 7);
        assert_eq!(f.refine(Water::NONE), Refine::Settled(frac(3, 7)));
        assert_eq!(f.refine(Water(1_000)), Refine::Settled(frac(3, 7)));
    }

    #[test]
    fn tier0_interval_is_a_point() {
        let f = frac(-5, 2);
        let iv = f.current_interval().expect("non-nil has an enclosure");
        assert!(iv.is_point());
        assert_eq!(iv.lo, frac(-5, 2));
        assert!(iv.width().is_zero());
    }

    #[test]
    fn nil_fraction_observes_as_empty() {
        let mut nil = Fraction::nil();
        assert_eq!(nil.current_interval(), None);
        assert_eq!(nil.refine(Water(8)), Refine::Empty);
    }

    #[test]
    fn tier0_refinement_is_monotone_and_deterministic() {
        let mut f = frac(22, 7);
        let before = f.current_interval().unwrap();
        f.refine(Water(4));
        let after = f.current_interval().unwrap();
        assert!(after.is_within(&before));
        assert_eq!(before, after);
    }

    #[test]
    fn interval_normalizes_reversed_endpoints() {
        let iv = RatInterval::new(frac(2, 1), frac(1, 1));
        assert!(iv.lo.le(&iv.hi));
        assert_eq!(iv.width(), frac(1, 1));
    }

    #[test]
    fn interval_containment_matches_endpoint_order() {
        let outer = RatInterval::new(frac(0, 1), frac(4, 1));
        let inner = RatInterval::new(frac(1, 1), frac(3, 1));
        assert!(inner.is_within(&outer));
        assert!(!outer.is_within(&inner));
        assert!(inner.is_within(&inner));
    }
}
