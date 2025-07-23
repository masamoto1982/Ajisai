use std::collections::HashMap;
use std::fmt;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Fraction {
    pub numerator: i64,
    pub denominator: i64,
}

impl Fraction {
    pub fn new(numerator: i64, denominator: i64) -> Self {
        if denominator == 0 {
            panic!("Division by zero");
        }
        let gcd = Self::gcd(numerator.abs(), denominator.abs());
        let sign = if (numerator < 0) ^ (denominator < 0) { -1 } else { 1 };
        Fraction {
            numerator: sign * numerator.abs() / gcd,
            denominator: denominator.abs() / gcd,
        }
    }

    fn gcd(mut a: i64, mut b: i64) -> i64 {
        while b != 0 {
            let temp = b;
            b = a % b;
            a = temp;
        }
        a
    }

    pub fn add(&self, other: &Fraction) -> Fraction {
        let num = self.numerator * other.denominator + other.numerator * self.denominator;
        let den = self.denominator * other.denominator;
        Fraction::new(num, den)
    }

    pub fn sub(&self, other: &Fraction) -> Fraction {
        let num = self.numerator * other.denominator - other.numerator * self.denominator;
        let den = self.denominator * other.denominator;
        Fraction::new(num, den)
    }

    pub fn mul(&self, other: &Fraction) -> Fraction {
        Fraction::new(self.numerator * other.numerator, self.denominator * other.denominator)
    }

    pub fn div(&self, other: &Fraction) -> Fraction {
        Fraction::new(self.numerator * other.denominator, self.denominator * other.numerator)
    }

    pub fn gt(&self, other: &Fraction) -> bool {
        self.numerator * other.denominator > other.numerator * self.denominator
    }

    pub fn ge(&self, other: &Fraction) -> bool {
        self.numerator * other.denominator >= other.numerator * self.denominator
    }

    pub fn lt(&self, other: &Fraction) -> bool {
        self.numerator * other.denominator < other.numerator * self.denominator
    }

    pub fn le(&self, other: &Fraction) -> bool {
        self.numerator * other.denominator <= other.numerator * self.denominator
    }

    pub fn to_f64(&self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }
}

impl fmt::Display for Fraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denominator == 1 {
            write!(f, "{}", self.numerator)
        } else {
            write!(f, "{}/{}", self.numerator, self.denominator)
        }
    }
}

impl From<i64> for Fraction {
    fn from(n: i64) -> Self {
        Fraction::new(n, 1)
    }
}

// トークン型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Token {
    Word(String),
    Number(Fraction),
    String(String),
    Symbol(String),
    Vector(Vec<Token>),
    Nil,
}

// 値型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValueType {
    Number(Fraction),
    String(String),
    Boolean(bool),
    Symbol(String),
    Vector(Vec<Value>),
    Quotation(Vec<Token>),
    Nil,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Value {
    pub val_type: ValueType,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.val_type {
            ValueType::Number(n) => write!(f, "{}", n),
            ValueType::String(s) => write!(f, "\"{}\"", s),
            ValueType::Boolean(b) => write!(f, "{}", b),
            ValueType::Symbol(s) => write!(f, "{}", s),
            ValueType::Vector(v) => {
                write!(f, "[ ")?;
                for (i, item) in v.iter().enumerate() {
                    if i > 0 { write!(f, " ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, " ]")
            },
            ValueType::Quotation(_) => write!(f, "{{ ... }}"),
            ValueType::Nil => write!(f, "nil"),
        }
    }
}

// ワード定義
#[derive(Debug, Clone)]
pub struct WordDefinition {
    pub tokens: Vec<Token>,
    pub is_builtin: bool,
    pub description: Option<String>,
}
// 型エイリアス
pub type Stack = Vec<Value>;
pub type Register = Option<Value>;