use super::fraction::{compute_gcd_i64, Fraction, FractionRepr};
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Zero};

impl Fraction {
    #[inline]
    pub fn mul_by_integer(&self, n: &Fraction) -> Fraction {
        debug_assert!(n.is_integer());

        if let (Some((a, b)), Some((n_val, _))) = (self.extract_i64_pair(), n.extract_i64_pair()) {
            let g: i64 = compute_gcd_i64(n_val, b);
            let n_r: i128 = (n_val / g) as i128;
            let b_r: i128 = (b / g) as i128;
            let num: i128 = (a as i128) * n_r;
            return Self::create_from_i128(num, b_r);
        }

        let (sn, sd): (BigInt, BigInt) = self.to_bigint_pair();
        let nn: BigInt = n.numerator();
        let g: BigInt = nn.gcd(&sd);
        let n_reduced: BigInt = &nn / &g;
        let b_reduced: BigInt = &sd / &g;
        Self::create_already_reduced(sn * n_reduced, b_reduced)
    }

    #[inline]
    pub fn div_by_integer(&self, n: &Fraction) -> Fraction {
        debug_assert!(n.is_integer());
        debug_assert!(!n.is_zero());

        if let (Some((a, b)), Some((n_val, _))) = (self.extract_i64_pair(), n.extract_i64_pair()) {
            let g: i64 = compute_gcd_i64(a, n_val);
            let a_r: i128 = (a / g) as i128;
            let n_r: i128 = (n_val / g) as i128;
            let den: i128 = (b as i128) * n_r;
            return Self::create_from_i128(a_r, den);
        }

        let (sn, sd): (BigInt, BigInt) = self.to_bigint_pair();
        let nn: BigInt = n.numerator();
        let g: BigInt = sn.gcd(&nn);
        let a_reduced: BigInt = &sn / &g;
        let n_reduced: BigInt = &nn / &g;
        Self::create_already_reduced(a_reduced, sd * n_reduced)
    }

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
            if let Some(num) = (a as i128).checked_mul(d as i128)
                .and_then(|ad| (c as i128).checked_mul(b as i128)
                    .and_then(|cb| ad.checked_add(cb)))
            {
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
            let g: BigInt = sum.gcd(&ad);
            if g.is_one() {
                return Self::create_already_reduced(sum, ad);
            }
            return Self::create_already_reduced(&sum / &g, &ad / &g);
        }

        Fraction::new(
            &an * &bd + &bn * &ad,
            &ad * &bd,
        )
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
            if let Some(num) = (a as i128).checked_mul(d as i128)
                .and_then(|ad| (c as i128).checked_mul(b as i128)
                    .and_then(|cb| ad.checked_sub(cb)))
            {
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
            let g: BigInt = diff.gcd(&ad);
            if g.is_one() {
                return Self::create_already_reduced(diff, ad);
            }
            return Self::create_already_reduced(&diff / &g, &ad / &g);
        }

        Fraction::new(
            &an * &bd - &bn * &ad,
            &ad * &bd,
        )
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
            let g: BigInt = an.gcd(&bd);
            let a_reduced: BigInt = &an / &g;
            let d_reduced: BigInt = &bd / &g;
            return Self::create_already_reduced(a_reduced * bn, d_reduced);
        }

        if bd.is_one() {
            let g: BigInt = bn.gcd(&ad);
            let c_reduced: BigInt = &bn / &g;
            let b_reduced: BigInt = &ad / &g;
            return Self::create_already_reduced(an * c_reduced, b_reduced);
        }

        let g1: BigInt = an.gcd(&bd);
        let g2: BigInt = bn.gcd(&ad);

        let a_reduced: BigInt = &an / &g1;
        let d_reduced: BigInt = &bd / &g1;
        let c_reduced: BigInt = &bn / &g2;
        let b_reduced: BigInt = &ad / &g2;

        Self::create_already_reduced(
            a_reduced * c_reduced,
            b_reduced * d_reduced,
        )
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
            let g: BigInt = an.gcd(&bn);
            let a_reduced: BigInt = &an / &g;
            let c_reduced: BigInt = &bn / &g;
            return Self::create_already_reduced(a_reduced * bd, c_reduced);
        }

        if bd.is_one() {
            let g: BigInt = an.gcd(&bn);
            let a_reduced: BigInt = &an / &g;
            let c_reduced: BigInt = &bn / &g;
            return Self::create_already_reduced(a_reduced, ad * c_reduced);
        }

        let g1: BigInt = an.gcd(&bn);
        let g2: BigInt = bd.gcd(&ad);

        let a_reduced: BigInt = &an / &g1;
        let c_reduced: BigInt = &bn / &g1;
        let d_reduced: BigInt = &bd / &g2;
        let b_reduced: BigInt = &ad / &g2;

        Self::create_already_reduced(
            a_reduced * d_reduced,
            b_reduced * c_reduced,
        )
    }

    #[inline]
    pub fn abs(&self) -> Fraction {
        if self.is_nil() {
            return self.clone();
        }
        match &self.repr {
            FractionRepr::Small(n, d) => {
                Fraction::from_repr(FractionRepr::Small(n.abs(), *d))
            }
            FractionRepr::Big { numerator, denominator } => {
                Fraction::from_repr(FractionRepr::Big {
                    numerator: if *numerator < BigInt::zero() {
                        -numerator.clone()
                    } else {
                        numerator.clone()
                    },
                    denominator: denominator.clone(),
                })
            }
        }
    }

    pub fn floor(&self) -> Fraction {
        if self.is_integer() {
            return self.clone();
        }

        match &self.repr {
            FractionRepr::Small(n, d) => {
                let q = n / d;
                let r = n % d;
                let floored = if *n < 0 && r != 0 { q - 1 } else { q };
                Fraction::from_repr(FractionRepr::Small(floored, 1))
            }
            FractionRepr::Big { numerator, denominator } => {
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

    pub fn ceil(&self) -> Fraction {
        if self.is_integer() {
            return self.clone();
        }

        match &self.repr {
            FractionRepr::Small(n, d) => {
                let q = n / d;
                let r = n % d;
                let ceiled = if *n > 0 && r != 0 { q + 1 } else { q };
                Fraction::from_repr(FractionRepr::Small(ceiled, 1))
            }
            FractionRepr::Big { numerator, denominator } => {
                let q = numerator / denominator;
                let r = numerator % denominator;
                let ceiled = if *numerator > BigInt::zero() && !r.is_zero() {
                    q + BigInt::one()
                } else {
                    q
                };
                Self::from_bigint_pair(ceiled, BigInt::one())
            }
        }
    }


    pub fn round(&self) -> Fraction {
        if self.is_integer() {
            return self.clone();
        }

        if self.is_zero() {
            return Fraction::from_repr(FractionRepr::Small(0, 1));
        }

        match &self.repr {
            FractionRepr::Small(n, d) => {
                let is_negative = *n < 0;
                let abs_n = n.abs() as i128;
                let d128 = *d as i128;
                let result = ((2 * abs_n + d128) / (2 * d128)) as i64;
                Fraction::from_repr(FractionRepr::Small(if is_negative { -result } else { result }, 1))
            }
            FractionRepr::Big { numerator, denominator } => {
                let is_negative = *numerator < BigInt::zero();
                let abs_num = if is_negative {
                    -numerator.clone()
                } else {
                    numerator.clone()
                };
                let two = BigInt::from(2);
                let two_abs_num = &abs_num * &two;
                let result = (&two_abs_num + denominator) / (&two * denominator);
                Self::from_bigint_pair(
                    if is_negative { -result } else { result },
                    BigInt::one(),
                )
            }
        }
    }


    pub fn modulo(&self, other: &Fraction) -> Fraction {
        if other.is_zero() {
            panic!("Modulo by zero");
        }

        if let (Some((a, b)), Some((c, d))) = (self.extract_i64_pair(), other.extract_i64_pair()) {
            if b == 1 && d == 1 {
                let rem = a % c;
                let result = if rem < 0 {
                    if c > 0 { rem + c } else { rem - c }
                } else {
                    rem
                };
                return Fraction::from_repr(FractionRepr::Small(result, 1));
            }

            let a = a as i128;
            let b = b as i128;
            let c = c as i128;
            let d = d as i128;
            let num = a * d;
            let mod_by = c * b;
            let den = b * d;
            let rem = num % mod_by;
            let result_num = if rem < 0 {
                if mod_by > 0 { rem + mod_by } else { rem - mod_by }
            } else {
                rem
            };
            return Self::create_from_i128(result_num, den);
        }

        let (sn, sd): (BigInt, BigInt) = self.to_bigint_pair();
        let (on, od): (BigInt, BigInt) = other.to_bigint_pair();

        if sd.is_one() && od.is_one() {
            let rem: BigInt = &sn % &on;
            let result: BigInt = if rem < BigInt::zero() {
                if on > BigInt::zero() {
                    rem + &on
                } else {
                    rem - &on
                }
            } else {
                rem
            };
            return Self::from_bigint_pair(result, BigInt::one());
        }

        let div_result: Fraction = self.div(other);
        let floored: Fraction = div_result.floor();
        self.sub(&other.mul(&floored))
    }
}
