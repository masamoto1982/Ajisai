use std::fmt;

pub type Result<T> = std::result::Result<T, AjisaiError>;

#[derive(Debug, Clone)]
pub enum AjisaiError {
    StackUnderflow,
    RegisterEmpty,
    TypeError { expected: String, got: String },
    UnknownWord(String),
    UnknownBuiltin(String),
    DivisionByZero,
    IndexOutOfBounds { index: i64, length: usize },
    VectorLengthMismatch { len1: usize, len2: usize },
    ProtectedWord { name: String, dependents: Vec<String> },
    ProtectedWord { name: String, dependents: Vec<String> },
    Custom(String),
    WithContext { error: Box<AjisaiError>, context: Vec<String> },
}

impl AjisaiError {
    pub fn with_context(self, call_stack: &[String]) -> Self {
        if call_stack.is_empty() {
            self
        } else {
            AjisaiError::WithContext {
                error: Box::new(self),
                context: call_stack.to_vec(),
            }
        }
    }
    
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
            AjisaiError::RegisterEmpty => write!(f, "Register is empty"),
            AjisaiError::TypeError { expected, got } => {
                write!(f, "Type error: expected {}, got {}", expected, got)
            },
            AjisaiError::UnknownWord(name) => write!(f, "Unknown word: {}", name),
            AjisaiError::UnknownBuiltin(name) => write!(f, "Unknown builtin: {}", name),
            AjisaiError::DivisionByZero => write!(f, "Division by zero"),
            AjisaiError::IndexOutOfBounds { index, length } => {
                write!(f, "Index {} out of bounds for vector of length {}", index, length)
            },
            AjisaiError::VectorLengthMismatch { len1, len2 } => {
                write!(f, "Vector length mismatch: {} vs {}", len1, len2)
            },
            AjisaiError::ProtectedWord { name, dependents } => {
                write!(f, "Cannot redefine '{}' because it is used by: {}", name, dependents.join(", "))
            },
            AjisaiError::Custom(msg) => write!(f, "{}", msg),
            AjisaiError::WithContext { error, context } => {
                write!(f, "{}\n  in word: {}", error, context.join(" -> "))
            },
        }
    }
}

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
