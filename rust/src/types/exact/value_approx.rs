//! Derived rational view of an exact real: the best rational approximation
//! within a denominator bound, by continued-fraction convergents. The CF is
//! computed here, never stored — the value is the normal form. Split from
//! `value.rs` to respect the file-size budget (docs/dev/specification-implementation-rules.md).

use crate::types::exact::value::ExactReal;
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

impl ExactReal {
    /// Best rational approximation within a denominator bound: the
    /// deepest principal convergent whose denominator does not exceed
    /// `max_denominator`. `None` for nil or a bound below 1.
    pub fn best_rational_approximation(&self, max_denominator: &BigInt) -> Option<Fraction> {
        if max_denominator < &BigInt::one() {
            return None;
        }
        match self {
            Self::Rational(f) => {
                if f.is_nil() {
                    return None;
                }
                if &f.denominator() <= max_denominator {
                    return Some(f.clone());
                }
                convergent_within(
                    &rational_partial_quotients(f.numerator(), f.denominator()),
                    max_denominator,
                )
            }
            Self::Algebraic(a) => a.best_rational_approximation(max_denominator),
        }
    }
}

/// Canonical (regular) CF of a rational, with the standard uniqueness
/// normalization: no trailing `1` term (`[…, a, 1]` folds to `[…, a+1]`).
pub(crate) fn rational_partial_quotients(mut num: BigInt, mut den: BigInt) -> Vec<BigInt> {
    debug_assert!(!den.is_zero());
    if den.is_negative() {
        num = -num;
        den = -den;
    }
    let mut terms: Vec<BigInt> = Vec::new();
    loop {
        let (q, r) = num.div_mod_floor(&den);
        terms.push(q);
        if r.is_zero() {
            break;
        }
        num = den;
        den = r;
    }
    if terms.len() >= 2 && terms.last().expect("non-empty").is_one() {
        let popped = terms.pop().expect("just checked length >= 2");
        *terms.last_mut().expect("length >= 1 after pop") += popped;
    }
    terms
}

/// The deepest principal convergent of `terms` whose denominator stays
/// within `max_denominator`.
fn convergent_within(terms: &[BigInt], max_denominator: &BigInt) -> Option<Fraction> {
    let mut h_prev2 = BigInt::zero();
    let mut h_prev1 = BigInt::one();
    let mut k_prev2 = BigInt::one();
    let mut k_prev1 = BigInt::zero();
    let mut best: Option<(BigInt, BigInt)> = None;
    for a in terms {
        let h = a * &h_prev1 + &h_prev2;
        let k = a * &k_prev1 + &k_prev2;
        if &k > max_denominator {
            break;
        }
        h_prev2 = std::mem::replace(&mut h_prev1, h.clone());
        k_prev2 = std::mem::replace(&mut k_prev1, k.clone());
        best = Some((h, k));
    }
    best.map(|(h, k)| Fraction::new(h, k))
}
