//! The exact-real scalar value behind `ValueData::ExactScalar` (SPEC §4.2).
//!
//! An enum over the numeric cost tiers: Tier 0 rationals (`Fraction`,
//! including the nil sentinel) and Tier 1 algebraic numbers. The variant
//! is a cost class, never an observable property (SPEC §4.8): values
//! demote to the cheapest tier that holds them exactly, so an
//! `Algebraic` payload is always irrational. A Tier 2 variant (general
//! computable reals) slots in here when a word that needs it exists.
//!
//! This type keeps the method surface of the retired continued-fraction
//! `ExactReal` so call sites migrate by import swap: arithmetic and
//! rounding have the same signatures, while the budgeted comparisons are
//! replaced by [`ExactReal::cmp_exact`] / [`ExactReal::cmp_within`] —
//! total over Tier ≤ 1, water-consuming only when Tier 2 is involved.

use crate::types::exact::algebraic::{Algebraic, AlgebraicResult};
use crate::types::exact::computable::Computable;
use crate::types::exact::observation::{RatInterval, Water};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::One;
use std::cmp::Ordering;

/// Default comparison water for the bare relations (SPEC §7.4.1). Not
/// observable over Tier ≤ 1 — those comparisons are decidable and spend
/// nothing; it bounds refinement only when a Tier 2 observation is
/// involved.
pub const DEFAULT_COMPARISON_WATER: Water = Water(256);

/// Water cap for the *internal* Tier 2 uses that have no explicit budget
/// word (floor/round, rational approximation at serialization
/// boundaries). A safety valve of the same species as the old display
/// budget, not observable semantics.
pub(crate) const TIER2_INTERNAL_WATER: u64 = 64;

#[derive(Debug, Clone, PartialEq)]
pub enum ExactReal {
    /// Tier 0: an exact rational (the nil fraction doubles as the absent
    /// value, exactly as in `Fraction` itself).
    Rational(Fraction),
    /// Tier 1: an algebraic irrational in multiquadratic normal form.
    /// Invariant: never rational (rational results demote eagerly).
    Algebraic(Algebraic),
    /// Tier 2: a general computable real — a lazily refined shrinking
    /// enclosure. No current vocabulary word constructs this variant;
    /// it is the wired receptacle for future words (π, e, log, …).
    Computable(Computable),
}

/// Outcome of an exact-real comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactCmp {
    /// The order is decided. Always the case over Tier ≤ 1.
    Decided(Ordering),
    /// A Tier 2 observation spent `steps` refinement steps without
    /// separating the operands. Projects to the logical `Unknown` (U)
    /// with `diagnosis.agreedPrefix = steps`.
    Starved { steps: usize },
    /// An operand is the absent value (nil); absence has no order.
    Absent,
}

impl ExactReal {
    #[inline]
    pub fn from_fraction(f: Fraction) -> Self {
        Self::Rational(f)
    }

    #[inline]
    pub fn from_integer(n: i64) -> Self {
        Self::Rational(Fraction::new(BigInt::from(n), BigInt::one()))
    }

    #[inline]
    pub fn from_bigint(n: BigInt) -> Self {
        Self::Rational(Fraction::new(n, BigInt::one()))
    }

    fn from_result(result: AlgebraicResult) -> Self {
        match result {
            AlgebraicResult::Rational(f) => Self::Rational(f),
            AlgebraicResult::Irrational(a) => Self::Algebraic(a),
        }
    }

    /// √`radicand` as an exact real. Returns `None` for nil or negative
    /// input; perfect squares and zero demote to `Rational` (same
    /// normalization as the retired CF constructor).
    pub fn from_sqrt_rational(radicand: Fraction) -> Option<Self> {
        Algebraic::sqrt_of_fraction(&radicand).map(Self::from_result)
    }

    #[inline]
    pub fn as_rational(&self) -> Option<&Fraction> {
        match self {
            Self::Rational(f) => Some(f),
            Self::Algebraic(_) | Self::Computable(_) => None,
        }
    }

    #[inline]
    pub fn to_fraction(&self) -> Option<Fraction> {
        self.as_rational().cloned()
    }

    #[inline]
    pub fn is_rational(&self) -> bool {
        matches!(self, Self::Rational(_))
    }

    #[inline]
    pub fn is_nil(&self) -> bool {
        match self {
            Self::Rational(f) => f.is_nil(),
            Self::Algebraic(_) | Self::Computable(_) => false,
        }
    }

    #[inline]
    pub fn is_integer(&self) -> bool {
        match self {
            Self::Rational(f) => f.is_integer(),
            Self::Algebraic(_) | Self::Computable(_) => false,
        }
    }

    /// Whether the value is *known* to be exactly zero. Total and never
    /// wrongly `true`: an `Algebraic` is never zero by the normal-form
    /// invariant, and a Tier 2 process cannot prove zero-ness, so both
    /// conservatively answer `false`.
    #[inline]
    pub fn is_structurally_zero(&self) -> bool {
        match self {
            Self::Rational(f) => f.is_zero(),
            Self::Algebraic(_) | Self::Computable(_) => false,
        }
    }

    // ---- Arithmetic (field operations, nil-propagating) ----

    /// Negation. Preserves nil. A Tier 2 operand negates its enclosure
    /// generator (interval negation preserves nesting).
    pub fn neg(&self) -> Self {
        match self {
            Self::Rational(f) => {
                if f.is_nil() {
                    return Self::Rational(Fraction::nil());
                }
                Self::Rational(Fraction::new(-f.numerator(), f.denominator()))
            }
            Self::Algebraic(a) => Self::Algebraic(a.neg()),
            Self::Computable(c) => {
                let source = c.clone();
                Self::Computable(Computable::from_enclosures("neg", move |step| {
                    let iv = source.enclosure_at(step);
                    RatInterval::new(
                        Fraction::new(-iv.hi.numerator(), iv.hi.denominator()),
                        Fraction::new(-iv.lo.numerator(), iv.lo.denominator()),
                    )
                }))
            }
        }
    }

    /// Size probe (CS5): the number of algebraic terms this value carries — a
    /// Tier 1 `Algebraic`'s normal-form term count, or `1` for a Tier 0
    /// rational / Tier 2 computable (neither can explode multiplicatively).
    /// Used to bound the term-pair work of an exact multiply *before* running
    /// it, and to reject a value whose term count crosses `max_algebraic_terms`.
    pub fn algebraic_term_count(&self) -> usize {
        match self {
            Self::Rational(_) | Self::Computable(_) => 1,
            Self::Algebraic(a) => a.term_count(),
        }
    }

    /// The multiquadratic normal-form terms `(monomial, coefficient)` of a
    /// Tier 1 value, or `None` for Tier 0/2. Used by the lossless state
    /// persistence codec (`crate::types::value_persist`) to capture the
    /// exact algebraic value; the reader reconstructs it by replaying
    /// `∑ cₘ·√m`, which the canonical normal form makes exact.
    #[cfg(any(test, feature = "wasm"))]
    pub(crate) fn algebraic_terms(&self) -> Option<Vec<(BigInt, Fraction)>> {
        match self {
            Self::Algebraic(a) => Some(
                a.terms()
                    .iter()
                    .map(|(m, c)| (m.clone(), c.clone()))
                    .collect(),
            ),
            Self::Rational(_) | Self::Computable(_) => None,
        }
    }

    /// Size probe (CS5): the largest coefficient bit-length in this value, used
    /// to bound BigInt blow-up against `max_bigint_bits`.
    pub fn max_coefficient_bits(&self) -> u64 {
        match self {
            Self::Rational(f) => f.numerator().bits().max(f.denominator().bits()),
            Self::Algebraic(a) => a.max_coefficient_bits(),
            Self::Computable(_) => 0,
        }
    }

    /// Reciprocal `1/x`. `Rational(nil)` for nil; `None` for an exactly
    /// zero operand — decided algebraically, with no budget, because an
    /// `Algebraic` is never zero. A Tier 2 operand also returns `None`:
    /// its zero-ness is not decidable, and no vocabulary word reaches
    /// this arm until the Tier 2 zero-separation protocol exists.
    pub fn reciprocal(&self) -> Option<Self> {
        match self {
            Self::Rational(f) => {
                if f.is_nil() {
                    return Some(Self::Rational(Fraction::nil()));
                }
                if f.is_zero() {
                    return None;
                }
                let (n, d) = f.to_bigint_pair();
                Some(Self::Rational(Fraction::new(d, n)))
            }
            Self::Algebraic(a) => Some(Self::from_result(a.reciprocal())),
            Self::Computable(_) => None,
        }
    }

    /// Addition. Nil-propagating; demotes to `Rational` whenever the sum
    /// is rational (cheapest-tier-wins). A Tier 2 operand yields a
    /// derived Tier 2 process via interval addition.
    pub fn add(&self, other: &Self) -> Self {
        if self.is_nil() || other.is_nil() {
            return Self::Rational(Fraction::nil());
        }
        match (self, other) {
            (Self::Rational(a), Self::Rational(b)) => Self::Rational(a.add(b)),
            (Self::Rational(q), Self::Algebraic(a)) | (Self::Algebraic(a), Self::Rational(q)) => {
                Self::from_result(a.add_fraction(q))
            }
            (Self::Algebraic(a), Self::Algebraic(b)) => Self::from_result(a.add(b)),
            (Self::Computable(_), _) | (_, Self::Computable(_)) => {
                let (a, b) = (self.clone(), other.clone());
                Self::Computable(Computable::from_enclosures("add", move |step| {
                    let (x, y) = (a.enclosure_at(step), b.enclosure_at(step));
                    RatInterval::new(x.lo.add(&y.lo), x.hi.add(&y.hi))
                }))
            }
        }
    }

    /// Subtraction `self − other`.
    pub fn sub(&self, other: &Self) -> Self {
        if self.is_nil() || other.is_nil() {
            return Self::Rational(Fraction::nil());
        }
        match (self, other) {
            (Self::Rational(a), Self::Rational(b)) => Self::Rational(a.sub(b)),
            _ => self.add(&other.neg()),
        }
    }

    /// Multiplication. A Tier 2 operand yields a derived Tier 2 process
    /// via interval multiplication (min/max of the endpoint products).
    pub fn mul(&self, other: &Self) -> Self {
        if self.is_nil() || other.is_nil() {
            return Self::Rational(Fraction::nil());
        }
        match (self, other) {
            (Self::Rational(a), Self::Rational(b)) => Self::Rational(a.mul(b)),
            (Self::Rational(q), Self::Algebraic(a)) | (Self::Algebraic(a), Self::Rational(q)) => {
                Self::from_result(a.mul_fraction(q))
            }
            (Self::Algebraic(a), Self::Algebraic(b)) => Self::from_result(a.mul(b)),
            (Self::Computable(_), _) | (_, Self::Computable(_)) => {
                let (a, b) = (self.clone(), other.clone());
                Self::Computable(Computable::from_enclosures("mul", move |step| {
                    let (x, y) = (a.enclosure_at(step), b.enclosure_at(step));
                    let mut products = [
                        x.lo.mul(&y.lo),
                        x.lo.mul(&y.hi),
                        x.hi.mul(&y.lo),
                        x.hi.mul(&y.hi),
                    ];
                    products.sort();
                    let [lo, .., hi] = products;
                    RatInterval::new(lo, hi)
                }))
            }
        }
    }

    /// Division `self / other`. `Rational(nil)` for nil operands; `None`
    /// for a zero divisor — exact over Tier ≤ 1, where zero-ness is
    /// decidable. A Tier 2 divisor returns `None` until the Tier 2
    /// zero-separation protocol exists (no vocabulary reaches it).
    pub fn div(&self, other: &Self) -> Option<Self> {
        if self.is_nil() || other.is_nil() {
            return Some(Self::Rational(Fraction::nil()));
        }
        if other.is_structurally_zero() {
            return None;
        }
        match (self, other) {
            (Self::Rational(a), Self::Rational(b)) => Some(Self::Rational(a.div(b))),
            (Self::Rational(q), Self::Algebraic(b)) => Some(Self::from_result(b.recip_scaled(q))),
            (Self::Algebraic(a), Self::Rational(q)) => {
                let inv = Fraction::new(q.denominator(), q.numerator());
                Some(Self::from_result(a.mul_fraction(&inv)))
            }
            (Self::Algebraic(a), Self::Algebraic(b)) => Some(Self::from_result(a.div(b))),
            (_, Self::Computable(_)) => None,
            (Self::Computable(_), _) => {
                let inv = other.reciprocal()?;
                Some(self.mul(&inv))
            }
        }
    }

    // ---- Observations ----

    /// Three-way comparison under a water budget (SPEC §7.4.1 / §7.4.2).
    /// Tier ≤ 1 pairs decide exactly without consuming any water; the
    /// budget bounds refinement only when a Tier 2 observation is
    /// involved, where exhaustion yields `Starved` — the source of the
    /// logical `Unknown` (U).
    pub fn cmp_within(&self, other: &Self, water: Water) -> ExactCmp {
        if self.is_nil() || other.is_nil() {
            return ExactCmp::Absent;
        }
        match (self, other) {
            (Self::Rational(a), Self::Rational(b)) => ExactCmp::Decided(a.cmp(b)),
            (Self::Rational(q), Self::Algebraic(b)) => {
                ExactCmp::Decided(b.cmp_fraction(q).reverse())
            }
            (Self::Algebraic(a), Self::Rational(q)) => ExactCmp::Decided(a.cmp_fraction(q)),
            (Self::Algebraic(a), Self::Algebraic(b)) => ExactCmp::Decided(a.cmp(b)),
            (Self::Computable(_), _) | (_, Self::Computable(_)) => {
                self.cmp_by_refinement(other, water)
            }
        }
    }

    /// Three-way comparison under the default water. Total (`Decided`)
    /// for every non-nil Tier ≤ 1 pair.
    pub fn cmp_exact(&self, other: &Self) -> ExactCmp {
        self.cmp_within(other, DEFAULT_COMPARISON_WATER)
    }

    /// Water-explicit rational enclosure of this value after spending `budget`
    /// refinement steps (the `MATH@ENCLOSE` observation, SPEC §4.2 / §14.3).
    /// `None` for nil (the empty observation). Tier ≤ 1 values return a point
    /// (or tight algebraic bounds); a Tier 2 value returns its generator's
    /// enclosure — the only tier whose width the budget actually governs.
    /// Representation-neutral: the caller sees rational endpoints, never a tier.
    pub fn observe_enclosure(&self, budget: u64) -> Option<RatInterval> {
        if self.is_nil() {
            return None;
        }
        Some(self.enclosure_at(budget))
    }

    /// Enclosure of a non-nil value after `step` refinement steps:
    /// a point for rationals, the doubling algebraic bounds for Tier 1,
    /// the generator's interval for Tier 2. Nested and shrinking in
    /// `step` for every tier.
    fn enclosure_at(&self, step: u64) -> RatInterval {
        match self {
            Self::Rational(f) => RatInterval::point(f.clone()),
            Self::Algebraic(a) => {
                let (lo, hi) = a.bounds(8 + 16 * step.min(4096));
                RatInterval::new(lo, hi)
            }
            Self::Computable(c) => c.enclosure_at(step),
        }
    }

    /// Interval-separation comparison for pairs involving Tier 2: refine
    /// both enclosures step by step under the water budget; disjoint
    /// enclosures decide the order, exhaustion starves. Equality is never
    /// proven here (undecidable for computable reals) — equal values
    /// starve, honestly.
    fn cmp_by_refinement(&self, other: &Self, water: Water) -> ExactCmp {
        for step in 0..water.0 {
            let a = self.enclosure_at(step);
            let b = other.enclosure_at(step);
            if a.hi.lt(&b.lo) {
                return ExactCmp::Decided(Ordering::Less);
            }
            if b.hi.lt(&a.lo) {
                return ExactCmp::Decided(Ordering::Greater);
            }
            if a.is_point() && b.is_point() && a.lo == b.lo {
                return ExactCmp::Decided(Ordering::Equal);
            }
        }
        ExactCmp::Starved {
            steps: water.0 as usize,
        }
    }

    /// Floor as an exact real. `None` for nil, and for a Tier 2 value
    /// whose enclosure does not pin the floor within the internal water
    /// cap (the undecidable outcome).
    pub fn floor(&self) -> Option<ExactReal> {
        match self {
            Self::Rational(f) => {
                if f.is_nil() {
                    return None;
                }
                Some(Self::from_bigint(f.numerator().div_floor(&f.denominator())))
            }
            Self::Algebraic(a) => Some(Self::from_bigint(a.floor_int())),
            Self::Computable(_) => self.tier2_pinned_integer(|iv| {
                let fl = iv.lo.numerator().div_floor(&iv.lo.denominator());
                let fh = iv.hi.numerator().div_floor(&iv.hi.denominator());
                (fl == fh).then_some(fl)
            }),
        }
    }

    /// Ceiling. `None` for nil or an unpinned Tier 2 value.
    pub fn ceil(&self) -> Option<ExactReal> {
        match self {
            Self::Rational(f) => {
                if f.is_nil() {
                    return None;
                }
                Some(Self::from_bigint(f.numerator().div_ceil(&f.denominator())))
            }
            Self::Algebraic(a) => Some(Self::from_bigint(a.ceil_int())),
            Self::Computable(_) => self.tier2_pinned_integer(|iv| {
                let cl = iv.lo.numerator().div_ceil(&iv.lo.denominator());
                let ch = iv.hi.numerator().div_ceil(&iv.hi.denominator());
                (cl == ch).then_some(cl)
            }),
        }
    }

    /// Round to the nearest integer, ties away from zero (matching
    /// `Fraction::round`). `None` for nil or an unpinned Tier 2 value.
    pub fn round(&self) -> Option<ExactReal> {
        match self {
            Self::Rational(f) => {
                if f.is_nil() {
                    return None;
                }
                Some(Self::Rational(f.round()))
            }
            Self::Algebraic(a) => Some(Self::from_bigint(a.round_int())),
            Self::Computable(_) => self.tier2_pinned_integer(|iv| {
                let rl = iv.lo.round();
                let rh = iv.hi.round();
                (rl == rh).then(|| rl.numerator())
            }),
        }
    }

    /// Refine a Tier 2 enclosure under the internal water cap until
    /// `pin` extracts a consistent integer from it; `None` on starvation.
    fn tier2_pinned_integer(
        &self,
        pin: impl Fn(&RatInterval) -> Option<BigInt>,
    ) -> Option<ExactReal> {
        for step in 0..TIER2_INTERNAL_WATER {
            if let Some(n) = pin(&self.enclosure_at(step)) {
                return Some(Self::from_bigint(n));
            }
        }
        None
    }
}
