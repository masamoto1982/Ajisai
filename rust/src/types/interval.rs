use crate::error::{AjisaiError, Result};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_traits::{Signed, Zero};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interval {
    pub lo: Fraction,
    pub hi: Fraction,
}

impl Interval {
    pub fn new(lo: Fraction, hi: Fraction) -> Result<Self> {
        if lo.gt(&hi) {
            return Err(AjisaiError::from("invalid interval: lo must be <= hi"));
        }
        Ok(Self { lo, hi })
    }

    pub fn from_scalar(v: Fraction) -> Self {
        Self {
            lo: v.clone(),
            hi: v,
        }
    }

    pub fn is_exact(&self) -> bool {
        self.lo == self.hi
    }

    pub fn width(&self) -> Fraction {
        self.hi.sub(&self.lo)
    }

    pub fn add(&self, other: &Self) -> Self {
        Self {
            lo: self.lo.add(&other.lo),
            hi: self.hi.add(&other.hi),
        }
    }

    pub fn sub(&self, other: &Self) -> Self {
        Self {
            lo: self.lo.sub(&other.hi),
            hi: self.hi.sub(&other.lo),
        }
    }

    pub fn neg(&self) -> Self {
        Self {
            lo: self.hi.mul(&Fraction::from(-1)),
            hi: self.lo.mul(&Fraction::from(-1)),
        }
    }

    pub fn mul(&self, other: &Self) -> Self {
        let ac = self.lo.mul(&other.lo);
        let ad = self.lo.mul(&other.hi);
        let bc = self.hi.mul(&other.lo);
        let bd = self.hi.mul(&other.hi);
        let mut lo = ac.clone();
        let mut hi = ac;
        for v in [&ad, &bc, &bd] {
            if v.lt(&lo) {
                lo = v.clone();
            }
            if v.gt(&hi) {
                hi = v.clone();
            }
        }
        Self { lo, hi }
    }

    pub fn contains_zero(&self) -> bool {
        self.lo.le(&Fraction::from(0)) && self.hi.ge(&Fraction::from(0))
    }

    pub fn reciprocal(&self) -> Result<Self> {
        if self.contains_zero() {
            return Err(AjisaiError::from("division by interval containing zero"));
        }
        let one = Fraction::from(1);
        let a = one.div(&self.lo);
        let b = one.div(&self.hi);
        if a.le(&b) {
            Ok(Self { lo: a, hi: b })
        } else {
            Ok(Self { lo: b, hi: a })
        }
    }

    pub fn div(&self, other: &Self) -> Result<Self> {
        let recip = other.reciprocal()?;
        Ok(self.mul(&recip))
    }
}

fn bigint_sqrt_floor(n: &BigInt) -> BigInt {
    if n.is_zero() {
        return BigInt::zero();
    }
    let mut x0 = n.clone();
    let two = BigInt::from(2u8);
    let mut x1 = (&x0 + (n / &x0)) / &two;
    while x1 < x0 {
        x0 = x1.clone();
        x1 = (&x0 + (n / &x0)) / &two;
    }
    x0
}

pub fn exact_rational_sqrt(q: &Fraction) -> Option<Fraction> {
    if q.lt(&Fraction::from(0)) {
        return None;
    }
    let n = q.numerator();
    let d = q.denominator();
    if n.is_negative() {
        return None;
    }
    let sn = bigint_sqrt_floor(&n);
    let sd = bigint_sqrt_floor(&d);
    if &sn * &sn == n && &sd * &sd == d {
        return Some(Fraction::new(sn, sd));
    }
    None
}

pub fn sqrt_rational_interval(q: &Fraction, eps: &Fraction) -> Result<Interval> {
    if q.lt(&Fraction::from(0)) {
        return Err(AjisaiError::from("sqrt of negative value"));
    }
    if eps.le(&Fraction::from(0)) {
        return Err(AjisaiError::from("sqrt precision must be positive"));
    }
    if q.is_zero() {
        return Ok(Interval::from_scalar(Fraction::from(0)));
    }
    if let Some(exact) = exact_rational_sqrt(q) {
        return Ok(Interval::from_scalar(exact));
    }

    let zero = Fraction::from(0);
    let one = Fraction::from(1);
    let mut lo = zero.clone();
    let mut hi = if q.ge(&one) { q.clone() } else { one };
    while hi.sub(&lo).gt(eps) {
        let mid = lo.add(&hi).div(&Fraction::from(2));
        let mid_sq = mid.mul(&mid);
        if mid_sq.le(q) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Interval::new(lo, hi)
}

pub fn default_sqrt_eps() -> Fraction {
    Fraction::new(BigInt::from(1), BigInt::from(1_000_000))
}
