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
    // ブラケットタイプは表示層で深さから計算される
    Vector(Vec<Value>),
    Nil,
    TailCallMarker,  // 内部専用：末尾再帰最適化のマーカー
}

// Display トレイトの実装を追加
impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueType::Number(_) => write!(f, "number"),
            ValueType::String(_) => write!(f, "string"),
            ValueType::Boolean(_) => write!(f, "boolean"),
            ValueType::Symbol(_) => write!(f, "symbol"),
            ValueType::Vector(_) => write!(f, "vector"),
            ValueType::Nil => write!(f, "nil"),
            ValueType::TailCallMarker => write!(f, "<tail-call-marker>"),
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

impl Value {
    // ブラケットタイプは深さに基づいて計算される
    fn fmt_with_depth(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
        match &self.val_type {
            ValueType::Number(n) => {
                if n.denominator == BigInt::one() {
                    write!(f, "{}", n.numerator)
                } else {
                    write!(f, "{}/{}", n.numerator, n.denominator)
                }
            }
            ValueType::String(s) => write!(f, "'{}'", s),
            ValueType::Boolean(b) => write!(f, "{}", if *b { "TRUE" } else { "FALSE" }),
            ValueType::Symbol(s) => write!(f, "{}", s),
            ValueType::Vector(v) => {
                // 深さに基づいてブラケットタイプを決定
                let (open, close) = match depth % 3 {
                    0 => ('[', ']'),  // Square
                    1 => ('{', '}'),  // Curly
                    2 => ('(', ')'),  // Round
                    _ => unreachable!(),
                };
                write!(f, "{}", open)?;
                for (i, item) in v.iter().enumerate() {
                    if i > 0 { write!(f, " ")?; }
                    item.fmt_with_depth(f, depth + 1)?;
                }
                write!(f, "{}", close)
            }
            ValueType::Nil => write!(f, "NIL"),
            ValueType::TailCallMarker => write!(f, "<TAIL_CALL>"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_depth(f, 0)
    }
}

pub type Stack = Vec<Value>;
