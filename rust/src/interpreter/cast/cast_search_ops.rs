//! Text search Words: `SEARCH` and `REPLACE` (LANG.VALUES.DISJOINT).
//!
//! `INDEX-OF` and a substitution, for Text. Spelled over `CHARS` each is a
//! window compared at every position; the Word is the one pass
//! (docs/dev/vocabulary-100-work-order-2026-09.md §1).

use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::cast::cast_value_helpers::is_string_value;
use crate::interpreter::value_extraction_helpers::{extract_operands, value_as_string};
use crate::interpreter::Interpreter;
use crate::semantic::Recoverability;
use crate::types::Value;

/// The texts the operands denote, or the `nonText` every text Word declares.
fn texts(interp: &mut Interpreter, word: &str, count: usize) -> Result<Vec<String>> {
    let operands = extract_operands(interp, count)?;
    if operands.iter().all(is_string_value) {
        return Ok(operands
            .iter()
            .map(|operand| value_as_string(operand).unwrap_or_default())
            .collect());
    }
    interp.stack.extend(operands);
    Err(AjisaiError::declared(
        "nonText",
        format!("{word}: expected Strings, got a non-text value"),
    ))
}

/// `SEARCH ( [ text ] [ needle ] -> [ index ] )`: the character position at
/// which `needle` first occurs, counted as `CHARS` counts; `missingField`
/// when it does not occur. An empty needle is found at 0.
pub fn op_search(interp: &mut Interpreter) -> Result<()> {
    let texts = texts(interp, "SEARCH", 2)?;
    let (haystack, needle) = (&texts[0], &texts[1]);
    match haystack.find(needle.as_str()) {
        Some(byte_offset) => {
            let position = haystack[..byte_offset].chars().count();
            interp.stack.push(Value::from_int(position as i64));
        }
        None => interp.stack.push(Value::nil_with_reason(
            NilReason::MissingField,
            Recoverability::Recoverable,
        )),
    }
    Ok(())
}

/// `REPLACE ( [ text ] [ from ] [ to ] -> [ text' ] )`: every occurrence of
/// `from` replaced by `to`, found left to right without overlap. An empty
/// `from` matches nothing, so the text comes back unchanged rather than
/// growing at every position.
pub fn op_replace(interp: &mut Interpreter) -> Result<()> {
    let texts = texts(interp, "REPLACE", 3)?;
    let (text, from, to) = (&texts[0], &texts[1], &texts[2]);
    let replaced = if from.is_empty() {
        text.clone()
    } else {
        text.replace(from.as_str(), to.as_str())
    };
    interp.stack.push(Value::from_string(&replaced));
    Ok(())
}
