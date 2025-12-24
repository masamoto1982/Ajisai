// rust/src/error.rs
//
// Ajisai言語のエラー型定義
// インタプリタ実行時およびパース時のエラーを統一的に管理

use std::fmt;

pub type Result<T> = std::result::Result<T, AjisaiError>;

#[derive(Debug, Clone)]
pub enum AjisaiError {
    StackUnderflow,
    TypeError { expected: String, got: String },
    UnknownWord(String),
    DivisionByZero,
    IndexOutOfBounds { index: i64, length: usize },
    VectorLengthMismatch { len1: usize, len2: usize },
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
            AjisaiError::TypeError { expected, got } => {
                write!(f, "Type error: expected {}, got {}", expected, got)
            },
            AjisaiError::UnknownWord(name) => write!(f, "Unknown word: {}", name),
            AjisaiError::DivisionByZero => write!(f, "Division by zero"),
            AjisaiError::IndexOutOfBounds { index, length } => {
                write!(f, "Index {} out of bounds for vector of length {}", index, length)
            },
            AjisaiError::VectorLengthMismatch { len1, len2 } => {
                write!(f, "Vector length mismatch: {} vs {}", len1, len2)
            },
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
