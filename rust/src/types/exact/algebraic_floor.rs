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

    /// The first `budget` canonical (regular) CF partial quotients, by
    /// floor-and-reciprocate. The expansion of an irrational never
    /// terminates, so the result always has exactly `budget` terms and is
    /// always a strict prefix.
    pub fn cf_prefix(&self, budget: usize) -> Vec<BigInt> {
        let mut out = Vec::with_capacity(budget);
        let mut state: Option<Algebraic> = Some(self.clone());
        while out.len() < budget {
            let Some(x) = state else { break };
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
        while let Some(x) = state {
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
