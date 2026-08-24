//! Building a Vector literal from source tokens.
//!
//! Split out of `execution_loop` when that file outgrew the §14.1 size budget.
//! The two concerns are genuinely separate: the execution loop decides *which*
//! token runs next, and this decides what the token sequence `[ ... ]` denotes.
//! LANG.VALUES.VECTOR governs here — inside a Vector literal a bare name is its
//! own text, and only `TRUE` / `FALSE` / `NIL` denote values — which is why no
//! dictionary is consulted anywhere below.

use crate::error::{AjisaiError, Result};
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Token, Value};

use super::Interpreter;

impl Interpreter {
    pub(crate) fn collect_vector(
        &mut self,
        tokens: &[Token],
        start_index: usize,
    ) -> Result<(Vec<Value>, usize, Interpretation)> {
        Self::collect_vector_with_depth(tokens, start_index, 1)
    }

    pub(crate) fn collect_vector_with_depth(
        tokens: &[Token],
        start_index: usize,
        depth: usize,
    ) -> Result<(Vec<Value>, usize, Interpretation)> {
        if !matches!(&tokens[start_index], Token::VectorStart) {
            return Err(AjisaiError::from("Expected vector start"));
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
                    // Hint 伝播フロー:
                    // collect_vector_with_depth(inner) -> nested_hint
                    //   -> Value::from_vector_with_hint(nested_values, nested_hint)
                    //   -> value_to_arena が Value.hint をそのまま Node hint として採用
                    // これにより、ネスト深度に依存せず明示 hint を維持する。
                    let (nested_values, consumed, nested_hint) =
                        Self::collect_vector_with_depth(tokens, i, depth + 1)?;
                    values.push(Value::from_vector_promoted_with_hint(
                        nested_values,
                        nested_hint,
                    ));
                    has_other = true;
                    i += consumed;
                }
                Token::VectorEnd => {
                    let element_hint: Interpretation = if has_other {
                        Interpretation::Unassigned
                    } else if has_bool && !has_number {
                        Interpretation::TruthValue
                    } else if has_number && !has_bool {
                        Interpretation::RawNumber
                    } else {
                        Interpretation::Unassigned
                    };
                    return Ok((values, i - start_index + 1, element_hint));
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
                            // LANG.VALUES.VECTOR: inside a Vector literal a name is
                            // data — its own text as a String — and no dictionary
                            // lookup occurs. Only TRUE / FALSE / NIL (handled above)
                            // denote values. This is what makes `[ FOO ]` denote the
                            // same Vector under every dictionary state instead of
                            // executing FOO when it happens to be a defined word,
                            // and it is why a misspelled name here is an element
                            // rather than an error.
                            values.push(Value::from_string(s));
                            has_other = true;
                        }
                    }
                    i += 1;
                }
                Token::CondClauseSep => {
                    // ControlDirective: '|' -> COND-CLAUSE (see surface_forms.rs).
                    return Err(AjisaiError::from(
                    "Unexpected '|' separator outside COND clause parsing. \
                     '|' is control directive sugar for COND-CLAUSE and is meaningful only inside a COND expression.",
                ));
                }
                Token::BlockStart => {
                    // A `{ ... }` written inside a Vector literal is data where
                    // it is written (LANG.SOURCE.FRAME): capture its tokens as
                    // an unevaluated CodeBlock element, the same way the
                    // top-level execution loop captures a bare block
                    // (`execution_loop.rs`'s `Token::BlockStart` arm), rather
                    // than reading through the delimiters and flattening the
                    // block's contents into this Vector. Depth-tracked scan,
                    // not a recursive call, so it carries no additional stack
                    // risk beyond the vector-nesting guard above.
                    let mut block_depth: i32 = 1;
                    let mut j = i + 1;
                    let mut block_tokens: Vec<Token> = Vec::new();
                    while j < tokens.len() && block_depth > 0 {
                        match &tokens[j] {
                            Token::BlockStart => {
                                block_depth += 1;
                                block_tokens.push(tokens[j].clone());
                            }
                            Token::BlockEnd => {
                                block_depth -= 1;
                                if block_depth > 0 {
                                    block_tokens.push(tokens[j].clone());
                                }
                            }
                            token => block_tokens.push(token.clone()),
                        }
                        j += 1;
                    }
                    if block_depth != 0 {
                        return Err(AjisaiError::from("Unclosed code block"));
                    }
                    values.push(Value::from_code_block(block_tokens));
                    has_other = true;
                    i = j;
                }
                Token::BlockEnd => {
                    return Err(AjisaiError::from("Unexpected code block end"));
                }
                Token::LineBreak | Token::NilCoalesce => {
                    i += 1;
                }
            }
        }
        Err(AjisaiError::from("Unclosed vector"))
    }
}
