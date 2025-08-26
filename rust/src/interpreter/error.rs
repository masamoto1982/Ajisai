use std::fmt;

pub type Result<T> = std::result::Result<T, LPLError>;

#[derive(Debug, Clone)]
pub enum LPLError {
    BookshelfUnderflow,  // StackUnderflow → BookshelfUnderflow
    TypeError { expected: String, got: String },
    UnknownWord(String),
    UnknownBuiltin(String),
    DivisionByZero,
    IndexOutOfBounds { index: i64, length: usize },
    VectorLengthMismatch { len1: usize, len2: usize },
    ProtectedWord { name: String, dependents: Vec<String> },
    Custom(String),
    WithContext { error: Box<LPLError>, context: Vec<String> },
}

impl LPLError {
    pub fn with_context(self, call_stack: &[String]) -> Self {
        if call_stack.is_empty() {
            self
        } else {
            LPLError::WithContext {
                error: Box::new(self),
                context: call_stack.to_vec(),
            }
        }
    }
    
    pub fn type_error(expected: &str, got: &str) -> Self {
        LPLError::TypeError {
            expected: expected.to_string(),
            got: got.to_string(),
        }
    }
}

impl fmt::Display for LPLError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LPLError::BookshelfUnderflow => write!(f, "Bookshelf underflow"),
            LPLError::TypeError { expected, got } => {
                write!(f, "Type error: expected {}, got {}", expected, got)
            },
            LPLError::UnknownWord(name) => write!(f, "Unknown word: {}", name),
            LPLError::UnknownBuiltin(name) => write!(f, "Unknown builtin: {}", name),
            LPLError::DivisionByZero => write!(f, "Division by zero"),
            LPLError::IndexOutOfBounds { index, length } => {
                write!(f, "Index {} out of bounds for vector of length {}", index, length)
            },
            LPLError::VectorLengthMismatch { len1, len2 } => {
                write!(f, "Vector length mismatch: {} vs {}", len1, len2)
            },
            LPLError::ProtectedWord { name, dependents } => {
                write!(f, "Cannot delete '{}' because it is used by: {}", name, dependents.join(", "))
            },
            LPLError::Custom(msg) => write!(f, "{}", msg),
            LPLError::WithContext { error, context } => {
                write!(f, "{}\n  in word: {}", error, context.join(" -> "))
            },
        }
    }
}

impl From<String> for LPLError {
    fn from(s: String) -> Self {
        LPLError::Custom(s)
    }
}

impl From<&str> for LPLError {
    fn from(s: &str) -> Self {
        LPLError::Custom(s.to_string())
    }
}
