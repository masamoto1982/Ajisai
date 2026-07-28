//! Errors: flows that never formed.
//!
//! An error is the third of Ajisai's three negative outcomes, and it is kept
//! strictly apart from the other two:
//!
//! * `NIL` — the flow arrived, and carried no value.
//! * `UNKNOWN` — the flow arrived, and observing it does not settle the
//!   question being asked.
//! * `Error` — the flow never formed. There is no value and no observation,
//!   only a rule that did not hold.
//!
//! Errors abort the current execution. They are never converted to `NIL`,
//! never converted to `UNKNOWN`, and never caught by a word inside the
//! language: nothing in Ajisai Core turns a broken rule back into a value.

use std::fmt;

use crate::mode::Mode;

/// A rule that did not hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// A word needed more of the flow than the basin held.
    StackUnderflow {
        word: String,
        needed: usize,
        found: usize,
    },
    /// A word was handed a value of the wrong kind.
    TypeMismatch {
        word: String,
        expected: String,
        found: String,
    },
    /// Division by zero. Exactness makes this unrepresentable, not infinite.
    DivisionByZero,
    /// A word that is neither built in, registered by a package, nor defined.
    UnknownWord(String),
    /// `DEF` may not shadow a word that Ajisai Core or a package owns.
    ReservedWord(String),
    /// The word does not accept the armed mode.
    ModeUnsupported { word: String, mode: Mode },
    /// A mode was armed and the body ended before a word consumed it.
    DanglingMode { mode: Mode },
    /// `VENT` was the last thing in a body; there is no unit to release.
    VentMissingUnit,
    /// A logical position was handed something that is not `TRUE`, `FALSE`,
    /// or `UNKNOWN`. `NIL` reaches this, deliberately: an absence is not a
    /// truth value, and silently reading it as one is how K3 collapses back
    /// into two-valued logic.
    NotATruthValue { word: String, found: String },
    /// A predicate answered `UNKNOWN` where the word must decide. Keeping the
    /// element would read `UNKNOWN` as `TRUE`; dropping it would read
    /// `UNKNOWN` as `FALSE`. Neither is honest, so the word refuses and the
    /// program says which it meant.
    UndecidedPredicate { word: String },
    /// An index fell outside the vector.
    IndexOutOfRange { index: String, length: usize },
    /// A role was asserted over a value that cannot carry it.
    BadRole { role: String, reason: String },
    /// A bracket, brace, or quotation mark that never closed, or closed
    /// without opening.
    Unbalanced { delimiter: String },
    /// A token that is neither a number, a text literal, nor a word.
    MalformedToken(String),
    /// Nested evaluation exceeded the interpreter's depth budget.
    DepthLimitExceeded { limit: usize },
    /// A single operation was asked to build a vector larger than the budget.
    SizeLimitExceeded { limit: usize },
    /// A package tried to register a word Ajisai Core or another package owns,
    /// or one a user definition already answers to.
    DuplicateWord { package: String, word: String },
    /// A package supplied a word whose contract does not describe its
    /// implementation. Caught at registration, before the word can run.
    MalformedContract {
        package: String,
        word: String,
        reason: String,
    },
    /// A word left the flow at a depth its declared stack effect does not
    /// allow. This is a defect in the word, not in the program that called it.
    ContractViolated {
        word: String,
        expected: usize,
        found: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::StackUnderflow {
                word,
                needed,
                found,
            } => write!(f, "{word}: needs {needed} value(s), the flow holds {found}"),
            Error::TypeMismatch {
                word,
                expected,
                found,
            } => write!(f, "{word}: expected {expected}, got {found}"),
            Error::DivisionByZero => write!(f, "DIV: division by zero"),
            Error::UnknownWord(name) => write!(f, "unknown word: {name}"),
            Error::ReservedWord(name) => {
                write!(f, "DEF: {name} is a reserved word and cannot be redefined")
            }
            Error::ModeUnsupported { word, mode } => {
                write!(f, "{word}: does not accept mode {mode}")
            }
            Error::DanglingMode { mode } => {
                write!(f, "mode {mode} was armed but no word consumed it")
            }
            Error::VentMissingUnit => write!(f, "VENT: no unit follows"),
            Error::NotATruthValue { word, found } => {
                write!(f, "{word}: expected TRUE, FALSE, or UNKNOWN, got {found}")
            }
            Error::UndecidedPredicate { word } => write!(
                f,
                "{word}: the predicate answered UNKNOWN; decide it explicitly before filtering"
            ),
            Error::IndexOutOfRange { index, length } => {
                write!(f, "index {index} outside vector of length {length}")
            }
            Error::BadRole { role, reason } => write!(f, ">{role}: {reason}"),
            Error::Unbalanced { delimiter } => write!(f, "unbalanced {delimiter}"),
            Error::MalformedToken(token) => write!(f, "malformed token: {token}"),
            Error::DepthLimitExceeded { limit } => {
                write!(f, "nested evaluation deeper than {limit}")
            }
            Error::SizeLimitExceeded { limit } => {
                write!(f, "vector larger than {limit} elements")
            }
            Error::DuplicateWord { package, word } => {
                write!(f, "package {package}: {word} is already registered")
            }
            Error::MalformedContract {
                package,
                word,
                reason,
            } => write!(f, "package {package}: {word}: {reason}"),
            Error::ContractViolated {
                word,
                expected,
                found,
            } => write!(
                f,
                "{word}: left the flow at depth {found}, but its stack effect says {expected}"
            ),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
