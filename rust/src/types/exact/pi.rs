//! Tier 2 constant π as a rigorous rational enclosure (Phase 7).
//!
//! π has no algebraic normal form, so it lives in Tier 2: a lazily refined
//! shrinking rational enclosure (`Computable`). Its base bounds are computed
//! once by Machin's formula `π = 16·arctan(1/5) − 4·arctan(1/239)`, each
//! arctangent summed as its alternating Taylor series with exact `Fraction`
//! arithmetic. **No floating point is used anywhere** (SPEC §14.4): the
//! endpoints are exact rationals and the enclosure is rigorous by the
//! alternating-series bracketing theorem — the limit of an alternating series
//! whose terms decrease monotonically to zero lies between any two consecutive
//! partial sums, so `[min(Sₙ, Sₙ₊₁), max(Sₙ, Sₙ₊₁)]` encloses it.
//!
//! Invariants (SPEC §4.2; Phase 7 §14.4):
//! - deterministic — the same step yields the same interval;
//! - rational endpoints;
//! - nested — `enclosure(k+1) ⊆ enclosure(k)`;
//! - always contains π;
//! - width is monotonically non-increasing and converges to the base width.
//!
//! The base bounds hold to ~`PI_PRECISION_BITS` bits. The generator refines by
//! dyadic outward rounding, nesting and shrinking toward the base enclosure and
//! plateauing there once the grid is finer than the base precision. So a
//! comparison needing finer than ~`PI_PRECISION_BITS`-bit separation starves
//! honestly (the sole source of the logical Unknown), and per-step cost stays
//! bounded for any water budget — no unbounded recomputation.

use std::sync::OnceLock;

use num_bigint::BigInt;
use num_integer::Integer;

use crate::types::exact::computable::Computable;
use crate::types::exact::observation::RatInterval;
use crate::types::fraction::Fraction;

/// Bits of precision of the base π enclosure. Beyond this the generator
/// plateaus (see module docs). 512 bits ≈ 154 decimal digits — far beyond any
/// value the current vocabulary can construct to compare against.
const PI_PRECISION_BITS: u64 = 512;

fn int(n: i64) -> BigInt {
    BigInt::from(n)
}

/// Partial sum of the first `terms` terms of
/// `arctan(1/m) = Σ_{k≥0} (-1)^k / ((2k+1)·m^(2k+1))`, as an exact `Fraction`.
fn arctan_partial_sum(m: u32, terms: u32) -> Fraction {
    let m_big = BigInt::from(m);
    let mut acc = Fraction::new(int(0), int(1));
    for k in 0..terms {
        let exp = 2 * k + 1;
        let denom = BigInt::from(exp) * m_big.pow(exp);
        let num = if k % 2 == 0 { int(1) } else { int(-1) };
        acc = acc.add(&Fraction::new(num, denom));
    }
    acc
}

/// Rigorous enclosure `[lo, hi]` with `lo ≤ arctan(1/m) ≤ hi`, from two
/// consecutive partial sums (alternating-series bracketing). `terms ≥ 1`; the
/// width is the first omitted term, on the order of `m^-(2·terms)`.
fn arctan_enclosure(m: u32, terms: u32) -> (Fraction, Fraction) {
    let s_n = arctan_partial_sum(m, terms);
    let s_n1 = arctan_partial_sum(m, terms + 1);
    if s_n.le(&s_n1) {
        (s_n, s_n1)
    } else {
        (s_n1, s_n)
    }
}

/// Base rigorous rational bounds `(lo, hi)` with `lo ≤ π ≤ hi`, computed once.
fn pi_bounds() -> &'static (Fraction, Fraction) {
    static BOUNDS: OnceLock<(Fraction, Fraction)> = OnceLock::new();
    BOUNDS.get_or_init(|| {
        // Term counts make each arctangent's width « 2^-PI_PRECISION_BITS:
        // arctan(1/5) ~ 5^-(2·140), arctan(1/239) ~ 239^-(2·60).
        let (a_lo, a_hi) = arctan_enclosure(5, 140);
        let (b_lo, b_hi) = arctan_enclosure(239, 60);
        let sixteen = Fraction::new(int(16), int(1));
        let four = Fraction::new(int(4), int(1));
        // π = 16·A − 4·B (A = arctan(1/5), B = arctan(1/239)), so the sound
        // combination is π ∈ [16·a_lo − 4·b_hi, 16·a_hi − 4·b_lo].
        let lo = sixteen.mul(&a_lo).sub(&four.mul(&b_hi));
        let hi = sixteen.mul(&a_hi).sub(&four.mul(&b_lo));
        (lo, hi)
    })
}

/// Largest multiple of `2^-k` that is `≤ f` (dyadic floor).
fn dyadic_floor(f: &Fraction, k: u64) -> Fraction {
    let scale = int(1) << (k as usize);
    let floored = (f.numerator() * &scale).div_floor(&f.denominator());
    Fraction::new(floored, scale)
}

/// Smallest multiple of `2^-k` that is `≥ f` (dyadic ceil).
fn dyadic_ceil(f: &Fraction, k: u64) -> Fraction {
    let scale = int(1) << (k as usize);
    let ceiled = (f.numerator() * &scale).div_ceil(&f.denominator());
    Fraction::new(ceiled, scale)
}

/// π as a Tier 2 computable real. The generator outward-rounds the base bounds
/// to a `2^-step` dyadic grid — nesting and shrinking toward the base
/// enclosure, plateauing there once the grid is finer than the base precision.
pub fn pi() -> Computable {
    Computable::from_enclosures("pi", |step| {
        let (lo, hi) = pi_bounds();
        if step >= PI_PRECISION_BITS {
            return RatInterval::new(lo.clone(), hi.clone());
        }
        RatInterval::new(dyadic_floor(lo, step), dyadic_ceil(hi, step))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frac(n: i64, d: i64) -> Fraction {
        Fraction::new(int(n), int(d))
    }

    #[test]
    fn base_bounds_bracket_known_pi_rationals() {
        let (lo, hi) = pi_bounds();
        // 22/7 ≈ 3.142857 is a known upper bound; 333/106 ≈ 3.141509 a known
        // lower bound. π ∈ (333/106, 22/7), so our sound bounds must sit inside.
        assert!(lo.lt(hi), "lo < hi");
        assert!(frac(333, 106).lt(lo), "lo must exceed 333/106 < π");
        assert!(hi.lt(&frac(22, 7)), "hi must be below 22/7 > π");
    }

    #[test]
    fn enclosures_are_nested_and_contain_pi() {
        let (base_lo, base_hi) = pi_bounds();
        let pi = pi();
        let mut prev = pi.enclosure_at(0);
        // Step 0 is the coarsest [3, 4]; π ∈ [base_lo, base_hi] ⊆ every step.
        assert!(prev.lo.le(base_lo) && base_hi.le(&prev.hi));
        for step in 1..40 {
            let now = pi.enclosure_at(step);
            assert!(now.is_within(&prev), "enclosure {step} must nest");
            assert!(
                now.lo.le(base_lo) && base_hi.le(&now.hi),
                "π stays enclosed at step {step}"
            );
            assert!(!now.width().lt(&frac(0, 1)), "width non-negative");
            prev = now;
        }
    }

    #[test]
    fn width_is_monotonically_non_increasing() {
        let pi = pi();
        let mut prev = pi.enclosure_at(0).width();
        for step in 1..40 {
            let w = pi.enclosure_at(step).width();
            assert!(!prev.lt(&w), "width must not grow at step {step}");
            prev = w;
        }
    }

    #[test]
    fn generator_is_deterministic() {
        let a = pi();
        let b = pi();
        for step in [0u64, 1, 7, 33, 200, 600] {
            assert_eq!(a.enclosure_at(step), b.enclosure_at(step));
        }
    }
}
