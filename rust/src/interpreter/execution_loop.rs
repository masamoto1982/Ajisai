use crate::error::{AjisaiError, ErrorCategory, NilReason, Result};
use crate::types::fraction::Fraction;
use crate::types::{ExecutionLine, Interpretation, Token, Value};

use super::debug_diagnosis::{DebugDiagnosis, ErrorPhase};
use super::error_flow_trace::{ErrorFlowEvent, ErrorFlowEventKind};
use super::value_extraction_helpers::create_number_value;
use super::{modules, ConsumptionMode, Interpreter, OperationTargetMode};

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

/// After a core/module word runs, retag the top-of-stack plane role from a small
/// name-keyed table (SPEC §12). The interpreted loop applies this after every
/// symbol; the compiled plan mirrors it after each call op so the two routes
/// leave identical `(value, role)` observations. A no-op for words not in the
/// table (e.g. user words).
pub(crate) fn apply_word_hint_override(interp: &mut Interpreter, word: &str) {
    let hint: Option<Interpretation> = match word {
        "STR" | "CHR" | "JOIN" | "TRIM" | "TRIM-LEFT" | "TRIM-RIGHT" | "SUBSTITUTE" => {
            Some(Interpretation::Text)
        }
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
        "CHARS" | "MAP" | "FILTER" | "SCAN" | "UNFOLD" | "REVERSE" | "CONCAT" | "SORT" | "TAKE"
        | "REORDER" | "SPLIT" | "COLLECT" | "RESHAPE" | "TRANSPOSE" | "FILL" | "TOKENIZE"
        | "CONSERVE" => Some(Interpretation::Unassigned),
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
fn tail_token_is_cond(tokens: &[Token]) -> bool {
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
        // The logical Unknown (U) is not an error/absence and never carries
        // an error category. It is excluded from error-flow tracing by
        // `top_direct_nil_reason`, so this arm is defensive.
        NilReason::LogicallyUnknown => None,
        NilReason::EmptySequence
        | NilReason::MissingField
        | NilReason::InvalidEncoding
        | NilReason::InvalidLens
        | NilReason::ExecutionFailure
        | NilReason::Undecidable
        | NilReason::NoData
        | NilReason::PortDisconnected => Some(ErrorCategory::Custom),
    }
}

fn top_direct_nil_reason(interp: &Interpreter) -> Option<NilReason> {
    let top = interp.stack.last()?;
    if !top.is_nil() {
        return None;
    }
    let reason = top.nil_reason()?.clone();
    // The logical Unknown (U) is a TruthValue result, not an error/absence:
    // keep it out of the error-flow trace (SPEC §4.5.2).
    if matches!(reason, NilReason::LogicallyUnknown) {
        None
    } else {
        Some(reason)
    }
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
    pub(crate) fn collect_vector(
        &mut self,
        tokens: &[Token],
        start_index: usize,
    ) -> Result<(Vec<Value>, usize, Interpretation)> {
        self.collect_vector_with_depth(tokens, start_index, 1)
    }

    pub(crate) fn collect_vector_with_depth(
        &mut self,
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
                        self.collect_vector_with_depth(tokens, i, depth + 1)?;
                    if nested_values.is_empty() {
                        return Err(AjisaiError::from(
                            "Empty vector is not allowed. Use NIL for empty values.",
                        ));
                    }
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
                            let resolved = if let Some(def) = self.resolve_word(upper.as_ref()) {
                                !def.is_builtin
                            } else {
                                false
                            };

                            if resolved {
                                let stack_backup = std::mem::take(&mut self.stack);
                                let output_backup = std::mem::take(&mut self.output_buffer);
                                let host_effects_backup = std::mem::take(&mut self.host_effects);
                                match self.execute_word_core(upper.as_ref()) {
                                    Ok(()) => {
                                        let results =
                                            std::mem::replace(&mut self.stack, stack_backup);
                                        self.output_buffer = output_backup;
                                        self.host_effects = host_effects_backup;
                                        if results.is_empty() {
                                            values.push(Value::from_string(s));
                                            has_other = true;
                                        } else {
                                            values.extend(results);
                                            has_other = true;
                                        }
                                    }
                                    Err(_) => {
                                        self.stack = stack_backup;
                                        self.output_buffer = output_backup;
                                        self.host_effects = host_effects_backup;
                                        values.push(Value::from_string(s));
                                        has_other = true;
                                    }
                                }
                            } else {
                                values.push(Value::from_string(s));
                                has_other = true;
                            }
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
                _ => {
                    i += 1;
                }
            }
        }
        Err(AjisaiError::from("Unclosed vector"))
    }

    pub(crate) fn execute_guard_structure_sync(&mut self, lines: &[ExecutionLine]) -> Result<()> {
        self.execute_guard_structure(lines)
    }

    pub(crate) fn execute_section_core(
        &mut self,
        tokens: &[Token],
        start_index: usize,
    ) -> Result<usize> {
        let mut i: usize = 0;
        let execute_tokens: &[Token] = &tokens[start_index..];

        while i < execute_tokens.len() {
            match &execute_tokens[i] {
                Token::Number(n) => {
                    let frac = Fraction::from_str(n).map_err(AjisaiError::from)?;
                    self.stack
                        .push_with_role(create_number_value(frac), Interpretation::RawNumber);
                }
                Token::String(s) => {
                    self.stack
                        .push_with_role(Value::from_string(s), Interpretation::Text);
                }
                Token::VectorStart => {
                    let (values, consumed, element_hint) =
                        self.collect_vector(execute_tokens, i)?;
                    if values.is_empty() {
                        return Err(AjisaiError::from(
                            "Empty vector is not allowed. Use NIL for empty values.",
                        ));
                    }
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
                        "STAK" => {
                            self.update_operation_target_mode(OperationTargetMode::Stack);
                        }
                        "TOP" => {
                            self.update_operation_target_mode(OperationTargetMode::StackTop);
                        }
                        "KEEP" => {
                            self.update_consumption_mode(ConsumptionMode::Keep);
                        }
                        "EAT" => {
                            self.update_consumption_mode(ConsumptionMode::Consume);
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
                                    let diagnosis = DebugDiagnosis::from_error(
                                        &err,
                                        Some(upper.as_ref()),
                                        stack_len_before,
                                        self.stack.len(),
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
                                    });
                                    return Err(err);
                                }
                            }
                            if !modules::is_mode_preserving_word(upper.as_ref()) {
                                self.reset_execution_modes();
                            }
                        }
                    }
                }
                Token::BlockEnd => {
                    return Err(AjisaiError::from("Unexpected code block end"));
                }
                Token::Pipeline => {}
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
        self.execution_step_count = 0;
        let tokens: Vec<Token> = crate::tokenizer::tokenize(code)?;
        let lines: Vec<ExecutionLine> = self.split_tokens_to_lines(&tokens)?;
        self.execute_guard_structure(&lines)?;
        Ok(())
    }
}
