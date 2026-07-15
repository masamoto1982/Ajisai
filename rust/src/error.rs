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
    /// Comparison-budget exhaustion per SPEC §7.4.1: two lazy CFs
    /// agreed on every emitted partial quotient up to the budget
    /// without diverging, or one of the operands' CF streams reported
    /// `CfStep::Exhausted`. The Bubble Rule projects this to NIL with
    /// `absence.origin = comparisonBudget` rather than a SAFE-caught
    /// error.
    Undecidable,
    /// The logical truth value `Unknown` (U) of the three-valued (Kleene)
    /// logic, SPEC §7.5. Unlike `Undecidable` (an operational NIL), this
    /// marks a *logical* undecidability: a continued-fraction comparison
    /// (SPEC §7.4.1) that did not settle true or false within its budget.
    /// A value carrying this reason together with `Interpretation::TruthValue`
    /// is the runtime representation of U; observe it through the
    /// `truthValue` axis (`unknown`), never by inspecting this reason
    /// directly. Use `Value::is_unknown()` / `Value::unknown()`.
    LogicallyUnknown,
    /// A host-mediated read (`SERIAL@READ`) found no buffered data. The Bubble
    /// Rule projects this to NIL with `absence.origin = hostEnvironment`.
    NoData,
    /// A host-mediated port was reported disconnected with no remaining
    /// buffered data. Projected to NIL with `absence.origin = hostEnvironment`.
    PortDisconnected,
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
    RecursionLimitExceeded,
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
            ErrorCategory::RecursionLimitExceeded => "recursionLimitExceeded",
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
            AjisaiError::RecursionLimitExceeded { .. } => ErrorCategory::RecursionLimitExceeded,
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
            NilReason::Undecidable => "undecidable",
            NilReason::LogicallyUnknown => "logicallyUnknown",
            NilReason::NoData => "noData",
            NilReason::PortDisconnected => "portDisconnected",
        }
    }
}

#[derive(Debug, Clone)]
pub enum AjisaiError {
    StackUnderflow,
    StructureError {
        expected: String,
        got: String,
    },
    UnknownWord(String),
    UnknownModule(String),
    DivisionByZero,
    IndexOutOfBounds {
        index: i64,
        length: usize,
    },
    VectorLengthMismatch {
        len1: usize,
        len2: usize,
    },
    ExecutionLimitExceeded {
        limit: usize,
    },
    /// The native recursion-depth guard (SPEC §8.4) tripped: `word` reached
    /// `limit` non-tail recursive activations. A runtime safety control of the
    /// same rank as the step budget (§5.3), not language semantics; guarded
    /// tail recursion (§7.7.1) never raises this.
    RecursionLimitExceeded {
        limit: usize,
        word: String,
    },
    ModeUnsupported {
        word: String,
        mode: String,
    },
    BuiltinProtection {
        word: String,
        operation: String,
    },
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
            AjisaiError::RecursionLimitExceeded { limit, word } => {
                write!(f, "recursion limit exceeded ({}) in '{}'", limit, word)
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
