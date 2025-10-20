// rust/src/types.rs

pub mod fraction;

use std::collections::HashSet;
use std::fmt;
use num_bigint::BigInt;
use num_traits::One;
use self::fraction::Fraction;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(String),
    String(String),
    Boolean(bool),
    Symbol(String),
    VectorStart(BracketType),
    VectorEnd(BracketType),
    GuardSeparator,  // : または ;
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
    pub body_tokens: Vec<Token>,
}

#[derive(Debug, Clone)]
pub struct WordDefinition {
    pub lines: Vec<ExecutionLine>,
    pub is_builtin: bool,
    pub description: Option<String>,
    pub dependencies: HashSet<String>,
    pub original_source: Option<String>,
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
            ValueType::Nil => write!(f, "nil"),
        }
    }
}

pub type Stack = Vec<Value>;

#[derive(Debug, Clone)]
pub enum AjisaiError {
    StackUnderflow,
    TypeError { expected: String, got: String },
    IndexOutOfBounds(i64),
    UnknownWord(String),
    DivisionByZero,
    InvalidOperation(String),
    ParseError(String),
    SyntaxError(String),
    RedefinitionError(String),
    Custom(String),
}

impl AjisaiError {
    pub fn type_error(expected: &str, got: &str) -> Self {
        AjisaiError::TypeError {
            expected: expected.to_string(),
            got: got.to_string(),
        }
    }
}

impl fmt::Display for AjisaiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AjisaiError::StackUnderflow => write!(f, "Stack underflow"),
            AjisaiError::TypeError { expected, got } => write!(f, "Type error: expected {}, got {}", expected, got),
            AjisaiError::IndexOutOfBounds(idx) => write!(f, "Index out of bounds: {}", idx),
            AjisaiError::UnknownWord(name) => write!(f, "Unknown word: {}", name),
            AjisaiError::DivisionByZero => write!(f, "Division by zero"),
            AjisaiError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            AjisaiError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            AjisaiError::SyntaxError(msg) => write!(f, "Syntax error: {}", msg),
            AjisaiError::RedefinitionError(msg) => write!(f, "Redefinition error: {}", msg),
            AjisaiError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for AjisaiError {}

impl From<String> for AjisaiError {
    fn from(s: String) -> Self {
        AjisaiError::Custom(s)
    }
}

impl From<&str> for AjisaiError {
    fn from(s: &str) -> Self {
        AjisaiError::Custom(s.to_string())
    }
}
