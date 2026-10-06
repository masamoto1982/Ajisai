//! The one-word routes of the factoring steps: Miller–Rabin and Pollard–Brent
//! in Montgomery form for a cofactor that fits 64 bits, step for step and
//! charge for charge with the `BigInt` routes in the parent module.

use super::{FactorBudgetExhausted, Meter, MR_BASES};
use num_bigint::BigInt;

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
pub(super) fn is_probable_prime_word(
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
pub(super) fn pollard_brent_word(
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
