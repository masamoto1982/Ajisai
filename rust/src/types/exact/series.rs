//! Rigorous rational enclosures of the elementary transcendental functions
//! at one rational point (LANG.VALUES.EXACT, Phase 7 of the vocabulary-100
//! work order).
//!
//! Every function here answers `[lo, hi]` with `lo ≤ f(q) ≤ hi` as exact
//! rationals, and nothing else: the enclosure is what a Tier 2 value is made
//! of, and a bound that is not rigorous would let a comparison decide a
//! wrong order with a straight face. The arithmetic is directed fixed point
//! at `BASE_BITS + GUARD_BITS` fractional bits — a lower chain rounds every
//! product and quotient toward −∞, an upper chain toward +∞ — so the
//! rounding of each step is inside the answer by construction and the
//! remainder of each series is added from its textbook bound:
//!
//!  * `exp`: `x = r·2ˢ` with `0 ≤ r ≤ ½`, `eʳ` by its Taylor series (positive
//!    terms, remainder ≤ the last term), then `s` squarings; a negative
//!    argument takes the reciprocal;
//!  * `ln`: `x = m·2ᵉ` with `⅔ ≤ m < 4⁄3`, `ln m = 2·atanh((m−1)/(m+1))` by
//!    the atanh series (`|y| ≤ 1⁄7`), plus `e·ln 2` from a cached `2·atanh(⅓)`;
//!  * `atan`: `atan x = π⁄2 − atan(1⁄x)` above 1, `atan ½ + atan z` above ½
//!    with `z ≤ ⅓`, then the alternating series, whose remainder is bounded
//!    by the first omitted term;
//!  * `sin`, `cos`: the argument reduced by `2kπ` through the 512-bit
//!    enclosure of π, then the Taylor series with the Lagrange remainder
//!    `|x|ⁿ⁺¹/(n+1)!`, which holds for every real argument.
//!
//! No floating point is used anywhere.

use std::sync::OnceLock;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

use crate::types::exact::observation::RatInterval;
use crate::types::exact::pi::pi_bounds;
use crate::types::fraction::Fraction;

/// Bits of precision every base enclosure is answered to. The same plateau
/// as π's, so a comparison between two transcendental values starves at the
/// same depth a comparison against π does.
pub(crate) const BASE_BITS: u64 = 512;
/// Extra bits the series work at, so the roundings they accumulate stay
/// below the answered precision.
const GUARD_BITS: u64 = 64;

fn int(n: i64) -> BigInt {
    BigInt::from(n)
}

/// Largest multiple of `2^-k` that is `≤ f`.
pub(crate) fn dyadic_floor(f: &Fraction, k: u64) -> Fraction {
    let scale = BigInt::one() << (k as usize);
    let floored = (f.numerator() * &scale).div_floor(&f.denominator());
    Fraction::new(floored, scale)
}

/// Smallest multiple of `2^-k` that is `≥ f`.
pub(crate) fn dyadic_ceil(f: &Fraction, k: u64) -> Fraction {
    let scale = BigInt::one() << (k as usize);
    let ceiled = (f.numerator() * &scale).div_ceil(&f.denominator());
    Fraction::new(ceiled, scale)
}

/// The interval rounded outward onto the `2^-k` grid.
pub(crate) fn outward(iv: &RatInterval, k: u64) -> RatInterval {
    RatInterval::new(dyadic_floor(&iv.lo, k), dyadic_ceil(&iv.hi, k))
}

/// Directed fixed-point arithmetic at `bits` fractional bits.
struct Fixed {
    bits: u64,
}

/// A fixed-point interval `[lo, hi]` (mantissas over one `Fixed` scale).
struct Fx {
    lo: BigInt,
    hi: BigInt,
}

impl Fixed {
    fn one(&self) -> BigInt {
        BigInt::one() << (self.bits as usize)
    }

    fn of(&self, f: &Fraction) -> Fx {
        let scaled = f.numerator() << (self.bits as usize);
        Fx {
            lo: scaled.div_floor(&f.denominator()),
            hi: scaled.div_ceil(&f.denominator()),
        }
    }

    fn mul_floor(&self, a: &BigInt, b: &BigInt) -> BigInt {
        (a * b).div_floor(&self.one())
    }

    fn mul_ceil(&self, a: &BigInt, b: &BigInt) -> BigInt {
        (a * b).div_ceil(&self.one())
    }

    fn fraction(&self, m: &BigInt) -> Fraction {
        Fraction::new(m.clone(), self.one())
    }

    fn interval(&self, x: &Fx) -> RatInterval {
        RatInterval::new(self.fraction(&x.lo), self.fraction(&x.hi))
    }
}

/// `eʳ` for `0 ≤ r ≤ ½`: `Σ rⁱ/i!`, positive terms, remainder after the
/// term that fell to one unit bounded by that term.
fn exp_series(fx: &Fixed, r: &Fx) -> Fx {
    let one = fx.one();
    let (mut t_lo, mut t_hi) = (one.clone(), one.clone());
    let (mut s_lo, mut s_hi) = (one.clone(), one);
    let mut i = BigInt::one();
    loop {
        t_lo = fx.mul_floor(&t_lo, &r.lo).div_floor(&i);
        t_hi = fx.mul_ceil(&t_hi, &r.hi).div_ceil(&i);
        s_lo += &t_lo;
        s_hi += &t_hi;
        if t_hi <= BigInt::one() {
            // Σ_{j>i} tⱼ ≤ tᵢ·(r/(i+1))/(1 − r/(i+2)) ≤ tᵢ for r ≤ ½.
            s_hi += 2;
            return Fx { lo: s_lo, hi: s_hi };
        }
        i += 1;
    }
}

/// `2·atanh(y)` for `0 ≤ y ≤ ⅓`: `2·Σ y²ⁱ⁺¹/(2i+1)`, positive terms,
/// remainder bounded by twice the first omitted term (ratio `y² ≤ ⅑`).
fn atanh_series(fx: &Fixed, y: &Fx) -> Fx {
    let (y2_lo, y2_hi) = (fx.mul_floor(&y.lo, &y.lo), fx.mul_ceil(&y.hi, &y.hi));
    let (mut p_lo, mut p_hi) = (y.lo.clone(), y.hi.clone());
    let (mut s_lo, mut s_hi) = (BigInt::zero(), BigInt::zero());
    let mut i: u64 = 0;
    loop {
        let t_lo = p_lo.div_floor(&int(2 * i as i64 + 1));
        let t_hi = p_hi.div_ceil(&int(2 * i as i64 + 1));
        s_lo += &t_lo;
        s_hi += &t_hi;
        if t_hi <= BigInt::one() {
            s_hi += 2;
            return Fx {
                lo: s_lo * 2,
                hi: s_hi * 2,
            };
        }
        p_lo = fx.mul_floor(&p_lo, &y2_lo);
        p_hi = fx.mul_ceil(&p_hi, &y2_hi);
        i += 1;
    }
}

/// An alternating series `Σ (−1)ⁱ tᵢ` with `tᵢ ≥ 0` given as directed
/// bounds by `terms`, which answers `None` once a term's upper bound has
/// fallen to one unit; the remainder is bounded by that term.
fn alternating(mut terms: impl FnMut() -> (BigInt, BigInt)) -> Fx {
    let (mut s_lo, mut s_hi) = (BigInt::zero(), BigInt::zero());
    let mut i: u64 = 0;
    loop {
        let (t_lo, t_hi) = terms();
        if i.is_multiple_of(2) {
            s_lo += &t_lo;
            s_hi += &t_hi;
        } else {
            s_lo -= &t_hi;
            s_hi -= &t_lo;
        }
        if t_hi <= BigInt::one() {
            return Fx {
                lo: s_lo - 1,
                hi: s_hi + 1,
            };
        }
        i += 1;
    }
}

/// `atan z` for `0 ≤ z ≤ ½`: `Σ (−1)ⁱ z²ⁱ⁺¹/(2i+1)`.
fn atan_series(fx: &Fixed, z: &Fx) -> Fx {
    let (z2_lo, z2_hi) = (fx.mul_floor(&z.lo, &z.lo), fx.mul_ceil(&z.hi, &z.hi));
    let (mut p_lo, mut p_hi) = (z.lo.clone(), z.hi.clone());
    let mut i: i64 = 0;
    alternating(|| {
        let term = (
            p_lo.div_floor(&int(2 * i + 1)),
            p_hi.div_ceil(&int(2 * i + 1)),
        );
        p_lo = fx.mul_floor(&p_lo, &z2_lo);
        p_hi = fx.mul_ceil(&p_hi, &z2_hi);
        i += 1;
        term
    })
}

/// `sin a` (`odd == true`) or `cos a` for `a ≥ 0`: the Taylor series, whose
/// Lagrange remainder is bounded by the first omitted term for every `a`.
fn trig_series(fx: &Fixed, a: &Fx, odd: bool) -> Fx {
    let (a2_lo, a2_hi) = (fx.mul_floor(&a.lo, &a.lo), fx.mul_ceil(&a.hi, &a.hi));
    let one = fx.one();
    let (mut t_lo, mut t_hi) = if odd {
        (a.lo.clone(), a.hi.clone())
    } else {
        (one.clone(), one)
    };
    let mut k: i64 = if odd { 1 } else { 0 };
    alternating(|| {
        let term = (t_lo.clone(), t_hi.clone());
        let divisor = int((k + 1) * (k + 2));
        t_lo = fx.mul_floor(&t_lo, &a2_lo).div_floor(&divisor);
        t_hi = fx.mul_ceil(&t_hi, &a2_hi).div_ceil(&divisor);
        k += 2;
        term
    })
}

/// Bit length of the integer part of `|f|` (0 for `|f| < 1`).
fn magnitude_bits(f: &Fraction) -> u64 {
    let (n, d) = f.to_bigint_pair();
    n.abs().div_floor(&d).bits()
}

/// `[lo, hi]` enclosing `eˣ`.
pub(crate) fn exp_bounds(x: &Fraction) -> RatInterval {
    if x.is_zero() {
        return RatInterval::point(Fraction::from(1));
    }
    let negative = !x.is_positive();
    let a = x.abs();
    // Halve until r ≤ ½; each squaring back doubles the relative error, and
    // the answer is as large as 2^(1.443·a), so work at that many more bits.
    let squarings = magnitude_bits(&a) + 1;
    let fx = Fixed {
        bits: BASE_BITS + GUARD_BITS + squarings + a_scaled_bits(&a),
    };
    let r = a.div(&Fraction::new(
        BigInt::one() << (squarings as usize),
        BigInt::one(),
    ));
    let mut e = exp_series(&fx, &fx.of(&r));
    for _ in 0..squarings {
        e = Fx {
            lo: fx.mul_floor(&e.lo, &e.lo),
            hi: fx.mul_ceil(&e.hi, &e.hi),
        };
    }
    let iv = fx.interval(&e);
    if negative {
        RatInterval::new(
            Fraction::new(iv.hi.denominator(), iv.hi.numerator()),
            Fraction::new(iv.lo.denominator(), iv.lo.numerator()),
        )
    } else {
        iv
    }
}

/// Bits `eᵃ` needs above the unit: `a·log₂e < 1.5·a`, plus the integer part.
fn a_scaled_bits(a: &Fraction) -> u64 {
    let (n, d) = a.to_bigint_pair();
    let whole = n.abs().div_ceil(&d);
    let whole: u64 = whole.to_string().parse().unwrap_or(u64::MAX / 4);
    whole.saturating_mul(3) / 2 + 4
}

/// `ln 2 = 2·atanh(⅓)`, computed once.
fn ln2_bounds() -> &'static RatInterval {
    static LN2: OnceLock<RatInterval> = OnceLock::new();
    LN2.get_or_init(|| {
        let fx = Fixed {
            bits: BASE_BITS + 2 * GUARD_BITS,
        };
        let third = fx.of(&Fraction::new(int(1), int(3)));
        fx.interval(&atanh_series(&fx, &third))
    })
}

/// `[lo, hi]` enclosing `ln x` for `x > 0`.
pub(crate) fn ln_bounds(x: &Fraction) -> RatInterval {
    debug_assert!(x.is_positive());
    if x == &Fraction::from(1) {
        return RatInterval::point(Fraction::from(0));
    }
    // x = m·2ᵉ with ⅔ ≤ m < 4⁄3.
    let (n, d) = x.to_bigint_pair();
    let mut e: i64 = n.bits() as i64 - d.bits() as i64;
    let two_to = |k: i64| -> Fraction {
        if k >= 0 {
            Fraction::new(BigInt::one() << (k as usize), BigInt::one())
        } else {
            Fraction::new(BigInt::one(), BigInt::one() << ((-k) as usize))
        }
    };
    let mut m = x.div(&two_to(e));
    let four_thirds = Fraction::new(int(4), int(3));
    let two_thirds = Fraction::new(int(2), int(3));
    while !m.lt(&four_thirds) {
        m = m.div(&Fraction::from(2));
        e += 1;
    }
    while m.lt(&two_thirds) {
        m = m.mul(&Fraction::from(2));
        e -= 1;
    }
    let one = Fraction::from(1);
    let y = m.sub(&one).div(&m.add(&one));
    let fx = Fixed {
        bits: BASE_BITS + GUARD_BITS + (e.unsigned_abs().max(1)).ilog2() as u64 + 1,
    };
    let series = atanh_series(&fx, &fx.of(&y.abs()));
    let ln_m = fx.interval(&series);
    let ln_m = if y.is_positive() {
        ln_m
    } else {
        RatInterval::new(
            Fraction::new(-ln_m.hi.numerator(), ln_m.hi.denominator()),
            Fraction::new(-ln_m.lo.numerator(), ln_m.lo.denominator()),
        )
    };
    let ln2 = ln2_bounds();
    let ef = Fraction::from(e);
    let (e_lo, e_hi) = if e >= 0 {
        (ef.mul(&ln2.lo), ef.mul(&ln2.hi))
    } else {
        (ef.mul(&ln2.hi), ef.mul(&ln2.lo))
    };
    RatInterval::new(ln_m.lo.add(&e_lo), ln_m.hi.add(&e_hi))
}

/// `atan ½`, computed once.
fn atan_half_bounds() -> &'static RatInterval {
    static HALF: OnceLock<RatInterval> = OnceLock::new();
    HALF.get_or_init(|| {
        let fx = Fixed {
            bits: BASE_BITS + 2 * GUARD_BITS,
        };
        let half = fx.of(&Fraction::new(int(1), int(2)));
        fx.interval(&atan_series(&fx, &half))
    })
}

fn negate(iv: &RatInterval) -> RatInterval {
    RatInterval::new(
        Fraction::new(-iv.hi.numerator(), iv.hi.denominator()),
        Fraction::new(-iv.lo.numerator(), iv.lo.denominator()),
    )
}

/// `[lo, hi]` enclosing `atan x`.
pub(crate) fn atan_bounds(x: &Fraction) -> RatInterval {
    if x.is_zero() {
        return RatInterval::point(Fraction::from(0));
    }
    if !x.is_positive() {
        return negate(&atan_bounds(&x.abs()));
    }
    let one = Fraction::from(1);
    if x.gt(&one) {
        // atan x = π/2 − atan(1/x).
        let inner = atan_bounds(&one.div(x));
        let (pi_lo, pi_hi) = pi_bounds();
        let two = Fraction::from(2);
        return RatInterval::new(
            pi_lo.div(&two).sub(&inner.hi),
            pi_hi.div(&two).sub(&inner.lo),
        );
    }
    let half = Fraction::new(int(1), int(2));
    let fx = Fixed {
        bits: BASE_BITS + GUARD_BITS,
    };
    if x.gt(&half) {
        // atan x = atan ½ + atan((x − ½)/(1 + x/2)), the reduced argument ≤ ⅓.
        let z = x.sub(&half).div(&one.add(&x.div(&Fraction::from(2))));
        let inner = fx.interval(&atan_series(&fx, &fx.of(&z)));
        let base = atan_half_bounds();
        return RatInterval::new(base.lo.add(&inner.lo), base.hi.add(&inner.hi));
    }
    fx.interval(&atan_series(&fx, &fx.of(x)))
}

/// `x − 2kπ` as an interval, with `k` the nearest integer to `x/2π`, so the
/// reduced argument lies within a hair of `[−π, π]`.
fn reduce_by_two_pi(x: &Fraction) -> RatInterval {
    let (pi_lo, pi_hi) = pi_bounds();
    let two = Fraction::from(2);
    let k = x
        .div(&two.mul(pi_lo))
        .add(&Fraction::new(int(1), int(2)))
        .floor();
    if k.is_zero() {
        return RatInterval::point(x.clone());
    }
    let twice_k = k.mul(&two);
    let a = x.sub(&twice_k.mul(pi_lo));
    let b = x.sub(&twice_k.mul(pi_hi));
    RatInterval::new(a, b)
}

/// `[lo, hi]` enclosing `sin x` (`odd == true`) or `cos x` — over the
/// reduced argument's interval, through the functions' Lipschitz constant 1.
fn trig_bounds(x: &Fraction, odd: bool) -> RatInterval {
    let reduced = reduce_by_two_pi(x);
    let two = Fraction::from(2);
    let mid = reduced.lo.add(&reduced.hi).div(&two);
    let half_width = reduced.width().div(&two);
    let fx = Fixed {
        bits: BASE_BITS + GUARD_BITS,
    };
    let at_mid = fx.interval(&trig_series(&fx, &fx.of(&mid.abs()), odd));
    let at_mid = if odd && !mid.is_positive() && !mid.is_zero() {
        negate(&at_mid)
    } else {
        at_mid
    };
    RatInterval::new(at_mid.lo.sub(&half_width), at_mid.hi.add(&half_width))
}

/// `[lo, hi]` enclosing `sin x`.
pub(crate) fn sin_bounds(x: &Fraction) -> RatInterval {
    if x.is_zero() {
        return RatInterval::point(Fraction::from(0));
    }
    trig_bounds(x, true)
}

/// `[lo, hi]` enclosing `cos x`.
pub(crate) fn cos_bounds(x: &Fraction) -> RatInterval {
    if x.is_zero() {
        return RatInterval::point(Fraction::from(1));
    }
    trig_bounds(x, false)
}
