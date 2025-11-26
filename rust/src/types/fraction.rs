// rust/src/types/fraction.rs

use num_bigint::BigInt;
use num_traits::{Zero, One, ToPrimitive, Signed};
use num_integer::Integer;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fraction {
    pub numerator: BigInt,
    pub denominator: BigInt,
}

impl Fraction {
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

    pub fn add(&self, other: &Fraction) -> Fraction { Fraction::new(&self.numerator * &other.denominator + &other.numerator * &self.denominator, &self.denominator * &other.denominator) }
    pub fn sub(&self, other: &Fraction) -> Fraction { Fraction::new(&self.numerator * &other.denominator - &other.numerator * &self.denominator, &self.denominator * &other.denominator) }
    pub fn mul(&self, other: &Fraction) -> Fraction { Fraction::new(&self.numerator * &other.numerator, &self.denominator * &other.denominator) }
    pub fn div(&self, other: &Fraction) -> Fraction { if other.numerator.is_zero() { panic!("Division by zero"); } Fraction::new(&self.numerator * &other.denominator, &self.denominator * &other.numerator) }
    pub fn lt(&self, other: &Fraction) -> bool { &self.numerator * &other.denominator < &other.numerator * &self.denominator }
    pub fn le(&self, other: &Fraction) -> bool { &self.numerator * &other.denominator <= &other.numerator * &self.denominator }
    pub fn gt(&self, other: &Fraction) -> bool { &self.numerator * &other.denominator > &other.numerator * &self.denominator }
    pub fn ge(&self, other: &Fraction) -> bool { &self.numerator * &other.denominator >= &other.numerator * &self.denominator }
    pub fn eq(&self, other: &Fraction) -> bool { self == other }
}

impl PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Fraction {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare a/b with c/d using a*d vs b*c (integer comparison)
        let lhs = &self.numerator * &other.denominator;
        let rhs = &other.numerator * &self.denominator;
        lhs.cmp(&rhs)
    }
}

impl ToPrimitive for Fraction {
    fn to_i64(&self) -> Option<i64> {
        // Division result as i64
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
        // Convert numerator and denominator to f64, then divide
        let num_f64 = self.numerator.to_f64()?;
        let den_f64 = self.denominator.to_f64()?;
        if den_f64 == 0.0 {
            None
        } else {
            Some(num_f64 / den_f64)
        }
    }
}
