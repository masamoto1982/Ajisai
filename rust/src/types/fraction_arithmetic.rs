use super::bigint_gcd::balanced_bigint_gcd;
use super::fraction::{compute_gcd_i64, Fraction, FractionRepr};
use num_bigint::BigInt;
use num_traits::{One, Zero};

impl Fraction {
    pub fn add(&self, other: &Fraction) -> Fraction {
        if self.is_nil() || other.is_nil() {
            return Self::nil();
        }

        if let (Some((a, b)), Some((c, d))) = (self.extract_i64_pair(), other.extract_i64_pair()) {
            if b == 1 && d == 1 {
                return Self::create_from_i128((a as i128) + (c as i128), 1);
            }
            if b == d {
                return Self::create_from_i128((a as i128) + (c as i128), b as i128);
            }
            if let Some(num) = (a as i128).checked_mul(d as i128).and_then(|ad| {
                (c as i128)
                    .checked_mul(b as i128)
                    .and_then(|cb| ad.checked_add(cb))
            }) {
                return Self::create_from_i128(num, (b as i128) * (d as i128));
            }
        }

        let (an, ad): (BigInt, BigInt) = self.to_bigint_pair();
        let (bn, bd): (BigInt, BigInt) = other.to_bigint_pair();

        if ad == bd {
            let sum: BigInt = &an + &bn;
            if sum.is_zero() {
                return Fraction::from_repr(FractionRepr::Small(0, 1));
            }
            let g: BigInt = balanced_bigint_gcd(&sum, &ad);
            if g.is_one() {
                return Self::create_already_reduced(sum, ad);
            }
            return Self::create_already_reduced(&sum / &g, &ad / &g);
        }

        Fraction::new(&an * &bd + &bn * &ad, &ad * &bd)
    }

    pub fn sub(&self, other: &Fraction) -> Fraction {
        if self.is_nil() || other.is_nil() {
            return Self::nil();
        }

        if let (Some((a, b)), Some((c, d))) = (self.extract_i64_pair(), other.extract_i64_pair()) {
            if b == 1 && d == 1 {
                return Self::create_from_i128((a as i128) - (c as i128), 1);
            }
            if b == d {
                return Self::create_from_i128((a as i128) - (c as i128), b as i128);
            }
            if let Some(num) = (a as i128).checked_mul(d as i128).and_then(|ad| {
                (c as i128)
                    .checked_mul(b as i128)
                    .and_then(|cb| ad.checked_sub(cb))
            }) {
                return Self::create_from_i128(num, (b as i128) * (d as i128));
            }
        }

        let (an, ad): (BigInt, BigInt) = self.to_bigint_pair();
        let (bn, bd): (BigInt, BigInt) = other.to_bigint_pair();

        if ad == bd {
            let diff: BigInt = &an - &bn;
            if diff.is_zero() {
                return Fraction::from_repr(FractionRepr::Small(0, 1));
            }
            let g: BigInt = balanced_bigint_gcd(&diff, &ad);
            if g.is_one() {
                return Self::create_already_reduced(diff, ad);
            }
            return Self::create_already_reduced(&diff / &g, &ad / &g);
        }

        Fraction::new(&an * &bd - &bn * &ad, &ad * &bd)
    }

    pub fn mul(&self, other: &Fraction) -> Fraction {
        if self.is_nil() || other.is_nil() {
            return Self::nil();
        }

        if let (Some((a, b)), Some((c, d))) = (self.extract_i64_pair(), other.extract_i64_pair()) {
            let g1 = compute_gcd_i64(a, d);
            let g2 = compute_gcd_i64(c, b);
            let a_r = (a / g1) as i128;
            let b_r = (b / g2) as i128;
            let c_r = (c / g2) as i128;
            let d_r = (d / g1) as i128;
            if let (Some(num), Some(den)) = (a_r.checked_mul(c_r), b_r.checked_mul(d_r)) {
                return Self::create_from_i128(num, den);
            }
        }

        let (an, ad): (BigInt, BigInt) = self.to_bigint_pair();
        let (bn, bd): (BigInt, BigInt) = other.to_bigint_pair();

        if ad.is_one() && bd.is_one() {
            return Self::from_bigint_pair(an * bn, BigInt::one());
        }

        if ad.is_one() {
            let g: BigInt = balanced_bigint_gcd(&an, &bd);
            let a_reduced: BigInt = &an / &g;
            let d_reduced: BigInt = &bd / &g;
            return Self::create_already_reduced(a_reduced * bn, d_reduced);
        }

        if bd.is_one() {
            let g: BigInt = balanced_bigint_gcd(&bn, &ad);
            let c_reduced: BigInt = &bn / &g;
            let b_reduced: BigInt = &ad / &g;
            return Self::create_already_reduced(an * c_reduced, b_reduced);
        }

        let g1: BigInt = balanced_bigint_gcd(&an, &bd);
        let g2: BigInt = balanced_bigint_gcd(&bn, &ad);

        let a_reduced: BigInt = &an / &g1;
        let d_reduced: BigInt = &bd / &g1;
        let c_reduced: BigInt = &bn / &g2;
        let b_reduced: BigInt = &ad / &g2;

        Self::create_already_reduced(a_reduced * c_reduced, b_reduced * d_reduced)
    }

    pub fn div(&self, other: &Fraction) -> Fraction {
        if self.is_nil() || other.is_nil() {
            return Self::nil();
        }
        if other.is_zero() {
            panic!("Division by zero");
        }

        if let (Some((a, b)), Some((c, d))) = (self.extract_i64_pair(), other.extract_i64_pair()) {
            let g1 = compute_gcd_i64(a, c);
            let g2 = compute_gcd_i64(d, b);
            let a_r = (a / g1) as i128;
            let b_r = (b / g2) as i128;
            let c_r = (c / g1) as i128;
            let d_r = (d / g2) as i128;
            if let (Some(num), Some(den)) = (a_r.checked_mul(d_r), b_r.checked_mul(c_r)) {
                return Self::create_from_i128(num, den);
            }
        }

        let (an, ad): (BigInt, BigInt) = self.to_bigint_pair();
        let (bn, bd): (BigInt, BigInt) = other.to_bigint_pair();

        if ad.is_one() && bd.is_one() {
            return Fraction::new(an, bn);
        }

        if ad.is_one() {
            let g: BigInt = balanced_bigint_gcd(&an, &bn);
            let a_reduced: BigInt = &an / &g;
            let c_reduced: BigInt = &bn / &g;
            return Self::create_already_reduced(a_reduced * bd, c_reduced);
        }

        if bd.is_one() {
            let g: BigInt = balanced_bigint_gcd(&an, &bn);
            let a_reduced: BigInt = &an / &g;
            let c_reduced: BigInt = &bn / &g;
            return Self::create_already_reduced(a_reduced, ad * c_reduced);
        }

        let g1: BigInt = balanced_bigint_gcd(&an, &bn);
        let g2: BigInt = balanced_bigint_gcd(&bd, &ad);

        let a_reduced: BigInt = &an / &g1;
        let c_reduced: BigInt = &bn / &g1;
        let d_reduced: BigInt = &bd / &g2;
        let b_reduced: BigInt = &ad / &g2;

        Self::create_already_reduced(a_reduced * d_reduced, b_reduced * c_reduced)
    }

    #[inline]
    pub fn abs(&self) -> Fraction {
        if self.is_nil() {
            return self.clone();
        }
        match &self.repr {
            FractionRepr::Small(n, d) => Fraction::from_repr(FractionRepr::Small(n.abs(), *d)),
            FractionRepr::Big {
                numerator,
                denominator,
            } => Fraction::from_repr(FractionRepr::Big {
                numerator: if *numerator < BigInt::zero() {
                    -numerator.clone()
                } else {
                    numerator.clone()
                },
                denominator: denominator.clone(),
            }),
        }
    }

    pub fn floor(&self) -> Fraction {
        // Absence propagates, as it does through `add`, `sub`, `mul` and
        // `div`: `Fraction::nil` is `0/0`, so without this an absent lane
        // reached the division below and `[ NIL 1 ] FLOOR` aborted the process.
        if self.is_nil() {
            return Self::nil();
        }
        if self.is_integer() {
            return Fraction::from_repr(self.repr.clone());
        }

        match &self.repr {
            FractionRepr::Small(n, d) => {
                let q = n / d;
                let r = n % d;
                let floored = if *n < 0 && r != 0 { q - 1 } else { q };
                Fraction::from_repr(FractionRepr::Small(floored, 1))
            }
            FractionRepr::Big {
                numerator,
                denominator,
            } => {
                let q = numerator / denominator;
                let r = numerator % denominator;
                let floored = if *numerator < BigInt::zero() && !r.is_zero() {
                    q - BigInt::one()
                } else {
                    q
                };
                Self::from_bigint_pair(floored, BigInt::one())
            }
        }
    }

    pub fn round(&self) -> Fraction {
        // As `floor`: an absent lane is `0/0` and must not reach the division.
        if self.is_nil() {
            return Self::nil();
        }
        if self.is_integer() {
            return Fraction::from_repr(self.repr.clone());
        }

        if self.is_zero() {
            return Fraction::from_repr(FractionRepr::Small(0, 1));
        }

        match &self.repr {
            FractionRepr::Small(n, d) => {
                let is_negative = *n < 0;
                // Widen to i128 *before* taking the absolute value: `i64::MIN`
                // has no positive i64 counterpart, so `i64::abs()` overflows and
                // panics in debug (reachable from a `-9223372036854775808/d`
                // operand). i128 holds it exactly.
                let abs_n = (*n as i128).abs();
                let d128 = *d as i128;
                let result = ((2 * abs_n + d128) / (2 * d128)) as i64;
                Fraction::from_repr(FractionRepr::Small(
                    if is_negative { -result } else { result },
                    1,
                ))
            }
            FractionRepr::Big {
                numerator,
                denominator,
            } => {
                let is_negative = *numerator < BigInt::zero();
                let abs_num = if is_negative {
                    -numerator.clone()
                } else {
                    numerator.clone()
                };
                let two = BigInt::from(2);
                let two_abs_num = &abs_num * &two;
                let result = (&two_abs_num + denominator) / (&two * denominator);
                Self::from_bigint_pair(if is_negative { -result } else { result }, BigInt::one())
            }
        }
    }
}
