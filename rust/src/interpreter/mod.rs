// rust/src/interpreter/mod.rs

pub mod helpers;
pub mod vector_ops;
pub mod tensor_ops;
pub mod arithmetic;
pub mod comparison;
pub mod logic;
pub mod control;
pub mod dictionary;
pub mod io;
pub mod higher_order;
pub mod cast;
pub mod datetime;
pub mod sort;
pub mod random;
pub mod hash;
pub mod audio;
pub mod vector_exec;

use std::collections::{HashMap, HashSet};
use crate::types::{Stack, Token, Value, WordDefinition, ExecutionLine, MAX_VISIBLE_DIMENSIONS};

pub const MAX_CALL_DEPTH: usize = 3;

use crate::types::fraction::Fraction;
use crate::error::{Result, AjisaiError};
use async_recursion::async_recursion;
use self::helpers::wrap_number;
use gloo_timers::future::sleep;
use std::time::Duration;

/// 操作の対象（What）を決定するモード
/// Operation target selector. Resets to StackTop after each word execution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperationTargetMode {
    /// Operate on the top element (default, activated by `.`)
    StackTop,
    /// Operate on the entire stack (activated by `..`)
    Stack,
}

/// 値の扱い（How）を決定するモード
/// Consumption mode selector. Resets to Consume after each word execution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConsumptionMode {
    /// Consume values from stack (default, activated by `,`)
    Consume,
    /// Keep values on stack (activated by `,,`)
    Keep,
}

/// Async action returned when async operation is needed during execution.
#[derive(Debug, Clone)]
pub enum AsyncAction {
    Wait {
        duration_ms: u64,
        word_name: String,
    },
}

pub struct Interpreter {
    pub(crate) stack: Stack,
    pub(crate) dictionary: HashMap<String, WordDefinition>,
    pub(crate) dependents: HashMap<String, HashSet<String>>,
    pub(crate) output_buffer: String,
    pub(crate) definition_to_load: Option<String>,
    /// 操作の対象（What）を決定するモード
    pub(crate) operation_target_mode: OperationTargetMode,
    /// 値の扱い（How）を決定するモード
    pub(crate) consumption_mode: ConsumptionMode,
    pub(crate) force_flag: bool,
    pub(crate) disable_no_change_check: bool,
    pub(crate) pending_tokens: Option<Vec<Token>>,
    pub(crate) pending_token_index: usize,
    pub(crate) play_mode: audio::PlayMode,
    pub(crate) call_stack: Vec<String>,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Interpreter {
            stack: Vec::new(),
            dictionary: HashMap::new(),
            dependents: HashMap::new(),
            output_buffer: String::new(),
            definition_to_load: None,
            operation_target_mode: OperationTargetMode::StackTop,
            consumption_mode: ConsumptionMode::Consume,
            force_flag: false,
            disable_no_change_check: false,
            pending_tokens: None,
            pending_token_index: 0,
            play_mode: audio::PlayMode::default(),
            call_stack: Vec::new(),
        };
        crate::builtins::register_builtins(&mut interpreter.dictionary);
        interpreter
    }

    // ========================================================================
    // モード設定メソッド（Setters）
    // ========================================================================

    /// 操作対象モードを設定する
    pub fn set_operation_target_mode(&mut self, mode: OperationTargetMode) {
        self.operation_target_mode = mode;
    }

    /// 消費モードを設定する
    pub fn set_consumption_mode(&mut self, mode: ConsumptionMode) {
        self.consumption_mode = mode;
    }

    /// 両方のモードをデフォルト（StackTop, Consume）にリセットする
    pub fn reset_modes(&mut self) {
        self.operation_target_mode = OperationTargetMode::StackTop;
        self.consumption_mode = ConsumptionMode::Consume;
    }

    fn collect_vector(&self, tokens: &[Token], start_index: usize) -> Result<(Vec<Value>, usize)> {
        self.collect_vector_with_depth(tokens, start_index, 1)
    }

    /// Collect tokens between : and ; to form a code block
    /// Returns (code_string, consumed_count)
    fn collect_code_block(&self, tokens: &[Token], start_index: usize) -> Result<(String, usize)> {
        if !matches!(&tokens[start_index], Token::CodeBlockStart) {
            return Err(AjisaiError::from("Expected code block start (:)"));
        }

        let mut code_parts = Vec::new();
        let mut i = start_index + 1;
        let mut depth = 1; // Track nested code blocks

        while i < tokens.len() {
            match &tokens[i] {
                Token::CodeBlockStart => {
                    depth += 1;
                    code_parts.push(":".to_string());
                },
                Token::CodeBlockEnd => {
                    depth -= 1;
                    if depth == 0 {
                        // End of code block
                        let code = code_parts.join(" ");
                        return Ok((code, i - start_index + 1));
                    }
                    code_parts.push(";".to_string());
                },
                Token::Number(n) => code_parts.push(n.clone()),
                Token::String(s) => code_parts.push(format!("'{}'", s)),
                Token::Symbol(s) => code_parts.push(s.clone()),
                Token::VectorStart => code_parts.push("[".to_string()),
                Token::VectorEnd => code_parts.push("]".to_string()),
                Token::ChevronBranch => code_parts.push(">>".to_string()),
                Token::ChevronDefault => code_parts.push(">>>".to_string()),
                Token::LineBreak => code_parts.push("\n".to_string()),
                Token::Pipeline => code_parts.push("==".to_string()),
                Token::NilCoalesce => code_parts.push("=>".to_string()),
            }
            i += 1;
        }
        Err(AjisaiError::from("Unclosed code block (missing ;)"))
    }

    fn collect_vector_with_depth(&self, tokens: &[Token], start_index: usize, depth: usize) -> Result<(Vec<Value>, usize)> {
        if depth > MAX_VISIBLE_DIMENSIONS {
            return Err(AjisaiError::from(format!(
                "Dimension limit exceeded: Ajisai supports up to 3 visible dimensions (plus dimension 0: the stack). Nesting depth {} exceeds the limit.",
                depth
            )));
        }

        if !matches!(&tokens[start_index], Token::VectorStart) {
            return Err(AjisaiError::from("Expected vector start"));
        }

        let mut values = Vec::new();
        let mut i = start_index + 1;

        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorStart => {
                    let (nested_values, consumed) = self.collect_vector_with_depth(tokens, i, depth + 1)?;
                    // 空のベクターは許容しない
                    if nested_values.is_empty() {
                        return Err(AjisaiError::from("Empty vector is not allowed. Use NIL for empty values."));
                    }
                    values.push(Value::from_vector(nested_values));
                    i += consumed;
                },
                Token::VectorEnd => {
                    return Ok((values, i - start_index + 1));
                },
                Token::Number(n) => {
                    values.push(Value::from_number(Fraction::from_str(n).map_err(AjisaiError::from)?));
                    i += 1;
                },
                Token::String(s) => {
                    values.push(Value::from_string(s));
                    i += 1;
                },
                Token::Symbol(s) => {
                    // TRUE/FALSE/NILは特別な値として処理
                    let upper = s.to_uppercase();
                    match upper.as_str() {
                        "TRUE" => values.push(Value::from_bool(true)),
                        "FALSE" => values.push(Value::from_bool(false)),
                        "NIL" => values.push(Value::nil()),
                        _ => values.push(Value::from_string(s)),
                    }
                    i += 1;
                },
                _ => {
                    i += 1;
                }
            }
        }
        Err(AjisaiError::from("Unclosed vector"))
    }

    pub(crate) fn execute_guard_structure_sync(
        &mut self,
        lines: &[ExecutionLine]
    ) -> Result<()> {
        let action = self.execute_guard_structure_core(lines)?;

        if let Some(async_action) = action {
            return Err(AjisaiError::from(format!(
                "Async operation {:?} requires async context",
                async_action
            )));
        }

        Ok(())
    }

    #[async_recursion(?Send)]
    pub(crate) async fn execute_guard_structure(&mut self, lines: &[ExecutionLine]) -> Result<()> {
        if lines.is_empty() {
            return Ok(());
        }

        // シェブロン分岐かどうかをチェック
        let is_chevron_structure = lines.iter().all(|line| {
            matches!(
                line.body_tokens.first(),
                Some(Token::ChevronBranch) | Some(Token::ChevronDefault)
            )
        });

        if !is_chevron_structure {
            // 通常の逐次実行
            for line in lines {
                self.execute_section(&line.body_tokens).await?;
            }
            return Ok(());
        }

        // シェブロン分岐の処理
        // 最後の行が >>> であることを検証
        let last_line = lines.last().unwrap();
        if last_line.body_tokens.first() != Some(&Token::ChevronDefault) {
            return Err(AjisaiError::from(
                "Chevron branch must end with >>> (default branch)"
            ));
        }

        let mut i = 0;
        while i < lines.len() - 1 {  // 最後の行（デフォルト）は別処理
            let line = &lines[i];

            if line.body_tokens.first() != Some(&Token::ChevronBranch) {
                return Err(AjisaiError::from("Expected >> at line start"));
            }

            let content_tokens = &line.body_tokens[1..];

            if i + 1 < lines.len() - 1 {
                // 条件行
                self.execute_section(content_tokens).await?;

                if self.is_condition_true()? {
                    // 次の行はアクション行
                    i += 1;
                    let action_line = &lines[i];
                    if action_line.body_tokens.first() != Some(&Token::ChevronBranch) {
                        return Err(AjisaiError::from("Expected >> for action line"));
                    }
                    let action_tokens = &action_line.body_tokens[1..];
                    self.execute_section(action_tokens).await?;
                    return Ok(());
                }
                i += 2;
            } else {
                // 最後の条件行の後にデフォルトがある
                self.execute_section(content_tokens).await?;

                if self.is_condition_true()? {
                    // デフォルト行を実行
                    let default_tokens = &lines[lines.len() - 1].body_tokens[1..];
                    self.execute_section(default_tokens).await?;
                    return Ok(());
                }
                i += 1;
            }
        }

        // デフォルト行を実行
        let default_line = &lines[lines.len() - 1];
        let default_tokens = &default_line.body_tokens[1..];
        self.execute_section(default_tokens).await?;
        Ok(())
    }

    fn prepare_wait_action(&mut self) -> Result<AsyncAction> {
        if self.stack.len() < 2 {
            return Err(AjisaiError::from(
                "WAIT requires word name and delay. Usage: 'WORD' [ ms ] WAIT"
            ));
        }

        let delay_val = self.stack.pop().unwrap();
        let name_val = self.stack.pop().unwrap();

        let n = helpers::get_integer_from_value(&delay_val)?;
        let duration_ms = if n < 0 {
            return Err(AjisaiError::from("Delay must be non-negative"));
        } else {
            n as u64
        };

        let word_name = helpers::get_word_name_from_value(&name_val)?;

        if let Some(def) = self.dictionary.get(&word_name) {
            if def.is_builtin {
                return Err(AjisaiError::from(
                    "WAIT can only be used with custom words"
                ));
            }
        } else {
            return Err(AjisaiError::UnknownWord(word_name));
        }

        Ok(AsyncAction::Wait { duration_ms, word_name })
    }

    /// Returns (next_index, Option<AsyncAction>).
    pub(crate) fn execute_section_core(
        &mut self,
        tokens: &[Token],
        start_index: usize
    ) -> Result<(usize, Option<AsyncAction>)> {
        let mut i = start_index;

        while i < tokens.len() {
            match &tokens[i] {
                Token::Number(n) => {
                    let frac = Fraction::from_str(n).map_err(AjisaiError::from)?;
                    self.stack.push(wrap_number(frac));
                },
                Token::String(s) => {
                    self.stack.push(Value::from_string(s));
                },
                // TRUE/FALSE/NIL are recognized as Symbols and executed as builtin words
                Token::VectorStart => {
                    let (values, consumed) = self.collect_vector(tokens, i)?;
                    // 空のベクターは許容しない
                    if values.is_empty() {
                        return Err(AjisaiError::from("Empty vector is not allowed. Use NIL for empty values."));
                    }
                    self.stack.push(Value::from_vector(values));
                    i += consumed;
                    continue;
                },
                Token::Symbol(s) => {
                    match s.as_str() {
                        // ターゲット修飾子
                        ".." => {
                            self.set_operation_target_mode(OperationTargetMode::Stack);
                        },
                        "." => {
                            self.set_operation_target_mode(OperationTargetMode::StackTop);
                        },
                        // 消費修飾子
                        ",," => {
                            self.set_consumption_mode(ConsumptionMode::Keep);
                        },
                        "," => {
                            self.set_consumption_mode(ConsumptionMode::Consume);
                        },
                        _ => {
                            let upper = s.to_uppercase();
                            match upper.as_str() {
                                "WAIT" => {
                                    let action = self.prepare_wait_action()?;
                                    return Ok((i + 1, Some(action)));
                                },
                                _ => {
                                    self.execute_word_core(&upper)?;
                                    // SEQ/SIM preserve modes for PLAY
                                    if upper != "SEQ" && upper != "SIM" {
                                        self.reset_modes();
                                    }
                                }
                            }
                        }
                    }
                },
                Token::CodeBlockStart => {
                    // コードブロックのトークン列を収集してスタックにプッシュ
                    let mut code_tokens = Vec::new();
                    let mut depth = 1;
                    i += 1;

                    while i < tokens.len() && depth > 0 {
                        match &tokens[i] {
                            Token::CodeBlockStart => {
                                depth += 1;
                                code_tokens.push(tokens[i].clone());
                            }
                            Token::CodeBlockEnd => {
                                depth -= 1;
                                if depth > 0 {
                                    code_tokens.push(tokens[i].clone());
                                }
                            }
                            _ => {
                                code_tokens.push(tokens[i].clone());
                            }
                        }
                        i += 1;
                    }

                    if depth != 0 {
                        return Err(AjisaiError::from("Unclosed code block: missing ';'"));
                    }

                    // コードブロック値としてスタックにプッシュ
                    self.stack.push(Value::from_code_block(code_tokens));
                    continue;  // i は既に進んでいるので、最後の i += 1 をスキップ
                }
                Token::Pipeline => {
                    // パイプライン演算子は視覚的マーカーのみ（no-op）
                    // 何もせず次のトークンへ進む
                }
                Token::NilCoalesce => {
                    // Nil Coalescing: value => default
                    // valueがNILならdefaultを、そうでなければvalueを返す
                    let default_val = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
                    let value = self.stack.pop().ok_or_else(|| {
                        // スタックアンダーフローの場合、defaultをプッシュバック
                        self.stack.push(default_val.clone());
                        AjisaiError::StackUnderflow
                    })?;

                    if value.is_nil() {
                        self.stack.push(default_val);
                    } else {
                        self.stack.push(value);
                    }
                }
                Token::CodeBlockEnd | Token::ChevronBranch | Token::ChevronDefault | Token::LineBreak => {},
                Token::VectorEnd => {
                    return Err(AjisaiError::from("Unexpected vector end"));
                },
            }
            i += 1;
        }

        Ok((i, None))
    }

    #[async_recursion(?Send)]
    async fn execute_section(&mut self, tokens: &[Token]) -> Result<()> {
        let mut current_index = 0;

        loop {
            let (next_index, action) = self.execute_section_core(tokens, current_index)?;

            match action {
                None => break,
                Some(AsyncAction::Wait { duration_ms, word_name }) => {
                    sleep(Duration::from_millis(duration_ms)).await;
                    self.execute_word_async(&word_name).await?;
                    current_index = next_index;
                }
            }
        }

        Ok(())
    }

    fn execute_guard_structure_core(
        &mut self,
        lines: &[ExecutionLine]
    ) -> Result<Option<AsyncAction>> {
        if lines.is_empty() {
            return Ok(None);
        }

        // シェブロン分岐かどうかをチェック
        let is_chevron_structure = lines.iter().all(|line| {
            matches!(
                line.body_tokens.first(),
                Some(Token::ChevronBranch) | Some(Token::ChevronDefault)
            )
        });

        if !is_chevron_structure {
            // 通常の逐次実行
            for line in lines {
                let (_, action) = self.execute_section_core(&line.body_tokens, 0)?;
                if action.is_some() {
                    return Ok(action);
                }
            }
            return Ok(None);
        }

        // シェブロン分岐の処理
        // 最後の行が >>> であることを検証
        let last_line = lines.last().unwrap();
        if last_line.body_tokens.first() != Some(&Token::ChevronDefault) {
            return Err(AjisaiError::from(
                "Chevron branch must end with >>> (default branch)"
            ));
        }

        let mut i = 0;
        while i < lines.len() - 1 {  // 最後の行（デフォルト）は別処理
            let line = &lines[i];

            if line.body_tokens.first() != Some(&Token::ChevronBranch) {
                return Err(AjisaiError::from("Expected >> at line start"));
            }

            let content_tokens = &line.body_tokens[1..];

            if i + 1 < lines.len() - 1 {
                // 条件行
                let (_, action) = self.execute_section_core(content_tokens, 0)?;
                if action.is_some() {
                    return Ok(action);
                }

                if self.is_condition_true()? {
                    // 次の行はアクション行
                    i += 1;
                    let action_line = &lines[i];
                    if action_line.body_tokens.first() != Some(&Token::ChevronBranch) {
                        return Err(AjisaiError::from("Expected >> for action line"));
                    }
                    let action_tokens = &action_line.body_tokens[1..];
                    let (_, action) = self.execute_section_core(action_tokens, 0)?;
                    return Ok(action);
                }
                i += 2;
            } else {
                // 最後の条件行の後にデフォルトがある
                let (_, action) = self.execute_section_core(content_tokens, 0)?;
                if action.is_some() {
                    return Ok(action);
                }

                if self.is_condition_true()? {
                    // デフォルト行を実行
                    let default_tokens = &lines[lines.len() - 1].body_tokens[1..];
                    let (_, action) = self.execute_section_core(default_tokens, 0)?;
                    return Ok(action);
                }
                i += 1;
            }
        }

        // デフォルト行を実行
        let default_line = &lines[lines.len() - 1];
        let default_tokens = &default_line.body_tokens[1..];
        let (_, action) = self.execute_section_core(default_tokens, 0)?;
        Ok(action)
    }

    /// NIL and all-zero vectors are falsy; everything else is truthy.
    fn is_condition_true(&mut self) -> Result<bool> {
        if self.stack.is_empty() {
            return Ok(false);
        }

        let top = self.stack.pop().unwrap();
        Ok(top.is_truthy())
    }

    fn tokens_to_lines(&self, tokens: &[Token]) -> Result<Vec<ExecutionLine>> {
        let mut lines = Vec::new();
        let mut current_line = Vec::new();
        let mut i = 0;

        while i < tokens.len() {
            match &tokens[i] {
                Token::LineBreak => {
                    if !current_line.is_empty() {
                        lines.push(ExecutionLine {
                            body_tokens: current_line.clone(),
                        });
                        current_line.clear();
                    }
                    i += 1;
                },
                Token::CodeBlockStart => {
                    // Preserve code block tokens for execute_section_core to handle
                    // This allows the code block to be pushed as a CodeBlock value
                    current_line.push(Token::CodeBlockStart);
                    i += 1;
                    let mut depth = 1;
                    while i < tokens.len() && depth > 0 {
                        match &tokens[i] {
                            Token::CodeBlockStart => {
                                depth += 1;
                                current_line.push(tokens[i].clone());
                            }
                            Token::CodeBlockEnd => {
                                depth -= 1;
                                current_line.push(tokens[i].clone());
                            }
                            _ => {
                                // Preserve all tokens including LineBreak inside code blocks
                                // (LineBreak is needed for chevron branching inside DEF)
                                current_line.push(tokens[i].clone());
                            }
                        }
                        i += 1;
                    }
                },
                _ => {
                    current_line.push(tokens[i].clone());
                    i += 1;
                }
            }
        }

        if !current_line.is_empty() {
            lines.push(ExecutionLine {
                body_tokens: current_line,
            });
        }

        Ok(lines)
    }

    pub(crate) fn execute_word_core(&mut self, name: &str) -> Result<()> {
        let def = self.dictionary.get(name).cloned()
            .ok_or_else(|| AjisaiError::UnknownWord(name.to_string()))?;

        if def.is_builtin {
            return self.execute_builtin(name);
        }

        if self.call_stack.len() >= MAX_CALL_DEPTH {
            let stack_trace = self.call_stack.join(" -> ");
            return Err(AjisaiError::from(format!(
                "Call depth limit ({}) exceeded: {} -> {}",
                MAX_CALL_DEPTH, stack_trace, name
            )));
        }

        self.call_stack.push(name.to_string());

        let action = self.execute_guard_structure_core(&def.lines);

        self.call_stack.pop();

        let action = action?;

        if action.is_some() {
            return Err(AjisaiError::from(
                "WAIT requires async execution context. Use execute() instead of execute_sync()."
            ));
        }

        Ok(())
    }

    #[async_recursion(?Send)]
    pub(crate) async fn execute_word_async(&mut self, name: &str) -> Result<()> {
        let def = self.dictionary.get(name).cloned()
            .ok_or_else(|| AjisaiError::UnknownWord(name.to_string()))?;

        if def.is_builtin {
            return self.execute_builtin(name);
        }

        if self.call_stack.len() >= MAX_CALL_DEPTH {
            let stack_trace = self.call_stack.join(" -> ");
            return Err(AjisaiError::from(format!(
                "Call depth limit ({}) exceeded: {} -> {}",
                MAX_CALL_DEPTH, stack_trace, name
            )));
        }

        self.call_stack.push(name.to_string());
        let result = self.execute_guard_structure(&def.lines).await;
        self.call_stack.pop();

        result
    }

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        if name != "DEL" && name != "DEF" && name != "!" {
            self.force_flag = false;
        }

        match name {
            "GET" => vector_ops::op_get(self),
            "INSERT" => vector_ops::op_insert(self),
            "REPLACE" => vector_ops::op_replace(self),
            "REMOVE" => vector_ops::op_remove(self),
            "LENGTH" => vector_ops::op_length(self),
            "TAKE" => vector_ops::op_take(self),
            "SPLIT" => vector_ops::op_split(self),
            "CONCAT" => vector_ops::op_concat(self),
            "REVERSE" => vector_ops::op_reverse(self),
            "RANGE" => vector_ops::op_range(self),
            "REORDER" => vector_ops::op_reorder(self),
            "COLLECT" => vector_ops::op_collect(self),
            "SORT" => sort::op_sort(self),
            "FLOOR" => tensor_ops::op_floor(self),
            "CEIL" => tensor_ops::op_ceil(self),
            "ROUND" => tensor_ops::op_round(self),
            "MOD" => tensor_ops::op_mod(self),
            "+" => arithmetic::op_add(self),
            "-" => arithmetic::op_sub(self),
            "*" => arithmetic::op_mul(self),
            "/" => arithmetic::op_div(self),
            "=" => comparison::op_eq(self),
            "<" => comparison::op_lt(self),
            "<=" => comparison::op_le(self),
            "AND" => logic::op_and(self),
            "OR" => logic::op_or(self),
            "NOT" => logic::op_not(self),
            "PRINT" => io::op_print(self),
            "DEF" => dictionary::op_def(self),
            "DEL" => dictionary::op_del(self),
            "?" => dictionary::op_lookup(self),
            "RESET" => self.execute_reset(),
            "MAP" => higher_order::op_map(self),
            "FILTER" => higher_order::op_filter(self),
            "FOLD" => higher_order::op_fold(self),
            "TIMES" => control::execute_times(self),
            "EXEC" => control::op_exec(self),
            "EVAL" => control::op_eval(self),
            "WAIT" => {
                Err(AjisaiError::from(
                    "WAIT should be handled by execute_section_core, not execute_builtin"
                ))
            },
            "TRUE" => {
                self.stack.push(Value::from_bool(true));
                Ok(())
            },
            "FALSE" => {
                self.stack.push(Value::from_bool(false));
                Ok(())
            },
            "NIL" => {
                self.stack.push(Value::nil());
                Ok(())
            },
            "STR" => cast::op_str(self),
            "NUM" => cast::op_num(self),
            "BOOL" => cast::op_bool(self),
            "CHR" => cast::op_chr(self),
            "CHARS" => cast::op_chars(self),
            "JOIN" => cast::op_join(self),
            "NOW" => datetime::op_now(self),
            "DATETIME" => datetime::op_datetime(self),
            "TIMESTAMP" => datetime::op_timestamp(self),
            "CSPRNG" => random::op_csprng(self),
            "HASH" => hash::op_hash(self),
            "SEQ" => audio::op_seq(self),
            "SIM" => audio::op_sim(self),
            "SLOT" => audio::op_slot(self),
            "GAIN" => audio::op_gain(self),
            "GAIN-RESET" => audio::op_gain_reset(self),
            "PAN" => audio::op_pan(self),
            "PAN-RESET" => audio::op_pan_reset(self),
            "FX-RESET" => audio::op_fx_reset(self),
            "PLAY" => audio::op_play(self),
            "CHORD" => audio::op_chord(self),
            "ADSR" => audio::op_adsr(self),
            "SINE" => audio::op_sine(self),
            "SQUARE" => audio::op_square(self),
            "SAW" => audio::op_saw(self),
            "TRI" => audio::op_tri(self),
            "!" => {
                self.force_flag = true;
                Ok(())
            },
            _ => Err(AjisaiError::UnknownWord(name.to_string())),
        }
    }

    pub fn token_to_string(&self, token: &Token) -> String {
        match token {
            Token::Number(n) => n.clone(),
            Token::String(s) => format!("'{}'", s),
            Token::Symbol(s) => s.clone(),
            Token::VectorStart => "[".to_string(),
            Token::VectorEnd => "]".to_string(),
            Token::CodeBlockStart => ":".to_string(),
            Token::CodeBlockEnd => ";".to_string(),
            Token::ChevronBranch => ">>".to_string(),
            Token::ChevronDefault => ">>>".to_string(),
            Token::Pipeline => "==".to_string(),
            Token::NilCoalesce => "=>".to_string(),
            Token::LineBreak => "\n".to_string(),
        }
    }
    
    pub fn get_word_definition_tokens(&self, name: &str) -> Option<String> {
        if let Some(def) = self.dictionary.get(name) {
            if !def.is_builtin && !def.lines.is_empty() {
                let mut result = String::new();
                for (i, line) in def.lines.iter().enumerate() {
                    if i > 0 { result.push('\n'); }
                    
                    for token in &line.body_tokens {
                        result.push_str(&self.token_to_string(token));
                        result.push(' ');
                    }
                }
                return Some(result.trim().to_string());
            }
        }
        None
    }
    
    pub fn execute_reset(&mut self) -> Result<()> {
        self.stack.clear();
        self.dictionary.clear();
        self.dependents.clear();
        self.output_buffer.clear();
        self.definition_to_load = None;
        self.reset_modes();
        self.force_flag = false;
        self.pending_tokens = None;
        self.pending_token_index = 0;
        self.play_mode = audio::PlayMode::default();
        self.call_stack.clear();
        crate::builtins::register_builtins(&mut self.dictionary);
        Ok(())
    }

    pub async fn execute(&mut self, code: &str) -> Result<()> {
        let custom_word_names: HashSet<String> = self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect();
        let tokens = crate::tokenizer::tokenize_with_custom_words(code, &custom_word_names)?;
        
        // トークンを行に分割
        let lines = self.tokens_to_lines(&tokens)?;

        // 行の配列としてガード構造を処理
        self.execute_guard_structure(&lines).await?;

        Ok(())
    }

    pub fn get_output(&mut self) -> String { 
        std::mem::take(&mut self.output_buffer) 
    }
    
    pub fn get_stack(&self) -> &Stack { 
        &self.stack 
    }
    
    pub fn set_stack(&mut self, stack: Stack) { 
        self.stack = stack; 
    }

    pub fn rebuild_dependencies(&mut self) -> Result<()> {
        self.dependents.clear();
        let custom_words: Vec<(String, WordDefinition)> = self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, def)| (name.clone(), def.clone()))
            .collect();

        for (word_name, word_def) in custom_words {
            let mut dependencies = HashSet::new();
            for line in &word_def.lines {
                for token in line.body_tokens.iter() {
                    if let Token::Symbol(s) = token {
                        let upper_s = s.to_uppercase();
                        if self.dictionary.contains_key(&upper_s) && !self.dictionary.get(&upper_s).unwrap().is_builtin {
                            dependencies.insert(upper_s.clone());
                            self.dependents.entry(upper_s).or_default().insert(word_name.clone());
                        }
                    }
                }
            }
            if let Some(def) = self.dictionary.get_mut(&word_name) {
                def.dependencies = dependencies;
            }
        }
        Ok(())
    }

    /// 指定されたワードを参照している他のワードの集合を取得
    pub fn get_dependents(&self, word_name: &str) -> HashSet<String> {
        let mut result = HashSet::new();
        for (name, def) in &self.dictionary {
            if !def.is_builtin && def.dependencies.contains(word_name) {
                result.insert(name.clone());
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ValueData;

    #[tokio::test]
    async fn test_stack_get_basic() {
        let mut interp = Interpreter::new();

        // Test basic .. GET behavior
        let code = "[5] [0] .. GET";

        println!("\n=== Basic .. GET Test ===");
        let result = interp.execute(code).await;
        println!("Result: {:?}", result);
        println!("Final stack length: {}", interp.stack.len());
        println!("Final stack contents:");
        for (i, val) in interp.stack.iter().enumerate() {
            println!("  [{}]: {:?}", i, val);
        }

        assert!(result.is_ok());
        // スタックには [5] と [5] が含まれるはず (元の値と取得した値)
        assert_eq!(interp.stack.len(), 2);
    }

    #[tokio::test]
    async fn test_stack_get_with_guard_and_comparison() {
        let mut interp = Interpreter::new();

        // Test .. GET with comparison
        // スタックモードでGETした値を比較できることを確認
        // 修正後: [1] .. GET は [20] を返す（二重ラップなし）
        // 修正前: [1] .. GET は [[20]] を返していた（二重ラップ）
        let code = "[10] [20] [30] [1] .. GET [20] =";

        println!("\n=== Stack GET with Comparison Test ===");
        let result = interp.execute(code).await;
        println!("Result: {:?}", result);
        println!("Final stack length: {}", interp.stack.len());
        println!("Final stack contents:");
        for (i, val) in interp.stack.iter().enumerate() {
            println!("  [{}]: {:?}", i, val);
        }

        assert!(result.is_ok());
        // スタックには [10], [20], [30], [TRUE] が含まれるはず
        // [1] .. GET は [20] を取得してプッシュ、[20] = で比較して TRUE
        assert_eq!(interp.stack.len(), 4);
        // 最後の値が TRUE であることを確認
        let val = &interp.stack[3];
        assert_eq!(val.len(), 1, "Expected single element");
        assert!(!val.as_scalar().expect("Expected scalar").is_zero(), "Expected TRUE from comparison");
    }

    #[tokio::test]
    async fn test_simple_addition() {
        let mut interp = Interpreter::new();

        // Simple test: add two numbers
        let code = "[2] [3] +";

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Simple addition should succeed: {:?}", result);

        // Verify result
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
    }

    #[tokio::test]
    async fn test_definition_and_call() {
        let mut interp = Interpreter::new();

        // Test defining a word and calling it (new syntax: : CODE ; 'NAME' DEF)
        let code = r#"
: [2] [3] + ; 'ADDTEST' DEF
ADDTEST
"#;

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Definition and call should succeed: {:?}", result);

        // Verify call stack is empty
    }

    #[tokio::test]
    async fn test_force_flag_del_without_dependents() {
        let mut interp = Interpreter::new();
        interp.execute(": [ 2 ] * ; 'DOUBLE' DEF").await.unwrap();

        // 依存なしなら ! 不要で削除可能
        let result = interp.execute("'DOUBLE' DEL").await;
        assert!(result.is_ok());
        assert!(!interp.dictionary.contains_key("DOUBLE"));
    }

    #[tokio::test]
    async fn test_force_flag_del_with_dependents_error() {
        let mut interp = Interpreter::new();
        interp.execute(": [ 2 ] * ; 'DOUBLE' DEF").await.unwrap();
        interp.execute(": DOUBLE DOUBLE ; 'QUAD' DEF").await.unwrap();

        // 依存ありで ! なしはエラー
        let result = interp.execute("'DOUBLE' DEL").await;
        assert!(result.is_err());
        assert!(interp.dictionary.contains_key("DOUBLE"));
    }

    #[tokio::test]
    async fn test_force_flag_del_with_dependents_forced() {
        let mut interp = Interpreter::new();
        interp.execute(": [ 2 ] * ; 'DOUBLE' DEF").await.unwrap();
        interp.execute(": DOUBLE DOUBLE ; 'QUAD' DEF").await.unwrap();

        // ! 付きなら削除可能
        let result = interp.execute("! 'DOUBLE' DEL").await;
        assert!(result.is_ok());
        assert!(!interp.dictionary.contains_key("DOUBLE"));
        assert!(interp.output_buffer.contains("Warning"));
    }

    #[tokio::test]
    async fn test_force_flag_def_with_dependents_error() {
        let mut interp = Interpreter::new();
        interp.execute(": [ 2 ] * ; 'DOUBLE' DEF").await.unwrap();
        interp.execute(": DOUBLE DOUBLE ; 'QUAD' DEF").await.unwrap();

        // 依存ありで ! なしの再定義はエラー
        let result = interp.execute(": [ 3 ] * ; 'DOUBLE' DEF").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_force_flag_def_with_dependents_forced() {
        let mut interp = Interpreter::new();
        interp.execute(": [ 2 ] * ; 'DOUBLE' DEF").await.unwrap();
        interp.execute(": DOUBLE DOUBLE ; 'QUAD' DEF").await.unwrap();

        // ! 付きなら再定義可能
        let result = interp.execute("! : [ 3 ] * ; 'DOUBLE' DEF").await;
        assert!(result.is_ok());
        assert!(interp.output_buffer.contains("Warning"));
    }

    #[tokio::test]
    async fn test_force_flag_builtin_always_error() {
        let mut interp = Interpreter::new();

        // 組み込みは ! があっても削除不可
        let result = interp.execute("! '+' DEL").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_force_flag_reset_after_other_word() {
        let mut interp = Interpreter::new();
        interp.execute(": [ 2 ] * ; 'DOUBLE' DEF").await.unwrap();
        interp.execute(": DOUBLE DOUBLE ; 'QUAD' DEF").await.unwrap();

        // ! の後に別のワードを実行するとフラグがリセットされる
        interp.execute("!").await.unwrap();
        interp.execute("[ 1 2 ] LENGTH").await.unwrap();  // 何か別のワード操作
        let result = interp.execute("'DOUBLE' DEL").await;
        assert!(result.is_err());  // フラグがリセットされているのでエラー
    }

    #[tokio::test]
    async fn test_chevron_with_def_true_case() {
        let mut interp = Interpreter::new();

        // Test: Define a custom word inside chevron branch (true case)
        // [ 3 ] [ 5 ] < is true, so ANSWER should be defined
        let code = r#"
>> [ 3 ] [ 5 ] <
>> : [ 42 ] ; 'ANSWER' DEF
>>> : [ 0 ] ; 'ZERO' DEF
"#;

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Chevron with DEF should succeed: {:?}", result);

        // Verify ANSWER is defined
        assert!(interp.dictionary.contains_key("ANSWER"), "ANSWER should be defined");
        assert!(!interp.dictionary.contains_key("ZERO"), "ZERO should not be defined");

        // Debug: Print ANSWER definition
        if let Some(def) = interp.dictionary.get("ANSWER") {
            println!("ANSWER definition has {} lines", def.lines.len());
            for (i, line) in def.lines.iter().enumerate() {
                println!("Line {}: {} tokens", i, line.body_tokens.len());
                for token in &line.body_tokens {
                    println!("  Token: {}", interp.token_to_string(token));
                }
            }
        }

        // Call ANSWER and verify result
        let call_code = "ANSWER";
        let call_result = interp.execute(call_code).await;
        if let Err(ref e) = call_result {
            println!("Error calling ANSWER: {:?}", e);
        }
        assert!(call_result.is_ok(), "Calling ANSWER should succeed");
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
    }

    #[tokio::test]
    async fn test_chevron_with_def_false_case() {
        let mut interp = Interpreter::new();

        // Test: Define a custom word inside chevron branch (false case)
        // [ 5 ] [ 3 ] < is false, so SMALL should be defined (default)
        let code = r#"
>> [ 5 ] [ 3 ] <
>> : [ 100 ] ; 'BIG' DEF
>>> : [ -1 ] ; 'SMALL' DEF
"#;

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Chevron with DEF (false case) should succeed: {:?}", result);

        // Verify SMALL is defined
        assert!(!interp.dictionary.contains_key("BIG"), "BIG should not be defined");
        assert!(interp.dictionary.contains_key("SMALL"), "SMALL should be defined");

        // Call SMALL and verify result
        let call_code = "SMALL";
        let call_result = interp.execute(call_code).await;
        assert!(call_result.is_ok(), "Calling SMALL should succeed: {:?}", call_result.err());
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
    }

    #[tokio::test]
    async fn test_chevron_default_clause_with_def() {
        let mut interp = Interpreter::new();

        // Test: Default clause in chevron structure defines a word
        let code = r#"
>> FALSE
>> : [ 100 ] ; 'HUNDRED' DEF
>>> : [ 999 ] ; 'DEFAULT' DEF
"#;

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Chevron default clause with DEF should succeed: {:?}", result);

        // Verify DEFAULT is defined
        assert!(!interp.dictionary.contains_key("HUNDRED"), "HUNDRED should not be defined");
        assert!(interp.dictionary.contains_key("DEFAULT"), "DEFAULT should be defined");

        // Call DEFAULT and verify result
        let call_code = "DEFAULT";
        let call_result = interp.execute(call_code).await;
        assert!(call_result.is_ok(), "Calling DEFAULT should succeed: {:?}", call_result.err());
    }

    #[tokio::test]
    async fn test_def_with_chevron_using_existing_custom_word() {
        let mut interp = Interpreter::new();

        // Test: Define a word inside chevron that uses an existing custom word
        let def_code = ": [ 2 ] * ; 'DOUBLE' DEF";
        let result = interp.execute(def_code).await;
        assert!(result.is_ok(), "DOUBLE definition should succeed: {:?}", result);

        let chevron_code = r#"
>> [ 5 ] [ 10 ] <
>> : [ 3 ] DOUBLE ; 'PROCESS' DEF
>>> : [ 0 ] ; 'NOPROCESS' DEF
"#;
        let result = interp.execute(chevron_code).await;
        assert!(result.is_ok(), "DEF with chevron using existing word should succeed: {:?}", result);

        // Verify PROCESS is defined
        assert!(interp.dictionary.contains_key("PROCESS"), "PROCESS should be defined");
        assert!(interp.dictionary.contains_key("DOUBLE"), "DOUBLE should exist");

        // Call PROCESS and verify result
        let call_code = "PROCESS";
        let call_result = interp.execute(call_code).await;
        assert!(call_result.is_ok(), "Calling PROCESS should succeed: {:?}", call_result.err());
    }

    #[tokio::test]
    async fn test_default_line_without_colon() {
        let mut interp = Interpreter::new();

        // Test: Simple expression without colon (default line)
        let code = "[5] [3] +";

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Default line without colon should succeed: {:?}", result);

        // Verify result
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
        if let Some(val) = interp.stack.last() {
            // 結果は [ 8 ] というベクタ（単一要素ベクタ）
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                assert_eq!(children[0].as_scalar().expect("Expected scalar").numerator.to_string(), "8", "Result should be 8");
            } else {
                panic!("Expected vector result from addition");
            }
        }
    }

    #[tokio::test]
    async fn test_def_with_new_syntax() {
        let mut interp = Interpreter::new();

        // Test: Define a word with new code block syntax : ... ;
        let code = ": [ 42 ] ; 'ANSWER' DEF";

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "DEF with new syntax should succeed: {:?}", result);

        // Verify ANSWER is defined
        assert!(interp.dictionary.contains_key("ANSWER"), "ANSWER should be defined");

        // Call ANSWER
        let call_result = interp.execute("ANSWER").await;
        assert!(call_result.is_ok(), "Calling ANSWER should succeed");
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
    }

    #[tokio::test]
    async fn test_multiple_lines_without_colon() {
        let mut interp = Interpreter::new();

        // Test: Multiple lines without colons (all treated as default lines)
        let code = r#"
[1] [2] +
[3] *
"#;

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Multiple lines without colon should succeed: {:?}", result);

        // Verify result: (1 + 2) * 3 = 9
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                assert_eq!(children[0].as_scalar().expect("Expected scalar").numerator.to_string(), "9", "Result should be 9");
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_sequential_execution() {
        let mut interp = Interpreter::new();

        // Test: Multiple lines without chevrons (executed sequentially)
        let code = r#"
[10] [20] +
[5] *
"#;

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Sequential lines should succeed: {:?}", result);

        // Verify result: (10 + 20) * 5 = 150
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                assert_eq!(children[0].as_scalar().expect("Expected scalar").numerator.to_string(), "150", "Result should be 150");
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_chevron_vs_sequential() {
        let mut interp = Interpreter::new();

        // Test: Chevron branch (conditional execution)
        let chevron_code = r#"
>> [ 3 ] [ 5 ] <
>> [100]
>>> [0]
"#;

        let result = interp.execute(chevron_code).await;
        assert!(result.is_ok(), "Chevron branch should succeed: {:?}", result);

        // Result should be [100] because 3 < 5 is true
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                assert_eq!(children[0].as_scalar().expect("Expected scalar").numerator.to_string(), "100", "Result should be 100");
            } else {
                panic!("Expected vector result");
            }
        }

        // Clear stack
        interp.stack.clear();

        // Test: Same logic but sequential (all lines executed)
        let sequential_code = r#"
[ 3 ] [ 5 ] <
[100]
[0]
"#;

        let result = interp.execute(sequential_code).await;
        assert!(result.is_ok(), "Sequential lines should succeed: {:?}", result);

        // All lines executed, so we should have 3 items on stack
        assert_eq!(interp.stack.len(), 3, "Stack should have three elements");
    }

    #[tokio::test]
    async fn test_chevron_five_lines_ok() {
        let mut interp = Interpreter::new();

        // 5行：2つの条件、2つのアクション、1つのデフォルト
        let code = r#"
>> FALSE
>> [100]
>> FALSE
>> [200]
>>> [999]
"#;

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Chevron with 5 lines should succeed: {:?}", result);

        // すべての条件がfalseなのでデフォルトの999
        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 1, "Result should have one element");
                assert_eq!(children[0].as_scalar().expect("Expected scalar").numerator.to_string(), "999");
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_map_with_increment() {
        let mut interp = Interpreter::new();
        // 統一分数アーキテクチャ: MAPワードはベクタ結果を返す必要がある
        let result = interp.execute(": [ 1 ] + ; 'INC' DEF [ 1 2 3 ] 'INC' MAP").await;
        assert!(result.is_ok(), "MAP with increment function should succeed: {:?}", result);

        // 結果が [ 2 3 4 ] であることを確認
        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            if let ValueData::Vector(children) = &val.data {
                assert_eq!(children.len(), 3, "Result should have 3 elements");
                assert_eq!(children[0].as_scalar().expect("Expected scalar").numerator.to_string(), "2", "First element should be 2");
                assert_eq!(children[1].as_scalar().expect("Expected scalar").numerator.to_string(), "3", "Second element should be 3");
                assert_eq!(children[2].as_scalar().expect("Expected scalar").numerator.to_string(), "4", "Third element should be 4");
            } else {
                panic!("Expected vector result");
            }
        }
    }

    #[tokio::test]
    async fn test_map_stack_mode() {
        let mut interp = Interpreter::new();
        // Stackモードでの動作確認
        let result = interp.execute(": [ 2 ] * ; 'DOUBLE' DEF [ 1 ] [ 2 ] [ 3 ] [ 3 ] 'DOUBLE' .. MAP").await;
        assert!(result.is_ok(), "MAP in Stack mode should work: {:?}", result);

        // スタックに3つの要素があること
        assert_eq!(interp.stack.len(), 3);
    }

    #[tokio::test]
    async fn test_empty_vector_error() {
        let mut interp = Interpreter::new();

        // 空のベクターは許容されない
        // [ [ ] ] → エラー
        let result = interp.execute("[ [ ] ]").await;
        assert!(result.is_err(), "Empty vector should be an error");
        assert!(result.unwrap_err().to_string().contains("Empty vector"));
    }

    #[tokio::test]
    async fn test_empty_brackets_error() {
        let mut interp = Interpreter::new();

        // 空の括弧は許容されない
        let result = interp.execute("[ ]").await;
        assert!(result.is_err(), "Empty brackets should be an error");
        assert!(result.unwrap_err().to_string().contains("Empty vector"));
    }

    #[tokio::test]
    async fn test_nil_keyword_works() {
        let mut interp = Interpreter::new();

        // NILキーワードを使用してNIL値を作成
        let result = interp.execute("NIL").await;
        assert!(result.is_ok(), "NIL keyword should work: {:?}", result);

        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil(), "Expected NIL, got {:?}", val);
        }
    }

    #[tokio::test]
    async fn test_nil_in_vector() {
        let mut interp = Interpreter::new();

        // ベクター内のNILキーワード
        let result = interp.execute("[ 1 NIL 2 ]").await;
        assert!(result.is_ok(), "NIL in vector should work: {:?}", result);

        assert_eq!(interp.stack.len(), 1);

        // Vectorは3要素を持つ（NILも1要素としてカウント）
        let vec_val = &interp.stack[0];
        assert_eq!(vec_val.shape(), vec![3], "Vector should have 3 elements including NIL");
        if let ValueData::Vector(children) = &vec_val.data {
            assert_eq!(children.len(), 3, "Data should have 3 elements");
            // 2番目の要素がNIL
            assert!(children[1].is_nil(), "Second element should be NIL");
        } else {
            panic!("Expected vector");
        }
    }

    #[tokio::test]
    async fn test_nil_is_value() {
        use crate::types::Value;

        // NILはValueData::Nil
        let nil = Value::nil();
        assert!(nil.is_nil(), "Value::nil() should be NIL");
        assert!(nil.shape().is_empty(), "NIL should be scalar (empty shape)");
        // 新アーキテクチャでは ValueData::Nil なので直接確認
        assert!(matches!(nil.data, ValueData::Nil), "NIL should be ValueData::Nil");
    }

    #[tokio::test]
    async fn test_nil_arithmetic_propagation() {
        // NIL + x = NIL
        let nil = crate::types::fraction::Fraction::nil();
        let one = crate::types::fraction::Fraction::from(1);
        let result = nil.add(&one);
        assert!(result.is_nil(), "NIL + 1 should be NIL");

        // x * NIL = NIL
        let result = one.mul(&nil);
        assert!(result.is_nil(), "1 * NIL should be NIL");
    }

    #[tokio::test]
    async fn test_nil_and_true_returns_nil() {
        let mut interp = Interpreter::new();
        let result = interp.execute("NIL TRUE AND").await;
        assert!(result.is_ok(), "NIL AND TRUE should work: {:?}", result);
        let val = interp.stack.pop().unwrap();
        assert!(val.is_nil(), "NIL AND TRUE should return NIL, got {:?}", val);
    }

    #[tokio::test]
    async fn test_nil_or_false_returns_nil() {
        let mut interp = Interpreter::new();
        let result = interp.execute("NIL FALSE OR").await;
        assert!(result.is_ok(), "NIL OR FALSE should work: {:?}", result);
        let val = interp.stack.pop().unwrap();
        assert!(val.is_nil(), "NIL OR FALSE should return NIL, got {:?}", val);
    }

    #[tokio::test]
    async fn test_not_nil_returns_nil() {
        let mut interp = Interpreter::new();
        let result = interp.execute("NIL NOT").await;
        assert!(result.is_ok(), "NOT NIL should work: {:?}", result);
        let val = interp.stack.pop().unwrap();
        assert!(val.is_nil(), "NOT NIL should return NIL, got {:?}", val);
    }

    #[tokio::test]
    async fn test_false_and_nil_returns_false() {
        let mut interp = Interpreter::new();
        let result = interp.execute("FALSE NIL AND").await;
        assert!(result.is_ok(), "FALSE AND NIL should work: {:?}", result);
        let val = interp.stack.pop().unwrap();
        // FALSE AND NIL = FALSE (FALSEが確定的に偽)
        assert!(!val.is_nil(), "FALSE AND NIL should return FALSE, not NIL");
        assert!(!val.is_truthy(), "FALSE AND NIL should be falsy");
    }

    #[tokio::test]
    async fn test_true_or_nil_returns_true() {
        let mut interp = Interpreter::new();
        let result = interp.execute("TRUE NIL OR").await;
        assert!(result.is_ok(), "TRUE OR NIL should work: {:?}", result);
        let val = interp.stack.pop().unwrap();
        // TRUE OR NIL = TRUE (TRUEが確定的に真)
        assert!(!val.is_nil(), "TRUE OR NIL should return TRUE, not NIL");
        assert!(val.is_truthy(), "TRUE OR NIL should be truthy");
    }

    // === 呼び出し深度制限テスト ===

    #[tokio::test]
    async fn test_call_depth_3_ok() {
        let mut interp = Interpreter::new();
        // A -> B -> C = 深度3、OK
        interp.execute(": C ; 'B' DEF").await.unwrap();
        interp.execute(": B ; 'A' DEF").await.unwrap();
        interp.execute(": [ 1 ] ; 'C' DEF").await.unwrap();

        let result = interp.execute("A").await;
        assert!(result.is_ok(), "Call depth 3 should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_call_depth_4_exceeds() {
        let mut interp = Interpreter::new();
        // A -> B -> C -> D = 深度4、エラー
        interp.execute(": B ; 'A' DEF").await.unwrap();
        interp.execute(": C ; 'B' DEF").await.unwrap();
        interp.execute(": D ; 'C' DEF").await.unwrap();
        interp.execute(": [ 1 ] ; 'D' DEF").await.unwrap();

        let result = interp.execute("A").await;
        assert!(result.is_err(), "Call depth 4 should fail");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Call depth limit"), "Error message should mention call depth limit: {}", err_msg);
    }

    #[tokio::test]
    async fn test_direct_recursion_limited() {
        let mut interp = Interpreter::new();
        // REC -> REC -> REC -> REC で深度超過
        // シンプルな再帰ワード：常に自分自身を呼び出す
        interp.execute(": REC ; 'REC' DEF").await.unwrap();

        let result = interp.execute("REC").await;
        assert!(result.is_err(), "Direct recursion should hit depth limit");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Call depth limit"), "Error message should mention call depth limit: {}", err_msg);
    }

    #[tokio::test]
    async fn test_call_depth_resets_after_completion() {
        let mut interp = Interpreter::new();
        // 深度2の呼び出しを2回実行しても問題ない
        interp.execute(": B ; 'A' DEF").await.unwrap();
        interp.execute(": [ 1 ] ; 'B' DEF").await.unwrap();

        // 1回目の呼び出し
        let result1 = interp.execute("A").await;
        assert!(result1.is_ok(), "First call should succeed");

        // 2回目の呼び出し（call_stackがリセットされていることを確認）
        let result2 = interp.execute("A").await;
        assert!(result2.is_ok(), "Second call should succeed (call_stack should reset)");
    }

    #[tokio::test]
    async fn test_call_depth_error_shows_trace() {
        let mut interp = Interpreter::new();
        // A -> B -> C -> D でエラーメッセージにスタックトレースが含まれることを確認
        interp.execute(": B ; 'A' DEF").await.unwrap();
        interp.execute(": C ; 'B' DEF").await.unwrap();
        interp.execute(": D ; 'C' DEF").await.unwrap();
        interp.execute(": [ 1 ] ; 'D' DEF").await.unwrap();

        let result = interp.execute("A").await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        // エラーメッセージにスタックトレースが含まれる
        assert!(err_msg.contains("A") && err_msg.contains("B") && err_msg.contains("C"),
            "Error message should show call trace: {}", err_msg);
    }

    // === 対称モードテスト ===

    #[tokio::test]
    async fn test_keep_mode_basic() {
        let mut interp = Interpreter::new();
        // Keepモード（,,）で加算：元の値が残り、結果が追加される
        // [1] [2] ,, + → [1] [2] [3]
        let result = interp.execute("[1] [2] ,, +").await;
        assert!(result.is_ok(), "Keep mode addition should succeed: {:?}", result);

        // スタックには3つの要素（元の[1], [2], 結果の[3]）
        assert_eq!(interp.stack.len(), 3, "Stack should have 3 elements after keep mode operation");
    }

    #[tokio::test]
    async fn test_modifiers_order_independent_stack_keep() {
        let mut interp = Interpreter::new();
        // [1] [2] .. ,, + と [1] [2] ,, .. + が同じ挙動になるか確認
        // Stackモード（..）+ Keepモード（,,）

        // 順序1: .. ,,
        let result1 = interp.execute("[1] [2] [3] [3] .. ,, +").await;
        assert!(result1.is_ok(), "Stack+Keep mode (.. ,,) should succeed: {:?}", result1);
        let stack1 = interp.stack.clone();

        interp.execute_reset().unwrap();

        // 順序2: ,, ..
        let result2 = interp.execute("[1] [2] [3] [3] ,, .. +").await;
        assert!(result2.is_ok(), "Stack+Keep mode (,, ..) should succeed: {:?}", result2);
        let stack2 = interp.stack.clone();

        // 両方の結果が同じであることを確認
        assert_eq!(stack1.len(), stack2.len(),
            "Both modifier orders should produce same stack length: {} vs {}",
            stack1.len(), stack2.len());
    }

    #[tokio::test]
    async fn test_consume_mode_default() {
        let mut interp = Interpreter::new();
        // デフォルトはConsumeモード
        let result = interp.execute("[1] [2] +").await;
        assert!(result.is_ok(), "Default consume mode should work: {:?}", result);

        // スタックには1つの要素（結果の[3]のみ）
        assert_eq!(interp.stack.len(), 1, "Stack should have 1 element after consume mode operation");
    }

    #[tokio::test]
    async fn test_explicit_consume_mode() {
        let mut interp = Interpreter::new();
        // 明示的なConsumeモード（,）
        let result = interp.execute("[1] [2] , +").await;
        assert!(result.is_ok(), "Explicit consume mode should work: {:?}", result);

        // スタックには1つの要素（結果の[3]のみ）
        assert_eq!(interp.stack.len(), 1, "Stack should have 1 element after explicit consume mode");
    }

    #[tokio::test]
    async fn test_mode_reset_after_word() {
        let mut interp = Interpreter::new();
        // モードはワード実行後にリセットされる
        // 最初の+は Keep モードで実行
        // 2番目の+は Consume モード（デフォルト）で実行
        let result = interp.execute("[1] [2] ,, + [3] +").await;
        assert!(result.is_ok(), "Mode should reset after word: {:?}", result);

        // [1] [2] ,, + → [1] [2] [3]
        // [1] [2] [3] [3] + → [1] [2] [6]  (consumeモードなので[3]が消費される)
        assert_eq!(interp.stack.len(), 3, "Stack should have 3 elements: {:?}", interp.stack);
    }

    #[tokio::test]
    async fn test_keep_mode_with_mul() {
        let mut interp = Interpreter::new();
        // Keepモードで乗算
        let result = interp.execute("[3] [4] ,, *").await;
        assert!(result.is_ok(), "Keep mode multiplication should succeed: {:?}", result);

        // スタックには3つの要素（元の[3], [4], 結果の[12]）
        assert_eq!(interp.stack.len(), 3, "Stack should have 3 elements after keep mode multiplication");
    }

    #[tokio::test]
    async fn test_keep_mode_with_sub() {
        let mut interp = Interpreter::new();
        // Keepモードで減算
        let result = interp.execute("[10] [3] ,, -").await;
        assert!(result.is_ok(), "Keep mode subtraction should succeed: {:?}", result);

        // スタックには3つの要素（元の[10], [3], 結果の[7]）
        assert_eq!(interp.stack.len(), 3, "Stack should have 3 elements after keep mode subtraction");
    }

    #[tokio::test]
    async fn test_keep_mode_with_div() {
        let mut interp = Interpreter::new();
        // Keepモードで除算
        let result = interp.execute("[12] [4] ,, /").await;
        assert!(result.is_ok(), "Keep mode division should succeed: {:?}", result);

        // スタックには3つの要素（元の[12], [4], 結果の[3]）
        assert_eq!(interp.stack.len(), 3, "Stack should have 3 elements after keep mode division");
    }
}
