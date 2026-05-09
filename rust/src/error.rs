use std::fmt;

pub type Result<T> = std::result::Result<T, AjisaiError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NilReason {
    DivisionByZero,
    EmptySequence,
    MissingField,
    InvalidEncoding,
    InvalidLens,
    StackUnderflow,
    IndexOutOfBounds,
    UnknownWord,
    ExecutionFailure,
    SafeCaught(Box<ErrorCategory>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    StackUnderflow,
    StructureError,
    UnknownWord,
    UnknownModule,
    DivisionByZero,
    IndexOutOfBounds,
    VectorLengthMismatch,
    ExecutionLimitExceeded,
    ModeUnsupported,
    BuiltinProtection,
    CondExhausted,
    Custom,
}

impl ErrorCategory {
    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            ErrorCategory::StackUnderflow => "stackUnderflow",
            ErrorCategory::StructureError => "structureError",
            ErrorCategory::UnknownWord => "unknownWord",
            ErrorCategory::UnknownModule => "unknownModule",
            ErrorCategory::DivisionByZero => "divisionByZero",
            ErrorCategory::IndexOutOfBounds => "indexOutOfBounds",
            ErrorCategory::VectorLengthMismatch => "vectorLengthMismatch",
            ErrorCategory::ExecutionLimitExceeded => "executionLimitExceeded",
            ErrorCategory::ModeUnsupported => "modeUnsupported",
            ErrorCategory::BuiltinProtection => "builtinProtection",
            ErrorCategory::CondExhausted => "condExhausted",
            ErrorCategory::Custom => "custom",
        }
    }

    pub fn from_error(err: &AjisaiError) -> Self {
        match err {
            AjisaiError::StackUnderflow => ErrorCategory::StackUnderflow,
            AjisaiError::StructureError { .. } => ErrorCategory::StructureError,
            AjisaiError::UnknownWord(_) => ErrorCategory::UnknownWord,
            AjisaiError::UnknownModule(_) => ErrorCategory::UnknownModule,
            AjisaiError::DivisionByZero => ErrorCategory::DivisionByZero,
            AjisaiError::IndexOutOfBounds { .. } => ErrorCategory::IndexOutOfBounds,
            AjisaiError::VectorLengthMismatch { .. } => ErrorCategory::VectorLengthMismatch,
            AjisaiError::ExecutionLimitExceeded { .. } => ErrorCategory::ExecutionLimitExceeded,
            AjisaiError::ModeUnsupported { .. } => ErrorCategory::ModeUnsupported,
            AjisaiError::BuiltinProtection { .. } => ErrorCategory::BuiltinProtection,
            AjisaiError::CondExhausted => ErrorCategory::CondExhausted,
            AjisaiError::Custom(_) => ErrorCategory::Custom,
        }
    }
}

impl NilReason {
    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            NilReason::DivisionByZero => "divisionByZero",
            NilReason::EmptySequence => "emptySequence",
            NilReason::MissingField => "missingField",
            NilReason::InvalidEncoding => "invalidEncoding",
            NilReason::InvalidLens => "invalidLens",
            NilReason::StackUnderflow => "stackUnderflow",
            NilReason::IndexOutOfBounds => "indexOutOfBounds",
            NilReason::UnknownWord => "unknownWord",
            NilReason::ExecutionFailure => "executionFailure",
            NilReason::SafeCaught(_) => "safeCaught",
        }
    }

    pub fn caught_category(&self) -> Option<&ErrorCategory> {
        match self {
            NilReason::SafeCaught(category) => Some(category),
            _ => None,
        }
    }

    pub fn from_error(err: &AjisaiError) -> Self {
        NilReason::SafeCaught(Box::new(ErrorCategory::from_error(err)))
    }
}

#[derive(Debug, Clone)]
pub enum AjisaiError {
    StackUnderflow,
    StructureError { expected: String, got: String },
    UnknownWord(String),
    UnknownModule(String),
    DivisionByZero,
    IndexOutOfBounds { index: i64, length: usize },
    VectorLengthMismatch { len1: usize, len2: usize },
    ExecutionLimitExceeded { limit: usize },
    ModeUnsupported { word: String, mode: String },
    BuiltinProtection { word: String, operation: String },
    Custom(String),

    CondExhausted,
}

impl AjisaiError {
    pub fn create_structure_error(expected: &str, got: &str) -> Self {
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
            }
            AjisaiError::UnknownWord(name) => write!(f, "Unknown word: {}", name),
            AjisaiError::UnknownModule(name) => write!(f, "Unknown module: {}", name),
            AjisaiError::DivisionByZero => write!(f, "Division by zero"),
            AjisaiError::IndexOutOfBounds { index, length } => {
                write!(
                    f,
                    "Index {} out of bounds for vector of length {}",
                    index, length
                )
            }
            AjisaiError::VectorLengthMismatch { len1, len2 } => {
                write!(f, "Vector length mismatch: {} vs {}", len1, len2)
            }
            AjisaiError::ExecutionLimitExceeded { limit } => {
                write!(f, "Execution step limit ({}) exceeded", limit)
            }
            AjisaiError::ModeUnsupported { word, mode } => {
                write!(f, "{} does not support {} mode (..)", word, mode)
            }
            AjisaiError::BuiltinProtection { word, operation } => {
                write!(f, "Cannot {} built-in word: {}", operation, word)
            }
            AjisaiError::Custom(msg) => write!(f, "{}", msg),
            AjisaiError::CondExhausted => {
                write!(f, "COND: all guards failed and no else clause")
            }
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
