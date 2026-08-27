//! Building a bracketed literal (`[ ... ]`) from source tokens.
//!
//! Split out of `execution_loop` when that file outgrew the §14.1 size budget.
//! The two concerns are genuinely separate: the execution loop decides *which*
//! token runs next, and this decides what a bracketed token sequence denotes.
//!
//! `[ ... ]` builds a `Value::Vector` whose elements are literal values, with
//! a bare name (other than `TRUE`/`FALSE`/`NIL`, which still denote their
//! values) becoming a `Value::Symbol` rather than a `Value::Text`. `{ }` was
//! a second spelling of the identical construction (the CodeBlock/Vector
//! unification, docs/dev/type-unification-work-order-2026-08.md) and was
//! later retired outright, leaving `[ ]` as the only bracket, for both data
//! and code. This function is dictionary-independent — no name lookup ever
//! occurs here.

use crate::error::{AjisaiError, Result};
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Token, Value};

use super::Interpreter;

impl Interpreter {
    pub(crate) fn collect_bracketed_with_depth(
        tokens: &[Token],
        start_index: usize,
        depth: usize,
    ) -> Result<(Vec<Value>, usize, Interpretation)> {
        if !matches!(tokens.get(start_index), Some(Token::VectorStart)) {
            return Err(AjisaiError::from("Expected a bracketed literal start"));
        }

        // Guard against unbounded nesting before recursing. Without this, a few
        // thousand levels of `[ [ [ ... ] ] ]` from plain source build a value
        // so deeply nested that recursively displaying or dropping it overflows
        // the native stack and aborts the process (a WASM trap). Rejecting here
        // keeps the value — and every later traversal of it — within a depth the
        // stack can handle, surfaced as a recoverable error.
        if depth > crate::interpreter::MAX_VECTOR_NESTING_DEPTH {
            return Err(AjisaiError::from(format!(
                "Vector nesting too deep (limit {})",
                crate::interpreter::MAX_VECTOR_NESTING_DEPTH
            )));
        }

        let mut values = Vec::new();
        let mut i = start_index + 1;
        let mut has_bool: bool = false;
        let mut has_number: bool = false;
        let mut has_other: bool = false;

        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorStart => {
                    let (nested_values, consumed, nested_hint) =
                        Self::collect_bracketed_with_depth(tokens, i, depth + 1)?;
                    values.push(Value::from_vector_promoted_with_hint(
                        nested_values,
                        nested_hint,
                    ));
                    has_other = true;
                    i += consumed;
                }
                Token::VectorEnd => {
                    return Ok((
                        values,
                        i - start_index + 1,
                        Self::element_hint(has_other, has_bool, has_number),
                    ));
                }
                Token::Number(n) => {
                    values.push(Value::from_number(
                        Fraction::from_str(n).map_err(AjisaiError::from)?,
                    ));
                    has_number = true;
                    i += 1;
                }
                Token::String(s) => {
                    values.push(Value::from_string(s));
                    has_other = true;
                    i += 1;
                }
                Token::Symbol(s) => {
                    let upper = Self::normalize_symbol(s);
                    match upper.as_ref() {
                        "TRUE" => {
                            values.push(Value::from_bool(true));
                            has_bool = true;
                        }
                        "FALSE" => {
                            values.push(Value::from_bool(false));
                            has_bool = true;
                        }
                        "NIL" => {
                            values.push(Value::nil());
                            has_other = true;
                        }
                        _ => {
                            // A bare name is a Symbol: data until something
                            // executes it, dictionary-independent (building
                            // the literal never looks anything up).
                            values.push(Value::from_symbol(s));
                            has_other = true;
                        }
                    }
                    i += 1;
                }
                Token::CondClauseSep => {
                    // `[ IDLE | 1 ]`: `|` inside a clause block is data until
                    // COND runs it — a bare Symbol("|"), like any other name,
                    // symmetric with how `Value::Symbol` was promoted for
                    // ordinary names. `value_as_code.rs` maps it back to
                    // `Token::CondClauseSep` when the clause is actually
                    // executed. Whether it actually sits inside a legitimate
                    // COND clause is decided later, when `split_clause_blocks`
                    // tries to split the block that holds it.
                    values.push(Value::from_symbol("|"));
                    has_other = true;
                    i += 1;
                }
                Token::LineBreak | Token::NilCoalesce => {
                    i += 1;
                }
            }
        }
        Err(AjisaiError::from("Unclosed bracketed literal"))
    }

    fn element_hint(has_other: bool, has_bool: bool, has_number: bool) -> Interpretation {
        if has_other {
            Interpretation::Unassigned
        } else if has_bool && !has_number {
            Interpretation::TruthValue
        } else if has_number && !has_bool {
            Interpretation::RawNumber
        } else {
            Interpretation::Unassigned
        }
    }
}
