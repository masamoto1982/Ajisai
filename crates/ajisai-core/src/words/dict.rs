//! Definition and removal.
//!
//! `DEF` takes a quote and a name, in that order — `{ 2 MUL } "DOUBLE" DEF`.
//! There is no defining syntax, no colon-word, and no parser special case: a
//! definition is two ordinary values and one ordinary word.
//!
//! **These are the two words that read the Semantic Plane** (`SPECIFICATION.md`
//! §6.3). A name must carry the `TEXT` role: `[ 68 79 85 ]` is a vector of
//! three numbers that happens to spell `DOU`, and treating it as a name would
//! mean the reading a program asserts about its own data counts for nothing.
//! Write `"DOUBLE"`, or say `>TEXT` and mean it.
//!
//! No word owned by Ajisai Core or by a registered package can be redefined,
//! and the failing word leaves the flow exactly as it found it.

use crate::alias;
use crate::error::{Error, Result};
use crate::interpreter::is_directive;
use crate::number::Number;
use crate::role::Role;
use crate::Interpreter;

/// Read a name operand as a canonical word name.
///
/// The flow is restored by the caller's atomicity wrapper on failure, so this
/// does not push operands back itself.
fn take_name(interpreter: &mut Interpreter, word: &str) -> Result<String> {
    let value = interpreter.pop(word)?;
    if value.role() != Role::Text {
        return Err(Error::TypeMismatch {
            word: word.to_string(),
            expected: "text — a vector read as TEXT, not a bare vector".to_string(),
            found: format!("{} read as {}", value.type_name(), value.role()),
        });
    }
    let Some(text) = value.as_text() else {
        return Err(Error::TypeMismatch {
            word: word.to_string(),
            expected: "text".to_string(),
            found: value.type_name().to_string(),
        });
    };
    let name = alias::canonical(text.trim());
    if !is_definable(&name) {
        return Err(Error::ReservedWord(name));
    }
    Ok(name)
}

/// A definable name is a word name and nothing else: not empty, not a number,
/// not whitespace-bearing, not a bracket, and not the canonical name of an
/// alias symbol or a directive.
fn is_definable(name: &str) -> bool {
    if name.is_empty() || name.chars().any(char::is_whitespace) {
        return false;
    }
    if Number::parse(name).is_some() {
        return false;
    }
    if matches!(name, "[" | "]" | "{" | "}") || name.starts_with('"') || name.starts_with('#') {
        return false;
    }
    if is_directive(name) {
        return false;
    }
    true
}

pub fn def(interpreter: &mut Interpreter) -> Result<()> {
    let name = take_name(interpreter, "DEF")?;
    let value = interpreter.pop("DEF")?;
    let Some(body) = value.as_quote() else {
        return Err(Error::TypeMismatch {
            word: "DEF".to_string(),
            expected: "quote".to_string(),
            found: value.type_name().to_string(),
        });
    };
    interpreter.define(name, std::sync::Arc::clone(body))
}

pub fn del(interpreter: &mut Interpreter) -> Result<()> {
    let name = take_name(interpreter, "DEL")?;
    interpreter.undefine(&name)
}
