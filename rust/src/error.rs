use std::fmt;

pub type Result<T> = std::result::Result<T, AjisaiError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NilReason {
    DivisionByZero,
    /// No longer produced. It existed so NIL could stand in for an empty
    /// collection — the empty Vector and the empty String were both
    /// inexpressible, so `FILTER` with no survivors, `TAKE 0`, an emptying
    /// `REMOVE`, a zero-sized `SPLIT` chunk and `''` all answered
    /// `NIL(EmptySequence)`. Both are ordinary values now, so nothing is left
    /// for this reason to describe, and the `projection: never` those Words
    /// register is finally true of them.
    ///
    /// Retained, unlike the retired `LogicallyUnknown`, because it *is*
    /// reverse-decoded: `value_persist::decode_value` hard-errors on a reason
    /// string it does not know, so dropping the variant would make a session
    /// snapshot taken before this change fail to restore rather than degrade.
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
    // CS4 PR-3: `LogicallyUnknown` was retired. The logical truth value
    // Unknown (U, SPEC §7.5) is now its own `ValueData::Unknown` variant, not a
    // NIL node carrying a reason, so no `NilReason` value represents U. There
    // is no reverse decode of `NilReason` from a protocol string, so the
    // retired name needs no boundary handling.
    /// A host-mediated read (`SERIAL@READ`) found no buffered data. The Bubble
    /// Rule projects this to NIL with `absence.origin = hostEnvironment`.
    NoData,
    /// A host-mediated port was reported disconnected with no remaining
    /// buffered data. Projected to NIL with `absence.origin = hostEnvironment`.
    PortDisconnected,
    /// A well-formed generative operation (`RANGE`, `FILL`) whose materialized
    /// result would exceed the space water level (`max_materialized_elements`).
    /// The Bubble Rule projects this to NIL with `absence.origin = spaceBudget`
    /// (SPEC §11.2) rather than aborting the process, so a pipeline can
    /// recover it with `^` (VENT). Malformed inputs (an infinite `RANGE`, a
    /// non-conforming `RESHAPE`) remain ordinary errors.
    SpaceExhausted,
    /// A well-formed operation applied to an input outside its domain — the
    /// canonical case being `SQRT` of a negative rational, which
    /// `SPECIFICATION.html` §5 calls a "well-formed domain miss". The Bubble
    /// Rule projects it to NIL with `absence.origin = domainMiss` (SPEC §11.2).
    ///
    /// Deliberately named for the classification, not for the operation: a
    /// domain miss is recoverable by supplying a different input, which is what
    /// distinguishes it from an execution failure and what makes the same
    /// variant right for future domain misses in other words.
    DomainMiss,
    /// A diagnostic accessor was asked for something the value does not carry —
    /// `NIL-REASON` applied to a value that is not an operational NIL, or to one
    /// that carries no reason. `spec/words.json` registers this as
    /// `NIL-REASON`'s projection reason, and `LANG.FAILURE.PROJECT` requires a
    /// projection to produce "NIL with the reason its contract registers", so
    /// the accessor's own absence is reasoned like any other.
    NotAvailable,
    /// A NIL the program *wrote* rather than computed — the `NIL` Word and the
    /// `NIL` symbol inside a vector literal.
    ///
    /// `LANG.VALUES.NIL` makes the reason a NIL's entire observable content, so
    /// a written absence needs one too; without it `NIL NIL-REASON` answered
    /// NIL and the value had nothing to observe (audit finding D24). It names
    /// the only thing true of it: nothing failed, it was written down.
    Literal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    StackUnderflow,
    StructureError,
    UnknownWord,
    DivisionByZero,
    IndexOutOfBounds,
    VectorLengthMismatch,
    ShapeMismatch,
    MalformedSource,
    NameConflict,
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
            ErrorCategory::DivisionByZero => "divisionByZero",
            ErrorCategory::IndexOutOfBounds => "indexOutOfBounds",
            ErrorCategory::VectorLengthMismatch => "vectorLengthMismatch",
            ErrorCategory::ShapeMismatch => "shapeMismatch",
            ErrorCategory::MalformedSource => "malformedSource",
            ErrorCategory::NameConflict => "nameConflict",
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
            AjisaiError::DivisionByZero => ErrorCategory::DivisionByZero,
            AjisaiError::IndexOutOfBounds { .. } => ErrorCategory::IndexOutOfBounds,
            AjisaiError::VectorLengthMismatch { .. } => ErrorCategory::VectorLengthMismatch,
            AjisaiError::CountExceedsLength { .. } => ErrorCategory::IndexOutOfBounds,
            AjisaiError::ShapeMismatch { .. } => ErrorCategory::ShapeMismatch,
            AjisaiError::MalformedSource(_) => ErrorCategory::MalformedSource,
            AjisaiError::NameConflict(_) => ErrorCategory::NameConflict,
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
            NilReason::NoData => "noData",
            NilReason::PortDisconnected => "portDisconnected",
            NilReason::SpaceExhausted => "spaceExhausted",
            NilReason::DomainMiss => "domainMiss",
            NilReason::NotAvailable => "notAvailable",
            NilReason::Literal => "literal",
        }
    }

    /// Every `NilReason`, so a boundary that must round-trip one can search
    /// this list instead of restating the mapping. `as_protocol_str` stays the
    /// single spelling authority: a new reason is added here and named there,
    /// and `from_protocol_str` follows without another table to update.
    pub const ALL: &'static [NilReason] = &[
        NilReason::DivisionByZero,
        NilReason::EmptySequence,
        NilReason::MissingField,
        NilReason::InvalidEncoding,
        NilReason::InvalidLens,
        NilReason::StackUnderflow,
        NilReason::IndexOutOfBounds,
        NilReason::UnknownWord,
        NilReason::ExecutionFailure,
        NilReason::Undecidable,
        NilReason::NoData,
        NilReason::PortDisconnected,
        NilReason::SpaceExhausted,
        NilReason::DomainMiss,
        NilReason::NotAvailable,
        NilReason::Literal,
    ];

    /// The reason a protocol string names, or `None` when it names none.
    ///
    /// LANG.VALUES.NIL makes the reason the entire observable content of a
    /// NIL, so a boundary that persists a value and reads it back has to carry
    /// the reason across or it changes the value. The persistence codec and
    /// the value arena both decode through here.
    pub fn from_protocol_str(s: &str) -> Option<NilReason> {
        NilReason::ALL
            .iter()
            .find(|reason| reason.as_protocol_str() == s)
            .copied()
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
    DivisionByZero,
    IndexOutOfBounds {
        index: i64,
        length: usize,
    },
    VectorLengthMismatch {
        len1: usize,
        len2: usize,
    },
    /// A count named more elements than the operand has: `[ 1 2 3 ] 5 TAKE`.
    /// An index question rather than a shape one — the count reaches a position
    /// past the end — so it is categorized with `IndexOutOfBounds` and carries
    /// both numbers, which the bare "exceeds vector length" never did.
    CountExceedsLength {
        count: i64,
        length: usize,
        target: String,
    },
    /// Two operands of an element-wise Word have shapes that do not broadcast:
    /// on some axis they disagree and neither extent is 1.
    ///
    /// Distinct from `VectorLengthMismatch`, which is the one-dimensional case
    /// discovered while walking a ragged value tree. This one carries both full
    /// shapes and the axis they part on, because with rank ≥ 2 "which operand
    /// is the wrong shape, and where" is the whole question — and it is the
    /// most frequent error there is in any program that multiplies matrices.
    ShapeMismatch {
        left: Vec<usize>,
        right: Vec<usize>,
        axis: usize,
    },
    /// Program text that does not parse: an unclosed `{ }` block, a `|` outside
    /// a clause, a mismatched delimiter. The fault is in the writing, not in
    /// any value, so it belongs to neither the value-shape nor the user-logic
    /// families.
    MalformedSource(String),
    /// A name was asked to mean two things at once — `BIND` to a name a Word
    /// already holds, or `DEF` to a name a live binding holds. The two name
    /// spaces are disjoint by rule (LANG.DICTIONARY.RESOLUTION), so this is a
    /// rule the program broke, not a value that came out wrong.
    NameConflict(String),
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
            AjisaiError::CountExceedsLength {
                count,
                length,
                target,
            } => write!(
                f,
                "Take count exceeds {} length: {} requested, {} available",
                target,
                count.unsigned_abs(),
                length
            ),
            AjisaiError::ShapeMismatch { left, right, axis } => {
                write!(
                    f,
                    "Cannot broadcast shapes {:?} and {:?}: axis {} is {} on the left and {} on the right, and neither is 1",
                    left,
                    right,
                    axis,
                    left.get(*axis).copied().unwrap_or(1),
                    right.get(*axis).copied().unwrap_or(1)
                )
            }
            AjisaiError::MalformedSource(msg) => write!(f, "{}", msg),
            AjisaiError::NameConflict(msg) => write!(f, "{}", msg),
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
