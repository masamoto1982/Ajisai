use crate::error::{AjisaiError, Result};
use crate::types::{Token, Value};

use super::value_extraction_helpers::create_number_value;
use super::Interpreter;

/// If the bracketed literal spanning `tokens[start..start+consumed)` is
/// immediately followed (no tokens between) by `<name-string> DEF`,
/// return its inner tokens (brackets excluded) — the body `DEF` is about to
/// define, as written. `None` on any mismatch, which just means this literal
/// is not a `DEF` body written in place — see `pending_def_body_tokens`'s
/// doc comment for why `op_def` wants this at all.
fn def_body_tokens_if_literal_precedes_def(
    tokens: &[Token],
    start: usize,
    consumed: usize,
) -> Option<Vec<Token>> {
    let mut j = start + consumed;
    if !matches!(tokens.get(j), Some(Token::String(_))) {
        return None;
    }
    j += 1;
    match tokens.get(j) {
        Some(Token::Symbol(s)) if crate::word_name::canonical_word_name(s).as_ref() == "DEF" => {
            Some(tokens[start + 1..start + consumed - 1].to_vec())
        }
        _ => None,
    }
}

impl Interpreter {
    pub(crate) fn execute_section_core(
        &mut self,
        tokens: &[Token],
        start_index: usize,
    ) -> Result<usize> {
        // Every execution route (EXEC and higher-order Words included) shares
        // the source-entry numeric ceiling; dynamically reflected tokens may
        // not bypass it.
        self.check_source_numeric_literals(&tokens[start_index..])?;

        // Depth 1 is the program's own token stream, the one `source_spans`
        // describes. A nested block, a word body or a COND clause is a
        // different stream with no source of its own, so the cursor is left
        // pointing at the top-level token that reached it — which is exactly
        // the token a reader needs to be sent to.
        self.section_depth += 1;
        let track = self.section_depth == 1 && !self.source_spans.is_empty();
        let result = self.execute_section_tokens(&tokens[start_index..], start_index, track);
        self.section_depth -= 1;
        result
    }

    fn execute_section_tokens(
        &mut self,
        execute_tokens: &[Token],
        start_index: usize,
        track_source_position: bool,
    ) -> Result<usize> {
        let mut i: usize = 0;

        while i < execute_tokens.len() {
            if track_source_position {
                self.current_source_span = self.source_spans.get(start_index + i).copied();
            }
            match &execute_tokens[i] {
                Token::Number(literal) => {
                    // Parsed when the lexeme was read, not here — this line ran
                    // once per element of every `MAP` block that mentioned a
                    // number. `parsed` only re-derives anything on the refusal
                    // path, which ends the program.
                    let frac = literal.parsed().map_err(AjisaiError::MalformedSource)?;
                    self.stack.push(create_number_value(frac));
                }
                Token::String(s) => {
                    self.stack.push(Value::from_string(s));
                }
                Token::VectorStart => {
                    // `[ ]` is the sole bracket, built through
                    // `vector_literal.rs`'s collector — see that module's
                    // doc comment. `COND` takes its clause blocks as a
                    // single ordinary Vector operand (one more literal built
                    // and pushed exactly like this one) rather than a
                    // variable-length run recognized here — see
                    // `control_cond.rs::op_cond`'s doc comment for why.
                    let (values, consumed) = Self::collect_bracketed_with_depth(
                        execute_tokens,
                        i,
                        1,
                        &self.runtime_limits,
                    )?;
                    // A literal immediately followed by `<name> DEF`
                    // is that DEF's body — captured here, as written, for
                    // `op_def` to prefer over re-deriving it from the Value
                    // just built (see `pending_def_body_tokens`'s doc
                    // comment). A mismatch here is not an error: it just
                    // means this literal is not a `DEF` body written
                    // in place, and `op_def` falls back normally.
                    self.pending_def_body_tokens =
                        def_body_tokens_if_literal_precedes_def(execute_tokens, i, consumed);
                    self.stack.push(Value::from_vector_promoted(values));
                    i += consumed;
                    continue;
                }
                Token::Symbol(s) => {
                    let canonical = crate::word_name::canonical_word_name(s);
                    {
                        let upper = canonical;

                        let witness = self.begin_dispatch();
                        match self.execute_word_core(upper.as_ref()) {
                            Ok(()) => self.trace_nil_outcome(upper.as_ref(), &witness),
                            Err(err) => {
                                self.record_word_dispatch_failure(
                                    upper.as_ref(),
                                    &err,
                                    witness.stack_len_before,
                                );
                                return Err(err);
                            }
                        }
                    }
                }
                Token::Value(value) => {
                    // A value the Vector-as-code bridge carried in whole: it
                    // is pushed as it is, like a literal. It is never a `DEF`
                    // body, so a pending body capture is cleared.
                    self.pending_def_body_tokens = None;
                    self.stack.push((**value).clone());
                    i += 1;
                    continue;
                }
                Token::VectorEnd => {
                    return Err(AjisaiError::MalformedSource(
                        "Unexpected vector end".to_string(),
                    ));
                }
            }
            i += 1;
        }

        Ok(start_index + i)
    }

    /// Evaluate the tokens of a *nested* code block — the block a higher-order
    /// Word (`MAP`, `FILTER`, `FOLD`, `SCAN`) applies, or the one `EXEC`
    /// runs — as its own token stream.
    pub(crate) fn execute_nested_block(&mut self, tokens: &[Token]) -> Result<()> {
        // A transparent frame: the block reads the names of the frame it was
        // written in, and the names it makes are gone when it ends. That is
        // what lets a bound threshold be used inside `{ T LT } FILTER` — and
        // why a `BIND` inside a `MAP` block is a fresh name per element rather
        // than a collision on the second.
        self.open_binding_scope(false);
        let result = self.execute_section_core(tokens, 0).map(|_| ());
        self.close_binding_scope();
        result
    }

    pub async fn execute(&mut self, code: &str) -> Result<()> {
        // CS5: bound the input before it is expanded into values. Source bytes
        // are checked before tokenization allocates per-character buffers; each
        // numeric literal's digit count is checked after tokenization but
        // before `Fraction::from_str` parses it into a (potentially enormous)
        // BigInt-backed value.
        self.runtime_limits.check_source_bytes(code.len())?;
        self.execution_step_count = 0;
        self.numeric_work_used = 0;
        self.collection_work_used = 0;
        self.dictionary_changes_this_run.clear();
        // A literal captured for a `DEF` that never ran — the dispatch failed
        // before `op_def` took it — must not become the body of the next run's
        // first `DEF`.
        self.pending_def_body_tokens = None;
        self.reset_binding_scopes();
        // Merge rather than replace: a `#:contract` line and the `DEF` it
        // documents can arrive in separate `execute()` calls (the Playground
        // runs one submission at a time), so an entry here must survive until
        // `op_def` actually consumes it for the Word it names.
        for (name, description) in
            crate::interpreter::execute_def::extract_pending_word_descriptions(code)
        {
            self.pending_word_descriptions.insert(name, description);
        }
        // Source entry is the one place a token has a position, so it is the
        // one place the positions are recorded. They are index-aligned with
        // `tokens` and consumed by the depth-1 cursor in `execute_section_core`.
        // A lexical failure is a fault in the writing, not in any value, so it
        // is classified as one rather than arriving as an uncategorized
        // `Custom` whose only next check was "read the message".
        let (tokens, spans) =
            crate::tokenizer::tokenize_with_spans(code).map_err(AjisaiError::MalformedSource)?;
        self.source_spans = spans;
        self.current_source_span = None;
        self.check_source_numeric_literals(&tokens)?;
        self.execute_section_core(&tokens, 0)?;
        self.check_fresh_nesting()
    }

    /// Enforce the numeric-literal digit ceiling on every `Token::Number`
    /// produced from source, before any of them is parsed into a value. The
    /// count is of the digits the literal denotes — written digits plus the
    /// exponent's magnitude (`tokenizer::denoted_digit_count`) — so the bound
    /// tracks the magnitude of the BigInt that would be built: `1e99999999`
    /// is nine characters and used to spend minutes building its integer.
    pub(crate) fn check_source_numeric_literals(&self, tokens: &[Token]) -> Result<()> {
        for token in tokens {
            if let Token::Number(literal) = token {
                let digits = crate::tokenizer::denoted_digit_count(literal.lexeme());
                self.runtime_limits
                    .check_numeric_literal_digits(usize::try_from(digits).unwrap_or(usize::MAX))?;
            }
        }
        Ok(())
    }
}
