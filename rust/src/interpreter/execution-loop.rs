use crate::error::{AjisaiError, Result};
use crate::types::fraction::Fraction;
use crate::types::{DisplayHint, ExecutionLine, Token, Value};

use super::value_extraction_helpers::create_number_value;
use super::{modules, ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::SemanticRegistry;

fn apply_word_hint_override(registry: &mut SemanticRegistry, word: &str) {
    let hint: Option<DisplayHint> = match word {
        "STR" | "CHR" | "CHARS" | "JOIN" => Some(DisplayHint::String),
        "NUM" | "+" | "-" | "*" | "/" | "MOD" | "FLOOR" | "CEIL" | "ROUND" | "FOLD" => {
            Some(DisplayHint::Number)
        }
        "SQRT" | "SQRT_EPS" | "INTERVAL" => Some(DisplayHint::Interval),
        "LOWER" | "UPPER" | "WIDTH" => Some(DisplayHint::Number),
        "BOOL" | "<" | "<=" | "=" | "AND" | "OR" | "NOT" => Some(DisplayHint::Boolean),
        "NOW" | "DATETIME" | "TIMESTAMP" => Some(DisplayHint::DateTime),
        _ => None,
    };
    if let Some(h) = hint {
        let len: usize = registry.len();
        if len > 0 {
            registry.update_hint_at(len - 1, h);
        }
    }
}
impl Interpreter {
    pub(crate) fn collect_vector(
        &mut self,
        tokens: &[Token],
        start_index: usize,
    ) -> Result<(Vec<Value>, usize, DisplayHint)> {
        self.collect_vector_with_depth(tokens, start_index, 1)
    }

    pub(crate) fn collect_vector_with_depth(
        &mut self,
        tokens: &[Token],
        start_index: usize,
        depth: usize,
    ) -> Result<(Vec<Value>, usize, DisplayHint)> {
        if !matches!(&tokens[start_index], Token::VectorStart) {
            return Err(AjisaiError::from("Expected vector start"));
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
                    values.push(Value::from_vector_with_hint(nested_values, nested_hint));
                    has_other = true;
                    i += consumed;
                }
                Token::VectorEnd => {
                    let element_hint: DisplayHint = if has_other {
                        DisplayHint::Auto
                    } else if has_bool && !has_number {
                        DisplayHint::Boolean
                    } else if has_number && !has_bool {
                        DisplayHint::Number
                    } else {
                        DisplayHint::Auto
                    };
                    return Ok((values, i - start_index + 1, element_hint));
                }
                Token::Number(n) => {
                    values.push(Value::from_number(
                        Fraction::parse_unreduced_from_str(n).map_err(AjisaiError::from)?,
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
                                match self.execute_word_core(upper.as_ref()) {
                                    Ok(()) => {
                                        let results =
                                            std::mem::replace(&mut self.stack, stack_backup);
                                        self.output_buffer = output_backup;
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
                    return Err(AjisaiError::from(
                        "Unexpected '$' separator outside COND clause parsing",
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
                    self.semantic_registry.push_hint(DisplayHint::Number);
                }
                Token::String(s) => {
                    self.stack.push(Value::from_string(s));
                    self.semantic_registry.push_hint(DisplayHint::String);
                }
                Token::VectorStart => {
                    let (values, consumed, element_hint) =
                        self.collect_vector(execute_tokens, i)?;
                    if values.is_empty() {
                        return Err(AjisaiError::from(
                            "Empty vector is not allowed. Use NIL for empty values.",
                        ));
                    }
                    self.stack.push(Value::from_vector(values));
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
                    self.semantic_registry.push_hint(DisplayHint::Auto);
                    i = j;
                    continue;
                }
                Token::Symbol(s) => match s.as_ref() {
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
                        if self.safe_mode {
                            let stack_snapshot = self.stack.clone();
                            self.safe_mode = false;
                            match self.execute_word_core(upper.as_ref()) {
                                Ok(()) => {
                                    self.semantic_registry
                                        .normalize_to_stack_len(self.stack.len());
                                    apply_word_hint_override(
                                        &mut self.semantic_registry,
                                        upper.as_ref(),
                                    );
                                }
                                Err(_) => {
                                    self.stack = stack_snapshot;
                                    self.stack.push(Value::nil());
                                    self.semantic_registry
                                        .normalize_to_stack_len(self.stack.len());
                                }
                            }
                        } else {
                            self.execute_word_core(upper.as_ref())?;
                            self.semantic_registry
                                .normalize_to_stack_len(self.stack.len());
                            apply_word_hint_override(&mut self.semantic_registry, upper.as_ref());
                        }
                        if !modules::is_mode_preserving_word(upper.as_ref()) {
                            self.reset_execution_modes();
                        }
                    }
                },
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
                Token::SafeMode => {
                    self.safe_mode = true;
                }
                Token::CondClauseSep => {
                    return Err(AjisaiError::from(
                        "Unexpected '$' separator outside COND clause parsing",
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
