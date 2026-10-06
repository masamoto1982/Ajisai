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

use crate::types::fraction_arithmetic::balanced_bigint_gcd;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

/// The work budget ran out before the radicand was factored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactorBudgetExhausted;

/// Primes below which the cofactor is cleared by trial division.
const TRIAL_BOUND: u64 = 1 << 12;
const MR_BASES: [u64; 13] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41];

/// The primes below `TRIAL_BOUND`, packed in ascending order into groups whose
/// product fits one word — the binary counterpart of testing a decimal number
/// for 3 and 9 at once by its digit sum: one wide remainder by a group's
/// product answers divisibility by every prime in it, by word remainders.
struct TrialGroups {
    /// The group of each prime below `TRIAL_BOUND`; `NOT_PRIME` elsewhere.
    group_of: [u16; TRIAL_BOUND as usize],
    /// The product of each group's primes; unused entries past the last are 0.
    products: [u64; TRIAL_BOUND as usize],
}

const NOT_PRIME: u16 = u16::MAX;

static TRIAL_GROUPS: TrialGroups = trial_groups();

const fn trial_groups() -> TrialGroups {
    let mut groups = TrialGroups {
        group_of: [NOT_PRIME; TRIAL_BOUND as usize],
        products: [0; TRIAL_BOUND as usize],
    };
    let mut group = 0usize;
    let mut product: u64 = 1;
    let mut d = 2u64;
    while d < TRIAL_BOUND {
        let mut prime = true;
        let mut k = 2u64;
        while k * k <= d {
            if d.is_multiple_of(k) {
                prime = false;
                break;
            }
            k += 1;
        }
        if prime {
            if product.checked_mul(d).is_none() {
                groups.products[group] = product;
                group += 1;
                product = 1;
            }
            product *= d;
            groups.group_of[d as usize] = group as u16;
        }
        d += 1;
    }
    groups.products[group] = product;
    groups
}

/// `|n| mod m`: on `u128` when `n` fits, else by reciprocal.
fn residue_mod(n: &BigInt, m: u64) -> u64 {
    match n.magnitude().to_u128() {
        Some(v) => (v % u128::from(m)) as u64,
        None => crate::types::small_divisor::rem_word(n, m),
    }
}

/// `d² > rest`, for `d < TRIAL_BOUND`, without building `d` wide.
fn square_exceeds(d: u64, rest: &BigInt) -> bool {
    rest.to_u64().is_some_and(|r| d * d > r)
}

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
    // The residue of `rest` modulo the current group's product, once per
    // group rather than one wide remainder per candidate; dropped whenever a
    // factor comes out of `rest`.
    let mut residue: Option<(usize, u64)> = None;
    while d < TRIAL_BOUND {
        if square_exceeds(d, &rest) {
            break;
        }
        meter.charge(&rest)?;
        let group = TRIAL_GROUPS.group_of[d as usize];
        if group != NOT_PRIME {
            let group = usize::from(group);
            let r = match residue {
                Some((g, r)) if g == group => r,
                _ => {
                    let r = residue_mod(&rest, TRIAL_GROUPS.products[group]);
                    residue = Some((group, r));
                    r
                }
            };
            if r % d == 0 {
                let big_d = BigInt::from(d);
                let mut exponent = 0;
                while (&rest % &big_d).is_zero() {
                    rest /= &big_d;
                    exponent += 1;
                }
                primes.push((big_d, exponent));
                residue = None;
            }
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

    /// The grouped residues find every small prime the one-by-one remainder
    /// did: a wide radicand built from primes at each group's edges, and every
    /// radicand below a bound checked against a naive split.
    #[test]
    fn grouped_trial_division_agrees_with_one_remainder_per_prime() {
        let naive = |mut n: u128| {
            let (mut s, mut m, mut d) = (1u128, 1u128, 2u128);
            while d * d <= n {
                let mut e = 0;
                while n.is_multiple_of(d) {
                    n /= d;
                    e += 1;
                }
                s *= d.pow(e / 2);
                if e % 2 == 1 {
                    m *= d;
                }
                d += 1;
            }
            (s, m * n)
        };
        for n in 1..20_000u128 {
            assert_eq!(split(n), naive(n), "{n}");
        }
        // The first and last prime of every group.
        let group_of = &TRIAL_GROUPS.group_of;
        let primes: Vec<usize> = (2..TRIAL_BOUND as usize)
            .filter(|&d| group_of[d] != NOT_PRIME)
            .collect();
        let edges: Vec<u64> = primes
            .iter()
            .enumerate()
            .filter(|&(i, &p)| {
                let before = i.checked_sub(1).map(|j| group_of[primes[j]]);
                let after = primes.get(i + 1).map(|&q| group_of[q]);
                before != Some(group_of[p]) || after != Some(group_of[p])
            })
            .map(|(_, &p)| p as u64)
            .collect();
        assert!(edges.len() > 10);
        let mut wide = BigInt::from(1_000_003u64);
        let mut square = BigInt::one();
        for (i, &p) in edges.iter().enumerate() {
            let e = 1 + (i % 3) as u32;
            wide *= num_traits::pow(BigInt::from(p), e as usize);
            square *= num_traits::pow(BigInt::from(p), (e / 2) as usize);
        }
        let mut budget = u64::MAX;
        let (s, _) = squarefree_split(&wide, &mut budget).unwrap();
        assert_eq!(s, square);
    }
}
