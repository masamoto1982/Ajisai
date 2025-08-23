use std::fmt;

pub type Result<T> = std::result::Result<T, AjisaiError>;

#[derive(Debug, Clone)]
pub enum AjisaiError {
    WorkspaceUnderflow,  // StackUnderflow → WorkspaceUnderflow
    TypeError { expected: String, got: String },
    UnknownWord(String),
    UnknownBuiltin(String),
    DivisionByZero,
    IndexOutOfBounds { index: i64, length: usize },
    VectorLengthMismatch { len1: usize, len2: usize },
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AjisaiError::WorkspaceUnderflow => write!(f, "Workspace underflow"),
            AjisaiError::InvalidToken(s) => write!(f, "Invalid token: {}", s),
            AjisaiError::TypeError { expected, found } => write!(f, "Type error: expected {:?}, found {:?}", expected, found),
            AjisaiError::WordNotFound(s) => write!(f, "Word not found: {}", s),
            
            // 修正箇所
            AjisaiError::EmptyVector => write!(f, "空のベクトルに対する操作です"),

            AjisaiError::IndexOutOfBounds => write!(f, "Index out of bounds"),
            AjisaiError::DivisionByZero => write!(f, "Division by zero"),
            AjisaiError::InvalidArguments => write!(f, "Invalid arguments"),
            AjisaiError::ReadOnlyViolation => write!(f, "Read-only violation"),
            AjisaiError::Custom(s) => write!(f, "{}", s),
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
