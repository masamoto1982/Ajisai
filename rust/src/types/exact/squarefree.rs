//! The square-free part of a radicand, so that one number has one normal form.
//!
//! `√n` is written `s·√m` with `n = s²·m` and `m` square-free. Monomials are
//! then subset products of square-free, pairwise-coprime basis elements, which
//! are themselves square-free integers — so a monomial is decided by the value
//! alone, whatever basis the value happens to carry (`√6` is keyed `6` over the
//! basis `{6}` and over `{2, 3}` alike). Without this, `8 SQRT` kept the
//! monomial `8` while `2 SQRT 2 SQRT ADD` kept `2`: one value, two displays and
//! two protocol normal forms, which LANG.VALUES.DENOTATION rules out.
//!
//! Extracting square factors needs the prime factorization, which has no known
//! cheap algorithm for a large integer. The work is therefore metered: every
//! step is charged against a budget the caller supplies (the run's remaining
//! `numericWork`), and a radicand the budget cannot factor is refused as a
//! resource limit rather than left in a non-canonical form. Trial division
//! takes the small primes, a perfect-square test and a primality test settle
//! the common large cofactors, and Pollard's rho (Brent's variant) splits the
//! rest.
//!
//! Primality is decided by Miller–Rabin over the first thirteen prime bases,
//! which is a proof below 3.3·10²⁴. Above that it is a strong probable-prime
//! test with no known counterexample; the only consequence of misjudging a
//! composite would be a non-square-free radicand — a display that differs,
//! never a value that is wrong, since equality and order do not depend on the
//! basis being square-free (`basis.rs`).

use crate::types::bigint_gcd::balanced_bigint_gcd;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

/// The work budget ran out before the radicand was factored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactorBudgetExhausted;

/// Primes below which the cofactor is cleared by trial division.
const TRIAL_BOUND: u64 = 1 << 12;
const MR_BASES: [u64; 13] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41];

/// What one factoring step costs, in the `numericWork` limb-multiply unit, on
/// top of its limb count squared. A step is a bignum multiply, a remainder, a
/// subtraction and the allocations between them — about as much as the
/// rational addition the unit is calibrated on, several times over — so it is
/// priced by measurement (`squarefree` timing in `docs/dev`), to within the
/// order of magnitude that keeps the ceiling meaningful.
const STEP_UNITS: u64 = 8;

/// A work meter over `budget`: each modular step costs `STEP_UNITS` times its
/// limb count squared.
struct Meter<'a> {
    budget: &'a mut u64,
}

impl Meter<'_> {
    fn charge(&mut self, n: &BigInt) -> Result<(), FactorBudgetExhausted> {
        let limbs = n.bits().div_ceil(64).max(1);
        let units = limbs.saturating_mul(limbs).saturating_mul(STEP_UNITS);
        if *self.budget < units {
            *self.budget = 0;
            return Err(FactorBudgetExhausted);
        }
        *self.budget -= units;
        Ok(())
    }
}

/// `n = s²·m` with `m` square-free, for `n ≥ 1`: returns `(s, m)`.
pub fn squarefree_split(
    n: &BigInt,
    budget: &mut u64,
) -> Result<(BigInt, BigInt), FactorBudgetExhausted> {
    let mut meter = Meter { budget };
    let mut primes: Vec<(BigInt, u32)> = Vec::new();
    let mut rest = n.clone();

    let mut d: u64 = 2;
    while d < TRIAL_BOUND {
        let big_d = BigInt::from(d);
        if &big_d * &big_d > rest {
            break;
        }
        meter.charge(&rest)?;
        let mut exponent = 0;
        while (&rest % &big_d).is_zero() {
            rest /= &big_d;
            exponent += 1;
        }
        if exponent > 0 {
            primes.push((big_d, exponent));
        }
        d += if d == 2 { 1 } else { 2 };
    }
    factor_into(&rest, 1, &mut primes, &mut meter)?;

    let mut s = BigInt::one();
    let mut m = BigInt::one();
    primes.sort();
    let mut i = 0;
    while i < primes.len() {
        let p = primes[i].0.clone();
        let mut exponent = 0;
        while i < primes.len() && primes[i].0 == p {
            exponent += primes[i].1;
            i += 1;
        }
        s *= num_traits::pow(p.clone(), (exponent / 2) as usize);
        if exponent % 2 == 1 {
            m *= p;
        }
    }
    Ok((s, m))
}

/// Push the prime factors of `n` (every one ≥ `TRIAL_BOUND`, or `n` is 1),
/// each with `multiplicity`.
fn factor_into(
    n: &BigInt,
    multiplicity: u32,
    primes: &mut Vec<(BigInt, u32)>,
    meter: &mut Meter,
) -> Result<(), FactorBudgetExhausted> {
    if n.is_one() {
        return Ok(());
    }
    let bound = BigInt::from(TRIAL_BOUND);
    if n < &(&bound * &bound) {
        // No factor below the trial bound and below its square: prime.
        primes.push((n.clone(), multiplicity));
        return Ok(());
    }
    let root = n.sqrt();
    meter.charge(n)?;
    if &root * &root == *n {
        return factor_into(&root, multiplicity * 2, primes, meter);
    }
    if is_probable_prime(n, meter)? {
        primes.push((n.clone(), multiplicity));
        return Ok(());
    }
    let d = pollard_brent(n, meter)?;
    factor_into(&d, multiplicity, primes, meter)?;
    factor_into(&(n / &d), multiplicity, primes, meter)
}

fn is_probable_prime(n: &BigInt, meter: &mut Meter) -> Result<bool, FactorBudgetExhausted> {
    let one = BigInt::one();
    let n_minus_1 = n - &one;
    let mut d = n_minus_1.clone();
    let mut r = 0u32;
    while d.is_even() {
        d >>= 1;
        r += 1;
    }
    'bases: for &a in &MR_BASES {
        let a = BigInt::from(a);
        if (&a % n).is_zero() {
            continue;
        }
        for _ in 0..n.bits() {
            meter.charge(n)?;
        }
        let mut x = a.modpow(&d, n);
        if x.is_one() || x == n_minus_1 {
            continue;
        }
        for _ in 1..r {
            meter.charge(n)?;
            x = (&x * &x) % n;
            if x == n_minus_1 {
                continue 'bases;
            }
        }
        return Ok(false);
    }
    Ok(true)
}

/// A non-trivial factor of the composite `n`, by Pollard's rho with Brent's
/// cycle detection, trying successive polynomial constants.
fn pollard_brent(n: &BigInt, meter: &mut Meter) -> Result<BigInt, FactorBudgetExhausted> {
    let one = BigInt::one();
    for c in 1u64.. {
        let c = BigInt::from(c);
        let f = |x: &BigInt| (x * x + &c) % n;
        let mut y = BigInt::from(2);
        let mut r: u64 = 1;
        let mut q = BigInt::one();
        let mut g = BigInt::one();
        let mut x = y.clone();
        let mut ys = y.clone();
        const M: u64 = 128;
        while g.is_one() {
            x = y.clone();
            for _ in 0..r {
                meter.charge(n)?;
                y = f(&y);
            }
            let mut k = 0;
            while k < r && g.is_one() {
                ys = y.clone();
                for _ in 0..M.min(r - k) {
                    meter.charge(n)?;
                    y = f(&y);
                    q = (q * (&x - &y).abs()) % n;
                }
                g = balanced_bigint_gcd(&q, n);
                k += M;
            }
            r *= 2;
        }
        if g == *n {
            loop {
                meter.charge(n)?;
                ys = f(&ys);
                g = balanced_bigint_gcd(&(&x - &ys).abs(), n);
                if !g.is_one() {
                    break;
                }
            }
        }
        if g != *n && g != one {
            return Ok(g);
        }
        // This constant cycled without splitting; try the next one.
        if c.to_u64().unwrap_or(u64::MAX) > 64 {
            return Err(FactorBudgetExhausted);
        }
    }
    unreachable!("the constant loop only exits by returning")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(n: u128) -> (u128, u128) {
        let mut budget = u64::MAX;
        let (s, m) = squarefree_split(&BigInt::from(n), &mut budget).unwrap();
        (s.to_u128().unwrap(), m.to_u128().unwrap())
    }

    #[test]
    fn small_radicands_split_into_square_and_square_free_parts() {
        assert_eq!(split(8), (2, 2));
        assert_eq!(split(12), (2, 3));
        assert_eq!(split(72), (6, 2));
        assert_eq!(split(30), (1, 30));
        assert_eq!(split(1), (1, 1));
        assert_eq!(split(49), (7, 1));
    }

    #[test]
    fn large_square_factors_are_found_past_trial_division() {
        let p: u128 = 1_000_003; // prime above the trial bound
        let q: u128 = 998_244_353; // prime
        assert_eq!(split(p * p * q), (p, q));
        assert_eq!(split(p * q), (1, p * q));
        assert_eq!(split(p * p * p), (p, p));
    }

    #[test]
    fn an_exhausted_budget_refuses_rather_than_guessing() {
        let p: u128 = 1_000_003;
        let q: u128 = 998_244_353;
        let mut budget = 10;
        assert_eq!(
            squarefree_split(&BigInt::from(p * p * q), &mut budget),
            Err(FactorBudgetExhausted)
        );
    }
}
