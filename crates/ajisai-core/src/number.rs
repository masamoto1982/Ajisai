//! Exact numbers.
//!
//! Every number in Ajisai is an exact rational held as a reduced pair of
//! arbitrary-precision integers. There is no floating point anywhere in the
//! language: no `f64` value can be constructed, observed, or serialized, so a
//! result never depends on how far a computation was carried.
//!
//! Water: a number is the volume of the flow, measured without loss.

use std::cmp::Ordering;
use std::fmt;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

/// An exact rational `num / den`, always reduced, with `den > 0`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Number {
    num: BigInt,
    den: BigInt,
}

impl Number {
    /// Build a reduced rational. Returns `None` when `den` is zero.
    pub fn new(num: BigInt, den: BigInt) -> Option<Self> {
        if den.is_zero() {
            return None;
        }
        let mut n = num;
        let mut d = den;
        if d.is_negative() {
            n = -n;
            d = -d;
        }
        let g = n.gcd(&d);
        if !g.is_one() && !g.is_zero() {
            n /= &g;
            d /= &g;
        }
        Some(Self { num: n, den: d })
    }

    pub fn integer(n: i64) -> Self {
        Self {
            num: BigInt::from(n),
            den: BigInt::one(),
        }
    }

    pub fn zero() -> Self {
        Self::integer(0)
    }

    pub fn one() -> Self {
        Self::integer(1)
    }

    pub fn numerator(&self) -> &BigInt {
        &self.num
    }

    pub fn denominator(&self) -> &BigInt {
        &self.den
    }

    pub fn is_zero(&self) -> bool {
        self.num.is_zero()
    }

    pub fn is_negative(&self) -> bool {
        self.num.is_negative()
    }

    pub fn is_integer(&self) -> bool {
        self.den.is_one()
    }

    /// The value as a `usize` index, if it is a non-negative integer that fits.
    pub fn as_index(&self) -> Option<usize> {
        if !self.is_integer() || self.num.is_negative() {
            return None;
        }
        u64::try_from(&self.num)
            .ok()
            .and_then(|v| usize::try_from(v).ok())
    }

    /// The value as a Unicode scalar value, if it denotes one.
    pub fn as_codepoint(&self) -> Option<char> {
        let i = self.as_index()?;
        u32::try_from(i).ok().and_then(char::from_u32)
    }

    /// Exact division. `None` when `other` is zero — the caller raises the
    /// `DivisionByZero` error, because a divide by zero is a flow that never
    /// formed, not a value. Division is the one arithmetic operation that is
    /// partial, so it is the one that is not a `std::ops` impl.
    pub fn checked_div(&self, other: &Self) -> Option<Self> {
        if other.is_zero() {
            return None;
        }
        Self::new(&self.num * &other.den, &self.den * &other.num)
    }

    pub fn abs(&self) -> Self {
        Self {
            num: self.num.abs(),
            den: self.den.clone(),
        }
    }

    /// Parse a numeric literal: `12`, `-3`, `7/4`, `-1/2`, `2.5`.
    ///
    /// A decimal literal is exact — `0.1` is `1/10`, not the nearest binary
    /// approximation of one tenth.
    pub fn parse(text: &str) -> Option<Self> {
        if text.is_empty() {
            return None;
        }
        if let Some((n, d)) = text.split_once('/') {
            let num = parse_decimal(n)?;
            let den = parse_decimal(d)?;
            return num.checked_div(&den);
        }
        parse_decimal(text)
    }
}

fn parse_decimal(text: &str) -> Option<Number> {
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    if digits.is_empty() {
        return None;
    }
    let value = match digits.split_once('.') {
        None => {
            if !digits.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            Number {
                num: digits.parse::<BigInt>().ok()?,
                den: BigInt::one(),
            }
        }
        Some((whole, frac)) => {
            if frac.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            if !frac.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let whole_part: BigInt = if whole.is_empty() {
                BigInt::zero()
            } else {
                whole.parse().ok()?
            };
            let frac_part: BigInt = frac.parse().ok()?;
            let scale = BigInt::from(10u8).pow(frac.len() as u32);
            Number::new(whole_part * &scale + frac_part, scale)?
        }
    };
    Some(if negative { -&value } else { value })
}

impl std::ops::Add for &Number {
    type Output = Number;

    fn add(self, other: &Number) -> Number {
        Number::new(
            &self.num * &other.den + &other.num * &self.den,
            &self.den * &other.den,
        )
        .expect("denominators are non-zero")
    }
}

impl std::ops::Sub for &Number {
    type Output = Number;

    fn sub(self, other: &Number) -> Number {
        Number::new(
            &self.num * &other.den - &other.num * &self.den,
            &self.den * &other.den,
        )
        .expect("denominators are non-zero")
    }
}

impl std::ops::Mul for &Number {
    type Output = Number;

    fn mul(self, other: &Number) -> Number {
        Number::new(&self.num * &other.num, &self.den * &other.den)
            .expect("denominators are non-zero")
    }
}

impl std::ops::Neg for &Number {
    type Output = Number;

    fn neg(self) -> Number {
        Number {
            num: -self.num.clone(),
            den: self.den.clone(),
        }
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        // Denominators are positive, so cross-multiplication preserves order.
        (&self.num * &other.den).cmp(&(&other.num * &self.den))
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den.is_one() {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(text: &str) -> Number {
        Number::parse(text).expect("literal parses")
    }

    #[test]
    fn literals_are_reduced_and_exact() {
        assert_eq!(n("4/2").to_string(), "2");
        assert_eq!(n("-6/-4").to_string(), "3/2");
        assert_eq!(n("2/-4").to_string(), "-1/2");
        assert_eq!(n("0.1").to_string(), "1/10");
        assert_eq!(n("-2.50").to_string(), "-5/2");
    }

    #[test]
    fn a_tenth_added_ten_times_is_exactly_one() {
        let mut total = Number::zero();
        for _ in 0..10 {
            total = &total + &n("0.1");
        }
        assert_eq!(total, Number::one());
    }

    #[test]
    fn precision_does_not_decay_over_large_magnitudes() {
        let huge = n("100000000000000000000000000000");
        let sum = &huge + &n("1/3");
        assert_eq!(&sum - &huge, n("1/3"));
    }

    #[test]
    fn division_by_zero_has_no_value() {
        assert!(n("1").checked_div(&Number::zero()).is_none());
    }

    #[test]
    fn ordering_is_by_value_not_representation() {
        assert!(n("1/3") < n("1/2"));
        assert!(n("-5") < n("-1/1000000"));
        assert_eq!(n("2/4").cmp(&n("1/2")), Ordering::Equal);
    }

    #[test]
    fn malformed_literals_are_rejected() {
        for bad in [
            "", "-", "1/", "/2", "1.", ".", "1.2.3", "0x10", "1e5", "--1",
        ] {
            assert!(Number::parse(bad).is_none(), "{bad} should not parse");
        }
    }
}
