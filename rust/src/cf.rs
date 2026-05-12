//! Continued fraction representation.
//!
//! A finite continued fraction `[a0; a1, a2, ..., an]` represents
//! `a0 + 1 / (a1 + 1 / (a2 + ... 1/an))`.
//!
//! Canonical nested display: `(a0 (a1 (a2 (... (an)))))`.
//!
//! Nil (the "bubble") is represented by an empty coefficient vector and
//! propagates through every arithmetic operation.

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuedFraction {
    /// Partial quotients `[a0, a1, ..., an]`.
    /// Empty vector encodes Nil (bubble).
    coeffs: Vec<BigInt>,
}

impl ContinuedFraction {
    pub fn nil() -> Self {
        Self { coeffs: Vec::new() }
    }

    pub fn is_nil(&self) -> bool {
        self.coeffs.is_empty()
    }

    pub fn coeffs(&self) -> &[BigInt] {
        &self.coeffs
    }

    pub fn from_int(n: BigInt) -> Self {
        Self { coeffs: vec![n] }
    }

    /// Build a continued fraction from rational `p / q`.
    /// Returns Nil when the denominator is zero.
    pub fn from_ratio(p: BigInt, q: BigInt) -> Self {
        if q.is_zero() {
            return Self::nil();
        }
        let (mut p, mut q) = if q.is_negative() { (-p, -q) } else { (p, q) };
        let mut coeffs: Vec<BigInt> = Vec::new();
        loop {
            let (quot, rem) = p.div_mod_floor(&q);
            coeffs.push(quot);
            if rem.is_zero() {
                break;
            }
            p = q;
            q = rem;
        }
        Self { coeffs }.canonicalize()
    }

    /// Convert a decimal string (e.g. "3.14", "-0.5") to a rational, then to CF.
    pub fn from_decimal_str(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        let (sign, rest) = match trimmed.strip_prefix('-') {
            Some(r) => (-1i32, r),
            None => (1i32, trimmed.strip_prefix('+').unwrap_or(trimmed)),
        };
        let mut parts = rest.splitn(2, '.');
        let int_part = parts.next().unwrap_or("");
        let frac_part = parts.next().unwrap_or("");
        if int_part.is_empty() && frac_part.is_empty() {
            return None;
        }
        let int_digits = if int_part.is_empty() { "0" } else { int_part };
        if !int_digits.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        if !frac_part.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let combined = format!("{}{}", int_digits, frac_part);
        let numerator_unsigned: BigInt = combined.parse().ok()?;
        let mut denom: BigInt = BigInt::one();
        for _ in 0..frac_part.len() {
            denom *= 10;
        }
        let numerator = if sign < 0 { -numerator_unsigned } else { numerator_unsigned };
        Some(Self::from_ratio(numerator, denom))
    }

    /// Convert back to a reduced rational `(p, q)` with `q > 0`.
    /// Returns `None` for Nil.
    pub fn to_ratio(&self) -> Option<(BigInt, BigInt)> {
        if self.coeffs.is_empty() {
            return None;
        }
        let mut p = BigInt::one();
        let mut q = BigInt::zero();
        for ai in self.coeffs.iter().rev() {
            let new_p = ai * &p + &q;
            let new_q = p;
            p = new_p;
            q = new_q;
        }
        Some((p, q))
    }

    /// Canonicalise: ensure that for length > 1 the last coefficient is >= 2.
    /// `[..., a, 1]` is rewritten as `[..., a+1]` because both denote the same value.
    fn canonicalize(mut self) -> Self {
        if self.coeffs.len() > 1 {
            if let Some(last) = self.coeffs.last() {
                if last.is_one() {
                    let last = self.coeffs.pop().unwrap();
                    if let Some(prev) = self.coeffs.last_mut() {
                        *prev += last;
                    }
                }
            }
        }
        self
    }

    /// Nested canonical display: `(a0 (a1 (a2)))`. Nil renders as `Nil`.
    pub fn nested_display(&self) -> String {
        if self.coeffs.is_empty() {
            return "Nil".to_string();
        }
        let mut s = String::new();
        for (i, c) in self.coeffs.iter().enumerate() {
            if i > 0 {
                s.push(' ');
            }
            s.push('(');
            s.push_str(&c.to_string());
        }
        for _ in 0..self.coeffs.len() {
            s.push(')');
        }
        s
    }

    /// Human display of the represented rational, e.g. `3/4`, `42`.
    pub fn rational_display(&self) -> String {
        match self.to_ratio() {
            None => "Nil".to_string(),
            Some((p, q)) => {
                if q.is_one() {
                    p.to_string()
                } else {
                    format!("{}/{}", p, q)
                }
            }
        }
    }
}

impl fmt::Display for ContinuedFraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.nested_display())
    }
}

fn ratio_add(a: (BigInt, BigInt), b: (BigInt, BigInt)) -> (BigInt, BigInt) {
    let (p1, q1) = a;
    let (p2, q2) = b;
    (&p1 * &q2 + &p2 * &q1, q1 * q2)
}

fn ratio_sub(a: (BigInt, BigInt), b: (BigInt, BigInt)) -> (BigInt, BigInt) {
    let (p1, q1) = a;
    let (p2, q2) = b;
    (&p1 * &q2 - &p2 * &q1, q1 * q2)
}

fn ratio_mul(a: (BigInt, BigInt), b: (BigInt, BigInt)) -> (BigInt, BigInt) {
    (a.0 * b.0, a.1 * b.1)
}

fn ratio_div(a: (BigInt, BigInt), b: (BigInt, BigInt)) -> Option<(BigInt, BigInt)> {
    if b.0.is_zero() {
        return None;
    }
    Some((a.0 * b.1, a.1 * b.0))
}

pub fn add(a: &ContinuedFraction, b: &ContinuedFraction) -> ContinuedFraction {
    match (a.to_ratio(), b.to_ratio()) {
        (Some(x), Some(y)) => {
            let (p, q) = ratio_add(x, y);
            ContinuedFraction::from_ratio(p, q)
        }
        _ => ContinuedFraction::nil(),
    }
}

pub fn sub(a: &ContinuedFraction, b: &ContinuedFraction) -> ContinuedFraction {
    match (a.to_ratio(), b.to_ratio()) {
        (Some(x), Some(y)) => {
            let (p, q) = ratio_sub(x, y);
            ContinuedFraction::from_ratio(p, q)
        }
        _ => ContinuedFraction::nil(),
    }
}

pub fn mul(a: &ContinuedFraction, b: &ContinuedFraction) -> ContinuedFraction {
    match (a.to_ratio(), b.to_ratio()) {
        (Some(x), Some(y)) => {
            let (p, q) = ratio_mul(x, y);
            ContinuedFraction::from_ratio(p, q)
        }
        _ => ContinuedFraction::nil(),
    }
}

pub fn div(a: &ContinuedFraction, b: &ContinuedFraction) -> ContinuedFraction {
    match (a.to_ratio(), b.to_ratio()) {
        (Some(x), Some(y)) => match ratio_div(x, y) {
            Some((p, q)) => ContinuedFraction::from_ratio(p, q),
            None => ContinuedFraction::nil(),
        },
        _ => ContinuedFraction::nil(),
    }
}
