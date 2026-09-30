//! Field operations on the Tier 1 normal form — multiplicative inverse and
//! division — and its integer rounding and rational approximation. Split
//! from `algebraic.rs` to respect the file-size budget (the file-size budget
//! in docs/dev/specification-implementation-rules.md).
//!
//! The continued fraction is **derived** here for the rational approximation
//! only — it is not an internal representation, and it is not a display
//! either (an irrational displays as its own source, `display.rs`). Because a
//! Tier 1 value has a decidable floor and exact field arithmetic, the
//! canonical CF terms fall out of the classical floor-and-reciprocate
//! iteration, exactly, to any requested depth.

use crate::types::exact::algebraic::{add_term as merge_term, Algebraic, AlgebraicResult};
use crate::types::fraction::Fraction;
use crate::types::fraction_arithmetic::balanced_bigint_gcd;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Zero};
use std::cmp::Ordering;
use std::collections::BTreeMap;

impl Algebraic {
    /// Multiplicative inverse `1/self` by recursive conjugation.
    ///
    /// Splitting on a basis element b as y = u + v (v = the terms whose
    /// monomial contains b), y·(u − v) = u² − v² has no b in its support,
    /// so each step eliminates one basis element and bottoms out at a
    /// rational. u² − v² = 0 would force y = 0 (a field has no zero
    /// divisors), and an `Algebraic` is never zero by the normal-form
    /// invariant, so the recursion is total — no budget, no failure path.
    pub fn reciprocal(&self) -> AlgebraicResult {
        let value = MqTerms(self.terms().clone());
        let inverse = value.inverse(self);
        AlgebraicResult::from_terms_over(self, inverse.0)
    }

    /// `self / other` for another algebraic (never-zero) value.
    pub fn div(&self, other: &Algebraic) -> AlgebraicResult {
        match other.reciprocal() {
            AlgebraicResult::Rational(f) => self.mul_fraction(&f),
            AlgebraicResult::Irrational(inv) => self.mul(&inv),
        }
    }

    /// `q / self` for a rational numerator.
    pub fn recip_scaled(&self, q: &Fraction) -> AlgebraicResult {
        match self.reciprocal() {
            AlgebraicResult::Rational(f) => AlgebraicResult::Rational(f.mul(q)),
            AlgebraicResult::Irrational(inv) => inv.mul_fraction(q),
        }
    }
}

impl AlgebraicResult {
    /// Package raw terms produced over `source`'s basis (demoting to a
    /// rational when the term shape allows).
    fn from_terms_over(source: &Algebraic, terms: BTreeMap<BigInt, Fraction>) -> AlgebraicResult {
        Algebraic::result_from_parts_of(source, terms)
    }
}

/// A raw coefficient map over an implicit shared basis: the internal
/// working state of the conjugation recursion. Unlike `Algebraic` it may
/// be zero or rational mid-recursion.
struct MqTerms(BTreeMap<BigInt, Fraction>);

impl MqTerms {
    fn zero() -> MqTerms {
        MqTerms(BTreeMap::new())
    }

    fn as_rational(&self) -> Option<Fraction> {
        match self.0.len() {
            0 => Some(Fraction::new(BigInt::zero(), BigInt::one())),
            1 => self
                .0
                .iter()
                .next()
                .filter(|(m, _)| m.is_one())
                .map(|(_, c)| c.clone()),
            _ => None,
        }
    }

    fn mul(&self, other: &MqTerms) -> MqTerms {
        let mut out = MqTerms::zero();
        for (m1, c1) in &self.0 {
            for (m2, c2) in &other.0 {
                let g = balanced_bigint_gcd(m1, m2);
                let monomial = (m1 / &g) * (m2 / &g);
                let coeff = c1.mul(c2).mul(&Fraction::new(g, BigInt::one()));
                merge_term(&mut out.0, monomial, coeff);
            }
        }
        out
    }

    fn sub(&self, other: &MqTerms) -> MqTerms {
        let mut out = MqTerms(self.0.clone());
        for (m, c) in &other.0 {
            merge_term(
                &mut out.0,
                m.clone(),
                Fraction::new(-c.numerator(), c.denominator()),
            );
        }
        out
    }

    /// Inverse by conjugation over `host`'s basis. Total for a non-zero
    /// value (see `reciprocal`); the recursion depth is bounded by the
    /// basis size.
    fn inverse(&self, host: &Algebraic) -> MqTerms {
        if let Some(q) = self.as_rational() {
            debug_assert!(!q.is_zero(), "inverse of zero is excluded by the invariant");
            let (n, d) = q.to_bigint_pair();
            let mut out = MqTerms::zero();
            merge_term(&mut out.0, BigInt::one(), Fraction::new(d, n));
            return out;
        }
        let split_on = host
            .basis()
            .elements()
            .iter()
            .find(|b| self.0.keys().any(|m| (m % *b).is_zero()))
            .expect("a non-rational term map uses at least one basis element")
            .clone();
        let mut with_b = MqTerms::zero();
        let mut without_b = MqTerms::zero();
        for (m, c) in &self.0 {
            if (m % &split_on).is_zero() {
                merge_term(&mut with_b.0, m.clone(), c.clone());
            } else {
                merge_term(&mut without_b.0, m.clone(), c.clone());
            }
        }
        let conjugate = without_b.sub(&with_b);
        let product = MqTerms(self.0.clone()).mul(&conjugate);
        let product_inverse = product.inverse(host);
        conjugate.mul(&product_inverse)
    }
}

fn fraction_floor(f: &Fraction) -> BigInt {
    f.numerator().div_floor(&f.denominator())
}

/// Work an observation surface may spend expanding one value's continued
/// fraction for its rational approximation, in the limb-multiply units the
/// runtime work meter counts in.
///
/// The cost of the expansion is not flat: each floor-and-reciprocate step
/// roughly doubles the term count once and then grows the coefficients without
/// bound, so the price per quotient climbs as the expansion deepens. Measured
/// on the reference container, a multiquadratic product of 2 / 4 / 8 / 16 / 32
/// terms took 0.1 ms to *build* throughout, while expanding 32 quotients of it
/// took 5 ms / 57 ms / 951 ms / 9.1 s / **147 s** — about 12x per doubling.
///
/// Budgeting the work costs the common case nothing: a one- or two-term value
/// (`2 SQRT`, `2 SQRT 3 SQRT ADD`) spends well under this and still reaches any
/// practical denominator bound. An expansion that would cost seconds stops
/// early and yields a coarser convergent, never a wrong one; the value itself
/// is `exactTerms` (and the stack display, which is the value's own source).
///
/// The value is chosen from measurement, not from a rate: it is the largest
/// budget under which a one- or two-term value still expands to 32 quotients
/// while a sixteen-term one is cut off before its first step. See
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

    /// Best rational approximation within a denominator bound: the
    /// deepest principal convergent whose denominator does not exceed
    /// `max_denominator`. Same contract as the historical
    /// `ExactReal::best_rational_approximation`; `None` when
    /// `max_denominator LT 1`.
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
        // Budgeted: this feeds the `approximate: true` rational beside a value
        // in the wire protocol, and a *convenience* field is the last thing
        // that should cost seconds. A shallower expansion yields a coarser
        // convergent, never a wrong one.
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
}
