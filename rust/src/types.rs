// rust/src/types.rs

use std::collections::HashSet;
use std::fmt;
use num_bigint::BigInt;
use num_traits::{Zero, One, ToPrimitive, Signed};
use num_integer::Integer;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(String),
    String(String),
    Boolean(bool),
    Symbol(String),
    VectorStart(BracketType),
    VectorEnd(BracketType),
    DefBlockStart, // : (now used for condition separator)
    DefBlockEnd,   // ; (deprecated)
    GuardSeparator, // : (replaces $)
    Modifier(String), // 3x, 5s
    Nil,
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
    DefinitionBody(Vec<Token>),
    Nil,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BracketType {
    Square, Curly, Round,
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

#[derive(Debug, Clone)]
pub struct ExecutionLine {
    pub condition_tokens: Vec<Token>,
    pub body_tokens: Vec<Token>,
    pub repeat_count: i64,
    pub delay_ms: u64,
}

#[derive(Debug, Clone)]
pub struct WordDefinition {
    pub lines: Vec<ExecutionLine>,
    pub is_builtin: bool,
    pub description: Option<String>,
    pub dependencies: HashSet<String>,
    pub original_source: Option<String>, // 🆕 元のソースコード保存
}

#[derive(Debug, Clone, PartialEq)]
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

    pub fn to_i64(&self) -> Option<i64> {
        if self.denominator == BigInt::one() { self.numerator.to_i64() } else { None }
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

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.val_type {
            ValueType::Number(n) => if n.denominator == BigInt::one() { write!(f, "{}", n.numerator) } else { write!(f, "{}/{}", n.numerator, n.denominator) },
            ValueType::String(s) => write!(f, "'{}'", s),
            ValueType::Boolean(b) => write!(f, "{}", b),
            ValueType::Symbol(s) => write!(f, "{}", s),
            ValueType::Vector(v, bracket_type) => {
                write!(f, "{}", bracket_type.opening_char())?;
                for (i, item) in v.iter().enumerate() {
                    if i > 0 { write!(f, " ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "{}", bracket_type.closing_char())
            },
            ValueType::DefinitionBody(_) => write!(f, ": ... ;"),
            ValueType::Nil => write!(f, "nil"),
        }
    }
}

pub type Workspace = Vec<Value>;
