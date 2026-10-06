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

const fn is_prime(d: u64) -> bool {
    let mut k = 2u64;
    while k * k <= d {
        if d.is_multiple_of(k) {
            return false;
        }
        k += 1;
    }
    d >= 2
}

const fn count_primes() -> usize {
    let mut count = 0;
    let mut d = 2;
    while d < TRIAL_BOUND {
        if is_prime(d) {
            count += 1;
        }
        d += 1;
    }
    count
}

const PRIME_COUNT: usize = count_primes();

/// A prime below `TRIAL_BOUND`, with what tests a word for it without a
/// division: for odd `p`, `p | r` exactly when `r·p⁻¹ mod 2⁶⁴ ≤ ⌊(2⁶⁴−1)/p⌋`,
/// because multiplying by `p⁻¹` maps the multiples of `p` onto `0..=limit`
/// and everything else above it (Granlund and Montgomery, "Division by
/// invariant integers using multiplication", 1994, §9).
#[derive(Clone, Copy)]
struct TrialPrime {
    p: u64,
    /// `p⁻¹ mod 2⁶⁴` (unused for 2).
    inverse: u64,
    /// `⌊(2⁶⁴−1)/p⌋`.
    limit: u64,
    /// The group whose product `p` is in.
    group: u16,
}

impl TrialPrime {
    #[inline]
    fn divides(&self, r: u64) -> bool {
        if self.p == 2 {
            r & 1 == 0
        } else {
            r.wrapping_mul(self.inverse) <= self.limit
        }
    }
}

/// The primes below `TRIAL_BOUND`, packed in ascending order into groups whose
/// product fits one word — the binary counterpart of testing a decimal number
/// for 3 and 9 at once by its digit sum: one wide remainder by a group's
/// product leaves a word that every prime of the group then tests.
struct TrialTable {
    primes: [TrialPrime; PRIME_COUNT],
    /// The product of each group's primes; unused entries past the last are 0.
    products: [u64; PRIME_COUNT],
}

static TRIAL_TABLE: TrialTable = trial_table();

const fn trial_table() -> TrialTable {
    let blank = TrialPrime {
        p: 0,
        inverse: 0,
        limit: 0,
        group: 0,
    };
    let mut table = TrialTable {
        primes: [blank; PRIME_COUNT],
        products: [0; PRIME_COUNT],
    };
    let mut index = 0;
    let mut group = 0usize;
    let mut product: u64 = 1;
    let mut d = 2u64;
    while d < TRIAL_BOUND {
        if is_prime(d) {
            if product.checked_mul(d).is_none() {
                table.products[group] = product;
                group += 1;
                product = 1;
            }
            product *= d;
            // Newton's iteration doubles the correct low bits each step.
            let mut inverse = d;
            let mut step = 0;
            while step < 5 {
                inverse = inverse.wrapping_mul(2u64.wrapping_sub(d.wrapping_mul(inverse)));
                step += 1;
            }
            table.primes[index] = TrialPrime {
                p: d,
                inverse,
                limit: u64::MAX / d,
                group: group as u16,
            };
            index += 1;
        }
        d += 1;
    }
    table.products[group] = product;
    table
}

/// How many trial candidates — 2, then every odd number — are at most `x`.
fn candidates_through(x: u64) -> u64 {
    if x < 2 {
        0
    } else {
        1 + (x - 1) / 2
    }
}

/// `|n| mod m`: on `u128` when `n` fits, else by reciprocal.
fn residue_mod(n: &BigInt, m: u64) -> u64 {
    match n.magnitude().to_u128() {
        Some(v) => (v % u128::from(m)) as u64,
        None => crate::types::small_divisor::rem_word(n, m),
    }
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
        self.charge_units(Self::units(n))
    }

    /// What one step on `n` costs.
    fn units(n: &BigInt) -> u64 {
        let limbs = n.bits().div_ceil(64).max(1);
        limbs.saturating_mul(limbs).saturating_mul(STEP_UNITS)
    }

    fn charge_units(&mut self, units: u64) -> Result<(), FactorBudgetExhausted> {
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

    // Each candidate below `TRIAL_BOUND` — 2, then every odd number, until
    // its square passes `rest` — is charged one step on `rest`, as when each
    // was divided in turn. Only the primes are tested; the composites between
    // them are charged together, which runs out of budget exactly when
    // charging them one by one would.
    let mut charged: u64 = 0;
    // The residue of `rest` modulo the current group's product, once per
    // group rather than one wide remainder per candidate; dropped whenever a
    // factor comes out of `rest`.
    let mut residue: Option<(u16, u64)> = None;
    // What a step costs, `rest` as a word and the last candidate its square
    // root admits, all fixed until a factor comes out of `rest`.
    let mut units = Meter::units(&rest);
    let mut rest_word = rest.to_u64();
    let mut last = rest_word.map_or(u64::MAX, u64::isqrt);
    for prime in TRIAL_TABLE
        .primes
        .iter()
        .map(Some)
        .chain(std::iter::once(None))
    {
        let through = prime.map_or(TRIAL_BOUND - 1, |prime| prime.p);
        let reach = through.min(last);
        let due = candidates_through(reach).saturating_sub(charged);
        meter.charge_units(units.saturating_mul(due))?;
        charged += due;
        let Some(prime) = prime.filter(|prime| prime.p == reach) else {
            break;
        };
        let r = match (rest_word, residue) {
            (Some(word), _) => word,
            (None, Some((group, r))) if group == prime.group => r,
            (None, _) => {
                let r = residue_mod(&rest, TRIAL_TABLE.products[usize::from(prime.group)]);
                residue = Some((prime.group, r));
                r
            }
        };
        if prime.divides(r) {
            let big_d = BigInt::from(prime.p);
            let mut exponent = 0;
            while (&rest % &big_d).is_zero() {
                rest /= &big_d;
                exponent += 1;
            }
            primes.push((big_d, exponent));
            residue = None;
            units = Meter::units(&rest);
            rest_word = rest.to_u64();
            last = rest_word.map_or(u64::MAX, u64::isqrt);
        }
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
    meter.charge(n)?;
    if let Some(root) = exact_sqrt(n) {
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
    match n.to_u64() {
        Some(word) => is_probable_prime_word(word, n, meter),
        None => is_probable_prime_wide(n, meter),
    }
}

fn is_probable_prime_wide(n: &BigInt, meter: &mut Meter) -> Result<bool, FactorBudgetExhausted> {
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
    match n.to_u64() {
        Some(word) => pollard_brent_word(word, n, meter).map(BigInt::from),
        None => pollard_brent_wide(n, meter),
    }
}

fn pollard_brent_wide(n: &BigInt, meter: &mut Meter) -> Result<BigInt, FactorBudgetExhausted> {
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

/// Arithmetic modulo an odd word `n` in Montgomery form (`x` held as
/// `x·2⁶⁴ mod n`), where a product is reduced by multiplications and a shift
/// rather than a 128-by-64-bit division (Montgomery, "Modular multiplication
/// without trial division", 1985).
struct Montgomery {
    n: u64,
    /// `n⁻¹ mod 2⁶⁴`.
    inverse: u64,
    /// `2¹²⁸ mod n`, to bring a value into the form.
    r2: u64,
}

impl Montgomery {
    fn new(n: u64) -> Self {
        debug_assert!(n % 2 == 1);
        // Newton's iteration doubles the correct low bits each step: 3 → 96.
        let mut inverse = n;
        for _ in 0..5 {
            inverse = inverse.wrapping_mul(2u64.wrapping_sub(n.wrapping_mul(inverse)));
        }
        let r = (1u128 << 64) % u128::from(n);
        let r2 = (r * r % u128::from(n)) as u64;
        Montgomery { n, inverse, r2 }
    }

    /// `t·2⁻⁶⁴ mod n` for `t < n·2⁶⁴`, in `[0, n)`.
    #[inline]
    fn reduce(&self, t: u128) -> u64 {
        let m = (t as u64).wrapping_mul(self.inverse);
        let mn = ((u128::from(m) * u128::from(self.n)) >> 64) as u64;
        let (difference, borrow) = ((t >> 64) as u64).overflowing_sub(mn);
        if borrow {
            difference.wrapping_add(self.n)
        } else {
            difference
        }
    }

    #[inline]
    fn mul(&self, a: u64, b: u64) -> u64 {
        self.reduce(u128::from(a) * u128::from(b))
    }

    #[inline]
    fn add(&self, a: u64, b: u64) -> u64 {
        let (sum, carry) = a.overflowing_add(b);
        if carry || sum >= self.n {
            sum.wrapping_sub(self.n)
        } else {
            sum
        }
    }

    fn to_form(&self, x: u64) -> u64 {
        self.mul(x % self.n, self.r2)
    }

    fn pow(&self, base: u64, mut exponent: u64) -> u64 {
        let mut result = self.to_form(1);
        let mut base = base;
        while exponent > 0 {
            if exponent & 1 == 1 {
                result = self.mul(result, base);
            }
            base = self.mul(base, base);
            exponent >>= 1;
        }
        result
    }
}

/// `is_probable_prime` for an `n` of one word, in Montgomery form instead of
/// on `BigInt`s: the same bases, the same steps, the same charges against `wide`
/// (which is `n`), so the answer and the work metered are the wide route's.
fn is_probable_prime_word(
    n: u64,
    wide: &BigInt,
    meter: &mut Meter,
) -> Result<bool, FactorBudgetExhausted> {
    let n_minus_1 = n - 1;
    let r = n_minus_1.trailing_zeros();
    let d = n_minus_1 >> r;
    let form = Montgomery::new(n);
    let (one, minus_one) = (form.to_form(1), form.to_form(n_minus_1));
    'bases: for &a in &MR_BASES {
        if a % n == 0 {
            continue;
        }
        for _ in 0..wide.bits() {
            meter.charge(wide)?;
        }
        let mut x = form.pow(form.to_form(a), d);
        if x == one || x == minus_one {
            continue;
        }
        for _ in 1..r {
            meter.charge(wide)?;
            x = form.mul(x, x);
            if x == minus_one {
                continue 'bases;
            }
        }
        return Ok(false);
    }
    Ok(true)
}

/// `pollard_brent` for an `n` of one word, step for step and charge for
/// charge, in Montgomery form instead of on `BigInt`s.
fn pollard_brent_word(
    n: u64,
    wide: &BigInt,
    meter: &mut Meter,
) -> Result<u64, FactorBudgetExhausted> {
    use crate::types::small_divisor::gcd_word;
    // Every value is held in Montgomery form, a unit multiple of itself, so
    // each gcd with `n` — and with it every branch — is the wide route's.
    let form = Montgomery::new(n);
    for c in 1u64.. {
        let c_form = form.to_form(c);
        let f = |x: u64| form.add(form.mul(x, x), c_form);
        let mut y = form.to_form(2);
        let mut r: u64 = 1;
        let mut q = form.to_form(1);
        let mut g = 1u64;
        let mut x = y;
        let mut ys = y;
        const M: u64 = 128;
        while g == 1 {
            x = y;
            for _ in 0..r {
                meter.charge(wide)?;
                y = f(y);
            }
            let mut k = 0;
            while k < r && g == 1 {
                ys = y;
                for _ in 0..M.min(r - k) {
                    meter.charge(wide)?;
                    y = f(y);
                    q = form.mul(q, x.abs_diff(y));
                }
                g = gcd_word(q, n);
                k += M;
            }
            r *= 2;
        }
        if g == n {
            loop {
                meter.charge(wide)?;
                ys = f(ys);
                g = gcd_word(x.abs_diff(ys), n);
                if g != 1 {
                    break;
                }
            }
        }
        if g != n && g != 1 {
            return Ok(g);
        }
        // This constant cycled without splitting; try the next one.
        if c > 64 {
            return Err(FactorBudgetExhausted);
        }
    }
    unreachable!("the constant loop only exits by returning")
}

/// Which residues modulo `m` are squares.
const fn squares_mod<const M: usize>() -> [bool; M] {
    let mut table = [false; M];
    let mut i = 0;
    while i < M {
        table[(i * i) % M] = true;
        i += 1;
    }
    table
}

static SQUARES_MOD_64: [bool; 64] = squares_mod::<64>();
static SQUARES_MOD_63: [bool; 63] = squares_mod::<63>();
static SQUARES_MOD_65: [bool; 65] = squares_mod::<65>();
static SQUARES_MOD_11: [bool; 11] = squares_mod::<11>();

/// `√n` when the non-negative `n` is a perfect square.
///
/// Like ruling out a multiple of 3 by its digit sum before dividing, a number
/// whose last six bits, or whose residue modulo 63, 65 or 11, no square can
/// have is turned away by table lookups — all but about one non-square in 120
/// — before the square root is taken (the screen GMP's `mpz_perfect_square_p`
/// uses).
pub(crate) fn exact_sqrt(n: &BigInt) -> Option<BigInt> {
    let low = n.magnitude().iter_u64_digits().next().unwrap_or(0);
    if !SQUARES_MOD_64[(low & 63) as usize] {
        return None;
    }
    let r = residue_mod(n, 63 * 65 * 11);
    if !SQUARES_MOD_63[(r % 63) as usize]
        || !SQUARES_MOD_65[(r % 65) as usize]
        || !SQUARES_MOD_11[(r % 11) as usize]
    {
        return None;
    }
    let root = n.sqrt();
    (&root * &root == *n).then_some(root)
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
        let table = &TRIAL_TABLE.primes;
        let edges: Vec<u64> = table
            .iter()
            .enumerate()
            .filter(|&(i, prime)| {
                let before = i.checked_sub(1).map(|j| table[j].group);
                let after = table.get(i + 1).map(|next| next.group);
                before != Some(prime.group) || after != Some(prime.group)
            })
            .map(|(_, prime)| prime.p)
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

    /// The word routes answer what the wide ones do and charge the same work,
    /// over primes, composites with and without small factors, squares of
    /// primes and numbers near the top of the word.
    #[test]
    fn word_routes_match_the_wide_routes_answer_and_work() {
        let mut odd: Vec<u64> = vec![
            16_777_259,                    // prime just above TRIAL_BOUND²
            2_147_483_647 * 1_000_003,     // two primes
            4_294_967_291 * 4_294_967_279, // near the top of the word
            1_000_003 * 1_000_003,         // a prime's square
            (1 << 61) - 1,                 // a Mersenne prime
            u64::MAX,                      // 3·5·17·257·641·65537·6700417
            18_446_744_073_709_551_557,    // the largest prime below 2⁶⁴
            3_215_031_751,                 // a strong pseudoprime to 2, 3, 5, 7
        ];
        let mut x: u64 = 0x9e37_79b9_7f4a_7c15;
        for _ in 0..200 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            odd.push((x >> (x % 40)) | (1 << 25) | 1);
        }
        for n in odd {
            let wide = BigInt::from(n);
            let (mut word_budget, mut wide_budget) = (u64::MAX, u64::MAX);
            let word_prime = is_probable_prime_word(
                n,
                &wide,
                &mut Meter {
                    budget: &mut word_budget,
                },
            );
            let wide_prime = is_probable_prime_wide(
                &wide,
                &mut Meter {
                    budget: &mut wide_budget,
                },
            );
            assert_eq!(word_prime, wide_prime, "{n}");
            assert_eq!(word_budget, wide_budget, "{n}");
            if word_prime == Ok(false) && exact_sqrt(&wide).is_none() {
                let word_factor = pollard_brent_word(
                    n,
                    &wide,
                    &mut Meter {
                        budget: &mut word_budget,
                    },
                );
                let wide_factor = pollard_brent_wide(
                    &wide,
                    &mut Meter {
                        budget: &mut wide_budget,
                    },
                );
                assert_eq!(word_factor.map(BigInt::from), wide_factor, "{n}");
                assert_eq!(word_budget, wide_budget, "{n}");
            }
        }
    }

    /// The residue screen never turns a square away, and what passes it is
    /// settled by the root.
    #[test]
    fn exact_sqrt_finds_every_square_and_only_squares() {
        for n in 0u64..100_000 {
            let root = (n as f64).sqrt() as u64;
            let expected = (0..=root + 1).find(|r| r * r == n);
            assert_eq!(
                exact_sqrt(&BigInt::from(n)),
                expected.map(BigInt::from),
                "{n}"
            );
        }
        let big = (BigInt::from(1u8) << 200usize) + 12_345u32;
        let square = &big * &big;
        assert_eq!(exact_sqrt(&square), Some(big.clone()));
        assert_eq!(exact_sqrt(&(&square + 1u8)), None);
        assert_eq!(exact_sqrt(&(&square - 1u8)), None);
    }
}
