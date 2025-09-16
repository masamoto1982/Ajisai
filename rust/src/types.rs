// rust/src/types.rs - ビルドエラー修正版

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
    VectorStart,
    VectorEnd,
    Nil,
    RepeatUnit(RepeatControl),
    TimeUnit(TimeControl),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RepeatControl {
    Times(u32),        // 3x
    Repetitions(u32),  // 5rep
    Iterations(u32),   // 10iter
    While,             // WHILE
    Until,             // UNTIL
    Forever,           // FOREVER
    Once,              // ONCE
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimeControl {
    Seconds(f64),      // 2s, 1.5s
    Milliseconds(u32), // 100ms, 500ms
    FPS(u32),          // 60fps
    Immediate,         // 即座実行
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
    Vector(Vec<Value>),
    Nil,
    ExecutionLine(ExecutionLine),
    WordDefinition(WordDefinition),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionLine {
    pub repeat: RepeatControl,
    pub timing: TimeControl,
    pub condition: Option<Vec<Value>>,
    pub action: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WordDefinition {
    pub name: String,
    pub lines: Vec<ExecutionLine>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fraction {
    pub numerator: BigInt,
    pub denominator: BigInt,
}

// Workspaceの型エイリアスを追加
pub type Workspace = Vec<Value>;

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
        if s.is_empty() {
            return Err("Empty string cannot be parsed as number".to_string());
        }
        
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
        } else if let Some(dot_pos) = s.find('.') {
            let int_part_str = &s[..dot_pos];
            let frac_part_str = &s[dot_pos+1..];
            
            let int_part_str = if int_part_str.is_empty() || int_part_str == "-" {
                if int_part_str == "-" { "-0" } else { "0" }
            } else {
                int_part_str
            };
            
            let int_part = BigInt::from_str(int_part_str).map_err(|e| format!("Failed to parse integer part: {}", e))?;
            
            if frac_part_str.is_empty() {
                return Ok(Fraction::new(int_part, BigInt::one()));
            }
            
            let frac_num = BigInt::from_str(frac_part_str).map_err(|e| format!("Failed to parse fractional part: {}", e))?;
            let frac_den = BigInt::from(10).pow(frac_part_str.len() as u32);
            
            let is_negative = int_part < BigInt::zero();
            let abs_int_part = if is_negative { -&int_part } else { int_part.clone() };
            
            let total_num = abs_int_part * &frac_den + frac_num;
            let final_num = if is_negative { -total_num } else { total_num };
            
            Ok(Fraction::new(final_num, frac_den))
        } else {
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

impl Default for RepeatControl {
    fn default() -> Self {
        RepeatControl::Once
    }
}

impl Default for TimeControl {
    fn default() -> Self {
        TimeControl::Immediate
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_depth(f, 0)
    }
}

impl Value {
    fn fmt_with_depth(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
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
            ValueType::Vector(v) => {
                let (open_bracket, close_bracket) = get_bracket_for_depth(depth);
                write!(f, "{}", open_bracket)?;
                for (i, item) in v.iter().enumerate() {
                    if i > 0 { write!(f, " ")?; }
                    item.fmt_with_depth(f, depth + 1)?;
                }
                write!(f, "{}", close_bracket)
            },
            ValueType::Nil => write!(f, "nil"),
            ValueType::ExecutionLine(line) => write!(f, "ExecutionLine({:?})", line),
            ValueType::WordDefinition(def) => write!(f, "WordDef({})", def.name),
        }
    }
}

fn get_bracket_for_depth(depth: usize) -> (char, char) {
    match depth % 3 {
        0 => ('[', ']'),  // レベル 0, 3, 6, ...
        1 => ('{', '}'),  // レベル 1, 4, 7, ...
        2 => ('(', ')'),  // レベル 2, 5, 8, ...
        _ => unreachable!(),
    }
}

impl fmt::Display for RepeatControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepeatControl::Times(n) => write!(f, "{}x", n),
            RepeatControl::Repetitions(n) => write!(f, "{}rep", n),
            RepeatControl::Iterations(n) => write!(f, "{}iter", n),
            RepeatControl::While => write!(f, "WHILE"),
            RepeatControl::Until => write!(f, "UNTIL"),
            RepeatControl::Forever => write!(f, "FOREVER"),
            RepeatControl::Once => write!(f, "ONCE"),
        }
    }
}

impl fmt::Display for TimeControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeControl::Seconds(s) => write!(f, "{}s", s),
            TimeControl::Milliseconds(ms) => write!(f, "{}ms", ms),
            TimeControl::FPS(fps) => write!(f, "{}fps", fps),
            TimeControl::Immediate => write!(f, "immediate"),
        }
    }
}
