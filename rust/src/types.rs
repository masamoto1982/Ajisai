// rust/src/types.rs (BigInt対応・エラー修正版)

use std::fmt;
use num_bigint::BigInt;
use num_traits::{Zero, One, ToPrimitive};
use num_integer::Integer;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(String),
    String(String),
    Boolean(bool),
    Symbol(String),
    VectorStart(BracketType),
    VectorEnd(BracketType),
    Nil,
    FunctionComment(String),
    Colon,
    LineBreak,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    pub val_type: ValueType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValueType {
    Number(Fraction),
    String(String),
    Boolean(bool),
    Symbol(String),
    Vector(Vec<Value>, BracketType),
    Nil,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BracketType {
    Square,
    Curly,
    Round,
}

impl BracketType {
    pub fn opening_char(&self) -> char {
        match self {
            BracketType::Square => '[',
            BracketType::Curly => '{',
            BracketType::Round => '(',
        }
    }
    
    pub fn closing_char(&self) -> char {
        match self {
            BracketType::Square => ']',
            BracketType::Curly => '}',
            BracketType::Round => ')',
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fraction {
    pub numerator: BigInt,
    pub denominator: BigInt,
}

impl Fraction {
    pub fn new(numerator: BigInt, denominator: BigInt) -> Self {
        if denominator.is_zero() {
            panic!("Division by zero");
        }
        
        let common = numerator.gcd(&denominator);
        let mut num = &numerator / &common;
        let mut den = &denominator / &common;
        
        if den < BigInt::zero() {
            num = -num;
            den = -den;
        }
        
        Fraction {
            numerator: num,
            denominator: den,
        }
    }
    
    pub fn from_str(s: &str) -> std::result::Result<Self, String> {
        if let Some(pos) = s.find('/') {
            let num_str = &s[..pos];
            let den_str = &s[pos+1..];
            let num = num_str.parse::<BigInt>().map_err(|e| e.to_string())?;
            let den = den_str.parse::<BigInt>().map_err(|e| e.to_string())?;
            Ok(Fraction::new(num, den))
        } else if let Some(_pos) = s.find('.') {
            let mut parts = s.split('.');
            let int_part_str = parts.next().unwrap_or("0");
            let frac_part_str = parts.next().unwrap_or("");
            
            let int_part = int_part_str.parse::<BigInt>().map_err(|e| e.to_string())?;
            
            let mut frac_num = BigInt::zero();
            if !frac_part_str.is_empty() {
                frac_num = frac_part_str.parse::<BigInt>().map_err(|e| e.to_string())?;
            }
            
            let frac_den = BigInt::from(10).pow(frac_part_str.len() as u32);
            
            let numerator = int_part * &frac_den + if s.starts_with('-') { -frac_num } else { frac_num };

            Ok(Fraction::new(numerator, frac_den))
        } else {
            let num = s.parse::<BigInt>().map_err(|e| e.to_string())?;
            Ok(Fraction::new(num, BigInt::one()))
        }
    }

    pub fn to_i64(&self) -> Option<i64> {
        if self.denominator == BigInt::one() {
            self.numerator.to_i64()
        } else {
            None
        }
    }
    
    pub fn add(&self, other: &Fraction) -> Fraction {
        let num = &self.numerator * &other.denominator + &other.numerator * &self.denominator;
        let den = &self.denominator * &other.denominator;
        Fraction::new(num, den)
    }
    
    pub fn sub(&self, other: &Fraction) -> Fraction {
        let num = &self.numerator * &other.denominator - &other.numerator * &self.denominator;
        let den = &self.denominator * &other.denominator;
        Fraction::new(num, den)
    }
    
    pub fn mul(&self, other: &Fraction) -> Fraction {
        let num = &self.numerator * &other.numerator;
        let den = &self.denominator * &other.denominator;
        Fraction::new(num, den)
    }
    
    pub fn div(&self, other: &Fraction) -> Fraction {
        if other.numerator.is_zero() {
            panic!("Division by zero");
        }
        let num = &self.numerator * &other.denominator;
        let den = &self.denominator * &other.numerator;
        Fraction::new(num, den)
    }
    
    pub fn lt(&self, other: &Fraction) -> bool {
        &self.numerator * &other.denominator < &other.numerator * &self.denominator
    }
    
    pub fn le(&self, other: &Fraction) -> bool {
        &self.numerator * &other.denominator <= &other.numerator * &self.denominator
    }
    
    pub fn gt(&self, other: &Fraction) -> bool {
        &self.numerator * &other.denominator > &other.numerator * &self.denominator
    }
    
    pub fn ge(&self, other: &Fraction) -> bool {
        &self.numerator * &other.denominator >= &other.numerator * &self.denominator
    }
    
    pub fn eq(&self, other: &Fraction) -> bool {
        self.numerator == other.numerator && self.denominator == other.denominator
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.val_type {
            ValueType::Number(n) => {
                if n.denominator == BigInt::one() {
                    write!(f, "{}", n.numerator)
                } else {
                    write!(f, "{}/{}", n.numerator, n.denominator)
                }
            },
            ValueType::String(s) => write!(f, "'{}'", s),
            ValueType::Boolean(b) => write!(f, "{}", b),
            ValueType::Symbol(s) => write!(f, "{}", s),
            ValueType::Vector(v, bracket_type) => {
                write!(f, "{} ", bracket_type.opening_char())?;
                for (i, item) in v.iter().enumerate() {
                    if i > 0 { write!(f, " ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, " {}", bracket_type.closing_char())
            },
            ValueType::Nil => write!(f, "nil"),
        }
    }
}

pub type Workspace = Vec<Value>;
