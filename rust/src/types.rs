// rust/src/types.rs (BigInt対応・完全修正版)

use std::fmt;
use num_bigint::BigInt;
use num_traits::{Zero, One, ToPrimitive};
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
    QuotationStart,
    QuotationEnd,
    Nil,
    FunctionComment(String),
    At,
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
    Quotation(Vec<Token>),
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
        // 空文字列チェック
        if s.is_empty() {
            return Err("Empty string cannot be parsed as number".to_string());
        }
        
        // 分数形式 (例: "3/4")
        if let Some(pos) = s.find('/') {
            let num_str = &s[..pos];
            let den_str = &s[pos+1..];
            
            if num_str.is_empty() || den_str.is_empty() {
                return Err("Invalid fraction format".to_string());
            }
            
            let num = BigInt::from_str(num_str).map_err(|e| format!("Failed to parse numerator: {}", e))?;
            let den = BigInt::from_str(den_str).map_err(|e| format!("Failed to parse denominator: {}", e))?;
            
            if den.is_zero() {
                return Err("Denominator cannot be zero".to_string());
            }
            
            Ok(Fraction::new(num, den))
        } 
        // 小数形式 (例: "3.14")
        else if let Some(dot_pos) = s.find('.') {
            let int_part_str = &s[..dot_pos];
            let frac_part_str = &s[dot_pos+1..];
            
            // 整数部が空の場合は"0"として扱う
            let int_part_str = if int_part_str.is_empty() || int_part_str == "-" {
                if int_part_str == "-" { "-0" } else { "0" }
            } else {
                int_part_str
            };
            
            let int_part = BigInt::from_str(int_part_str).map_err(|e| format!("Failed to parse integer part: {}", e))?;
            
            if frac_part_str.is_empty() {
                // ".5"のような形式は"0.5"として扱う
                return Ok(Fraction::new(int_part, BigInt::one()));
            }
            
            // 小数部を分数に変換
            let frac_num = BigInt::from_str(frac_part_str).map_err(|e| format!("Failed to parse fractional part: {}", e))?;
            let frac_den = BigInt::from(10).pow(frac_part_str.len() as u32);
            
            // 負の数の場合の処理
            let is_negative = int_part < BigInt::zero();
            let abs_int_part = if is_negative { -&int_part } else { int_part.clone() };
            
            let total_num = abs_int_part * &frac_den + frac_num;
            let final_num = if is_negative { -total_num } else { total_num };
            
            Ok(Fraction::new(final_num, frac_den))
        } 
        // 整数形式
        else {
            let num = BigInt::from_str(s).map_err(|e| format!("Failed to parse integer: {}", e))?;
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
                write!(f, "{}", bracket_type.opening_char())?;
                for (i, item) in v.iter().enumerate() {
                    if i > 0 { write!(f, " ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "{}", bracket_type.closing_char())
            },
            ValueType::Quotation(_) => write!(f, ": ... ;"),
            ValueType::Nil => write!(f, "nil"),
        }
    }
}

pub type Workspace = Vec<Value>;
