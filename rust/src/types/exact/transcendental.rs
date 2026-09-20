//! The elementary transcendental functions over every tier
//! (LANG.VALUES.EXACT, Phase 7 of the vocabulary-100 work order).
//!
//! `exp`, `ln`, `sin`, `cos` and `atan` of an exact real are computable
//! reals: a Tier 2 value whose 512-bit base enclosure is the image of the
//! argument's own enclosure under a rigorous interval extension of the
//! function (`series`). A monotone function maps an interval by its
//! endpoints; `sin`, `cos` and `atan` map it through their Lipschitz
//! constant 1 — the image of `[a, b]` lies within `f(mid) ± (b − a)/2`.
//! The construction is constant-time; the cost is paid when the value is
//! observed, as with every Tier 2 value, and a comparison that the budget
//! cannot settle answers UNKNOWN rather than a guess.
//!
//! What a function cannot answer it says so: `ln` of a non-positive argument
//! has no real value (`DomainMiss`); a Tier 2 argument whose sign the
//! internal budget cannot separate from zero is `Undecidable`; and an
//! argument so large that the enclosure would not fit the machine is
//! `SpaceExhausted`, the outcome of every materialization past a ceiling.
//! Rational arguments with exact answers (`exp 0`, `ln 1`, `sin 0`, `cos 0`,
//! `atan 0`) stay rational: cheapest tier wins.

use std::cmp::Ordering;

use num_integer::Integer;
use num_traits::Signed;

use crate::types::exact::computable::Computable;
use crate::types::exact::observation::RatInterval;
use crate::types::exact::series::{
    atan_bounds, cos_bounds, exp_bounds, ln_bounds, outward, sin_bounds, BASE_BITS,
};
use crate::types::exact::value::{ExactReal, TIER2_INTERNAL_WATER};
use crate::types::fraction::Fraction;

/// What asking for a transcendental value produced.
#[derive(Debug, Clone)]
pub enum Transcendental {
    /// The value, at the cheapest tier that holds it.
    Value(ExactReal),
    /// The argument lies outside the function's real domain.
    DomainMiss,
    /// A Tier 2 argument whose position against the domain boundary the
    /// internal budget could not decide.
    Undecidable,
    /// The argument is so large the enclosure would not fit the machine.
    SpaceExhausted,
}

/// Bit length above which `exp` refuses its argument: `e^(2^20)` needs a
/// million-bit enclosure per observation, and beyond that the machine is
/// spending its budget on digits no comparison will read.
const EXP_ARGUMENT_BITS: u64 = 20;
/// Bit length above which `sin`/`cos` refuse: the reduction by `2kπ` spends
/// `log₂ k` of π's 512 bits, and an argument past `2^64` has spent them on
/// nothing a program can distinguish.
const TRIG_ARGUMENT_BITS: u64 = 64;

/// The sign of a Tier 2 value, separated from zero within the internal
/// budget, or `None` when it could not be.
pub(crate) fn tier2_sign(c: &Computable) -> Option<Ordering> {
    for step in 0..TIER2_INTERNAL_WATER {
        let iv = c.enclosure_at(step);
        if iv.lo.is_positive() {
            return Some(Ordering::Greater);
        }
        if iv.hi.is_positive() || iv.hi.is_zero() {
            continue;
        }
        return Some(Ordering::Less);
    }
    None
}

/// Whether every value in the argument's internal-budget enclosure has an
/// integer part shorter than `bits` bits.
fn magnitude_within(x: &ExactReal, bits: u64) -> bool {
    let iv = x.enclosure_at(TIER2_INTERNAL_WATER);
    let (n, d) = iv.lo.to_bigint_pair();
    let (m, e) = iv.hi.to_bigint_pair();
    n.abs().div_floor(&d).bits() < bits && m.abs().div_floor(&e).bits() < bits
}

/// A Tier 2 value whose base enclosure is `image` of the argument's
/// 512-bit enclosure.
fn derived(tag: &'static str, x: &ExactReal, image: fn(&RatInterval) -> RatInterval) -> ExactReal {
    let source = x.clone();
    ExactReal::Computable(Computable::from_base_bounds(tag, move || {
        outward(&image(&source.enclosure_at(BASE_BITS)), BASE_BITS)
    }))
}

/// The image of an interval under an increasing function.
fn monotone(iv: &RatInterval, f: fn(&Fraction) -> RatInterval) -> RatInterval {
    RatInterval::new(f(&iv.lo).lo, f(&iv.hi).hi)
}

/// The image of an interval under a function with Lipschitz constant 1.
fn lipschitz_one(iv: &RatInterval, f: fn(&Fraction) -> RatInterval) -> RatInterval {
    let two = Fraction::from(2);
    let mid = iv.lo.add(&iv.hi).div(&two);
    let half = iv.width().div(&two);
    let at_mid = f(&mid);
    RatInterval::new(at_mid.lo.sub(&half), at_mid.hi.add(&half))
}

impl ExactReal {
    /// `eˣ`. Always a value for a numeric argument that fits.
    pub fn exp(&self) -> Transcendental {
        if let Some(q) = self.as_rational() {
            if q.is_zero() {
                return Transcendental::Value(ExactReal::from_fraction(Fraction::from(1)));
            }
        }
        if !magnitude_within(self, EXP_ARGUMENT_BITS) {
            return Transcendental::SpaceExhausted;
        }
        Transcendental::Value(derived("exp", self, |iv| monotone(iv, exp_bounds)))
    }

    /// `ln x` for `x > 0`.
    pub fn ln(&self) -> Transcendental {
        match self {
            Self::Rational(q) => {
                if !q.is_positive() {
                    return Transcendental::DomainMiss;
                }
                if q == &Fraction::from(1) {
                    return Transcendental::Value(ExactReal::from_fraction(Fraction::from(0)));
                }
            }
            Self::Algebraic(a) => {
                if a.sign() != Ordering::Greater {
                    return Transcendental::DomainMiss;
                }
            }
            Self::Computable(c) => match tier2_sign(c) {
                Some(Ordering::Greater) => {}
                Some(_) => return Transcendental::DomainMiss,
                None => return Transcendental::Undecidable,
            },
        }
        Transcendental::Value(derived("ln", self, |iv| monotone(iv, ln_bounds)))
    }

    /// `sin x`, `x` in radians.
    pub fn sin(&self) -> Transcendental {
        self.trig(Fraction::from(0), |x| {
            derived("sin", x, |iv| lipschitz_one(iv, sin_bounds))
        })
    }

    /// `cos x`, `x` in radians.
    pub fn cos(&self) -> Transcendental {
        self.trig(Fraction::from(1), |x| {
            derived("cos", x, |iv| lipschitz_one(iv, cos_bounds))
        })
    }

    fn trig(&self, at_zero: Fraction, build: fn(&ExactReal) -> ExactReal) -> Transcendental {
        if let Some(q) = self.as_rational() {
            if q.is_zero() {
                return Transcendental::Value(ExactReal::from_fraction(at_zero));
            }
        }
        if !magnitude_within(self, TRIG_ARGUMENT_BITS) {
            return Transcendental::SpaceExhausted;
        }
        Transcendental::Value(build(self))
    }

    /// `atan x`. Always a value for a numeric argument.
    pub fn atan(&self) -> Transcendental {
        if let Some(q) = self.as_rational() {
            if q.is_zero() {
                return Transcendental::Value(ExactReal::from_fraction(Fraction::from(0)));
            }
        }
        Transcendental::Value(derived("atan", self, |iv| lipschitz_one(iv, atan_bounds)))
    }

    /// `1/x` for a Tier 2 `x` separated from zero within the internal
    /// budget; `None` when it could not be. Tier ≤ 1 reciprocals are exact
    /// and live in `reciprocal`.
    pub(crate) fn tier2_reciprocal(c: &Computable) -> Option<ExactReal> {
        tier2_sign(c)?;
        let source = c.clone();
        Some(ExactReal::Computable(Computable::from_base_bounds(
            "recip",
            move || {
                let iv = source.enclosure_at(BASE_BITS);
                outward(
                    &RatInterval::new(
                        Fraction::new(iv.hi.denominator(), iv.hi.numerator()),
                        Fraction::new(iv.lo.denominator(), iv.lo.numerator()),
                    ),
                    BASE_BITS,
                )
            },
        )))
    }
}
