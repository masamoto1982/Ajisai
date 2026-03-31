use crate::error::{AjisaiError, Result};
use crate::types::fraction::Fraction;
use crate::types::{ExecutionLine, Token, Value, MAX_VISIBLE_DIMENSIONS};

use super::value_extraction_helpers::{create_number_value, extract_integer_from_value, extract_word_name_from_value};
use super::{modules, AsyncAction, ConsumptionMode, Interpreter, OperationTargetMode};

use async_recursion::async_recursion;
use gloo_timers::future::sleep;
use std::time::Duration;

impl Interpreter {
    pub(crate) fn collect_vector(
        &mut self,
        tokens: &[Token],
        start_index: usize,
    ) -> Result<(Vec<Value>, usize)> {
        self.collect_vector_with_depth(tokens, start_index, 1)
    }

    pub(crate) fn collect_vector_with_depth(
        &mut self,
        tokens: &[Token],
        start_index: usize,
        depth: usize,
    ) -> Result<(Vec<Value>, usize)> {
        if depth > MAX_VISIBLE_DIMENSIONS {
            return Err(AjisaiError::DimensionLimitExceeded { depth });
        }

        if !matches!(&tokens[start_index], Token::VectorStart) {
            return Err(AjisaiError::from("Expected vector start"));
        }

        let mut values = Vec::new();
        let mut i = start_index + 1;

        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorStart => {
                    let (nested_values, consumed) =
                        self.collect_vector_with_depth(tokens, i, depth + 1)?;
                    if nested_values.is_empty() {
                        return Err(AjisaiError::from(
                            "Empty vector is not allowed. Use NIL for empty values.",
                        ));
                    }
                    values.push(Value::from_vector(nested_values));
                    i += consumed;
                }
                Token::VectorEnd => {
                    return Ok((values, i - start_index + 1));
                }
                Token::Number(n) => {
                    values.push(Value::from_number(
                        Fraction::parse_unreduced_from_str(n).map_err(AjisaiError::from)?,
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
                        _ => {
                            let resolved = if let Some(def) = self.resolve_word(upper.as_ref()) {
                                !def.is_builtin
                            } else {
                                false
                            };

                            if resolved {
                                let stack_backup = std::mem::take(&mut self.stack);
                                let output_backup = std::mem::take(&mut self.output_buffer);
                                match self.execute_word_core(upper.as_ref()) {
                                    Ok(()) => {
                                        let results =
                                            std::mem::replace(&mut self.stack, stack_backup);
                                        self.output_buffer = output_backup;
                                        if results.is_empty() {
                                            values.push(Value::from_string(s));
                                        } else {
                                            values.extend(results);
                                        }
                                    }
                                    Err(_) => {
                                        self.stack = stack_backup;
                                        self.output_buffer = output_backup;
                                        values.push(Value::from_string(s));
                                    }
                                }
                            } else {
                                values.push(Value::from_string(s));
                            }
                        }
                    }
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }
        Err(AjisaiError::from("Unclosed vector"))
    }

    pub(crate) fn execute_guard_structure_sync(&mut self, lines: &[ExecutionLine]) -> Result<()> {
        let action = self.execute_guard_structure(lines)?;

        if let Some(async_action) = action {
            return Err(AjisaiError::from(format!(
                "Async operation {:?} requires async context",
                async_action
            )));
        }

        Ok(())
    }

    #[async_recursion(?Send)]
    pub(crate) async fn execute_guard_structure_async(&mut self, lines: &[ExecutionLine]) -> Result<()> {
        match self.execute_guard_structure(lines)? {
            None => Ok(()),
            Some(AsyncAction::Wait {
                duration_ms,
                word_name,
            }) => {
                sleep(Duration::from_millis(duration_ms)).await;
                self.execute_word_async(&word_name).await
            }
        }
    }

    pub(crate) fn build_wait_action(&mut self) -> Result<AsyncAction> {
        if self.stack.len() < 2 {
            return Err(AjisaiError::from(
                "WAIT requires word name and delay. Usage: 'WORD' [ ms ] WAIT",
            ));
        }

        let delay_val = self.stack.pop().unwrap();
        let name_val = self.stack.pop().unwrap();

        let n = extract_integer_from_value(&delay_val)?;
        let duration_ms = if n < 0 {
            return Err(AjisaiError::from("Delay must be non-negative"));
        } else {
            n as u64
        };

        let word_name = extract_word_name_from_value(&name_val)?;

        if let Some(def) = self.resolve_word(&word_name) {
            if def.is_builtin {
                return Err(AjisaiError::from("WAIT can only be used with custom words"));
            }
        } else {
            return Err(AjisaiError::UnknownWord(word_name));
        }

        Ok(AsyncAction::Wait {
            duration_ms,
            word_name,
        })
    }

    pub(crate) fn execute_section_core(
        &mut self,
        tokens: &[Token],
        start_index: usize,
    ) -> Result<(usize, Option<AsyncAction>)> {
        let source_tokens: Vec<Token> = tokens[start_index..].to_vec();
        let mut vector_depth: i32 = 0;
        let mut buffered_tokens: Vec<Token> = Vec::new();
        let mut prelude_tokens: Vec<Token> = Vec::new();
        let mut separated_blocks: Vec<Vec<Token>> = Vec::new();
        let mut trailing_tokens: Vec<Token> = Vec::new();
        let mut has_separator: bool = false;

        for token in source_tokens {
            match token {
                Token::VectorStart => {
                    vector_depth += 1;
                    buffered_tokens.push(Token::VectorStart);
                }
                Token::VectorEnd => {
                    vector_depth -= 1;
                    buffered_tokens.push(Token::VectorEnd);
                }
                Token::BlockSeparator if vector_depth == 0 => {
                    if !has_separator {
                        let maybe_start_index: Option<usize> =
                            buffered_tokens.iter().position(|token| {
                                matches!(
                                    token,
                                    Token::Symbol(symbol)
                                        if symbol.as_ref() == ",,"
                                            || symbol.as_ref() == ","
                                            || symbol.as_ref() == "."
                                            || symbol.as_ref() == ".."
                                )
                            });
                        if let Some(start_index) = maybe_start_index {
                            if start_index > 0 {
                                prelude_tokens.extend_from_slice(&buffered_tokens[..start_index]);
                                buffered_tokens = buffered_tokens[start_index..].to_vec();
                            }
                        }
                    }
                    separated_blocks.push(buffered_tokens.clone());
                    buffered_tokens.clear();
                    has_separator = true;
                }
                _ => buffered_tokens.push(token),
            }
        }

        if has_separator {
            trailing_tokens = buffered_tokens;
        } else {
            trailing_tokens = tokens[start_index..].to_vec();
        }

        if has_separator && !prelude_tokens.is_empty() {
            let (_, action): (usize, Option<AsyncAction>) =
                self.execute_section_core(&prelude_tokens, 0)?;
            if action.is_some() {
                return Ok((start_index, action));
            }
        }

        for block_tokens in separated_blocks {
            self.stack.push(Value::from_code_block(block_tokens));
        }

        let execute_tokens_vec: Vec<Token> = trailing_tokens;

        let mut i: usize = 0;
        let execute_tokens: &[Token] = &execute_tokens_vec;

        while i < execute_tokens.len() {
            match &execute_tokens[i] {
                Token::Number(n) => {
                    let frac = Fraction::from_str(n).map_err(AjisaiError::from)?;
                    self.stack.push(create_number_value(frac));
                }
                Token::String(s) => {
                    self.stack.push(Value::from_string(s));
                }
                Token::VectorStart => {
                    let (values, consumed) = self.collect_vector(execute_tokens, i)?;
                    if values.is_empty() {
                        return Err(AjisaiError::from(
                            "Empty vector is not allowed. Use NIL for empty values.",
                        ));
                    }
                    self.stack.push(Value::from_vector(values));
                    i += consumed;
                    continue;
                }
                Token::Symbol(s) => {
                    match s.as_ref() {
                        ".." => {
                            self.update_operation_target_mode(OperationTargetMode::Stack);
                        }
                        "." => {
                            self.update_operation_target_mode(OperationTargetMode::StackTop);
                        }
                        ",," => {
                            self.update_consumption_mode(ConsumptionMode::Keep);
                        }
                        "," => {
                            self.update_consumption_mode(ConsumptionMode::Consume);
                        }
                        _ => {
                            let upper = Self::normalize_symbol(s);
                            match upper.as_ref() {
                                "WAIT" => {
                                    let action = self.build_wait_action()?;
                                    self.reset_execution_modes();
                                    return Ok((start_index + i + 1, Some(action)));
                                }
                                _ => {
                                    if self.safe_mode {
                                        let stack_snapshot = self.stack.clone();
                                        self.safe_mode = false;
                                        match self.execute_word_core(upper.as_ref()) {
                                            Ok(()) => {}
                                            Err(_) => {
                                                self.stack = stack_snapshot;
                                                self.stack.push(Value::nil());
                                            }
                                        }
                                    } else {
                                        self.execute_word_core(upper.as_ref())?;
                                    }
                                    if !modules::is_mode_preserving_word(upper.as_ref()) {
                                        self.reset_execution_modes();
                                    }
                                }
                            }
                        }
                    }
                }
                Token::BlockSeparator => {}
                Token::Pipeline => {
                    // no-op visual marker
                }
                Token::NilCoalesce => {
                    let value = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

                    if !value.is_nil() {
                        self.stack.push(value);
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
                                _ => {
                                    i += 1;
                                    continue;
                                }
                            }
                        }
                    }
                }
                Token::SafeMode => {
                    self.safe_mode = true;
                }
                Token::LineBreak => {}
                Token::VectorEnd => {
                    return Err(AjisaiError::from("Unexpected vector end"));
                }
            }
            i += 1;
        }

        Ok((start_index + i, None))
    }

    pub(crate) fn execute_guard_structure(
        &mut self,
        lines: &[ExecutionLine],
    ) -> Result<Option<AsyncAction>> {
        if lines.is_empty() {
            return Ok(None);
        }

        for line in lines {
            let (_, action) = self.execute_section_core(&line.body_tokens, 0)?;
            if action.is_some() {
                return Ok(action);
            }
        }
        Ok(None)
    }

    pub(crate) fn check_condition_on_stack(&mut self) -> Result<bool> {
        if self.stack.is_empty() {
            return Ok(false);
        }

        let top = self.stack.pop().unwrap();
        Ok(top.is_truthy())
    }

    pub(crate) fn split_tokens_to_lines(&self, tokens: &[Token]) -> Result<Vec<ExecutionLine>> {
        Ok(vec![ExecutionLine {
            body_tokens: tokens.to_vec().into(),
        }])
    }

    pub async fn execute(&mut self, code: &str) -> Result<()> {
        let tokens = crate::tokenizer::tokenize(code)?;
        let lines = self.split_tokens_to_lines(&tokens)?;
        self.execute_guard_structure_async(&lines).await?;
        Ok(())
    }
}
