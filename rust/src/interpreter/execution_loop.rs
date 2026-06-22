use crate::error::{AjisaiError, ErrorCategory, NilReason, Result};
use crate::types::fraction::Fraction;
use crate::types::{ExecutionLine, Interpretation, Token, Value};

use super::debug_diagnosis::{DebugDiagnosis, ErrorPhase};
use super::error_flow_trace::{ErrorFlowEvent, ErrorFlowEventKind};
use super::value_extraction_helpers::create_number_value;
use super::{modules, ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::SemanticRegistry;

fn apply_word_hint_override(interp: &mut Interpreter, word: &str) {
    let hint: Option<Interpretation> = match word {
        "STR" | "CHR" | "JOIN" | "TRIM" | "TRIM-LEFT" | "TRIM-RIGHT" | "SUBSTITUTE" => {
            Some(Interpretation::Text)
        }
        "NUM" | "ADD" | "SUB" | "MUL" | "DIV" | "MOD" | "FLOOR" | "CEIL" | "ROUND" | "FOLD" => {
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
        | "REORDER" | "SPLIT" | "COLLECT" | "RESHAPE" | "TRANSPOSE" | "FILL" | "TOKENIZE" => {
            Some(Interpretation::Unassigned)
        }
        _ => None,
    };
    if let Some(h) = hint {
        let registry: &mut SemanticRegistry = &mut interp.semantic_registry;
        let len: usize = registry.len();
        if len > 0 {
            registry.update_hint_at(len - 1, h);
        }
    }
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
                    self.stack.push(create_number_value(frac));
                    self.semantic_registry.push_hint(Interpretation::RawNumber);
                }
                Token::String(s) => {
                    self.stack.push(Value::from_string(s));
                    self.semantic_registry.push_hint(Interpretation::Text);
                }
                Token::VectorStart => {
                    let (values, consumed, element_hint) =
                        self.collect_vector(execute_tokens, i)?;
                    if values.is_empty() {
                        return Err(AjisaiError::from(
                            "Empty vector is not allowed. Use NIL for empty values.",
                        ));
                    }
                    self.stack.push(Value::from_vector_promoted(values));
                    self.semantic_registry.push_hint(element_hint);
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

                    self.stack.push(Value::from_code_block(block_tokens));
                    self.semantic_registry.push_hint(Interpretation::Unassigned);
                    i = j;
                    continue;
                }
                Token::Symbol(s) => {
                    let canonical = crate::core_word_aliases::canonicalize_core_word_name(s);
                    match canonical.as_str() {
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
                            let stack_len_before = self.stack.len();
                            match self.execute_word_core(upper.as_ref()) {
                                Ok(()) => {
                                    trace_direct_nil_produced(
                                        self,
                                        upper.as_ref(),
                                        stack_len_before,
                                    );
                                    self.semantic_registry
                                        .normalize_to_stack_len(self.stack.len());
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
                    let value = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
                    let hint = self.semantic_registry.pop_hint();

                    if !value.is_nil() {
                        self.stack.push(value);
                        self.semantic_registry.push_hint(hint);
                        i += 1;
                        if i < execute_tokens.len() {
                            match &execute_tokens[i] {
                                Token::VectorStart => {
                                    let mut depth = 1;
                                    i += 1;
                                    while i < execute_tokens.len() && depth > 0 {
                                        match &execute_tokens[i] {
                                            Token::VectorStart => depth += 1,
                                            Token::VectorEnd => depth -= 1,
                                            _ => {}
                                        }
                                        i += 1;
                                    }
                                    continue;
                                }
                                Token::BlockStart => {
                                    let mut depth = 1;
                                    i += 1;
                                    while i < execute_tokens.len() && depth > 0 {
                                        match &execute_tokens[i] {
                                            Token::BlockStart => depth += 1,
                                            Token::BlockEnd => depth -= 1,
                                            _ => {}
                                        }
                                        i += 1;
                                    }
                                    continue;
                                }
                                _ => {
                                    i += 1;
                                    continue;
                                }
                            }
                        }
                    }
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
        for line in lines {
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
        self.execution_step_count = 0;
        let tokens: Vec<Token> = crate::tokenizer::tokenize(code)?;
        let lines: Vec<ExecutionLine> = self.split_tokens_to_lines(&tokens)?;
        self.execute_guard_structure(&lines)?;
        Ok(())
    }
}
