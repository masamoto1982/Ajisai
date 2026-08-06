use crate::error::{AjisaiError, ErrorCategory, NilReason, Result};
use crate::types::fraction::Fraction;
use crate::types::{ExecutionLine, Interpretation, Token, Value};

use super::debug_diagnosis::{DebugDiagnosis, ErrorPhase};
use super::error_flow_trace::{ErrorFlowEvent, ErrorFlowEventKind};
use super::value_extraction_helpers::create_number_value;
use super::{ConsumptionMode, Interpreter};

/// Index just past the single *source unit* that begins at `start` in `tokens`:
/// either one ordinary token, or one balanced `[ ]` / `{ }` group (nesting
/// respected). This is the one, canonical definition of the unit that a non-NIL
/// `VENT` (`^` or the spelled-out name — both `Token::NilCoalesce`) skips
/// unevaluated (SPEC §6.4). `start` at or past the end is returned unchanged, so
/// a directive with no following unit is a no-op skip.
pub(crate) fn end_of_source_unit(tokens: &[Token], start: usize) -> usize {
    let open = match tokens.get(start) {
        Some(tok @ (Token::VectorStart | Token::BlockStart)) => tok.clone(),
        Some(_) => return start + 1,
        None => return start,
    };
    let close = match open {
        Token::VectorStart => Token::VectorEnd,
        _ => Token::BlockEnd,
    };
    let mut depth = 1usize;
    let mut i = start + 1;
    while i < tokens.len() && depth > 0 {
        if tokens[i] == open {
            depth += 1;
        } else if tokens[i] == close {
            depth -= 1;
        }
        i += 1;
    }
    i
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
        | "REORDER" | "SPLIT" | "COLLECT" | "RESHAPE" | "TRANSPOSE" | "FILL" | "TOKENIZE"
        | "CONSERVE" | "REFLECT" => Some(Interpretation::Unassigned),
        _ => None,
    };
    if let Some(h) = hint {
        let len: usize = interp.stack.len();
        if len > 0 {
            interp.stack.set_role_at(len - 1, h);
        }
    }
}

/// True when the last executable token of a line is the `COND` word. Used by
/// the tail-call trampoline: only a body line that ends in `COND` carries a
/// guarded tail self-call eligible for the internal backward jump. Trailing
/// `LineBreak`s are ignored.
pub(crate) fn tail_token_is_cond(tokens: &[Token]) -> bool {
    for token in tokens.iter().rev() {
        match token {
            Token::LineBreak => continue,
            Token::Symbol(s) => {
                return crate::core_word_aliases::canonicalize_core_word_name(s).as_ref() == "COND";
            }
            _ => return false,
        }
    }
    false
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
        | NilReason::NoData
        | NilReason::PortDisconnected
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
    let top = interp.stack.last()?;
    // Only operational NIL participates in error-flow tracing. The logical
    // Unknown (U) is a `ValueData::Unknown` truth value, not a NIL, so it is
    // excluded here by `is_nil()` (SPEC §4.5.2 / §7.5); no NIL reason can
    // represent U since CS4 PR-3 retired `LogicallyUnknown`.
    if !top.is_nil() {
        return None;
    }
    top.nil_reason().cloned()
}

fn trace_direct_nil_produced(interp: &mut Interpreter, word: &str, stack_len_before: usize) {
    let Some(reason) = top_direct_nil_reason(interp) else {
        return;
    };

    let category = error_category_for_nil_reason(&reason);
    let stack_len_after = interp.stack.len();
    let diagnosis = DebugDiagnosis::from_error_category(
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
    let absence = interp
        .stack
        .last()
        .and_then(|value| value.normalized_absence_metadata());
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
                    let (values, consumed, element_hint) =
                        self.collect_vector(execute_tokens, i)?;
                    self.stack
                        .push_with_role(Value::from_vector_promoted(values), element_hint);
                    i += consumed;
                    continue;
                }
                Token::BlockStart => {
                    let mut depth: i32 = 1;
                    let mut j: usize = i + 1;
                    let mut block_tokens: Vec<Token> = Vec::new();

                    while j < execute_tokens.len() && depth > 0 {
                        match &execute_tokens[j] {
                            Token::BlockStart => {
                                depth += 1;
                                block_tokens.push(execute_tokens[j].clone());
                            }
                            Token::BlockEnd => {
                                depth -= 1;
                                if depth > 0 {
                                    block_tokens.push(execute_tokens[j].clone());
                                }
                            }
                            token => block_tokens.push(token.clone()),
                        }
                        j += 1;
                    }

                    if depth != 0 {
                        return Err(AjisaiError::from("Unclosed code block"));
                    }

                    self.stack.push_with_role(
                        Value::from_code_block(block_tokens),
                        Interpretation::Unassigned,
                    );
                    i = j;
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

                            // Internal tail-call elimination ("internal GOTO").
                            // A guarded tail self-call — the last executable
                            // token of a COND clause body that resolves to the
                            // word currently trampolining — is not executed.
                            // Its arguments are left on the stack and
                            // `tail_jump_pending` is raised; the trampoline loop
                            // in `execute_word_core_inner` re-runs the body as a
                            // backward jump, so the recursion never grows
                            // `call_depth` or the native stack.
                            if self.in_tail_context && self.tail_call_enabled {
                                let is_last_executable = execute_tokens[i + 1..]
                                    .iter()
                                    .all(|t| matches!(t, Token::LineBreak));
                                if is_last_executable {
                                    if let Some(target) = self.tail_self_word.clone() {
                                        if let Some((resolved, def)) =
                                            self.resolve_word_entry_readonly(upper.as_ref())
                                        {
                                            if !def.is_builtin && resolved == target {
                                                self.tail_jump_pending = true;
                                                self.in_tail_context = false;
                                                return Ok(start_index + i + 1);
                                            }
                                        }
                                    }
                                }
                            }

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
                                    // The top-level token that reached this
                                    // failure. Every frame on the way out pushes
                                    // an event and the outermost one wins, so
                                    // the position a reader is sent to is the
                                    // token they actually wrote.
                                    let diagnosis = DebugDiagnosis::from_error(
                                        &err,
                                        Some(upper.as_ref()),
                                        stack_len_before,
                                        self.stack.len(),
                                    )
                                    .with_source_position(self.current_source_span);
                                    self.push_error_flow_trace(ErrorFlowEvent {
                                        kind: ErrorFlowEventKind::WordError,
                                        word: Some(upper.to_string()),
                                        error_category: Some(category),
                                        absence: None,
                                        stack_len_before,
                                        stack_len_after: self.stack.len(),
                                        message: format!("word error word={} error={}", upper, err),
                                        diagnosis: Some(diagnosis),
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
                Token::BlockEnd => {
                    return Err(AjisaiError::from("Unexpected code block end"));
                }
                Token::NilCoalesce => {
                    // VENT (`^` / spelled-out `VENT`, SPEC §6.4): inspect the top.
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
    ///
    /// A nested block is never the enclosing word's tail position, even when it
    /// is written at the very end of a tail `COND` clause body: the block is
    /// data at that point, and the Word that receives it decides when (and how
    /// many times) it runs. Inheriting `in_tail_context` here made
    /// `{ FIB } MAP` inside `FIB`'s own body look like a guarded tail self-call,
    /// so `FIB` was *not applied to the first element* — the deferral site left
    /// the element untouched and raised `tail_jump_pending`, which a later
    /// frame's cleanup then swallowed. The result was a silently wrong answer
    /// from a program with no error and no NIL, which is exactly the failure
    /// mode LANG.FAILURE.TRICHOTOMY exists to prevent. With `EXEC` the same
    /// deferral instead re-ran the body forever until the step limit.
    ///
    /// Saving and clearing the flag around the block keeps genuine tail
    /// self-calls (a bare `... COND`-guarded self-call written directly in the
    /// body's token stream) trampolined, while a self-call reached *through* a
    /// block is an ordinary call.
    pub(crate) fn execute_nested_block(&mut self, tokens: &[Token]) -> Result<()> {
        let saved_tail_context: bool = std::mem::replace(&mut self.in_tail_context, false);
        // A transparent frame: the block reads the names of the frame it was
        // written in, and the names it makes are gone when it ends. That is
        // what lets a bound threshold be used inside `{ T LT } FILTER` — and
        // why a `BIND` inside a `MAP` block is a fresh name per element rather
        // than a collision on the second.
        self.open_binding_scope(false);
        let result = self.execute_section_core(tokens, 0).map(|_| ());
        self.close_binding_scope();
        self.in_tail_context = saved_tail_context;
        result
    }

    pub(crate) fn execute_guard_structure(&mut self, lines: &[ExecutionLine]) -> Result<()> {
        // Mirror the compiled-plan tail marking on the plain interpreter path so
        // the two stay behaviorally identical (shadow validation runs both). A
        // word body is in tail position on its last line, and we only propagate
        // tail context when that line ends in `COND` — exactly the compiled
        // path's `CallBuiltin("COND")` tail op. A bare tail self-call (e.g.
        // `{ REC }`, with no base case) is deliberately *not* trampolined: it
        // keeps the legacy native-recursion behavior and its depth-limit error.
        let tail_enabled = self.tail_call_enabled && self.tail_self_word.is_some();
        let last_line = lines.len().saturating_sub(1);
        for (idx, line) in lines.iter().enumerate() {
            self.in_tail_context =
                tail_enabled && idx == last_line && tail_token_is_cond(&line.body_tokens);
            let r = self.execute_section_core(&line.body_tokens, 0);
            self.in_tail_context = false;
            r?;
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
        self.dictionary_changes_this_run.clear();
        self.reset_binding_scopes();
        // Source entry is the one place a token has a position, so it is the
        // one place the positions are recorded. They are index-aligned with
        // `tokens` and consumed by the depth-1 cursor in `execute_section_core`.
        let (tokens, spans) = crate::tokenizer::tokenize_with_spans(code)?;
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
