//! Derived rational views of an exact real: best rational approximation
//! and canonical continued-fraction terms (SPEC §4.2.3). The CF here is a
//! **display-side derivation** from the value — floor-and-reciprocate for
//! Tier 1, the certain common prefix of a refined enclosure for Tier 2 —
//! not an internal representation. Split from `value.rs` to respect the
//! file-size budget (SPEC §14.1).

use crate::types::exact::value::{ExactReal, TIER2_INTERNAL_WATER};
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
            Self::Computable(c) => {
                // Serialization-boundary approximation for a Tier 2 value:
                // refine to the internal cap and take the midpoint's best
                // convergent. Deterministic; consumers already see the
                // `approximate` marker for any non-rational ExactScalar.
                let iv = c.enclosure_at(TIER2_INTERNAL_WATER);
                let two = Fraction::new(BigInt::from(2), BigInt::one());
                let mid = iv.lo.add(&iv.hi).div(&two);
                if &mid.denominator() <= max_denominator {
                    return Some(mid);
                }
                convergent_within(
                    &rational_partial_quotients(mid.numerator(), mid.denominator()),
                    max_denominator,
                )
            }
        }
    }

    /// Canonical partial quotients for finite (rational) values; `None`
    /// for nil and for irrationals (whose CF is infinite — use
    /// `partial_quotients_bounded`).
    pub fn partial_quotients(&self) -> Option<Vec<BigInt>> {
        match self {
            Self::Rational(f) => {
                if f.is_nil() {
                    return None;
                }
                Some(rational_partial_quotients(f.numerator(), f.denominator()))
            }
            Self::Algebraic(_) | Self::Computable(_) => None,
        }
    }

    /// Up to `budget` canonical partial quotients, derived exactly: the
    /// full canonical CF (truncated) for rationals, the floor-and-
    /// reciprocate prefix for Tier 1 irrationals. The CF is a display
    /// form derived from the value, not a representation (SPEC §4.2.3).
    pub fn partial_quotients_bounded(&self, budget: usize) -> Vec<BigInt> {
        if budget == 0 {
            return Vec::new();
        }
        match self {
            Self::Rational(f) => {
                if f.is_nil() {
                    return Vec::new();
                }
                let mut qs = rational_partial_quotients(f.numerator(), f.denominator());
                qs.truncate(budget);
                qs
            }
            Self::Algebraic(a) => a.cf_prefix(budget),
            Self::Computable(c) => {
                // Certain CF prefix of a Tier 2 value: terms shared by both
                // endpoints of a refined enclosure. The set of reals with a
                // given CF prefix is an interval, so a prefix carried by
                // both endpoints holds for the enclosed value. Endpoint CFs
                // terminate, and a terminating final term is not stable
                // under perturbation, so each endpoint's last term is
                // dropped before intersecting. Often empty — displayed as
                // the undetermined-CF marker `( ...)`.
                let iv = c.enclosure_at(budget as u64);
                let mut lo_cf = rational_partial_quotients(iv.lo.numerator(), iv.lo.denominator());
                let mut hi_cf = rational_partial_quotients(iv.hi.numerator(), iv.hi.denominator());
                lo_cf.pop();
                hi_cf.pop();
                let mut out = Vec::new();
                for (a, b) in lo_cf.iter().zip(hi_cf.iter()) {
                    if a != b || out.len() >= budget {
                        break;
                    }
                    out.push(a.clone());
                }
                out
            }
        }
    }
}

/// Canonical (regular) CF of a rational, with the standard uniqueness
/// normalization: no trailing `1` term (`[…, a, 1]` folds to `[…, a+1]`).
/// Same expansion the retired CF module used, so displays are unchanged.
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
