//! Integer rounding, continued-fraction derivation, and the observation
//! adapter for Tier 1 values.
//!
//! The continued fraction is **derived** here for display and rational
//! approximation — it is no longer an internal representation. Because a
//! Tier 1 value has a decidable floor and exact field arithmetic, the
//! canonical CF terms fall out of the classical floor-and-reciprocate
//! iteration, exactly, to any requested depth.

use crate::types::exact::algebraic::{Algebraic, AlgebraicResult};
use crate::types::exact::observation::{Observation, RatInterval, Refine, Water};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::One;
use std::cmp::Ordering;

fn fraction_floor(f: &Fraction) -> BigInt {
    f.numerator().div_floor(&f.denominator())
}

/// Work an observation surface may spend expanding one value's continued
/// fraction, in the limb-multiply units the runtime work meter counts in.
///
/// SPEC §4.2.3 leaves the display budget implementation-defined. It used to be
/// a *term count* — 32 partial quotients, however much each one cost — and the
/// cost is not flat: each floor-and-reciprocate step roughly doubles the term
/// count once and then grows the coefficients without bound, so the price per
/// quotient climbs as the expansion deepens.
///
/// That made writing a value down far dearer than computing it, and nothing
/// priced the difference. Measured on the reference container, a multiquadratic
/// product of 2 / 4 / 8 / 16 / 32 terms took 0.1 ms to *build* throughout,
/// while rendering it took 5 ms / 57 ms / 951 ms / 9.1 s / **147 s** — about
/// 12x per doubling. `wallTimeMs` was the only ceiling that noticed, and it
/// answers with a timeout, which names nothing and tells an agent nothing.
///
/// Budgeting the work instead of the terms costs the common case nothing: a
/// one- or two-term value (`2 SQRT`, `2 SQRT 3 SQRT +`) spends well under this
/// and still gets all 32 quotients. An expansion that would cost seconds stops
/// early and renders the trailing `…` truncation it was always entitled to, or
/// the `[ … ]` undetermined marker when not even `a0` is affordable. Neither loses
/// anything: for an algebraic value the CF is a rendering, and `exactTerms`
/// (with `exactDisplay` beside it) is the value.
///
/// The value is chosen from measurement, not from a rate: it is the largest
/// budget under which a one- or two-term value still expands to all 32
/// quotients while a sixteen-term one is cut off before its first step. See
/// `examples/work_meter_calibration`, whose second section is this table.
pub const CF_OBSERVATION_WORK_BUDGET: u64 = 16_384;

impl Algebraic {
    /// ⌊self⌋. An `Algebraic` is irrational (rationals demote), so it is
    /// never an integer and nested enclosures eventually separate it from
    /// every integer: the doubling loop is a decidable computation, not a
    /// budgeted search.
    pub fn floor_int(&self) -> BigInt {
        let mut bits = 8u64;
        loop {
            let (lo, hi) = self.bounds(bits);
            let fl = fraction_floor(&lo);
            let fh = fraction_floor(&hi);
            if fl == fh {
                return fl;
            }
            bits *= 2;
        }
    }

    /// ⌈self⌉ = ⌊self⌋ + 1 (an irrational is never an integer).
    pub fn ceil_int(&self) -> BigInt {
        self.floor_int() + BigInt::one()
    }

    /// Round to the nearest integer. An irrational is never a half-integer,
    /// so the tie rule (away from zero, matching `Fraction::round`) can
    /// never fire; the order against ⌊self⌋ + 1/2 decides exactly.
    pub fn round_int(&self) -> BigInt {
        let floor = self.floor_int();
        let half_up = Fraction::new(&floor * BigInt::from(2) + BigInt::one(), BigInt::from(2));
        match self.cmp_fraction(&half_up) {
            Ordering::Less => floor,
            _ => floor + BigInt::one(),
        }
    }

    /// What one floor-and-reciprocate step on this value costs, in the
    /// limb-multiply units the work meter counts in.
    ///
    /// Two things happen per step and both grow with the term count. The
    /// reciprocal conjugates and multiplies out, which is term-pairs wide —
    /// the shape the meter already prices algebraic arithmetic with. Then
    /// `floor_int` doubles its precision until the enclosure separates from an
    /// integer, and more terms mean more cancellation to see through, so the
    /// bits it needs grow with the term count too.
    ///
    /// Hence the cube rather than the square, which is measured rather than
    /// derived: with the square, one step on a 64-term value was priced at the
    /// same 4,096 units as one step on a 16-term value while costing 815 ms
    /// against 5.7 ms. The cube separates them, and the budget then cuts the
    /// expensive value off before its first step instead of after it.
    fn cf_step_units(&self) -> u64 {
        let terms = self.term_count() as u64;
        let bits = self.max_coefficient_bits().max(self.max_radicand_bits());
        let limbs = bits.div_ceil(64).max(1);
        terms
            .saturating_mul(terms)
            .saturating_mul(terms)
            .saturating_mul(limbs)
    }

    /// The first `budget` canonical (regular) CF partial quotients, by
    /// floor-and-reciprocate, or as many of them as
    /// [`CF_OBSERVATION_WORK_BUDGET`] affords.
    ///
    /// The expansion of an irrational never terminates, so the result is always
    /// a strict prefix and a caller must always render it as truncated —
    /// including when it is shorter than `budget`, or empty.
    pub fn cf_prefix(&self, budget: usize) -> Vec<BigInt> {
        let mut out = Vec::with_capacity(budget);
        let mut state: Option<Algebraic> = Some(self.clone());
        let mut remaining = CF_OBSERVATION_WORK_BUDGET;
        while out.len() < budget {
            let Some(x) = state else { break };
            // Charged before the step runs, like every other work budget here:
            // an expansion is refused rather than measured.
            let units = x.cf_step_units();
            if units > remaining {
                break;
            }
            remaining -= units;
            let a = x.floor_int();
            let minus_a = Fraction::new(-a.clone(), BigInt::one());
            out.push(a);
            // x_{k+1} = 1 / (x_k − a_k); the fractional part of an
            // irrational is irrational and in (0, 1), so the iteration
            // never leaves Tier 1 — the demotion arms are unreachable
            // (kept total for safety).
            state = match x.add_fraction(&minus_a) {
                AlgebraicResult::Irrational(frac_part) => match frac_part.reciprocal() {
                    AlgebraicResult::Irrational(next) => Some(next),
                    AlgebraicResult::Rational(_) => None,
                },
                AlgebraicResult::Rational(_) => None,
            };
        }
        out
    }

    /// Best rational approximation within a denominator bound: the
    /// deepest principal convergent whose denominator does not exceed
    /// `max_denominator`. Same contract as the historical
    /// `ExactReal::best_rational_approximation`; `None` when
    /// `max_denominator < 1`.
    pub fn best_rational_approximation(&self, max_denominator: &BigInt) -> Option<Fraction> {
        if max_denominator < &BigInt::one() {
            return None;
        }
        let mut h_prev2 = BigInt::from(0);
        let mut h_prev1 = BigInt::one();
        let mut k_prev2 = BigInt::one();
        let mut k_prev1 = BigInt::from(0);
        let mut best: Option<(BigInt, BigInt)> = None;
        let mut state: Option<Algebraic> = Some(self.clone());
        // The same expansion as `cf_prefix`, so the same budget: this feeds the
        // `approximate: true` rational beside a value in the wire protocol, and
        // a *convenience* field is the last thing that should cost seconds. A
        // shallower expansion yields a coarser convergent, never a wrong one.
        let mut remaining = CF_OBSERVATION_WORK_BUDGET;
        while let Some(x) = state {
            let units = x.cf_step_units();
            if units > remaining {
                break;
            }
            remaining -= units;
            let a = x.floor_int();
            let h = &a * &h_prev1 + &h_prev2;
            let k = &a * &k_prev1 + &k_prev2;
            if &k > max_denominator {
                break;
            }
            h_prev2 = std::mem::replace(&mut h_prev1, h.clone());
            k_prev2 = std::mem::replace(&mut k_prev1, k.clone());
            best = Some((h, k));
            let minus_a = Fraction::new(-a, BigInt::one());
            state = match x.add_fraction(&minus_a) {
                AlgebraicResult::Irrational(frac_part) => match frac_part.reciprocal() {
                    AlgebraicResult::Irrational(next) => Some(next),
                    AlgebraicResult::Rational(_) => None,
                },
                AlgebraicResult::Rational(_) => None,
            };
        }
        best.map(|(h, k)| Fraction::new(h, k))
    }

    /// Open this value as an observation process (Tier 1 adapter).
    pub fn observe(&self) -> AlgebraicObservation {
        AlgebraicObservation {
            value: self.clone(),
            bits: 8,
        }
    }
}

/// Tier 1 as an [`Observation`]: nested enclosures from `bounds`,
/// narrowed deterministically by the water spent. The value is
/// irrational, so refinement always reports `Narrower` — never
/// `Settled`, never `Starved` (Tier 1 cannot starve).
pub struct AlgebraicObservation {
    value: Algebraic,
    bits: u64,
}

impl Observation for AlgebraicObservation {
    fn current_interval(&self) -> Option<RatInterval> {
        let (lo, hi) = self.value.bounds(self.bits);
        Some(RatInterval::new(lo, hi))
    }

    fn refine(&mut self, w: Water) -> Refine {
        self.bits = self.bits.saturating_add(w.0);
        Refine::Narrower
    }
}
