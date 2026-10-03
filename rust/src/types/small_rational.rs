//! Arithmetic on rationals whose numerator and denominator each fit a
//! machine word: the `Small` form `Fraction` stores them in, held as an
//! `(i64, i64)` pair in lowest terms with a positive denominator.
//!
//! `Fraction`'s own small path dispatches on its enum, widens to `i128` and
//! re-normalises through `create_from_i128` on every operation. The kernels
//! that hold many such values unboxed (`dense_kernels`, the fused walk's
//! `fused_block_rat`) use these instead: Henrici's sum and cross-cancelled
//! product and quotient, with gcds and quotients in machine words and only
//! the final products widened. Each answers `None` when its result no longer
//! fits a pair (or, for `div`, on a zero divisor), for the caller to hand the
//! work to `Fraction`. A rational's lowest-terms form with a positive
//! denominator is unique, so these agree with `Fraction` wherever they
//! answer; `fraction_gcd_tests` holds them to it.

use crate::types::fraction::{binary_gcd_u128, binary_gcd_u64};
use std::cmp::Ordering;

/// A rational in lowest terms with a positive denominator.
pub(crate) type Pair = (i64, i64);

#[inline]
pub(crate) fn fit(n: i128, d: i128) -> Option<Pair> {
    Some((i64::try_from(n).ok()?, i64::try_from(d).ok()?))
}

/// `a ± b` by Henrici's algorithm, as `Fraction::add` reduces a wide sum:
/// `g = gcd(b, d)` first, and when it is 1 the sum is already reduced.
/// The gcds and quotients stay in machine words where the operands do.
pub(crate) fn add((an, ad): Pair, (bn, bd): Pair, subtract: bool) -> Option<Pair> {
    let bn = if subtract {
        -i128::from(bn)
    } else {
        i128::from(bn)
    };
    if ad == 1 && bd == 1 {
        return fit(i128::from(an) + bn, 1);
    }
    let g = gcd64(ad, bd);
    if g == 1 {
        let n = i128::from(an) * i128::from(bd) + bn * i128::from(ad);
        if n == 0 {
            return Some((0, 1));
        }
        return fit(n, i128::from(ad) * i128::from(bd));
    }
    let (s, bd_g) = (ad / g, bd / g);
    let t = i128::from(an) * i128::from(bd_g) + bn * i128::from(s);
    if t == 0 {
        return Some((0, 1));
    }
    let (n, g2) = match i64::try_from(t) {
        Ok(t) => {
            let g2 = gcd64(t, g);
            (i128::from(t / g2), g2)
        }
        Err(_) => {
            let g2 = binary_gcd_u128(t.unsigned_abs(), g.unsigned_abs().into()) as i64;
            (t / i128::from(g2), g2)
        }
    };
    fit(n, i128::from(s) * i128::from(bd / g2))
}

/// `gcd(|a|, b)` for `b > 0`, in machine words.
#[inline]
fn gcd64(a: i64, b: i64) -> i64 {
    // `b` is a positive denominator, so the gcd divides it and fits.
    binary_gcd_u64(a.unsigned_abs(), b.unsigned_abs()) as i64
}

/// `a × b` with the cross gcds divided out first, so the product is already
/// in lowest terms. The gcds and quotients stay in machine words — an `i128`
/// division is a software routine — and only the products widen.
pub(crate) fn mul((an, ad): Pair, (bn, bd): Pair) -> Option<Pair> {
    // An integer half has nothing to cancel against: gcd(x, 1) is 1.
    let g1 = if bd == 1 { 1 } else { gcd64(an, bd) };
    let g2 = if ad == 1 { 1 } else { gcd64(bn, ad) };
    fit(
        i128::from(an / g1) * i128::from(bn / g2),
        i128::from(ad / g2) * i128::from(bd / g1),
    )
}

/// `a ÷ b`: the cross gcds divided out as in `mul`, with `b`'s halves
/// exchanged and the sign moved to the numerator. `None` for a zero divisor.
pub(crate) fn div((an, ad): Pair, (bn, bd): Pair) -> Option<Pair> {
    if bn == 0 {
        return None;
    }
    // Two integers where the divisor divides: the quotient is an integer
    // (`checked_*` declines i64::MIN / -1, which the general route widens).
    if ad == 1 && bd == 1 && an.checked_rem(bn) == Some(0) {
        if let Some(q) = an.checked_div(bn) {
            return Some((q, 1));
        }
    }
    let g2 = if ad == 1 || bd == 1 { 1 } else { gcd64(bd, ad) };
    // gcd(|a|, |b|) is 2^63 only for halves drawn from {0, i64::MIN}; that
    // one case divides in `i128`.
    let g1 = binary_gcd_u64(an.unsigned_abs(), bn.unsigned_abs());
    let (an_g, bn_g) = match i64::try_from(g1) {
        Ok(g1) => (i128::from(an / g1), i128::from(bn / g1)),
        Err(_) => (
            i128::from(an) / i128::from(g1),
            i128::from(bn) / i128::from(g1),
        ),
    };
    let n = an_g * i128::from(bd / g2);
    let d = i128::from(ad / g2) * bn_g;
    if d < 0 {
        fit(-n, -d)
    } else {
        fit(n, d)
    }
}

#[inline]
pub(crate) fn order((an, ad): Pair, (bn, bd): Pair) -> Ordering {
    (i128::from(an) * i128::from(bd)).cmp(&(i128::from(bn) * i128::from(ad)))
}
