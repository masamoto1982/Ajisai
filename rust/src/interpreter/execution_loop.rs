use crate::error::{AjisaiError, ErrorCategory, NilReason, Result};
use crate::types::{Token, Value, ValueData};

use super::debug_diagnosis::{DebugDiagnosis, ErrorPhase};
use super::error_flow_trace::{ErrorFlowEvent, ErrorFlowEventKind};
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
        Some(Token::Symbol(s))
            if crate::core_word_aliases::canonicalize_core_word_name(s).as_ref() == "DEF" =>
        {
            Some(tokens[start + 1..start + consumed - 1].to_vec())
        }
        _ => None,
    }
}

fn error_category_for_nil_reason(reason: &NilReason) -> Option<ErrorCategory> {
    match reason {
        NilReason::DivisionByZero => Some(ErrorCategory::DivisionByZero),
        // No `ErrorCategory` names a domain miss, an unavailable diagnostic,
        // or an index past the end, and inventing one would add a category
        // with no `AjisaiError` behind it — `None` here means the trace's
        // `category` evidence is simply absent, not a catch-all category
        // standing in for it. `indexOutOfBounds` joined this group when `TAKE`
        // and `PUT` stopped raising it: no Word raises past-the-end any more,
        // so the reason names a projection and nothing else.
        NilReason::IndexOutOfBounds
        | NilReason::NotFound
        | NilReason::InvalidEncoding
        | NilReason::SpaceExhausted
        | NilReason::DomainMiss
        | NilReason::NotAvailable
        | NilReason::Literal
        | NilReason::UserDeclared => None,
    }
}

fn top_direct_nil_reason(interp: &Interpreter) -> Option<NilReason> {
    projected_nil_reason(interp.stack.last()?)
}

/// The reason a Word's result records for an absence *it produced*, or `None`.
///
/// Looking only at the value itself missed every lifted projection. A Word
/// lifted over a collection projects per lane (`LANG.COLLECTIONS.LIFT`), so
/// the absence it produced sits inside the result rather than being the
/// result: `6 0 /` was traced and `[ 6 ] [ 0 ] /` was not, and `[ 4 -1 ] SQRT`
/// never was, though all three project for a reason the Word can name.
///
/// `Literal` is excluded because it is the absence a Word *received*, not one
/// it made: a `NIL` written in source, and — since a dense lane carries
/// presence but no reason — any absence that has passed through a tensor. So
/// `[ 1 NIL 3 ] [ 2 ] *` records nothing, which is right: `*` propagated that
/// NIL, it did not produce it.
///
/// The first reasoned absence in reading order names the event, keeping one
/// event per Word call as the trace's shape requires.
fn projected_nil_reason(value: &Value) -> Option<NilReason> {
    // UNKNOWN is a NIL (LANG.VALUES.TRUTH), so it is traced like any other.
    if value.is_nil() {
        return match value.nil_reason() {
            Some(NilReason::Literal) | None => None,
            Some(reason) => Some(*reason),
        };
    }
    // A dense tensor already keeps *why* each absent lane is absent, in a map
    // holding only the absent ones. Materializing every lane to look for it
    // read the rare fact out of the common one: `as_vector_view` on a `Tensor`
    // rebuilds the whole buffer as boxed `Value`s, and this runs after every
    // Word. The map is the same evidence, in lane order, sized to the failures
    // rather than to the data.
    if let ValueData::Tensor { data, .. } = &value.data {
        return dense_projected_nil_reason(data);
    }
    let lanes = value.as_vector_view()?;
    lanes.iter().find_map(projected_nil_reason)
}

/// [`projected_nil_reason`] for a dense tensor, read from its absence map.
///
/// Equivalent to the materialized walk lane by lane: `absences()` yields the
/// absent lanes in ascending lane order and screens each against the presence
/// sentinel, which is exactly the order and the filter a walk over the boxed
/// lanes applied. A lane the tensor was never told a reason for carries none,
/// and is skipped here as `with_reasonless_unknown` was skipped there.
fn dense_projected_nil_reason(data: &crate::types::DenseTensor) -> Option<NilReason> {
    data.absences()
        .find_map(|(_, metadata)| match metadata.reason {
            Some(NilReason::Literal) | None => None,
            Some(reason) => Some(reason),
        })
}

/// The absence envelope of the same value [`projected_nil_reason`] answered
/// for, so the traced `absence` and the traced reason always describe one
/// value rather than two.
fn projected_absence_metadata(value: &Value) -> Option<crate::semantic::AbsenceMetadata> {
    if value.is_nil() {
        return match value.nil_reason() {
            Some(NilReason::Literal) | None => None,
            Some(_) => value.normalized_absence_metadata(),
        };
    }
    // Same lane, same map, same reason as `projected_nil_reason` picked — see
    // `dense_projected_nil_reason`. The two must agree on *which* lane they
    // describe, which is why both read the absence map in its lane order.
    if let ValueData::Tensor { data, .. } = &value.data {
        return data
            .absences()
            .find_map(|(_, metadata)| match metadata.reason {
                Some(NilReason::Literal) | None => None,
                Some(_) => Some(metadata.clone()),
            });
    }
    let lanes = value.as_vector_view()?;
    lanes.iter().find_map(projected_absence_metadata)
}

fn trace_direct_nil_produced(interp: &mut Interpreter, word: &str, stack_len_before: usize) {
    let Some(reason) = top_direct_nil_reason(interp) else {
        return;
    };

    let category = error_category_for_nil_reason(&reason);
    let stack_len_after = interp.stack.len();
    let mut diagnosis = DebugDiagnosis::from_error_category(
        ErrorPhase::ExecuteWord,
        Some(word),
        category.as_ref(),
        Some(&reason),
        stack_len_before,
        stack_len_after,
        Some(format!(
            "NIL produced by {} reason={}",
            word,
            reason.as_protocol_str()
        )),
    );
    // The absence envelope belongs to the value that actually carries the
    // projection, which for a lifted Word is a lane rather than the result.
    let absence = interp.stack.last().and_then(projected_absence_metadata);
    // The ceiling facts behind a resource projection are decided at the
    // projection site — the only place that knows which limit fired and at what
    // size — so they are carried over rather than rebuilt from the category
    // here, which could only say that *a* limit was crossed.
    diagnosis.resource_limit = absence
        .as_ref()
        .and_then(|metadata| metadata.diagnosis.as_ref())
        .and_then(|d| d.resource_limit.clone());
    interp.push_error_flow_trace(ErrorFlowEvent {
        kind: ErrorFlowEventKind::NilProduced,
        word: Some(word.to_string()),
        error_category: category,
        absence,
        stack_len_before,
        stack_len_after,
        message: format!(
            "NIL produced by {} reason={}",
            word,
            reason.as_protocol_str()
        ),
        diagnosis: Some(diagnosis),
        error_text: String::new(),
    });
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
                // Cleared per token, so a spelling recorded for one token can
                // never be read against another one's position.
                self.current_source_word = None;
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
                    let (values, consumed) =
                        Self::collect_bracketed_with_depth(execute_tokens, i, 1)?;
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
                    let canonical = crate::core_word_aliases::canonicalize_core_word_name(s);
                    // The one place the surface spelling still exists. Kept
                    // only where it differs from the canonical name and only
                    // for a top-level token, which is the token the recorded
                    // position describes; an `Arc` clone, so a dispatch that
                    // writes the name as it resolves pays nothing.
                    if track_source_position && canonical.as_ref() != s.as_ref() {
                        self.current_source_word = Some(std::sync::Arc::clone(s));
                    }
                    {
                        let upper = canonical;

                        let stack_len_before = self.stack.len();
                        match self.execute_word_core(upper.as_ref()) {
                            Ok(()) => {
                                trace_direct_nil_produced(self, upper.as_ref(), stack_len_before);
                            }
                            Err(err) => {
                                self.record_word_dispatch_failure(
                                    upper.as_ref(),
                                    &err,
                                    stack_len_before,
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
        self.current_source_word = None;
        self.check_source_numeric_literals(&tokens)?;
        self.execute_section_core(&tokens, 0)?;
        Ok(())
    }

    /// Enforce the numeric-literal digit ceiling on every `Token::Number`
    /// produced from source, before any of them is parsed into a value. Digit
    /// characters are counted directly (sign, radix point, and `/` excluded),
    /// so the bound tracks the magnitude of the BigInt that would be built.
    pub(crate) fn check_source_numeric_literals(&self, tokens: &[Token]) -> Result<()> {
        for token in tokens {
            if let Token::Number(literal) = token {
                let digits = literal
                    .lexeme()
                    .chars()
                    .filter(|c| c.is_ascii_digit())
                    .count();
                self.runtime_limits.check_numeric_literal_digits(digits)?;
            }
        }
        Ok(())
    }
}
