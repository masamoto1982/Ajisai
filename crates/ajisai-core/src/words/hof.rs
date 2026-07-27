//! Words that take a quote.
//!
//! These run a quote in a basin seeded with the operands, so a quote handed to
//! `MAP` cannot reach past its own element into the surrounding flow. A quote
//! that leaves anything other than exactly one value is an error rather than a
//! silently reshaped result.
//!
//! `STAK` works across the standing flow; these words work across a vector.
//! The two are different axes and neither is a substitute for the other.

use crate::error::{Error, Result};
use crate::k3::Truth;
use crate::role::{self};
use crate::value::Value;
use crate::words::vector::SIZE_LIMIT;
use crate::Interpreter;

use super::support::vector_of;

/// Run a quote body on `seed` and require exactly one value back.
fn apply_once(
    interpreter: &mut Interpreter,
    word: &str,
    body: &[crate::syntax::Node],
    seed: Vec<Value>,
) -> Result<Value> {
    let mut produced = interpreter.run_in_basin(body, seed)?;
    if produced.len() != 1 {
        return Err(Error::TypeMismatch {
            word: word.to_string(),
            expected: "a quote leaving exactly 1 value".to_string(),
            found: format!("{} value(s)", produced.len()),
        });
    }
    Ok(produced.remove(0))
}

fn take_quote(
    interpreter: &mut Interpreter,
    word: &str,
) -> Result<std::sync::Arc<Vec<crate::syntax::Node>>> {
    let value = interpreter.pop(word)?;
    match value.as_quote() {
        Some(body) => Ok(std::sync::Arc::clone(body)),
        None => {
            let found = value.type_name().to_string();
            interpreter.push(value);
            Err(Error::TypeMismatch {
                word: word.to_string(),
                expected: "quote".to_string(),
                found,
            })
        }
    }
}

pub fn map(interpreter: &mut Interpreter) -> Result<()> {
    let body = take_quote(interpreter, "MAP")?;
    let source = interpreter.pop("MAP")?;
    let items = vector_of("MAP", &source)?.to_vec();
    let mut mapped = Vec::with_capacity(items.len());
    for item in items {
        mapped.push(apply_once(interpreter, "MAP", &body, vec![item])?);
    }
    // The elements changed, so a reading of the container only survives if the
    // new shape still admits it.
    let result = Value::vector(mapped);
    let kept = role::retain(source.role(), &result);
    interpreter.push(result.with_role(kept));
    Ok(())
}

pub fn filter(interpreter: &mut Interpreter) -> Result<()> {
    let body = take_quote(interpreter, "FILTER")?;
    let source = interpreter.pop("FILTER")?;
    let items = vector_of("FILTER", &source)?.to_vec();
    let mut kept_items = Vec::new();
    for item in items {
        let verdict = apply_once(interpreter, "FILTER", &body, vec![item.clone()])?;
        match Truth::read("FILTER", &verdict)? {
            Truth::True => kept_items.push(item),
            Truth::False => {}
            Truth::Unknown => {
                return Err(Error::UndecidedPredicate {
                    word: "FILTER".to_string(),
                })
            }
        }
    }
    let result = Value::vector(kept_items);
    let kept = role::retain(source.role(), &result);
    interpreter.push(result.with_role(kept));
    Ok(())
}

pub fn fold(interpreter: &mut Interpreter) -> Result<()> {
    let body = take_quote(interpreter, "FOLD")?;
    let seed = interpreter.pop("FOLD")?;
    let source = interpreter.pop("FOLD")?;
    let items = vector_of("FOLD", &source)?.to_vec();
    let mut accumulator = seed;
    for item in items {
        accumulator = apply_once(interpreter, "FOLD", &body, vec![accumulator, item])?;
    }
    interpreter.push(accumulator);
    Ok(())
}

pub fn exec(interpreter: &mut Interpreter) -> Result<()> {
    let body = take_quote(interpreter, "EXEC")?;
    interpreter.run_here(&body)
}

pub fn depth(interpreter: &mut Interpreter) -> Result<()> {
    let depth = interpreter.depth();
    if depth > SIZE_LIMIT {
        return Err(Error::SizeLimitExceeded { limit: SIZE_LIMIT });
    }
    interpreter.push(Value::integer(depth as i64));
    Ok(())
}
