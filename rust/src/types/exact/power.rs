//! `xʸ` inside the exact field (LANG.VALUES.EXACT).
//!
//! An integer exponent stays in the base's own tier: a rational base powers
//! exactly, an algebraic base by repeated multiplication in the field. An
//! exponent `p/2` over a non-negative rational base stays in the field too —
//! `x^(p/2)` is `(√x)ᵖ`, which is why `SQRT` remains the Word that builds the
//! field and `POW` is not sugar for it. A negative base under `p/2` has no
//! real value, and every other exponent — a denominator other than 1 or 2, an
//! algebraic base under `p/2`, an irrational exponent — leaves the field:
//! both are `DomainMiss`. A zero base under a negative exponent divides by
//! zero.

use std::cmp::Ordering;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

use crate::types::exact::value::ExactReal;
use crate::types::fraction::Fraction;

/// What asking for a power produced.
#[derive(Debug, Clone)]
pub enum PowOutcome {
    Value(ExactReal),
    /// `0ʸ` with `y < 0`.
    DivisionByZero,
    /// A negative base under `p/2`, or an answer outside the field.
    DomainMiss,
    /// An exponent too large to materialize.
    SpaceExhausted,
    /// The work budget could not factor the base's radicand into its
    /// square-free normal form (`squarefree.rs`).
    WorkExhausted,
}

/// Bits the result of an integer power may reach: the exponent times the
/// base's own bit length, so `2^(500000)` is answered and `999^1000001`,
/// a ten-million-bit number no comparison will read, is refused.
const INTEGER_POWER_RESULT_BITS: u64 = 1 << 20;

/// The sign of a non-nil exact real. Decidable over the whole field.
fn sign_of(x: &ExactReal) -> Ordering {
    match x {
        ExactReal::Rational(q) => q.cmp(&Fraction::from(0)),
        ExactReal::Algebraic(a) => a.sign(),
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
    match positive.reciprocal() {
        Some(inverse) => PowOutcome::Value(inverse),
        None => PowOutcome::DivisionByZero,
    }
}

impl ExactReal {
    /// `self` raised to `exponent`, with no bound on the work a root's
    /// normal form may take.
    pub fn pow(&self, exponent: &ExactReal) -> PowOutcome {
        self.pow_within(exponent, &mut u64::MAX.clone())
    }

    /// `self` raised to `exponent`, charging a root's factorization to
    /// `budget`.
    pub fn pow_within(&self, exponent: &ExactReal, budget: &mut u64) -> PowOutcome {
        let Some(y) = exponent.as_rational() else {
            return PowOutcome::DomainMiss;
        };
        let (p, q) = y.to_bigint_pair();
        if q.is_one() {
            return power_by_integer(self, &p);
        }
        if q != BigInt::from(2) {
            return PowOutcome::DomainMiss;
        }
        match sign_of(self) {
            Ordering::Less => PowOutcome::DomainMiss,
            Ordering::Equal if p.is_positive() => {
                PowOutcome::Value(ExactReal::from_fraction(Fraction::from(0)))
            }
            Ordering::Equal => PowOutcome::DivisionByZero,
            Ordering::Greater => match self.as_rational() {
                Some(x) => {
                    let root = match ExactReal::try_sqrt_rational(x.clone(), budget) {
                        Ok(root) => root.expect("a positive rational has a square root"),
                        Err(_) => return PowOutcome::WorkExhausted,
                    };
                    power_by_integer(&root, &p)
                }
                None => PowOutcome::DomainMiss,
            },
        }
    }
}
