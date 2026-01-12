// rust/src/error.rs
//
// Ajisai言語のエラー型定義
// インタプリタ実行時およびパース時のエラーを統一的に管理
//
// 統一分数アーキテクチャ：
// Ajisaiでは「型」という概念は廃止されています。
// すべての値は内部的に Vec<Fraction> として表現され、
// エラーは構造的な要件（要素数、形状等）の不一致として報告されます。

use std::fmt;

pub type Result<T> = std::result::Result<T, AjisaiError>;

#[derive(Debug, Clone)]
pub enum AjisaiError {
    StackUnderflow,
    /// 構造エラー: 期待される構造（要素数、形状等）と実際の構造が一致しない
    StructureError { expected: String, got: String },
    UnknownWord(String),
    DivisionByZero,
    IndexOutOfBounds { index: i64, length: usize },
    VectorLengthMismatch { len1: usize, len2: usize },
    Custom(String),
}

impl AjisaiError {
    /// 構造エラーを生成する
    /// 統一分数アーキテクチャでは「型」ではなく「構造」の不一致としてエラーを報告
    pub fn structure_error(expected: &str, got: &str) -> Self {
        AjisaiError::StructureError {
            expected: expected.to_string(),
            got: got.to_string(),
        }
    }
}

impl fmt::Display for AjisaiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AjisaiError::StackUnderflow => write!(f, "Stack underflow"),
            AjisaiError::StructureError { expected, got } => {
                write!(f, "Structure error: expected {}, got {}", expected, got)
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
