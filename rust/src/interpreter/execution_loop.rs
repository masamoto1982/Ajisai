use crate::error::{AjisaiError, ErrorCategory, NilReason, Result};
use crate::types::fraction::Fraction;
use crate::types::{ExecutionLine, Interpretation, Token, Value};

use super::debug_diagnosis::{DebugDiagnosis, ErrorPhase};
use super::error_flow_trace::{ErrorFlowEvent, ErrorFlowEventKind};
use super::value_extraction_helpers::create_number_value;
use super::{ConsumptionMode, Interpreter};

/// Index just past the single *source unit* that begins at `start` in `tokens`:
/// either one ordinary token, or one balanced `[ ]` group (nesting
/// respected). This is the one, canonical definition of the unit that a non-NIL
/// `OR-NIL` (the sole spelling — `Token::NilCoalesce`) skips
/// unevaluated (SPEC §6.4). `start` at or past the end is returned unchanged, so
/// a directive with no following unit is a no-op skip.
pub(crate) fn end_of_source_unit(tokens: &[Token], start: usize) -> usize {
    match tokens.get(start) {
        Some(Token::VectorStart) => {}
        Some(_) => return start + 1,
        None => return start,
    };
    let mut depth = 1usize;
    let mut i = start + 1;
    while i < tokens.len() && depth > 0 {
        if tokens[i] == Token::VectorStart {
            depth += 1;
        } else if tokens[i] == Token::VectorEnd {
            depth -= 1;
        }
        i += 1;
    }
    i
}

/// If the bracketed literal spanning `tokens[start..start+consumed)` is
/// immediately followed (no tokens between) by `<name-string> [KEEP] DEF`,
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
    if matches!(tokens.get(j), Some(Token::Symbol(s))
        if crate::core_word_aliases::canonicalize_core_word_name(s).as_ref() == "KEEP")
    {
        j += 1;
    }
    match tokens.get(j) {
        Some(Token::Symbol(s))
            if crate::core_word_aliases::canonicalize_core_word_name(s).as_ref() == "DEF" =>
        {
            Some(tokens[start + 1..start + consumed - 1].to_vec())
        }
        _ => None,
    }
}

/// After a core word runs, retag the top-of-stack plane role from a small
/// name-keyed table (SPEC §12). The interpreted loop applies this after every
/// symbol; the compiled plan mirrors it after each call op so the two routes
/// leave identical `(value, role)` observations. A no-op for words not in the
/// table (e.g. user words).
pub(crate) fn apply_word_hint_override(interp: &mut Interpreter, word: &str) {
    let hint: Option<Interpretation> = match word {
        "NUM" | "ADD" | "SUB" | "MUL" | "DIV" | "MOD" | "FLOOR" | "CEIL" | "ROUND" | "QUANTIZE"
        | "QUANTIZE-HALF-AWAY" | "QUANTIZE-FLOOR" | "QUANTIZE-CEIL" | "QUANTIZE-TRUNC" | "FOLD" => {
            Some(Interpretation::RawNumber)
        }
        "SQRT" | "SQRT_EPS" | "INTERVAL" | "MATH@SQRT" | "MATH@SQRT-EPS" | "MATH@INTERVAL" => {
            Some(Interpretation::Interval)
        }
        "LOWER" | "UPPER" | "WIDTH" | "MATH@LOWER" | "MATH@UPPER" | "MATH@WIDTH" => {
            Some(Interpretation::RawNumber)
        }
        "BOOL" | "LT" | "LTE" | "GT" | "GTE" | "EQ" | "NEQ" | "AND" | "OR" | "NOT"
        | "STARTS-WITH?" | "ENDS-WITH?" => Some(Interpretation::TruthValue),
        "NOW" | "TIMESTAMP" => Some(Interpretation::Timestamp),
        // `CONCAT` is deliberately absent: its result role depends on its
        // operands (joining two Texts yields a Text), so `op_concat` pushes the
        // slot role itself. Stamping `Unassigned` here is what made
        // `'ab' 'c' CONCAT` render as `[ 97/1 98/1 99/1 ]` — the join was
        // right, the role was thrown away.
        "CHARS" | "MAP" | "FILTER" | "SCAN" | "UNFOLD" | "REVERSE" | "SORT" | "TAKE"
        | "REORDER" | "SPLIT" | "COLLECT" | "FILL" | "TOKENIZE" | "CONSERVE" | "REFLECT" => {
            Some(Interpretation::Unassigned)
        }
        _ => None,
    };
    if let Some(h) = hint {
        let len: usize = interp.stack.len();
        if len > 0 {
            // `Interval` describes a *number*'s presentation. `SQRT` lifts over
            // a vector now, and stamping the role on the vector itself rendered
            // `[ 4 9 ] SQRT` as `[2/1, 3/1]` — a scalar's notation wrapped
            // around a collection. A collection keeps whatever role it was
            // built with; the lanes inside it are numbers either way.
            let stamps_a_scalar_role = matches!(h, Interpretation::Interval);
            let top_is_collection = interp
                .stack
                .last()
                .is_some_and(|value| value.as_vector_view().is_some());
            if !(stamps_a_scalar_role && top_is_collection) {
                interp.stack.set_role_at(len - 1, h);
            }
        }
    }
}

fn error_category_for_nil_reason(reason: &NilReason) -> Option<ErrorCategory> {
    match reason {
        NilReason::DivisionByZero => Some(ErrorCategory::DivisionByZero),
        NilReason::IndexOutOfBounds => Some(ErrorCategory::IndexOutOfBounds),
        NilReason::StackUnderflow => Some(ErrorCategory::StackUnderflow),
        NilReason::UnknownWord => Some(ErrorCategory::UnknownWord),
        NilReason::EmptySequence
        | NilReason::MissingField
        | NilReason::InvalidEncoding
        | NilReason::InvalidLens
        | NilReason::ExecutionFailure
        | NilReason::Undecidable
        | NilReason::SpaceExhausted
        // No `ErrorCategory` names a domain miss or an unavailable diagnostic,
        // and inventing one would add a category with no `AjisaiError` behind
        // it. `Custom` is where every reason without a matching error variant
        // already lands.
        | NilReason::DomainMiss
        | NilReason::NotAvailable
        | NilReason::Literal => Some(ErrorCategory::Custom),
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
    // Only operational NIL is meant to participate in error-flow tracing
    // (SPEC §4.5.2 / §7.5); the logical Unknown (U) — `Nil` data carrying
    // the `TruthValue` hint, not a dedicated variant — should not. `is_nil`
    // does not look at `hint`, so it does not currently distinguish the
    // two; this has no observable effect today because U is unreachable
    // from the current vocabulary (see `types/exact/computable.rs`).
    if value.is_nil() {
        return match value.nil_reason() {
            Some(NilReason::Literal) | None => None,
            Some(reason) => Some(*reason),
        };
    }
    let lanes = value.as_vector_view()?;
    lanes.iter().find_map(projected_nil_reason)
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
    /// Synchronous single-line entry point used by the WASM step controller.
    #[cfg(feature = "wasm")]
    pub(crate) fn execute_guard_structure_sync(&mut self, lines: &[ExecutionLine]) -> Result<()> {
        self.execute_guard_structure(lines)
    }

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
                Token::Number(n) => {
                    let frac = Fraction::from_str(n).map_err(AjisaiError::from)?;
                    self.stack
                        .push_with_role(create_number_value(frac), Interpretation::RawNumber);
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
                    let (values, consumed, element_hint) =
                        Self::collect_bracketed_with_depth(execute_tokens, i, 1)?;
                    // A literal immediately followed by `<name> [KEEP] DEF`
                    // is that DEF's body — captured here, as written, for
                    // `op_def` to prefer over re-deriving it from the Value
                    // just built (see `pending_def_body_tokens`'s doc
                    // comment). A mismatch here is not an error: it just
                    // means this literal is not a `DEF` body written
                    // in place, and `op_def` falls back normally.
                    self.pending_def_body_tokens =
                        def_body_tokens_if_literal_precedes_def(execute_tokens, i, consumed);
                    self.stack
                        .push_with_role(Value::from_vector_promoted(values), element_hint);
                    i += consumed;
                    continue;
                }
                Token::Symbol(s) => {
                    let canonical = crate::core_word_aliases::canonicalize_core_word_name(s);
                    match canonical.as_ref() {
                        "KEEP" => {
                            self.update_consumption_mode(ConsumptionMode::Keep);
                        }
                        _ => {
                            let upper = canonical;

                            let stack_len_before = self.stack.len();
                            match self.execute_word_core(upper.as_ref()) {
                                Ok(()) => {
                                    trace_direct_nil_produced(
                                        self,
                                        upper.as_ref(),
                                        stack_len_before,
                                    );
                                    apply_word_hint_override(self, upper.as_ref());
                                }
                                Err(err) => {
                                    let category = ErrorCategory::from_error(&err);
                                    let error_text = err.to_string();
                                    // A failure raised inside a block this Word
                                    // applied, or inside a User Word's body, is
                                    // already recorded under the name of the
                                    // Word that raised it. This frame is the
                                    // one it happened *inside*, so it adds
                                    // itself as context and leaves the answer
                                    // to "which Word failed" alone.
                                    if self.attribute_enclosing_word(upper.as_ref(), &error_text) {
                                        return Err(err);
                                    }
                                    // The top-level token that reached this
                                    // failure. A block and a Word body are each
                                    // their own token stream with no source of
                                    // their own, so the position a reader is
                                    // sent to is the top-level token they
                                    // actually wrote.
                                    let mut diagnosis = DebugDiagnosis::from_error(
                                        &err,
                                        Some(upper.as_ref()),
                                        stack_len_before,
                                        self.stack.len(),
                                    )
                                    .with_source_position(self.current_source_span);
                                    // A misspelled *user* Word is only
                                    // knowable here: the compiled-in registry
                                    // has never heard of it, and this is the
                                    // frame that holds the live dictionary.
                                    diagnosis.with_user_vocabulary(
                                        self.user_words.keys().map(String::as_str),
                                    );
                                    self.push_error_flow_trace(ErrorFlowEvent {
                                        kind: ErrorFlowEventKind::WordError,
                                        word: Some(upper.to_string()),
                                        error_category: Some(category),
                                        absence: None,
                                        stack_len_before,
                                        stack_len_after: self.stack.len(),
                                        message: format!("word error word={} error={}", upper, err),
                                        diagnosis: Some(diagnosis),
                                        error_text,
                                    });
                                    return Err(err);
                                }
                            }
                            if true {
                                self.reset_execution_modes();
                            }
                        }
                    }
                }
                Token::NilCoalesce => {
                    // OR-NIL (SPEC §6.4): inspect the top.
                    let (value, hint) = self.stack.pop_slot().ok_or(AjisaiError::StackUnderflow)?;

                    if !value.is_nil() {
                        // Non-NIL: keep it and skip the following source unit
                        // unevaluated (one token or one balanced group).
                        self.stack.push_with_role(value, hint);
                        i = end_of_source_unit(execute_tokens, i + 1);
                        continue;
                    }
                    // NIL: discard it and let the trailing `i += 1` fall through
                    // so the following source unit is evaluated as the fallback.
                }
                Token::CondClauseSep => {
                    // ControlDirective: '|' -> COND-CLAUSE (see surface_forms.rs).
                    return Err(AjisaiError::from(
                        "Unexpected '|' separator outside COND clause parsing. \
                         '|' is control directive sugar for COND-CLAUSE and is meaningful only inside a COND expression.",
                    ));
                }

                Token::LineBreak => {}
                Token::VectorEnd => {
                    return Err(AjisaiError::from("Unexpected vector end"));
                }
            }
            i += 1;
        }

        Ok(start_index + i)
    }

    /// Evaluate the tokens of a *nested* code block — the block a higher-order
    /// Word (`MAP`, `FILTER`, `ANY`, `ALL`, `FOLD`) applies, or the one `EXEC`
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

    pub(crate) fn execute_guard_structure(&mut self, lines: &[ExecutionLine]) -> Result<()> {
        for line in lines.iter() {
            self.execute_section_core(&line.body_tokens, 0)?;
        }
        Ok(())
    }

    pub(crate) fn split_tokens_to_lines(&self, tokens: &[Token]) -> Result<Vec<ExecutionLine>> {
        Ok(vec![ExecutionLine {
            body_tokens: tokens.to_vec().into(),
        }])
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
        let lines: Vec<ExecutionLine> = self.split_tokens_to_lines(&tokens)?;
        self.execute_guard_structure(&lines)?;
        Ok(())
    }

    /// Enforce the numeric-literal digit ceiling on every `Token::Number`
    /// produced from source, before any of them is parsed into a value. Digit
    /// characters are counted directly (sign, radix point, and `/` excluded),
    /// so the bound tracks the magnitude of the BigInt that would be built.
    pub(crate) fn check_source_numeric_literals(&self, tokens: &[Token]) -> Result<()> {
        for token in tokens {
            if let Token::Number(literal) = token {
                let digits = literal.chars().filter(|c| c.is_ascii_digit()).count();
                self.runtime_limits.check_numeric_literal_digits(digits)?;
            }
        }
        Ok(())
    }
}
