//! Building a Vector literal (`[ ... ]`) from source tokens.
//!
//! Split out of `execution_loop` when that file outgrew the file-size budget
//! in docs/dev/specification-implementation-rules.md. The two concerns are
//! genuinely separate: the execution loop decides *which* token runs next,
//! and this decides what a delimited token sequence denotes.
//!
//! What an *element* denotes is one piece of knowledge: a literal value, with
//! a bare name (other than `TRUE`/`FALSE`/`NIL`, which still denote their
//! values) becoming a `Value::Symbol` rather than a `Value::Text`. Nothing is
//! looked up: a literal denotes the same value under every dictionary state.

use crate::error::{AjisaiError, Result};
use crate::types::{Token, Value};

use super::Interpreter;

impl Interpreter {
    /// The elements of one Vector literal, and how many tokens it spans.
    pub(crate) fn collect_bracketed_with_depth(
        tokens: &[Token],
        start_index: usize,
        depth: usize,
        limits: &crate::interpreter::RuntimeLimits,
    ) -> Result<(Vec<Value>, usize)> {
        if tokens.get(start_index) != Some(&Token::VectorStart) {
            return Err(AjisaiError::MalformedSource(
                "Expected a literal start".to_string(),
            ));
        }

        // The nesting ceiling (LANG.MACHINE.LIMITS), checked before recursing:
        // this builder descends one native frame per level, so a few thousand
        // levels of `[ [ [ ... ] ] ]` would overflow the native stack here,
        // before the value could be checked at all. It is the same ceiling a
        // value Words build meets, and fails the same way.
        limits.check_nesting_depth(depth)?;

        let mut values = Vec::new();
        let mut i = start_index + 1;

        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorStart => {
                    let (nested_values, consumed) =
                        Self::collect_bracketed_with_depth(tokens, i, depth + 1, limits)?;
                    values.push(Value::from_vector_promoted(nested_values));
                    i += consumed;
                }
                Token::VectorEnd => {
                    return Ok((values, i - start_index + 1));
                }
                Token::Value(value) => {
                    values.push((**value).clone());
                    i += 1;
                }
                Token::Number(literal) => {
                    values.push(Value::from_number(
                        literal.parsed().map_err(AjisaiError::MalformedSource)?,
                    ));
                    i += 1;
                }
                Token::String(s) => {
                    values.push(Value::from_string(s));
                    i += 1;
                }
                Token::Symbol(s) => {
                    let upper = Self::normalize_symbol(s);
                    match upper.as_ref() {
                        "TRUE" => values.push(Value::from_bool(true)),
                        "FALSE" => values.push(Value::from_bool(false)),
                        "NIL" => values.push(Value::nil()),
                        // A bare name is a Symbol: data until something
                        // executes it, dictionary-independent (building the
                        // literal never looks anything up).
                        _ => values.push(Value::from_symbol(s)),
                    }
                    i += 1;
                }
            }
        }
        Err(AjisaiError::MalformedSource(
            "Unclosed bracketed literal".to_string(),
        ))
    }
}
