//! Building a literal (`[ ... ]`, `{ ... }`) from source tokens.
//!
//! Split out of `execution_loop` when that file outgrew the the file-size budget in docs/dev/specification-implementation-rules.md size budget.
//! The two concerns are genuinely separate: the execution loop decides *which*
//! token runs next, and this decides what a delimited token sequence denotes.
//!
//! Both literals are built from the one element scan below, because what an
//! *element* denotes is one piece of knowledge: a literal value, with a bare
//! name (other than `TRUE`/`FALSE`/`NIL`, which still denote their values)
//! becoming a `Value::Symbol` rather than a `Value::Text`. The two kinds
//! differ only in what closes them and what they build out of the elements —
//! a Vector in order, or the paired key and value sequences of a Record
//! (`record_literal.rs`). Neither looks anything up: a literal denotes the
//! same value under every dictionary state.

use crate::error::{AjisaiError, Result};
use crate::types::{Token, Value};

use super::Interpreter;

/// Which literal is being collected. The scan is shared; the delimiter that
/// closes it and the diagnosis for one that never closes are not.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LiteralKind {
    Vector,
    Record,
}

impl LiteralKind {
    fn opener(self) -> Token {
        match self {
            LiteralKind::Vector => Token::VectorStart,
            LiteralKind::Record => Token::RecordStart,
        }
    }

    fn closes(self, token: &Token) -> bool {
        match self {
            LiteralKind::Vector => matches!(token, Token::VectorEnd),
            LiteralKind::Record => matches!(token, Token::RecordEnd),
        }
    }

    fn unclosed(self) -> &'static str {
        match self {
            LiteralKind::Vector => "Unclosed bracketed literal",
            LiteralKind::Record => "Unclosed Record literal",
        }
    }
}

impl Interpreter {
    pub(crate) fn collect_bracketed_with_depth(
        tokens: &[Token],
        start_index: usize,
        depth: usize,
    ) -> Result<(Vec<Value>, usize)> {
        Self::collect_literal_elements(tokens, start_index, depth, LiteralKind::Vector)
    }

    /// The elements of one literal, and how many tokens it spans.
    ///
    /// Recursion goes through the *element's own* kind, so a Record nested in
    /// a Vector and a Vector nested in a Record both fall out of one walk
    /// rather than two mirrored ones.
    pub(crate) fn collect_literal_elements(
        tokens: &[Token],
        start_index: usize,
        depth: usize,
        kind: LiteralKind,
    ) -> Result<(Vec<Value>, usize)> {
        if tokens.get(start_index) != Some(&kind.opener()) {
            return Err(AjisaiError::MalformedSource(
                "Expected a literal start".to_string(),
            ));
        }

        // Guard against unbounded nesting before recursing. Without this, a few
        // thousand levels of `[ [ [ ... ] ] ]` (or of `{ 'k' { ... } }`, which
        // shares the counter because it shares the recursion) from plain source build a value
        // so deeply nested that recursively displaying or dropping it overflows
        // the native stack and aborts the process (a WASM trap). Rejecting here
        // keeps the value — and every later traversal of it — within a depth the
        // stack can handle, surfaced as a recoverable error.
        if depth > crate::interpreter::MAX_VECTOR_NESTING_DEPTH {
            return Err(AjisaiError::MalformedSource(format!(
                "Vector nesting too deep (limit {})",
                crate::interpreter::MAX_VECTOR_NESTING_DEPTH
            )));
        }

        let mut values = Vec::new();
        let mut i = start_index + 1;

        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorStart => {
                    let (nested_values, consumed) =
                        Self::collect_bracketed_with_depth(tokens, i, depth + 1)?;
                    values.push(Value::from_vector_promoted(nested_values));
                    i += consumed;
                }
                Token::RecordStart => {
                    let (record, consumed) = Self::collect_record_literal(tokens, i, depth + 1)?;
                    values.push(record);
                    i += consumed;
                }
                token if kind.closes(token) => {
                    return Ok((values, i - start_index + 1));
                }
                // The other pair's closer, which `validate_code_tokens`
                // refuses before anything runs (`mismatched code delimiter`).
                // Reached only by a caller that assembled tokens without going
                // through it, so it is reported rather than assumed away.
                Token::VectorEnd | Token::RecordEnd => {
                    return Err(AjisaiError::MalformedSource(
                        "mismatched code delimiter".to_string(),
                    ));
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
                Token::LineBreak => {
                    i += 1;
                }
            }
        }
        Err(AjisaiError::MalformedSource(kind.unclosed().to_string()))
    }
}
