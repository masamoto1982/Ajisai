//! Building a bracketed literal (`[ ... ]` or `{ ... }`) from source tokens.
//!
//! Split out of `execution_loop` when that file outgrew the §14.1 size budget.
//! The two concerns are genuinely separate: the execution loop decides *which*
//! token runs next, and this decides what a bracketed token sequence denotes.
//!
//! `[ ... ]` and `{ ... }` build the identical value — a `Value::Vector`
//! whose elements are literal values, with a bare name (other than
//! `TRUE`/`FALSE`/`NIL`, which still denote their values) becoming a
//! `Value::Symbol` rather than a `Value::Text`. This is the CodeBlock/Vector
//! unification (docs/dev/type-unification-work-order-2026-08.md): this
//! function is now the single place either spelling is built, and it is
//! dictionary-independent either way — no name lookup ever occurs here,
//! matching the pre-unification `LANG.VALUES.VECTOR` rule this supersedes.
//!
//! The bracket kind (`[`/`{`) that opened a literal must still match its own
//! closer (`]`/`}`) lexically — a parser-level convenience the unification
//! does not need to erase, and one `compiled_plan.rs`'s `COND` lowering still
//! relies on (a `{ }`-spelled literal compiles to a distinct `PushCodeBlock`
//! op precisely so `COND`'s clause-block count is known at compile time,
//! before the two spellings' values become indistinguishable).

use crate::error::{AjisaiError, Result};
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Token, Value};

use super::Interpreter;

/// Which bracket spelling opened the literal being collected, purely to
/// require its own matching closer — see the module doc.
#[derive(Clone, Copy, PartialEq)]
enum BracketKind {
    Vector,
    Block,
}

impl Interpreter {
    pub(crate) fn collect_bracketed_with_depth(
        tokens: &[Token],
        start_index: usize,
        depth: usize,
    ) -> Result<(Vec<Value>, usize, Interpretation)> {
        let kind = match tokens.get(start_index) {
            Some(Token::VectorStart) => BracketKind::Vector,
            Some(Token::BlockStart) => BracketKind::Block,
            _ => return Err(AjisaiError::from("Expected a bracketed literal start")),
        };

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
                Token::VectorStart | Token::BlockStart => {
                    // Hint 伝播フロー:
                    // collect_bracketed_with_depth(inner) -> nested_hint
                    //   -> Value::from_vector_with_hint(nested_values, nested_hint)
                    //   -> value_to_arena が Value.hint をそのまま Node hint として採用
                    // これにより、ネスト深度に依存せず明示 hint を維持する。
                    let (nested_values, consumed, nested_hint) =
                        Self::collect_bracketed_with_depth(tokens, i, depth + 1)?;
                    values.push(Value::from_vector_promoted_with_hint(
                        nested_values,
                        nested_hint,
                    ));
                    has_other = true;
                    i += consumed;
                }
                Token::VectorEnd if kind == BracketKind::Vector => {
                    return Ok((
                        values,
                        i - start_index + 1,
                        Self::element_hint(has_other, has_bool, has_number),
                    ));
                }
                Token::BlockEnd if kind == BracketKind::Block => {
                    return Ok((
                        values,
                        i - start_index + 1,
                        Self::element_hint(has_other, has_bool, has_number),
                    ));
                }
                Token::VectorEnd | Token::BlockEnd => {
                    return Err(AjisaiError::from(
                        "Mismatched bracket: a literal opened with `[` must close with `]`, \
                         and one opened with `{` must close with `}`",
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
                            // A bare name — in either bracket spelling — is a
                            // Symbol: data until something executes it,
                            // dictionary-independent either way (building
                            // the literal never looks anything up).
                            values.push(Value::from_symbol(s));
                            has_other = true;
                        }
                    }
                    i += 1;
                }
                Token::CondClauseSep => {
                    // `{ IDLE | 1 }`: `|` inside a clause block is data until
                    // COND runs it — a bare Symbol("|"), like any other name,
                    // symmetric with how `Value::Symbol` was promoted for
                    // ordinary names. `value_as_code.rs` maps it back to
                    // `Token::CondClauseSep` when the clause is actually
                    // executed. Accepted under either bracket spelling, like
                    // `tokenizer::validate_code_tokens`: a `DEF`'d word's
                    // clause blocks round-trip through that bridge and lose
                    // their original `{` vs `[` once they exist as a Vector
                    // (CodeBlock/Vector unification), so a stored body's `|`
                    // always reaches here re-expanded as `[ ]`. Whether it
                    // actually sits inside a legitimate COND clause is
                    // decided later, when `split_clause_blocks` tries to
                    // split the block that holds it.
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
