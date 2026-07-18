//! Field operations on the Tier 1 normal form: multiplicative inverse
//! and division. Split from `algebraic.rs` to respect the file-size
//! budget (SPEC §14.1).

use crate::types::exact::algebraic::{add_term as merge_term, Algebraic, AlgebraicResult};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Zero};
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
                let g = m1.gcd(m2);
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
