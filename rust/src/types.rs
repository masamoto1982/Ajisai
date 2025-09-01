// rust/src/types.rs (括弧タイプ保持版)

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(i64, i64),
    String(String),
    Boolean(bool),
    Symbol(String),
    VectorStart(BracketType),    // 括弧タイプを保持
    VectorEnd(BracketType),      // 括弧タイプを保持
    Nil,
    FunctionComment(String),
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
    Vector(Vec<Value>, BracketType),  // 括弧タイプを追加
    Nil,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BracketType {
    Square,  // [ ]
    Curly,   // { }
    Round,   // ( )
}

impl BracketType {
    pub fn from_char(c: char) -> Self {
        match c {
            '[' | ']' => BracketType::Square,
            '{' | '}' => BracketType::Curly,
            '(' | ')' => BracketType::Round,
            _ => panic!("Invalid bracket character: {}", c),
        }
    }
    
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

// Fraction構造体とその実装
#[derive(Debug, Clone, PartialEq)]
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
        let mut num = numerator / gcd;
        let mut den = denominator / gcd;
        
        if den < 0 {
            num = -num;
            den = -den;
        }
        
        Fraction {
            numerator: num,
            denominator: den,
        }
    }
    
    fn gcd(a: i64, b: i64) -> i64 {
        if b == 0 { a } else { Self::gcd(b, a % b) }
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
        let num = self.numerator * other.numerator;
        let den = self.denominator * other.denominator;
        Fraction::new(num, den)
    }
    
    pub fn div(&self, other: &Fraction) -> Fraction {
        if other.numerator == 0 {
            panic!("Division by zero");
        }
        let num = self.numerator * other.denominator;
        let den = self.denominator * other.numerator;
        Fraction::new(num, den)
    }
    
    pub fn gt(&self, other: &Fraction) -> bool {
        self.numerator * other.denominator > other.numerator * self.denominator
    }
    
    pub fn ge(&self, other: &Fraction) -> bool {
        self.numerator * other.denominator >= other.numerator * self.denominator
    }
    
    pub fn eq(&self, other: &Fraction) -> bool {
        self.numerator * other.denominator == other.numerator * self.denominator
    }
    
    pub fn lt(&self, other: &Fraction) -> bool {
        self.numerator * other.denominator < other.numerator * self.denominator
    }
    
    pub fn le(&self, other: &Fraction) -> bool {
        self.numerator * other.denominator <= other.numerator * self.denominator
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.val_type {
            ValueType::Number(n) => {
                if n.denominator == 1 {
                    write!(f, "{}", n.numerator)
                } else {
                    write!(f, "{}/{}", n.numerator, n.denominator)
                }
            },
            ValueType::String(s) => write!(f, "'{}'", s),
            ValueType::Boolean(b) => write!(f, "{}", b),
            ValueType::Symbol(s) => write!(f, "{}", s),
            ValueType::Vector(v, bracket_type) => {
                let open = bracket_type.opening_char();
                let close = bracket_type.closing_char();
                
                write!(f, "{} ", open)?;
                for (i, item) in v.iter().enumerate() {
                    if i > 0 { write!(f, " ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, " {}", close)
            },
            ValueType::Nil => write!(f, "nil"),
        }
    }
}

pub type Workspace = Vec<Value>;
