//! Definition and removal.
//!
//! `DEF` takes a quote and a name, in that order — `{ 2 MUL } "DOUBLE" DEF`.
//! There is no defining syntax, no colon-word, and no parser special case: a
//! definition is two ordinary values and one ordinary word. The name is text,
//! which is a vector carrying the `TEXT` role, so the Semantic Plane is load
//! bearing in the one place a language most needs a name to be a name.
//!
//! No word owned by Ajisai Core or by a registered package can be redefined.

use crate::alias;
use crate::error::{Error, Result};
use crate::interpreter::is_directive;
use crate::number::Number;
use crate::Interpreter;

/// Read a name operand as a canonical word name.
fn take_name(interpreter: &mut Interpreter, word: &str) -> Result<String> {
    let value = interpreter.pop(word)?;
    let Some(text) = value.as_text() else {
        let found = value.type_name().to_string();
        interpreter.push(value);
        return Err(Error::TypeMismatch {
            word: word.to_string(),
            expected: "text".to_string(),
            found,
        });
    };
    let name = alias::canonical(text.trim());
    if !is_definable(&name) {
        interpreter.push(value);
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
        let found = value.type_name().to_string();
        interpreter.push(value.clone());
        return Err(Error::TypeMismatch {
            word: "DEF".to_string(),
            expected: "quote".to_string(),
            found,
        });
    };
    interpreter.define(name, std::sync::Arc::clone(body))
}

pub fn del(interpreter: &mut Interpreter) -> Result<()> {
    let name = take_name(interpreter, "DEL")?;
    interpreter.undefine(&name)
}
