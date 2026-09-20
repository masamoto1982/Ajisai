//! `xʸ` over every tier (LANG.VALUES.EXACT, Phase 7 of the vocabulary-100
//! work order).
//!
//! An integer exponent stays in the base's own tier: a rational base powers
//! exactly, an algebraic base by repeated multiplication in the field, a
//! computable base by the same multiplication over its enclosures. An
//! exponent `p/2` stays in the field too — `x^(p/2)` is `(√x)ᵖ`, which is
//! why `SQRT` remains the Word that builds the field and `POW` is not sugar
//! for it. A rational exponent whose root the base takes exactly (`8^(1/3)`)
//! answers the rational. Everything else is `exp(y·ln x)`, a computable real,
//! and needs `x > 0`: a negative base under a fractional exponent has no real
//! value (`DomainMiss`), a zero base under a negative exponent divides by zero,
//! and a Tier 2 base or exponent whose sign the internal budget cannot settle
//! is `Undecidable`.

use std::cmp::Ordering;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

use crate::types::exact::transcendental::{tier2_sign, Transcendental};
use crate::types::exact::value::ExactReal;
use crate::types::fraction::Fraction;

/// What asking for a power produced.
#[derive(Debug, Clone)]
pub enum PowOutcome {
    Value(ExactReal),
    /// `0ʸ` with `y < 0`.
    DivisionByZero,
    /// A negative base under a non-integer exponent.
    DomainMiss,
    /// A Tier 2 sign the internal budget could not settle.
    Undecidable,
    /// An exponent too large to materialize, or an `exp` argument that is.
    SpaceExhausted,
}

/// Bits the result of an integer power may reach: the exponent times the
/// base's own bit length, so `2^(500000)` is answered and `999^1000001`,
/// a ten-million-bit number no comparison will read, is refused.
const INTEGER_POWER_RESULT_BITS: u64 = 1 << 20;

fn from_transcendental(t: Transcendental) -> PowOutcome {
    match t {
        Transcendental::Value(v) => PowOutcome::Value(v),
        Transcendental::DomainMiss => PowOutcome::DomainMiss,
        Transcendental::Undecidable => PowOutcome::Undecidable,
        Transcendental::SpaceExhausted => PowOutcome::SpaceExhausted,
    }
}

/// The sign of a non-nil exact real, where it can be decided.
fn sign_of(x: &ExactReal) -> Option<Ordering> {
    match x {
        ExactReal::Rational(q) => Some(q.cmp(&Fraction::from(0))),
        ExactReal::Algebraic(a) => Some(a.sign()),
        ExactReal::Computable(c) => tier2_sign(c),
    }
}

/// `xⁿ` for an integer `n ≥ 0` by square-and-multiply, in `x`'s own tier.
fn integer_power(x: &ExactReal, n: &BigInt) -> ExactReal {
    if let Some(q) = x.as_rational() {
        let (num, den) = q.to_bigint_pair();
        let e = u32::try_from(n).expect("bounded by INTEGER_POWER_RESULT_BITS");
        return ExactReal::from_fraction(Fraction::new(num.pow(e), den.pow(e)));
    }
    let mut result = ExactReal::from_fraction(Fraction::from(1));
    let mut base = x.clone();
    let mut e = n.clone();
    while !e.is_zero() {
        if e.is_odd() {
            result = result.mul(&base);
        }
        e >>= 1;
        if !e.is_zero() {
            base = base.mul(&base);
        }
    }
    result
}

/// `1/x` in `x`'s own tier; `None` where zero-ness stops it.
fn reciprocal(x: &ExactReal) -> Option<PowOutcome> {
    match x {
        ExactReal::Computable(c) => ExactReal::tier2_reciprocal(c).map(PowOutcome::Value),
        _ => x.reciprocal().map(PowOutcome::Value),
    }
}

/// `x^n` for an integer exponent.
fn power_by_integer(x: &ExactReal, n: &BigInt) -> PowOutcome {
    let base_bits = BigInt::from(x.max_coefficient_bits().max(2));
    if n.abs() * base_bits > BigInt::from(INTEGER_POWER_RESULT_BITS) {
        return PowOutcome::SpaceExhausted;
    }
    if n.is_zero() {
        return PowOutcome::Value(ExactReal::from_fraction(Fraction::from(1)));
    }
    let positive = integer_power(x, &n.abs());
    if n.is_positive() {
        return PowOutcome::Value(positive);
    }
    match sign_of(x) {
        Some(Ordering::Equal) => PowOutcome::DivisionByZero,
        None => PowOutcome::Undecidable,
        Some(_) => reciprocal(&positive).unwrap_or(PowOutcome::Undecidable),
    }
}

/// `x^(p/q)` for a positive rational base, exactly where the root is exact
/// or `q == 2`, otherwise through `exp(y·ln x)`.
fn rational_root_power(x: &Fraction, p: &BigInt, q: &BigInt) -> PowOutcome {
    let (num, den) = x.to_bigint_pair();
    if let Ok(root) = u32::try_from(q) {
        let (rn, rd) = (num.nth_root(root), den.nth_root(root));
        if rn.pow(root) == num && rd.pow(root) == den {
            let base = ExactReal::from_fraction(Fraction::new(rn, rd));
            return power_by_integer(&base, p);
        }
        if root == 2 {
            let sqrt = ExactReal::from_sqrt_rational(x.clone())
                .expect("a positive rational has a square root");
            return power_by_integer(&sqrt, p);
        }
    }
    let exponent = ExactReal::from_fraction(Fraction::new(p.clone(), q.clone()));
    exp_of_log(&ExactReal::from_fraction(x.clone()), &exponent)
}

/// `exp(y·ln x)` for `x > 0`.
fn exp_of_log(x: &ExactReal, y: &ExactReal) -> PowOutcome {
    match x.ln() {
        Transcendental::Value(ln_x) => from_transcendental(y.mul(&ln_x).exp()),
        other => from_transcendental(other),
    }
}

impl ExactReal {
    /// `self` raised to `exponent`.
    pub fn pow(&self, exponent: &ExactReal) -> PowOutcome {
        if let Some(y) = exponent.as_rational() {
            let (p, q) = y.to_bigint_pair();
            if q.is_one() {
                return power_by_integer(self, &p);
            }
            return match sign_of(self) {
                Some(Ordering::Less) => PowOutcome::DomainMiss,
                Some(Ordering::Equal) => {
                    if p.is_positive() {
                        PowOutcome::Value(ExactReal::from_fraction(Fraction::from(0)))
                    } else {
                        PowOutcome::DivisionByZero
                    }
                }
                None => PowOutcome::Undecidable,
                Some(Ordering::Greater) => match self.as_rational() {
                    Some(x) => rational_root_power(x, &p, &q),
                    None => exp_of_log(self, exponent),
                },
            };
        }
        // An irrational exponent.
        match sign_of(self) {
            Some(Ordering::Less) => PowOutcome::DomainMiss,
            Some(Ordering::Equal) => match sign_of(exponent) {
                Some(Ordering::Greater) => {
                    PowOutcome::Value(ExactReal::from_fraction(Fraction::from(0)))
                }
                Some(_) => PowOutcome::DivisionByZero,
                None => PowOutcome::Undecidable,
            },
            None => PowOutcome::Undecidable,
            Some(Ordering::Greater) => {
                if self.as_rational() == Some(&Fraction::from(1)) {
                    return PowOutcome::Value(ExactReal::from_fraction(Fraction::from(1)));
                }
                exp_of_log(self, exponent)
            }
        }
    }
}
