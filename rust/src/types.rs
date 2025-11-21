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

#[derive(Debug, Clone)]
pub enum ValueType {
    Number(Fraction),
    String(String),
    Boolean(bool),
    Symbol(String),
    // 単一要素ベクタの最適化版（メモリ効率向上）
    SingletonVector(Box<Value>, BracketType),
    Vector(Vec<Value>, BracketType),
    Nil,
}

// カスタムPartialEq実装: SingletonVectorとVectorの単一要素版を同等に扱う
impl PartialEq for ValueType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ValueType::Number(a), ValueType::Number(b)) => a == b,
            (ValueType::String(a), ValueType::String(b)) => a == b,
            (ValueType::Boolean(a), ValueType::Boolean(b)) => a == b,
            (ValueType::Symbol(a), ValueType::Symbol(b)) => a == b,
            (ValueType::Nil, ValueType::Nil) => true,
            // SingletonVector同士の比較
            (ValueType::SingletonVector(a, bt_a), ValueType::SingletonVector(b, bt_b)) => {
                bt_a == bt_b && a == b
            },
            // Vector同士の比較
            (ValueType::Vector(a, bt_a), ValueType::Vector(b, bt_b)) => {
                bt_a == bt_b && a == b
            },
            // SingletonVectorとVectorの相互比較（単一要素の場合）
            (ValueType::SingletonVector(a, bt_a), ValueType::Vector(b, bt_b)) => {
                bt_a == bt_b && b.len() == 1 && **a == b[0]
            },
            (ValueType::Vector(a, bt_a), ValueType::SingletonVector(b, bt_b)) => {
                bt_a == bt_b && a.len() == 1 && a[0] == **b
            },
            _ => false,
        }
    }
}

// Display トレイトの実装を追加
impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueType::Number(_) => write!(f, "number"),
            ValueType::String(_) => write!(f, "string"),
            ValueType::Boolean(_) => write!(f, "boolean"),
            ValueType::Symbol(_) => write!(f, "symbol"),
            ValueType::SingletonVector(_, _) => write!(f, "vector"),
            ValueType::Vector(_, _) => write!(f, "vector"),
            ValueType::Nil => write!(f, "nil"),
        }
    }
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
            ValueType::Boolean(b) => write!(f, "{}", if *b { "TRUE" } else { "FALSE" }),
            ValueType::Symbol(s) => write!(f, "{}", s),
            ValueType::SingletonVector(val, bracket_type) => {
                write!(f, "{}{}{}", bracket_type.opening_char(), val, bracket_type.closing_char())
            },
            ValueType::Vector(v, bracket_type) => {
                write!(f, "{}", bracket_type.opening_char())?;
                for (i, item) in v.iter().enumerate() {
                    if i > 0 { write!(f, " ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "{}", bracket_type.closing_char())
            },
            ValueType::Nil => write!(f, "NIL"),
        }
    }
}

pub type Stack = Vec<Value>;
