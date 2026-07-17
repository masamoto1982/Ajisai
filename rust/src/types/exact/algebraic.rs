//! Tier 1: algebraic numbers as a first-class multiquadratic normal form
//! (SPEC §4.2).
//!
//! An [`Algebraic`] is an element of \(\mathbb{Q}(\sqrt{d_1}, \sqrt{d_2},
//! \dots)\) in normal form \(\sum_m c_m \sqrt{m}\), where the monomials
//! `m` are distinct subset products of the value's GCD-free radicand
//! basis and the coefficients are non-zero rationals. Square roots of
//! distinct subset products of a GCD-free basis are linearly independent
//! over ℚ, so a non-empty term map is never zero — the fact that makes
//! sign, order, and equality **decidable without any budget**: zero tests
//! are algebraic on the normal form, and interval refinement is used only
//! to speed up sign resolution, never to gate its correctness.
//!
//! Everything the current vocabulary can construct (√ of rationals closed
//! under field operations) lives in this form. Values that become
//! rational demote eagerly (cheapest-tier-wins): arithmetic never returns
//! an `Algebraic` that merely wraps a rational — callers receive the
//! `Fraction` itself and route it back to Tier 0.

use crate::types::exact::basis::Basis;
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// An irrational algebraic number in multiquadratic normal form. The
/// public constructors uphold two invariants: the term map always has a
/// term with a non-`1` monomial (rational values demote to `Fraction`
/// instead), and every coefficient is a non-zero, non-nil rational.
#[derive(Debug, Clone)]
pub struct Algebraic {
    basis: Basis,
    /// Monomial (subset product of `basis`; `1` keys the rational part)
    /// → non-zero rational coefficient.
    terms: BTreeMap<BigInt, Fraction>,
}

/// Result of an arithmetic step: the exact value, already demoted to
/// Tier 0 when it is rational (cheapest-tier-wins).
#[derive(Debug, Clone, PartialEq)]
pub enum AlgebraicResult {
    Rational(Fraction),
    Irrational(Algebraic),
}

impl AlgebraicResult {
    fn from_parts(basis: Basis, terms: BTreeMap<BigInt, Fraction>) -> AlgebraicResult {
        let rational = match terms.len() {
            0 => Some(Fraction::new(BigInt::zero(), BigInt::one())),
            1 => terms
                .iter()
                .next()
                .filter(|(m, _)| m.is_one())
                .map(|(_, c)| c.clone()),
            _ => None,
        };
        match rational {
            Some(f) => AlgebraicResult::Rational(f),
            None => {
                let basis = basis.pruned_to(terms.keys());
                AlgebraicResult::Irrational(Algebraic { basis, terms })
            }
        }
    }

    /// The irrational payload, if any (test/observation convenience).
    pub fn as_irrational(&self) -> Option<&Algebraic> {
        match self {
            AlgebraicResult::Irrational(a) => Some(a),
            AlgebraicResult::Rational(_) => None,
        }
    }

    /// The rational payload, if the value demoted (test convenience).
    pub fn as_rational(&self) -> Option<&Fraction> {
        match self {
            AlgebraicResult::Rational(f) => Some(f),
            AlgebraicResult::Irrational(_) => None,
        }
    }
}

impl Algebraic {
    // ---- Construction ----

    /// √`radicand` with the same normalization as the historical
    /// `ExactReal::from_sqrt_rational`: `None` for nil or negative input,
    /// `Rational` for zero and perfect squares, `Irrational` otherwise.
    pub fn sqrt_of_fraction(radicand: &Fraction) -> Option<AlgebraicResult> {
        if radicand.is_nil() {
            return None;
        }
        let num = radicand.numerator();
        let den = radicand.denominator();
        if num.is_negative() {
            return None;
        }
        if num.is_zero() {
            return Some(AlgebraicResult::Rational(Fraction::new(
                BigInt::zero(),
                BigInt::one(),
            )));
        }
        let sn = num.sqrt();
        let sd = den.sqrt();
        if &sn * &sn == num && &sd * &sd == den {
            return Some(AlgebraicResult::Rational(Fraction::new(sn, sd)));
        }
        // √(p/q) = √(p·q)/q over the integer radicand p·q.
        let integer_radicand = &num * &den;
        let basis = Basis::build(vec![integer_radicand.clone()]);
        let (outside, monomial) = basis
            .decompose_sqrt(&integer_radicand)
            .expect("basis was built from this radicand");
        let mut terms = BTreeMap::new();
        terms.insert(monomial, Fraction::new(outside, den));
        Some(AlgebraicResult::from_parts(basis, terms))
    }

    /// Number of terms in the normal form (test/diagnostic surface).
    pub fn term_count(&self) -> usize {
        self.terms.len()
    }

    /// Structural identity of the stored normal form — same basis, same
    /// term map. Cheaper than semantic equality (`==`, which rebases) and
    /// used where a conservative "unchanged?" check suffices; a false
    /// negative (equal values, different granularity) is always safe.
    pub fn same_representation(&self, other: &Algebraic) -> bool {
        self.basis == other.basis && self.terms == other.terms
    }

    /// Package raw terms produced over `source`'s basis into a result
    /// (demoting to Tier 0 when the shape allows). Internal helper for
    /// the field-operation module.
    pub(crate) fn result_from_parts_of(
        source: &Algebraic,
        terms: BTreeMap<BigInt, Fraction>,
    ) -> AlgebraicResult {
        AlgebraicResult::from_parts(source.basis.clone(), terms)
    }

    pub(crate) fn basis(&self) -> &Basis {
        &self.basis
    }

    pub(crate) fn terms(&self) -> &BTreeMap<BigInt, Fraction> {
        &self.terms
    }

    /// Rebuild `self`'s terms over `target`, which must cover every
    /// monomial (callers merge bases before rebasing).
    fn rebased_terms(&self, target: &Basis) -> BTreeMap<BigInt, Fraction> {
        let mut out: BTreeMap<BigInt, Fraction> = BTreeMap::new();
        for (m, c) in &self.terms {
            let (outside, monomial) = target
                .decompose_sqrt(m)
                .expect("merged basis covers both operands' monomials");
            let scaled = c.mul(&Fraction::new(outside, BigInt::one()));
            add_term(&mut out, monomial, scaled);
        }
        out
    }

    // ---- Ring operations ----

    /// `-self`. Negation never changes rationality, so the result stays
    /// irrational and keeps the basis.
    pub fn neg(&self) -> Algebraic {
        let terms = self
            .terms
            .iter()
            .map(|(m, c)| {
                (
                    m.clone(),
                    Fraction::new(-c.numerator(), c.denominator()),
                )
            })
            .collect();
        Algebraic {
            basis: self.basis.clone(),
            terms,
        }
    }

    /// `self + other`.
    pub fn add(&self, other: &Algebraic) -> AlgebraicResult {
        let basis = Basis::merged(&self.basis, &other.basis, &[]);
        let mut terms = self.rebased_terms(&basis);
        for (m, c) in other.rebased_terms(&basis) {
            add_term(&mut terms, m, c);
        }
        AlgebraicResult::from_parts(basis, terms)
    }

    /// `self - other`.
    pub fn sub(&self, other: &Algebraic) -> AlgebraicResult {
        self.add(&other.neg())
    }

    /// `self + q` for a rational `q`.
    pub fn add_fraction(&self, q: &Fraction) -> AlgebraicResult {
        let mut terms = self.terms.clone();
        add_term(&mut terms, BigInt::one(), q.clone());
        AlgebraicResult::from_parts(self.basis.clone(), terms)
    }

    /// `self · q` for a rational `q`. Multiplying by zero demotes to the
    /// rational `0`; any other rational keeps the value irrational.
    pub fn mul_fraction(&self, q: &Fraction) -> AlgebraicResult {
        if q.is_zero() {
            return AlgebraicResult::Rational(Fraction::new(BigInt::zero(), BigInt::one()));
        }
        let terms = self
            .terms
            .iter()
            .map(|(m, c)| (m.clone(), c.mul(q)))
            .collect();
        AlgebraicResult::from_parts(self.basis.clone(), terms)
    }

    /// `self · other`. For monomials over a shared GCD-free basis,
    /// √m₁·√m₂ = g·√(m₁m₂/g²) with g = gcd(m₁, m₂), again a subset
    /// product — so the product stays in normal form.
    pub fn mul(&self, other: &Algebraic) -> AlgebraicResult {
        let basis = Basis::merged(&self.basis, &other.basis, &[]);
        let a = self.rebased_terms(&basis);
        let b = other.rebased_terms(&basis);
        let mut terms: BTreeMap<BigInt, Fraction> = BTreeMap::new();
        for (m1, c1) in &a {
            for (m2, c2) in &b {
                let g = m1.gcd(m2);
                let monomial = (m1 / &g) * (m2 / &g);
                let coeff = c1.mul(c2).mul(&Fraction::new(g, BigInt::one()));
                add_term(&mut terms, monomial, coeff);
            }
        }
        AlgebraicResult::from_parts(basis, terms)
    }

    // ---- Decidable observations ----

    /// Exact sign. A non-empty normal form is non-zero by linear
    /// independence, so interval refinement over rational √ enclosures
    /// separates the sum from zero after finitely many doublings; the
    /// loop is a speed-up of a decidable fact, not a budget.
    pub fn sign(&self) -> Ordering {
        debug_assert!(!self.terms.is_empty(), "normal form of an irrational");
        if self.terms.len() == 1 {
            // c·√m with √m > 0: the sign is the coefficient's.
            let coeff = self.terms.values().next().expect("single term");
            return if coeff.is_positive() {
                Ordering::Greater
            } else {
                Ordering::Less
            };
        }
        let mut bits = 8u64;
        loop {
            let (lo, hi) = self.bounds(bits);
            if lo.is_positive() {
                return Ordering::Greater;
            }
            if hi.numerator().is_negative() {
                return Ordering::Less;
            }
            bits *= 2;
        }
    }

    /// Exact, total three-way comparison with another algebraic value.
    pub fn cmp(&self, other: &Algebraic) -> Ordering {
        match self.sub(other) {
            AlgebraicResult::Rational(f) => rational_sign(&f),
            AlgebraicResult::Irrational(d) => d.sign(),
        }
    }

    /// Exact, total three-way comparison with a rational.
    pub fn cmp_fraction(&self, q: &Fraction) -> Ordering {
        let neg_q = Fraction::new(-q.numerator(), q.denominator());
        match self.add_fraction(&neg_q) {
            AlgebraicResult::Rational(f) => rational_sign(&f),
            AlgebraicResult::Irrational(d) => d.sign(),
        }
    }

    /// A rational enclosure `[lo, hi]` at 2⁻ᵇⁱᵗˢ per-monomial precision:
    /// with s = ⌊√(m·4ᵇⁱᵗˢ)⌋, √m ∈ [s, s+1]/2ᵇⁱᵗˢ; a point for m = 1.
    /// Deeper `bits` give nested, shrinking enclosures — the observation
    /// (`refine`) surface of Tier 1.
    pub fn bounds(&self, bits: u64) -> (Fraction, Fraction) {
        let scale = BigInt::one() << bits;
        let mut lo = Fraction::new(BigInt::zero(), BigInt::one());
        let mut hi = lo.clone();
        for (m, c) in &self.terms {
            let (m_lo, m_hi) = if m.is_one() {
                let one = Fraction::new(BigInt::one(), BigInt::one());
                (one.clone(), one)
            } else {
                let s = (m.clone() << (2 * bits)).sqrt();
                (
                    Fraction::new(s.clone(), scale.clone()),
                    Fraction::new(s + 1, scale.clone()),
                )
            };
            let (term_lo, term_hi) = if c.numerator().is_negative() {
                (c.mul(&m_hi), c.mul(&m_lo))
            } else {
                (c.mul(&m_lo), c.mul(&m_hi))
            };
            lo = lo.add(&term_lo);
            hi = hi.add(&term_hi);
        }
        (lo, hi)
    }
}

/// Semantic equality: two normal forms may differ structurally when their
/// bases have different granularity (√12 alone keeps the coarse basis
/// {12}; √12 built as 2·√3 carries {3}), so equality is decided by
/// comparison, which rebases both sides over a common refinement.
impl PartialEq for Algebraic {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Algebraic {}

fn rational_sign(f: &Fraction) -> Ordering {
    if f.is_zero() {
        Ordering::Equal
    } else if f.is_positive() {
        Ordering::Greater
    } else {
        Ordering::Less
    }
}

/// Merge a term into a coefficient map, dropping the monomial when the
/// coefficients cancel (the no-zero-coefficients invariant).
pub(crate) fn add_term(terms: &mut BTreeMap<BigInt, Fraction>, monomial: BigInt, coeff: Fraction) {
    if coeff.is_zero() {
        return;
    }
    match terms.entry(monomial) {
        std::collections::btree_map::Entry::Vacant(e) => {
            e.insert(coeff);
        }
        std::collections::btree_map::Entry::Occupied(mut e) => {
            let sum = e.get().add(&coeff);
            if sum.is_zero() {
                e.remove();
            } else {
                e.insert(sum);
            }
        }
    }
}
