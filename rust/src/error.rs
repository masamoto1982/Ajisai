use std::fmt;

pub type Result<T> = std::result::Result<T, AjisaiError>;

#[derive(Debug, Clone)]
pub enum AjisaiError {
    StackUnderflow,
    StructureError { expected: String, got: String },
    UnknownWord(String),
    DivisionByZero,
    IndexOutOfBounds { index: i64, length: usize },
    VectorLengthMismatch { len1: usize, len2: usize },
    DepthLimitExceeded { depth: usize, chain: String },
    DimensionLimitExceeded { depth: usize },
    NoChange { word: String },
    ModeUnsupported { word: String, mode: String },
    BuiltinProtection { word: String, operation: String },
    Custom(String),
}

impl AjisaiError {
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
            AjisaiError::DepthLimitExceeded { depth, chain } => {
                write!(f, "Call depth limit ({}) exceeded: {}", depth, chain)
            },
            AjisaiError::DimensionLimitExceeded { depth } => {
                write!(f, "Nesting depth limit exceeded: Ajisai supports up to 10 dimensions. Nesting depth {} exceeds the limit.", depth)
            },
            AjisaiError::NoChange { word } => {
                write!(f, "No change: {} produced no effect", word)
            },
            AjisaiError::ModeUnsupported { word, mode } => {
                write!(f, "{} does not support {} mode (..)", word, mode)
            },
            AjisaiError::BuiltinProtection { word, operation } => {
                write!(f, "Cannot {} built-in word: {}", operation, word)
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
