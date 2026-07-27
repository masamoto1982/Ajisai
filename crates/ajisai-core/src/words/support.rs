//! Shared operand helpers for word implementations.

use crate::error::{Error, Result};
use crate::number::Number;
use crate::value::Value;

/// Read an operand as a number, or fail with the word's name attached.
pub fn number_of<'a>(word: &str, value: &'a Value) -> Result<&'a Number> {
    value.as_number().ok_or_else(|| Error::TypeMismatch {
        word: word.to_string(),
        expected: "number".to_string(),
        found: value.type_name().to_string(),
    })
}

/// Read an operand as a vector.
pub fn vector_of<'a>(word: &str, value: &'a Value) -> Result<&'a [Value]> {
    value
        .as_vector()
        .map(|items| items.as_slice())
        .ok_or_else(|| Error::TypeMismatch {
            word: word.to_string(),
            expected: "vector".to_string(),
            found: value.type_name().to_string(),
        })
}

/// The operand layer guarantees arity before calling a word, so a mismatch
/// here is an internal inconsistency rather than a user error. Reporting it as
/// underflow keeps the interpreter total.
pub fn arity_bug(word: &str, needed: usize, found: usize) -> Error {
    Error::StackUnderflow {
        word: word.to_string(),
        needed,
        found,
    }
}
