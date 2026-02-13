// rust/src/types/fraction.rs

use num_bigint::BigInt;
use num_traits::{Zero, One, ToPrimitive, Signed};
use num_integer::Integer;
use std::str::FromStr;

#[inline]
fn gcd_i64(mut a: i64, mut b: i64) -> i64 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

#[inline]
fn bigint_from_i128(n: i128) -> BigInt {
    if n >= i64::MIN as i128 && n <= i64::MAX as i128 {
        BigInt::from(n as i64)
    } else {
        let sign = n.signum();
        let abs_n = n.unsigned_abs();
        let high = (abs_n >> 64) as u64;
        let low = abs_n as u64;
        let result = if high == 0 {
            BigInt::from(low)
        } else {
            BigInt::from(high) * BigInt::from(1u128 << 64) + BigInt::from(low)
        };
        if sign < 0 { -result } else { result }
    }
}

#[derive(Debug, Clone)]
pub struct Fraction {
    pub numerator: BigInt,
    pub denominator: BigInt,
}

/// Mathematical equality via cross-multiplication.
/// This correctly handles unreduced fractions: 440/2 == 220/1.
impl PartialEq for Fraction {
    fn eq(&self, other: &Self) -> bool {
        // NIL special case: NIL == NIL, NIL != anything else
        if self.is_nil() || other.is_nil() {
            return self.is_nil() && other.is_nil();
        }
        // Cross-multiplication: a/b == c/d ⟺ a*d == c*b
        &self.numerator * &other.denominator == &other.numerator * &self.denominator
    }
}

impl Eq for Fraction {}

impl Fraction {
    /// NIL sentinel: denominator = 0. Propagates through arithmetic (three-valued logic).
    #[inline]
    pub fn nil() -> Self {
        Fraction {
            numerator: BigInt::zero(),
            denominator: BigInt::zero(),
        }
    }

    #[inline]
    pub fn is_nil(&self) -> bool {
        self.denominator.is_zero()
    }

    pub fn new(numerator: BigInt, denominator: BigInt) -> Self {
        if denominator.is_zero() { panic!("Division by zero"); }
        let common = numerator.gcd(&denominator);
        let mut num = &numerator / &common;
        let mut den = &denominator / &common;
        if den < BigInt::zero() {
            num = -num;
            den = -den;
        }
        Fraction { numerator: num, denominator: den }
    }

    /// Constructs a fraction without GCD reduction. Only normalizes sign.
    /// Used for music DSL where n/d represents frequency/duration as independent parameters.
    #[inline]
    pub fn new_unreduced(mut numerator: BigInt, mut denominator: BigInt) -> Self {
        if denominator.is_zero() { panic!("Division by zero"); }
        if denominator < BigInt::zero() {
            numerator = -numerator;
            denominator = -denominator;
        }
        Fraction { numerator, denominator }
    }

    /// Constructs a fraction that is already in lowest terms. Only normalizes sign.
    #[inline]
    fn new_already_reduced(mut numerator: BigInt, mut denominator: BigInt) -> Self {
        debug_assert!(!denominator.is_zero());
        if denominator < BigInt::zero() {
            numerator = -numerator;
            denominator = -denominator;
        }
        Fraction { numerator, denominator }
    }

    #[inline]
    pub fn is_integer(&self) -> bool {
        self.denominator.is_one()
    }

    /// (a/b) × n: Cross-cancellation with gcd(n, b).
    #[inline]
    pub fn mul_by_integer(&self, n: &Fraction) -> Fraction {
        debug_assert!(n.denominator.is_one());

        if let Some((a, b)) = self.try_as_i64_pair() {
            if let Some(n_val) = n.numerator.to_i64() {
                let g = gcd_i64(n_val, b);
                let n_r = (n_val / g) as i128;
                let b_r = (b / g) as i128;
                let num = (a as i128) * n_r;
                return Self::new_from_i128(num, b_r);
            }
        }

        let g = n.numerator.gcd(&self.denominator);
        let n_reduced = &n.numerator / &g;
        let b_reduced = &self.denominator / &g;
        Self::new_already_reduced(
            &self.numerator * n_reduced,
            b_reduced,
        )
    }

    /// (a/b) ÷ n: Cross-cancellation with gcd(a, n).
    #[inline]
    pub fn div_by_integer(&self, n: &Fraction) -> Fraction {
        debug_assert!(n.denominator.is_one());
        debug_assert!(!n.numerator.is_zero());

        if let Some((a, b)) = self.try_as_i64_pair() {
            if let Some(n_val) = n.numerator.to_i64() {
                let g = gcd_i64(a, n_val);
                let a_r = (a / g) as i128;
                let n_r = (n_val / g) as i128;
                let den = (b as i128) * n_r;
                return Self::new_from_i128(a_r, den);
            }
        }

        let g = self.numerator.gcd(&n.numerator);
        let a_reduced = &self.numerator / &g;
        let n_reduced = &n.numerator / &g;
        Self::new_already_reduced(
            a_reduced,
            &self.denominator * n_reduced,
        )
    }

    #[inline]
    fn new_from_i128(num: i128, den: i128) -> Self {
        debug_assert!(den != 0);
        fn gcd_i128(mut a: i128, mut b: i128) -> i128 {
            a = a.abs();
            b = b.abs();
            while b != 0 {
                let t = b;
                b = a % b;
                a = t;
            }
            a
        }
        let g = gcd_i128(num, den);
        let mut n = num / g;
        let mut d = den / g;
        if d < 0 {
            n = -n;
            d = -d;
        }
        Fraction {
            numerator: bigint_from_i128(n),
            denominator: bigint_from_i128(d),
        }
    }

    #[inline]
    fn try_as_i64_pair(&self) -> Option<(i64, i64)> {
        let n = self.numerator.to_i64()?;
        let d = self.denominator.to_i64()?;
        Some((n, d))
    }

    pub fn from_str(s: &str) -> std::result::Result<Self, String> {
        if s.is_empty() { return Err("Empty string".to_string()); }

        if let Some(e_pos) = s.find(|c| c == 'e' || c == 'E') {
            let mantissa_str = &s[..e_pos];
            let exponent_str = &s[e_pos+1..];

            let mantissa = Self::from_str(mantissa_str)?;
            let exponent = exponent_str.parse::<i32>().map_err(|e| e.to_string())?;

            if exponent >= 0 {
                let power = BigInt::from(10).pow(exponent as u32);
                return Ok(Fraction::new(mantissa.numerator * power, mantissa.denominator));
            } else {
                let power = BigInt::from(10).pow((-exponent) as u32);
                return Ok(Fraction::new(mantissa.numerator, mantissa.denominator * power));
            }
        }
        if let Some(pos) = s.find('/') {
            let num = BigInt::from_str(&s[..pos]).map_err(|e| e.to_string())?;
            let den = BigInt::from_str(&s[pos+1..]).map_err(|e| e.to_string())?;
            Ok(Fraction::new(num, den))
        } else if let Some(dot_pos) = s.find('.') {
            let int_part_str = if s.starts_with('.') { "0" } else { &s[..dot_pos] };
            let frac_part_str = &s[dot_pos+1..];
            if frac_part_str.is_empty() { return Self::from_str(int_part_str); }
            let int_part = BigInt::from_str(int_part_str).map_err(|e| e.to_string())?;
            let frac_num = BigInt::from_str(frac_part_str).map_err(|e| e.to_string())?;
            let frac_den = BigInt::from(10).pow(frac_part_str.len() as u32);
            let total_num = int_part.abs() * &frac_den + frac_num;
            Ok(Fraction::new(if int_part < BigInt::zero() { -total_num } else { total_num }, frac_den))
        } else {
            let num = BigInt::from_str(s).map_err(|e| e.to_string())?;
            Ok(Fraction::new(num, BigInt::one()))
        }
    }

    /// Parses a fraction string without GCD reduction for explicit a/b forms.
    /// Integers and decimals are still reduced (they represent single mathematical values).
    /// Used for vector construction where a/b may represent frequency/duration.
    pub fn from_str_unreduced(s: &str) -> std::result::Result<Self, String> {
        if s.is_empty() { return Err("Empty string".to_string()); }

        // Scientific notation: delegate to from_str (reduction is appropriate)
        if s.contains(|c: char| c == 'e' || c == 'E') {
            return Self::from_str(s);
        }

        // Explicit fraction a/b: preserve original numerator and denominator
        if let Some(pos) = s.find('/') {
            let num = BigInt::from_str(&s[..pos]).map_err(|e| e.to_string())?;
            let den = BigInt::from_str(&s[pos+1..]).map_err(|e| e.to_string())?;
            Ok(Self::new_unreduced(num, den))
        } else {
            // Integer or decimal: regular parsing (reduction is fine for single values)
            Self::from_str(s)
        }
    }

    /// (a/b) + (c/d) = (a*d + c*b) / (b*d)
    pub fn add(&self, other: &Fraction) -> Fraction {
        if self.is_nil() || other.is_nil() {
            return Self::nil();
        }

        if let (Some((a, b)), Some((c, d))) = (self.try_as_i64_pair(), other.try_as_i64_pair()) {
            let a = a as i128;
            let b = b as i128;
            let c = c as i128;
            let d = d as i128;
            let num = a * d + c * b;
            let den = b * d;
            return Self::new_from_i128(num, den);
        }

        if self.denominator.is_one() && other.denominator.is_one() {
            return Fraction {
                numerator: &self.numerator + &other.numerator,
                denominator: BigInt::one(),
            };
        }

        if self.denominator == other.denominator {
            return Fraction::new(
                &self.numerator + &other.numerator,
                self.denominator.clone(),
            );
        }

        Fraction::new(
            &self.numerator * &other.denominator + &other.numerator * &self.denominator,
            &self.denominator * &other.denominator,
        )
    }

    /// (a/b) - (c/d) = (a*d - c*b) / (b*d)
    pub fn sub(&self, other: &Fraction) -> Fraction {
        if self.is_nil() || other.is_nil() {
            return Self::nil();
        }

        if let (Some((a, b)), Some((c, d))) = (self.try_as_i64_pair(), other.try_as_i64_pair()) {
            let a = a as i128;
            let b = b as i128;
            let c = c as i128;
            let d = d as i128;
            let num = a * d - c * b;
            let den = b * d;
            return Self::new_from_i128(num, den);
        }

        if self.denominator.is_one() && other.denominator.is_one() {
            return Fraction {
                numerator: &self.numerator - &other.numerator,
                denominator: BigInt::one(),
            };
        }

        if self.denominator == other.denominator {
            return Fraction::new(
                &self.numerator - &other.numerator,
                self.denominator.clone(),
            );
        }

        Fraction::new(
            &self.numerator * &other.denominator - &other.numerator * &self.denominator,
            &self.denominator * &other.denominator,
        )
    }

    /// (a/b) × (c/d): Cross-cancellation with g1 = gcd(a,d), g2 = gcd(c,b).
    /// Result = (a/g1 × c/g2) / (b/g2 × d/g1), already in lowest terms.
    pub fn mul(&self, other: &Fraction) -> Fraction {
        if self.is_nil() || other.is_nil() {
            return Self::nil();
        }

        if let (Some((a, b)), Some((c, d))) = (self.try_as_i64_pair(), other.try_as_i64_pair()) {
            let g1 = gcd_i64(a, d);
            let g2 = gcd_i64(c, b);
            let a_r = (a / g1) as i128;
            let b_r = (b / g2) as i128;
            let c_r = (c / g2) as i128;
            let d_r = (d / g1) as i128;
            let num = a_r * c_r;
            let den = b_r * d_r;
            return Self::new_from_i128(num, den);
        }

        if self.denominator.is_one() && other.denominator.is_one() {
            return Fraction {
                numerator: &self.numerator * &other.numerator,
                denominator: BigInt::one(),
            };
        }

        if self.denominator.is_one() {
            let g = self.numerator.gcd(&other.denominator);
            let a_reduced = &self.numerator / &g;
            let d_reduced = &other.denominator / &g;
            return Self::new_already_reduced(
                a_reduced * &other.numerator,
                d_reduced,
            );
        }

        if other.denominator.is_one() {
            let g = other.numerator.gcd(&self.denominator);
            let c_reduced = &other.numerator / &g;
            let b_reduced = &self.denominator / &g;
            return Self::new_already_reduced(
                &self.numerator * c_reduced,
                b_reduced,
            );
        }

        let g1 = self.numerator.gcd(&other.denominator);
        let g2 = other.numerator.gcd(&self.denominator);

        let a_reduced = &self.numerator / &g1;
        let d_reduced = &other.denominator / &g1;
        let c_reduced = &other.numerator / &g2;
        let b_reduced = &self.denominator / &g2;

        Self::new_already_reduced(
            a_reduced * c_reduced,
            b_reduced * d_reduced,
        )
    }

    /// (a/b) ÷ (c/d) = (a/b) × (d/c): Cross-cancellation with g1 = gcd(a,c), g2 = gcd(d,b).
    pub fn div(&self, other: &Fraction) -> Fraction {
        if self.is_nil() || other.is_nil() {
            return Self::nil();
        }
        if other.numerator.is_zero() {
            panic!("Division by zero");
        }

        if let (Some((a, b)), Some((c, d))) = (self.try_as_i64_pair(), other.try_as_i64_pair()) {
            let g1 = gcd_i64(a, c);
            let g2 = gcd_i64(d, b);
            let a_r = (a / g1) as i128;
            let b_r = (b / g2) as i128;
            let c_r = (c / g1) as i128;
            let d_r = (d / g2) as i128;
            let num = a_r * d_r;
            let den = b_r * c_r;
            return Self::new_from_i128(num, den);
        }

        if self.denominator.is_one() && other.denominator.is_one() {
            return Fraction::new(
                self.numerator.clone(),
                other.numerator.clone(),
            );
        }

        if self.denominator.is_one() {
            let g = self.numerator.gcd(&other.numerator);
            let a_reduced = &self.numerator / &g;
            let c_reduced = &other.numerator / &g;
            return Self::new_already_reduced(
                a_reduced * &other.denominator,
                c_reduced,
            );
        }

        if other.denominator.is_one() {
            let g = self.numerator.gcd(&other.numerator);
            let a_reduced = &self.numerator / &g;
            let c_reduced = &other.numerator / &g;
            return Self::new_already_reduced(
                a_reduced,
                &self.denominator * c_reduced,
            );
        }

        let g1 = self.numerator.gcd(&other.numerator);
        let g2 = other.denominator.gcd(&self.denominator);

        let a_reduced = &self.numerator / &g1;
        let c_reduced = &other.numerator / &g1;
        let d_reduced = &other.denominator / &g2;
        let b_reduced = &self.denominator / &g2;

        Self::new_already_reduced(
            a_reduced * d_reduced,
            b_reduced * c_reduced,
        )
    }

    /// a/b < c/d ⟺ a*d < c*b (denominators are positive)
    pub fn lt(&self, other: &Fraction) -> bool {
        if let (Some((a, b)), Some((c, d))) = (self.try_as_i64_pair(), other.try_as_i64_pair()) {
            return (a as i128) * (d as i128) < (c as i128) * (b as i128);
        }
        &self.numerator * &other.denominator < &other.numerator * &self.denominator
    }

    pub fn le(&self, other: &Fraction) -> bool {
        if let (Some((a, b)), Some((c, d))) = (self.try_as_i64_pair(), other.try_as_i64_pair()) {
            return (a as i128) * (d as i128) <= (c as i128) * (b as i128);
        }
        &self.numerator * &other.denominator <= &other.numerator * &self.denominator
    }

    pub fn gt(&self, other: &Fraction) -> bool {
        if let (Some((a, b)), Some((c, d))) = (self.try_as_i64_pair(), other.try_as_i64_pair()) {
            return (a as i128) * (d as i128) > (c as i128) * (b as i128);
        }
        &self.numerator * &other.denominator > &other.numerator * &self.denominator
    }

    pub fn ge(&self, other: &Fraction) -> bool {
        if let (Some((a, b)), Some((c, d))) = (self.try_as_i64_pair(), other.try_as_i64_pair()) {
            return (a as i128) * (d as i128) >= (c as i128) * (b as i128);
        }
        &self.numerator * &other.denominator >= &other.numerator * &self.denominator
    }

    /// Floor: rounds toward negative infinity.
    pub fn floor(&self) -> Fraction {
        if self.denominator.is_one() {
            return self.clone();
        }

        let q = &self.numerator / &self.denominator;
        let r = &self.numerator % &self.denominator;

        let floored = if self.numerator < BigInt::zero() && !r.is_zero() {
            q - BigInt::one()
        } else {
            q
        };

        Fraction {
            numerator: floored,
            denominator: BigInt::one(),
        }
    }

    /// Ceil: rounds toward positive infinity.
    pub fn ceil(&self) -> Fraction {
        if self.denominator.is_one() {
            return self.clone();
        }

        let q = &self.numerator / &self.denominator;
        let r = &self.numerator % &self.denominator;

        let ceiled = if self.numerator > BigInt::zero() && !r.is_zero() {
            q + BigInt::one()
        } else if self.numerator < BigInt::zero() && !r.is_zero() {
            q
        } else {
            q
        };

        Fraction {
            numerator: ceiled,
            denominator: BigInt::one(),
        }
    }

    /// Round half away from zero: floor(|x| + 0.5) with original sign.
    /// Formula: floor((2*|num| + den) / (2*den))
    pub fn round(&self) -> Fraction {
        if self.denominator.is_one() {
            return self.clone();
        }

        if self.numerator.is_zero() {
            return Fraction {
                numerator: BigInt::zero(),
                denominator: BigInt::one(),
            };
        }

        let is_negative = self.numerator < BigInt::zero();
        let abs_num = if is_negative {
            -&self.numerator
        } else {
            self.numerator.clone()
        };

        let two = BigInt::from(2);
        let two_abs_num = &abs_num * &two;
        let result = (&two_abs_num + &self.denominator) / (&two * &self.denominator);

        Fraction {
            numerator: if is_negative { -result } else { result },
            denominator: BigInt::one(),
        }
    }

    pub fn is_exact_integer(&self) -> bool {
        self.denominator == BigInt::one()
    }

    pub fn as_usize(&self) -> Option<usize> {
        if self.is_exact_integer() && self.numerator >= BigInt::zero() {
            self.numerator.to_usize()
        } else {
            None
        }
    }

    /// Mathematical modulo: a mod b = a - b * floor(a/b).
    /// Result has the same sign as b.
    pub fn modulo(&self, other: &Fraction) -> Fraction {
        if other.numerator.is_zero() {
            panic!("Modulo by zero");
        }

        if self.denominator.is_one() && other.denominator.is_one() {
            let rem = &self.numerator % &other.numerator;
            let result = if rem < BigInt::zero() {
                if other.numerator > BigInt::zero() {
                    rem + &other.numerator
                } else {
                    rem - &other.numerator
                }
            } else {
                rem
            };
            return Fraction {
                numerator: result,
                denominator: BigInt::one(),
            };
        }

        let div_result = self.div(other);
        let floored = div_result.floor();
        self.sub(&other.mul(&floored))
    }
}

impl PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Fraction {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if let (Some((a, b)), Some((c, d))) = (self.try_as_i64_pair(), other.try_as_i64_pair()) {
            let lhs = (a as i128) * (d as i128);
            let rhs = (c as i128) * (b as i128);
            return lhs.cmp(&rhs);
        }
        let lhs = &self.numerator * &other.denominator;
        let rhs = &other.numerator * &self.denominator;
        lhs.cmp(&rhs)
    }
}

impl ToPrimitive for Fraction {
    fn to_i64(&self) -> Option<i64> {
        (&self.numerator / &self.denominator).to_i64()
    }

    fn to_u64(&self) -> Option<u64> {
        if self.numerator < BigInt::zero() {
            None
        } else {
            (&self.numerator / &self.denominator).to_u64()
        }
    }

    fn to_f64(&self) -> Option<f64> {
        let num_f64 = self.numerator.to_f64()?;
        let den_f64 = self.denominator.to_f64()?;
        if den_f64 == 0.0 {
            None
        } else {
            Some(num_f64 / den_f64)
        }
    }
}

impl Fraction {
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.numerator.is_zero()
    }

    #[inline]
    pub fn to_i64(&self) -> Option<i64> {
        if self.denominator.is_one() {
            self.numerator.to_i64()
        } else {
            None
        }
    }
}

impl From<i64> for Fraction {
    #[inline]
    fn from(n: i64) -> Self {
        Fraction {
            numerator: BigInt::from(n),
            denominator: BigInt::one(),
        }
    }
}

impl From<i32> for Fraction {
    #[inline]
    fn from(n: i32) -> Self {
        Fraction {
            numerator: BigInt::from(n),
            denominator: BigInt::one(),
        }
    }
}

impl std::fmt::Display for Fraction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.denominator.is_one() {
            write!(f, "{}", self.numerator)
        } else {
            write!(f, "{}/{}", self.numerator, self.denominator)
        }
    }
}
