//! Multiquadratic normal form for the admitted exact-real domain \(D\)
//! (SPEC §4.2.7).
//!
//! Every value the current Coreword set can construct lies in the
//! multiquadratic closure of the rationals: the field
//! \(\mathbb{Q}(\sqrt{d_1}, \sqrt{d_2}, \dots)\). Each element of that field
//! has a unique normal form \(\sum_d c_d \sqrt{d}\) as a finite
//! \(\mathbb{Q}\)-linear combination of square-root monomials, because the
//! square roots of multiplicatively independent integers are linearly
//! independent over \(\mathbb{Q}\). This module derives that normal form
//! from an `ExactReal` (including composed `Gosper` trees, whose leaves are
//! always `Rational` or `AlgebraicSqrt` in the current Coreword set) and
//! decides equality and order on it **exactly and totally** — the mechanism
//! behind the §7.4 guarantee that the six comparison relations never return
//! `unknown` over \(D\).
//!
//! Per SPEC §2.3 / §4.2.4 the normal form is a representation detail: it is
//! never observable, display stays the canonical continued fraction, and no
//! protocol string changes. `algebraic_cmp` returns `None` — sending the
//! caller to the budgeted CF path — only for operands outside its reach
//! (nil, or a degenerate transform recording a division by exact zero), so
//! it can never *wrongly* decide.
//!
//! Instead of factoring radicands into primes (unbounded cost for large
//! semiprimes), the normal form is taken over a **GCD-free basis**: the
//! radicands of both operands are refined into pairwise-coprime integers,
//! none a perfect square, such that every radicand is a product of powers
//! of basis elements. Distinct subset products of such a basis are never
//! perfect squares, which is exactly the hypothesis the linear-independence
//! theorem needs; primality is not required.

use crate::types::continued_fraction::{ExactReal, Gosper};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// Exact three-way comparison of two admitted-domain (SPEC §4.2.7) exact
/// reals via the multiquadratic normal form. Total for every operand pair
/// the current Coreword set can construct; returns `None` (fall back to the
/// budgeted CF comparison) only when an operand is nil or contains a
/// degenerate division-by-exact-zero transform.
pub fn algebraic_cmp(a: &ExactReal, b: &ExactReal) -> Option<Ordering> {
    let basis = Basis::for_operands(&[a, b])?;
    let va = eval(a, &basis)?;
    let vb = eval(b, &basis)?;
    Some(va.sub(&vb).sign())
}

/// Whether an admitted-domain exact real is exactly zero. Used by `DIV` to
/// project a lazily-built zero divisor onto the `divisionByZero` Bubble
/// (SPEC §4.2.7: "Division by an element that is exactly zero bubbles to
/// NIL per Section 11.2"). `None` when the value is outside the normal
/// form's reach (nil or a degenerate transform).
pub fn algebraic_is_zero(x: &ExactReal) -> Option<bool> {
    let basis = Basis::for_operands(&[x])?;
    Some(eval(x, &basis)?.is_zero())
}

// =========================================================================
// GCD-free radicand basis
// =========================================================================

/// Pairwise-coprime positive integers, each ≥ 2 and none a perfect square,
/// such that every collected radicand is a product of powers of basis
/// elements. Distinct subset products of the basis are then never perfect
/// squares (a product of pairwise-coprime integers is a square iff each
/// factor is), so the square roots of distinct subset products are linearly
/// independent over ℚ and the coefficient map below is a true normal form.
struct Basis(Vec<BigInt>);

impl Basis {
    fn for_operands(operands: &[&ExactReal]) -> Option<Basis> {
        let mut radicands = Vec::new();
        for op in operands {
            collect_radicands(op, &mut radicands)?;
        }
        Some(Self::build(radicands))
    }

    fn build(radicands: Vec<BigInt>) -> Basis {
        let one = BigInt::one();
        let mut elems: Vec<BigInt> = radicands.into_iter().filter(|n| *n > one).collect();
        elems.sort();
        elems.dedup();
        // GCD-free refinement: replace any pair sharing a factor by
        // {gcd, a/gcd, b/gcd}. Each replacement strictly decreases the
        // product of all elements, so the loop terminates.
        loop {
            let mut shared: Option<(usize, usize)> = None;
            'search: for i in 0..elems.len() {
                for j in (i + 1)..elems.len() {
                    if !elems[i].gcd(&elems[j]).is_one() {
                        shared = Some((i, j));
                        break 'search;
                    }
                }
            }
            let Some((i, j)) = shared else { break };
            let a = elems.swap_remove(j);
            let b = elems.swap_remove(i);
            let g = a.gcd(&b);
            for part in [&a / &g, &b / &g, g] {
                if part > one && !elems.contains(&part) {
                    elems.push(part);
                }
            }
        }
        // Square reduction: a basis element that is a perfect square would
        // make √element rational, breaking independence. Replacing b = c²
        // by c preserves pairwise coprimality (any common factor of c and
        // another element would divide b too) and keeps every radicand a
        // product of basis-element powers.
        for e in &mut elems {
            loop {
                let root = e.sqrt();
                if &(&root * &root) == e {
                    *e = root;
                } else {
                    break;
                }
            }
        }
        elems.sort();
        Basis(elems)
    }

    /// √n as `outside · √monomial` over the basis: factor n into basis-
    /// element powers, halve even exponents into `outside`, and keep the
    /// odd-exponent elements' product as the monomial key. `None` only if
    /// n is not covered by the basis, which the construction rules out for
    /// collected radicands (kept as a defensive fallback, never a wrong
    /// answer).
    fn decompose_sqrt(&self, n: &BigInt) -> Option<(BigInt, BigInt)> {
        let mut rest = n.clone();
        let mut outside = BigInt::one();
        let mut monomial = BigInt::one();
        for b in &self.0 {
            let mut exp = 0u32;
            while (&rest % b).is_zero() {
                rest /= b;
                exp += 1;
            }
            outside *= b.pow(exp / 2);
            if exp % 2 == 1 {
                monomial *= b;
            }
        }
        if !rest.is_one() {
            return None;
        }
        Some((outside, monomial))
    }
}

/// Push the integer radicand of every `AlgebraicSqrt` leaf: √(p/q) is
/// carried as √(p·q)/q, so the integer whose square-root monomial matters
/// is p·q. `None` for a nil leaf (outside any comparison).
fn collect_radicands(er: &ExactReal, out: &mut Vec<BigInt>) -> Option<()> {
    match er {
        ExactReal::Rational(f) => {
            if f.is_nil() {
                return None;
            }
        }
        ExactReal::AlgebraicSqrt { radicand } => {
            out.push(radicand.numerator() * radicand.denominator());
        }
        ExactReal::Gosper(g) => match &**g {
            Gosper::Mobius { x, .. } => collect_radicands(x, out)?,
            Gosper::Bihomographic { x, y, .. } => {
                collect_radicands(x, out)?;
                collect_radicands(y, out)?;
            }
        },
    }
    Some(())
}

// =========================================================================
// Normal-form values and field arithmetic
// =========================================================================

/// An element of the multiquadratic field in normal form: a map from a
/// square-root monomial (a product of a subset of the basis; `1` keys the
/// rational part) to its non-zero rational coefficient. The empty map is
/// exactly zero. Keys of two values are comparable only when produced over
/// the same `Basis`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MqValue {
    terms: BTreeMap<BigInt, Fraction>,
}

impl MqValue {
    fn zero() -> MqValue {
        MqValue {
            terms: BTreeMap::new(),
        }
    }

    fn from_rational(f: &Fraction) -> MqValue {
        let mut v = Self::zero();
        v.add_term(BigInt::one(), f.clone());
        v
    }

    fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    /// Sole rational coefficient when the value is rational, else `None`.
    fn as_rational(&self) -> Option<Fraction> {
        if self.terms.is_empty() {
            return Some(Fraction::new(BigInt::zero(), BigInt::one()));
        }
        if self.terms.len() == 1 {
            let (m, c) = self.terms.iter().next().unwrap();
            if m.is_one() {
                return Some(c.clone());
            }
        }
        None
    }

    fn add_term(&mut self, monomial: BigInt, coeff: Fraction) {
        if coeff.is_zero() {
            return;
        }
        match self.terms.entry(monomial) {
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

    fn add(&self, other: &MqValue) -> MqValue {
        let mut out = self.clone();
        for (m, c) in &other.terms {
            out.add_term(m.clone(), c.clone());
        }
        out
    }

    fn sub(&self, other: &MqValue) -> MqValue {
        let mut out = self.clone();
        for (m, c) in &other.terms {
            out.add_term(m.clone(), Fraction::new(-c.numerator(), c.denominator()));
        }
        out
    }

    /// Product. For monomials over a pairwise-coprime basis,
    /// √m₁·√m₂ = g·√(m₁m₂/g²) with g = gcd(m₁, m₂), which is again a
    /// subset-product monomial.
    fn mul(&self, other: &MqValue) -> MqValue {
        let mut out = MqValue::zero();
        for (m1, c1) in &self.terms {
            for (m2, c2) in &other.terms {
                let g = m1.gcd(m2);
                let monomial = (m1 / &g) * (m2 / &g);
                let coeff = c1.mul(c2).mul(&Fraction::new(g, BigInt::one()));
                out.add_term(monomial, coeff);
            }
        }
        out
    }

    /// Multiplicative inverse by recursive conjugation, or `None` for the
    /// zero value. Splitting on a basis element b as y = u + v (v = the
    /// terms whose monomial contains b), the product y·(u − v) = u² − v²
    /// contains no b in its support, so recursion eliminates one basis
    /// element per step and bottoms out at a rational. u² − v² = 0 would
    /// force y = 0 (a field has no zero divisors), which is excluded
    /// upfront, so the recursion never divides by zero.
    fn inverse(&self, basis: &Basis) -> Option<MqValue> {
        if self.is_zero() {
            return None;
        }
        if let Some(q) = self.as_rational() {
            let (n, d) = q.to_bigint_pair();
            return Some(MqValue::from_rational(&Fraction::new(d, n)));
        }
        let split_on = basis
            .0
            .iter()
            .find(|b| self.terms.keys().any(|m| (m % *b).is_zero()))?;
        let mut with_b = MqValue::zero();
        let mut without_b = MqValue::zero();
        for (m, c) in &self.terms {
            if (m % split_on).is_zero() {
                with_b.add_term(m.clone(), c.clone());
            } else {
                without_b.add_term(m.clone(), c.clone());
            }
        }
        let conjugate = without_b.sub(&with_b);
        let product = self.mul(&conjugate);
        let inverse = product.inverse(basis)?;
        Some(conjugate.mul(&inverse))
    }

    /// Exact sign of the value. A non-empty normal form is non-zero by
    /// linear independence, so interval refinement over rational √ bounds
    /// is guaranteed to separate the sum from zero after finitely many
    /// doublings; no budget and no floating point are involved (SPEC
    /// §4.2.6).
    fn sign(&self) -> Ordering {
        if self.terms.is_empty() {
            return Ordering::Equal;
        }
        if self.terms.len() == 1 {
            // c·√m with √m > 0: the sign is the coefficient's (never zero
            // by the no-zero-coefficients invariant).
            let coeff = self.terms.values().next().unwrap();
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

    /// Rational enclosure of the value at 2⁻ᵇⁱᵗˢ per-monomial precision:
    /// with s = ⌊√(m·4ᵇⁱᵗˢ)⌋, √m ∈ [s, s+1]/2ᵇⁱᵗˢ, tightened to a point for
    /// the rational monomial m = 1.
    fn bounds(&self, bits: u64) -> (Fraction, Fraction) {
        let scale = BigInt::one() << bits;
        let mut lo = Fraction::new(BigInt::zero(), BigInt::one());
        let mut hi = lo.clone();
        for (m, c) in &self.terms {
            let (m_lo, m_hi) = if m.is_one() {
                (
                    Fraction::new(BigInt::one(), BigInt::one()),
                    Fraction::new(BigInt::one(), BigInt::one()),
                )
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

// =========================================================================
// ExactReal → normal form
// =========================================================================

/// Evaluate an `ExactReal` into the normal form over `basis`. Möbius and
/// bihomographic transforms are replayed as exact field arithmetic, so a
/// composed Gosper tree collapses to the same normal form as the value it
/// denotes. `None` for nil and for a transform whose denominator is exactly
/// zero (a degenerate division recorded before the divisor was known to be
/// zero) — the budgeted CF path remains the honest fallback there.
fn eval(er: &ExactReal, basis: &Basis) -> Option<MqValue> {
    match er {
        ExactReal::Rational(f) => {
            if f.is_nil() {
                return None;
            }
            Some(MqValue::from_rational(f))
        }
        ExactReal::AlgebraicSqrt { radicand } => {
            // √(p/q) = √(p·q)/q.
            let (p, q) = radicand.to_bigint_pair();
            let (outside, monomial) = basis.decompose_sqrt(&(p * &q))?;
            let mut v = MqValue::zero();
            v.add_term(monomial, Fraction::new(outside, q));
            Some(v)
        }
        ExactReal::Gosper(g) => match &**g {
            Gosper::Mobius { a, b, c, d, x } => {
                let xv = eval(x, basis)?;
                let num = xv.mul(&int_value(a)).add(&int_value(b));
                let den = xv.mul(&int_value(c)).add(&int_value(d));
                Some(num.mul(&den.inverse(basis)?))
            }
            Gosper::Bihomographic {
                a,
                b,
                c,
                d,
                e,
                f,
                g,
                h,
                x,
                y,
            } => {
                let xv = eval(x, basis)?;
                let yv = eval(y, basis)?;
                let xy = xv.mul(&yv);
                let num = xy
                    .mul(&int_value(a))
                    .add(&xv.mul(&int_value(b)))
                    .add(&yv.mul(&int_value(c)))
                    .add(&int_value(d));
                let den = xy
                    .mul(&int_value(e))
                    .add(&xv.mul(&int_value(f)))
                    .add(&yv.mul(&int_value(g)))
                    .add(&int_value(h));
                Some(num.mul(&den.inverse(basis)?))
            }
        },
    }
}

fn int_value(n: &BigInt) -> MqValue {
    MqValue::from_rational(&Fraction::new(n.clone(), BigInt::one()))
}
