// rust/src/interpreter/mod.rs

pub mod vector_ops;
pub mod arithmetic;
pub mod control;
pub mod io;
pub mod error;
pub mod flow_control;

use std::collections::{HashMap, HashSet};
use crate::types::{Workspace, Token, Value, ValueType, BracketType, Fraction, WordDefinition, ExecutionLine};
use self::error::{Result, AjisaiError};
use num_traits::Zero;

pub struct Interpreter {
    pub(crate) workspace: Workspace,
    pub(crate) dictionary: HashMap<String, WordDefinition>,
    pub(crate) output_buffer: String,
    pub(crate) execution_state: Option<WordExecutionState>,
}

pub struct WordExecutionState {
    pub program_counter: usize,
    pub repeat_counters: Vec<i64>,
    pub word_name: String,
    pub continue_loop: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Interpreter {
            workspace: Vec::new(),
            dictionary: HashMap::new(),
            output_buffer: String::new(),
            execution_state: None,
        };
        crate::builtins::register_builtins(&mut interpreter.dictionary);
        interpreter
    }

    pub fn execute(&mut self, code: &str) -> Result<()> {
        let custom_word_names: HashSet<String> = self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect();
        let tokens = crate::tokenizer::tokenize_with_custom_words(code, &custom_word_names)?;
        self.execute_tokens(&tokens)
    }

    pub fn execute_tokens(&mut self, tokens: &[Token]) -> Result<()> {
        let mut i = 0;
        while i < tokens.len() {
            let token = &tokens[i];
            match token {
                Token::Number(s) => {
                    let frac = Fraction::from_str(s).map_err(AjisaiError::from)?;
                    self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Number(frac) }], BracketType::Square)});
                },
                Token::String(s) => self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::String(s.clone()) }], BracketType::Square)}),
                Token::Boolean(b) => self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Boolean(*b) }], BracketType::Square)}),
                Token::Nil => self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Nil }], BracketType::Square)}),
                Token::VectorStart(bt) => {
                    let (values, consumed) = self.collect_vector(tokens, i)?;
                    self.workspace.push(Value { val_type: ValueType::Vector(values, bt.clone()) });
                    i += consumed - 1;
                },
                Token::DefBlockStart => {
                    // ネストした定義ブロック構造の検出
                    if self.is_nested_definition_structure(&tokens[i..])? {
                        self.output_buffer.push_str("[DEBUG] Detected nested definition structure\n");
                        let (nested_def, consumed) = self.parse_nested_definition(&tokens[i..])?;
                        self.workspace.push(Value { val_type: ValueType::DefinitionBody(nested_def) });
                        i += consumed - 1;
                    } else {
                        // 従来の単一定義ブロック
                        self.output_buffer.push_str("[DEBUG] Detected simple definition block\n");
                        let (body_tokens, block_consumed) = self.collect_def_block(tokens, i)?;
                        self.output_buffer.push_str(&format!("[DEBUG] Collected {} tokens in definition block\n", body_tokens.len()));
                        
                        // ブロック後の修飾子を収集
                        let mut modifier_start = i + block_consumed;
                        let mut modifiers = Vec::new();
                        while modifier_start < tokens.len() {
                            if let Token::Modifier(m) = &tokens[modifier_start] {
                                modifiers.push(m.clone());
                                self.output_buffer.push_str(&format!("[DEBUG] Found modifier: {}\n", m));
                                modifier_start += 1;
                            } else {
                                break;
                            }
                        }
                        
                        // 修飾子を解析
                        let (repeat_count, delay_ms, debug_messages) = Self::parse_modifiers_static(&modifiers);
                        self.output_buffer.push_str(&debug_messages);
                        self.output_buffer.push_str(&format!("[DEBUG] Parsed modifiers: repeat={}, delay={}ms\n", repeat_count, delay_ms));
                        
                        // ブロックを指定回数実行
                        for iteration in 0..repeat_count {
                            self.output_buffer.push_str(&format!("[DEBUG] Executing iteration {}/{}\n", iteration + 1, repeat_count));
                            self.execute_tokens(&body_tokens)?;
                            
                            if delay_ms > 0 {
                                self.output_buffer.push_str(&format!("[DEBUG] Waiting {}ms...\n", delay_ms));
                                crate::wasm_sleep(delay_ms);
                            }
                        }
                        self.output_buffer.push_str("[DEBUG] Definition block execution completed\n");
                        
                        i = modifier_start - 1;
                    }
                }
                Token::Symbol(name) => self.execute_word(name)?,
                _ => {} 
            }
            i += 1;
        }
        Ok(())
    }

    fn is_nested_definition_structure(&self, tokens: &[Token]) -> Result<bool> {
        // 実際にネストした :...; 構造があるかチェック
        let mut depth = 0;
        let mut found_nested = false;
        
        for token in tokens {
            match token {
                Token::DefBlockStart => {
                    depth += 1;
                    if depth > 1 {
                        // 2階層目以上の定義ブロックが見つかった
                        found_nested = true;
                    }
                },
                Token::DefBlockEnd => {
                    depth -= 1;
                    if depth == 0 {
                        // 外側のブロック終了
                        break;
                    }
                },
                _ => {}
            }
        }
        
        self.output_buffer.push_str(&format!("[DEBUG] Nested definition check: found_nested={}\n", found_nested));
        Ok(found_nested)
    }

    fn parse_nested_definition(&mut self, tokens: &[Token]) -> Result<(Vec<Token>, usize)> {
        self.output_buffer.push_str("[DEBUG] Parsing nested definition structure\n");
        
        let mut result_tokens = Vec::new();
        let mut i = 1; // Skip the initial DefBlockStart
        let mut outer_depth = 1;
        
        while i < tokens.len() && outer_depth > 0 {
            match &tokens[i] {
                Token::DefBlockStart => {
                    // 内側の定義ブロックの開始
                    self.output_buffer.push_str(&format!("[DEBUG] Found inner definition block at position {}\n", i));
                    let (inner_tokens, consumed) = self.collect_inner_def_block(tokens, i)?;
                    self.output_buffer.push_str(&format!("[DEBUG] Collected inner block with {} tokens\n", inner_tokens.len()));
                    
                    // 内側のブロックを解析してExecutionLineとして変換
                    let execution_line_tokens = self.convert_inner_block_to_execution_line(&inner_tokens)?;
                    result_tokens.extend(execution_line_tokens);
                    
                    i += consumed;
                },
                Token::DefBlockEnd => {
                    outer_depth -= 1;
                    if outer_depth == 0 {
                        // 外側のブロック終了
                        self.output_buffer.push_str("[DEBUG] Found outer definition block end\n");
                        break;
                    }
                    i += 1;
                },
                _ => {
                    i += 1;
                }
            }
        }
        
        self.output_buffer.push_str(&format!("[DEBUG] Parsed nested definition with {} result tokens\n", result_tokens.len()));
        Ok((result_tokens, i + 1))
    }

    fn collect_inner_def_block(&self, tokens: &[Token], start: usize) -> Result<(Vec<Token>, usize)> {
        let mut depth = 1;
        let mut i = start + 1;
        
        while i < tokens.len() {
            match tokens[i] {
                Token::DefBlockStart => depth += 1,
                Token::DefBlockEnd => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok((tokens[start + 1..i].to_vec(), i - start + 1));
                    }
                },
                _ => {}
            }
            i += 1;
        }
        
        Err(AjisaiError::from("Unclosed inner definition block"))
    }

    fn convert_inner_block_to_execution_line(&self, tokens: &[Token]) -> Result<Vec<Token>> {
        // 内側のブロックをExecutionLine形式に変換
        let mut result = vec![Token::Symbol("INNER_DEF_LINE".to_string())];
        
        // ガードセパレータ（$）の位置を探す
        let guard_pos = tokens.iter().position(|t| matches!(t, Token::GuardSeparator));
        
        if let Some(pos) = guard_pos {
            // 条件付きライン
            result.push(Token::Symbol("WITH_CONDITION".to_string()));
            result.extend(tokens[..pos].to_vec());
            result.push(Token::GuardSeparator);
            result.extend(tokens[pos + 1..].to_vec());
        } else {
            // デフォルトライン（条件なし）
            result.push(Token::Symbol("DEFAULT_LINE".to_string()));
            result.extend(tokens.to_vec());
        }
        
        result.push(Token::Symbol("END_INNER_DEF_LINE".to_string()));
        
        self.output_buffer.push_str(&format!("[DEBUG] Converted inner block to {} tokens\n", result.len()));
        Ok(result)
    }

    fn parse_modifiers_static(modifiers: &[String]) -> (i64, u64, String) {
        let mut repeat_count = 1;
        let mut delay_ms = 0;
        let mut debug_messages = String::new();
        
        debug_messages.push_str(&format!("[DEBUG] Parsing {} modifiers: {:?}\n", modifiers.len(), modifiers));
        
        for modifier in modifiers {
            debug_messages.push_str(&format!("[DEBUG] Processing modifier: {}\n", modifier));
            
            if modifier.ends_with('x') {
                let num_part = &modifier[..modifier.len()-1];
                match num_part.parse::<i64>() {
                    Ok(count) => {
                        repeat_count = count;
                        debug_messages.push_str(&format!("[DEBUG] Set repeat count to {}\n", count));
                    }
                    Err(_) => {
                        debug_messages.push_str(&format!("[DEBUG] Failed to parse repeat count from {}\n", modifier));
                    }
                }
            } else if modifier.ends_with("ms") {
                let num_part = &modifier[..modifier.len()-2];
                match num_part.parse::<u64>() {
                    Ok(ms) => {
                        delay_ms = ms;
                        debug_messages.push_str(&format!("[DEBUG] Set delay to {}ms\n", ms));
                    }
                    Err(_) => {
                        debug_messages.push_str(&format!("[DEBUG] Failed to parse ms from {}\n", modifier));
                    }
                }
            } else if modifier.ends_with('s') {
                let num_part = &modifier[..modifier.len()-1];
                match num_part.parse::<u64>() {
                    Ok(s) => {
                        delay_ms = s * 1000;
                        debug_messages.push_str(&format!("[DEBUG] Set delay to {}s ({}ms)\n", s, delay_ms));
                    }
                    Err(_) => {
                        debug_messages.push_str(&format!("[DEBUG] Failed to parse seconds from {}\n", modifier));
                    }
                }
            } else {
                debug_messages.push_str(&format!("[DEBUG] Unknown modifier format: {}\n", modifier));
            }
        }
        
        (repeat_count, delay_ms, debug_messages)
    }

    fn collect_def_block(&self, tokens: &[Token], start: usize) -> Result<(Vec<Token>, usize)> {
        let mut depth = 1;
        let mut i = start + 1;
        while i < tokens.len() {
            match tokens[i] {
                Token::DefBlockStart => depth += 1,
                Token::DefBlockEnd => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok((tokens[start + 1..i].to_vec(), i - start + 1));
                    }
                },
                _ => {}
            }
            i += 1;
        }
        Err(AjisaiError::from("Unclosed definition block"))
    }

    fn execute_word(&mut self, name: &str) -> Result<()> {
        let def = self.dictionary.get(&name.to_uppercase()).cloned()
            .ok_or_else(|| AjisaiError::UnknownWord(name.to_string()))?;

        if def.is_builtin {
            return self.execute_builtin(name);
        }

        self.output_buffer.push_str(&format!("[DEBUG] Executing custom word: {}\n", name));
        self.output_buffer.push_str(&format!("[DEBUG] Word has {} lines\n", def.lines.len()));

        // パターンマッチング的条件評価による行選択
        let selected_line_index = self.select_matching_line(&def.lines)?;
        
        if let Some(line_index) = selected_line_index {
            self.output_buffer.push_str(&format!("[DEBUG] Selected line {} for execution\n", line_index + 1));
            self.execute_selected_line(&def.lines[line_index])?;
        } else {
            return Err(AjisaiError::from("No matching condition found and no default line available"));
        }

        Ok(())
    }

    fn select_matching_line(&mut self, lines: &[ExecutionLine]) -> Result<Option<usize>> {
        // 各行の条件を上から順にパターンマッチング評価
        for (index, line) in lines.iter().enumerate() {
            if line.condition_tokens.is_empty() {
                // デフォルト行は条件なしなので常に選択対象
                self.output_buffer.push_str(&format!("[DEBUG] Found default line at index {}\n", index));
                continue;
            }
            
            // パターンマッチング評価（非破壊的）
            if self.evaluate_pattern_condition(&line.condition_tokens)? {
                self.output_buffer.push_str(&format!("[DEBUG] Pattern matched for line {}\n", index + 1));
                return Ok(Some(index));
            }
        }
        
        // 条件に合致するものがなかった場合、デフォルト行を探す
        for (index, line) in lines.iter().enumerate() {
            if line.condition_tokens.is_empty() {
                self.output_buffer.push_str(&format!("[DEBUG] Using default line at index {}\n", index));
                return Ok(Some(index));
            }
        }
        
        Ok(None)
    }

    fn evaluate_pattern_condition(&mut self, condition_tokens: &[Token]) -> Result<bool> {
        // パターンマッチング的条件評価（スタックを消費しない）
        if condition_tokens.is_empty() {
            return Ok(true);
        }
        
        // ワークスペースのトップ値を取得（消費しない）
        let workspace_top = match self.workspace.last() {
            Some(value) => value.clone(), // borrowing issue を回避するためclone
            None => return Err(AjisaiError::from("Workspace is empty for condition evaluation")),
        };
        
        // 基本的なパターン: [値] 演算子 の形式
        if condition_tokens.len() >= 2 {
            // 最初のトークンがベクトルかチェック
            if let Token::VectorStart(_) = &condition_tokens[0] {
                let (pattern_value, vector_end) = self.extract_pattern_value(condition_tokens)?;
                
                if vector_end + 1 < condition_tokens.len() {
                    if let Token::Symbol(op) = &condition_tokens[vector_end + 1] {
                        return self.compare_values(&workspace_top, &pattern_value, op);
                    }
                }
            }
        }
        
        // パターンが認識できない場合はfalse
        self.output_buffer.push_str(&format!("[DEBUG] Unrecognized pattern: {:?}\n", condition_tokens));
        Ok(false)
    }

    fn extract_pattern_value(&self, tokens: &[Token]) -> Result<(Value, usize)> {
        // ベクトル開始から終了までのトークンを解析して値を構築
        let (values, consumed) = self.collect_vector(tokens, 0)?;
        let pattern_value = Value { 
            val_type: ValueType::Vector(values, BracketType::Square) 
        };
        Ok((pattern_value, consumed - 1))
    }

    fn compare_values(&mut self, workspace_value: &Value, pattern_value: &Value, operator: &str) -> Result<bool> {
        self.output_buffer.push_str(&format!("[DEBUG] Comparing {} {} {}\n", workspace_value, operator, pattern_value));
        
        match operator {
            "=" => Ok(workspace_value == pattern_value),
            "!=" => Ok(workspace_value != pattern_value),
            ">" | "<" | ">=" | "<=" => {
                // 数値比較の場合
                self.compare_numbers(workspace_value, pattern_value, operator)
            },
            _ => {
                self.output_buffer.push_str(&format!("[DEBUG] Unknown comparison operator: {}\n", operator));
                Ok(false)
            }
        }
    }

    fn compare_numbers(&self, workspace_value: &Value, pattern_value: &Value, operator: &str) -> Result<bool> {
        // 両方が単一要素のベクトルで数値の場合のみ比較
        if let (ValueType::Vector(w_vec, _), ValueType::Vector(p_vec, _)) = (&workspace_value.val_type, &pattern_value.val_type) {
            if w_vec.len() == 1 && p_vec.len() == 1 {
                if let (ValueType::Number(w_num), ValueType::Number(p_num)) = (&w_vec[0].val_type, &p_vec[0].val_type) {
                    return Ok(match operator {
                        ">" => w_num.gt(p_num),
                        "<" => w_num.lt(p_num),
                        ">=" => w_num.ge(p_num),
                        "<=" => w_num.le(p_num),
                        _ => false,
                    });
                }
            }
        }
        Ok(false)
    }

    fn execute_selected_line(&mut self, line: &ExecutionLine) -> Result<()> {
        self.output_buffer.push_str(&format!("[DEBUG] Executing line with {} repeats, {}ms delay\n", line.repeat_count, line.delay_ms));
        
        for iteration in 0..line.repeat_count {
            self.output_buffer.push_str(&format!("[DEBUG] Iteration {}/{}\n", iteration + 1, line.repeat_count));
            
            // 処理部を実行（他のカスタムワードも含む可能性がある）
            self.execute_tokens(&line.body_tokens)?;
            
            if line.delay_ms > 0 && iteration < line.repeat_count - 1 {
                self.output_buffer.push_str(&format!("[DEBUG] Waiting {}ms...\n", line.delay_ms));
                crate::wasm_sleep(delay_ms);
            }
        }
        
        Ok(())
    }

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        match name.to_uppercase().as_str() {
            "GET" => vector_ops::op_get(self),
            "INSERT" => vector_ops::op_insert(self),
            "REPLACE" => vector_ops::op_replace(self),
            "REMOVE" => vector_ops::op_remove(self),
            "LENGTH" => vector_ops::op_length(self),
            "TAKE" => vector_ops::op_take(self),
            "DROP" => vector_ops::op_drop_vector(self),
            "SPLIT" => vector_ops::op_split(self),
            "DUP" => vector_ops::op_dup_workspace(self),
            "SWAP" => vector_ops::op_swap_workspace(self),
            "ROT" => vector_ops::op_rot_workspace(self),
            "CONCAT" => vector_ops::op_concat(self),
            "REVERSE" => vector_ops::op_reverse(self),
            "+" => arithmetic::op_add(self),
            "-" => arithmetic::op_sub(self),
            "*" => arithmetic::op_mul(self),
            "/" => arithmetic::op_div(self),
            "=" => arithmetic::op_eq(self),
            "<" => arithmetic::op_lt(self),
            "<=" => arithmetic::op_le(self),
            ">" => arithmetic::op_gt(self),
            ">=" => arithmetic::op_ge(self),
            "AND" => arithmetic::op_and(self),
            "OR" => arithmetic::op_or(self),
            "NOT" => arithmetic::op_not(self),
            "PRINT" => io::op_print(self),
            "DEF" => control::op_def(self),
            "DEL" => control::op_del(self),
            "RESET" => self.execute_reset(),
            "GOTO" => flow_control::op_goto(self),
            _ => Err(AjisaiError::UnknownBuiltin(name.to_string())),
        }
    }
    
    fn collect_vector(&self, tokens: &[Token], start: usize) -> Result<(Vec<Value>, usize)> {
        let mut values = Vec::new();
        let mut i = start + 1;
        let mut depth = 1;
        
        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorStart(bt) => {
                    depth += 1;
                    let (nested_values, consumed) = self.collect_vector(tokens, i)?;
                    values.push(Value { val_type: ValueType::Vector(nested_values, bt.clone()) });
                    i += consumed - 1;
                },
                Token::VectorEnd(_) => {
                    depth -= 1;
                    if depth == 0 { 
                        return Ok((values, i - start + 1)); 
                    }
                },
                Token::Number(s) => {
                    let frac = Fraction::from_str(s).map_err(AjisaiError::from)?;
                    values.push(Value { val_type: ValueType::Number(frac) });
                },
                Token::String(s) => {
                    values.push(Value { val_type: ValueType::String(s.clone()) });
                },
                Token::Boolean(b) => {
                    values.push(Value { val_type: ValueType::Boolean(*b) });
                },
                Token::Nil => {
                    values.push(Value { val_type: ValueType::Nil });
                },
                Token::Symbol(name) => {
                    values.push(Value { val_type: ValueType::Symbol(name.clone()) });
                },
                _ => {}
            }
            i += 1;
        }
        Err(AjisaiError::from("Unclosed vector"))
    }

    // control.rsで使用するメソッド
    pub(crate) fn contains_nested_definition(&self, tokens: &[Token]) -> bool {
        tokens.iter().any(|t| matches!(t, Token::Symbol(s) if s == "INNER_DEF_LINE"))
    }
    
    pub(crate) fn parse_nested_definition_body(&mut self, tokens: &[Token]) -> Result<Vec<ExecutionLine>> {
        let mut lines = Vec::new();
        let mut i = 0;
        
        while i < tokens.len() {
            if let Token::Symbol(s) = &tokens[i] {
                if s == "INNER_DEF_LINE" {
                    let (line, consumed) = self.parse_single_inner_line(&tokens[i..])?;
                    lines.push(line);
                    i += consumed;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
        
        // デフォルト行の存在チェック
        let has_default = lines.iter().any(|line| line.condition_tokens.is_empty());
        if !has_default {
            return Err(AjisaiError::from("Nested definition must have at least one default line (without condition)"));
        }
        
        self.output_buffer.push_str(&format!("[DEBUG] Nested definition validation passed. {} lines total\n", lines.len()));
        Ok(lines)
    }
    
    fn parse_single_inner_line(&self, tokens: &[Token]) -> Result<(ExecutionLine, usize)> {
        // INNER_DEF_LINE から END_INNER_DEF_LINE までを解析
        let mut i = 1; // Skip INNER_DEF_LINE
        let mut condition_tokens = Vec::new();
        let mut body_tokens = Vec::new();
        let mut repeat_count = 1;
        let mut delay_ms = 0;
        let mut is_default_line = false;
        
        // WITH_CONDITION または DEFAULT_LINE かチェック
        if i < tokens.len() {
            if let Token::Symbol(s) = &tokens[i] {
                if s == "DEFAULT_LINE" {
                    is_default_line = true;
                    i += 1;
                } else if s == "WITH_CONDITION" {
                    i += 1;
                } else {
                    return Err(AjisaiError::from("Invalid inner definition line format"));
                }
            }
        }
        
        // トークンを解析
        while i < tokens.len() {
            match &tokens[i] {
                Token::Symbol(s) if s == "END_INNER_DEF_LINE" => {
                    break;
                },
                Token::GuardSeparator => {
                    // ガードセパレータが見つかった場合、それまでが条件部
                    i += 1;
                    // 残りが処理部（修飾子を除く）
                    break;
                },
                Token::Modifier(m) => {
                    // 修飾子の解析
                    self.parse_modifier(m, &mut repeat_count, &mut delay_ms);
                    i += 1;
                },
                _ => {
                    if is_default_line || condition_tokens.is_empty() {
                        // デフォルト行または条件部
                        if !is_default_line && !tokens[i..].iter().any(|t| matches!(t, Token::GuardSeparator)) {
                            // ガードセパレータがない場合はデフォルト行として扱う
                            is_default_line = true;
                        }
                        
                        if is_default_line {
                            body_tokens.push(tokens[i].clone());
                        } else {
                            condition_tokens.push(tokens[i].clone());
                        }
                    } else {
                        body_tokens.push(tokens[i].clone());
                    }
                    i += 1;
                }
            }
        }
        
        // ガードセパレータ以降の処理部を取得
        if !is_default_line {
            while i < tokens.len() {
                match &tokens[i] {
                    Token::Symbol(s) if s == "END_INNER_DEF_LINE" => break,
                    Token::Modifier(m) => {
                        self.parse_modifier(m, &mut repeat_count, &mut delay_ms);
                        i += 1;
                    },
                    _ => {
                        body_tokens.push(tokens[i].clone());
                        i += 1;
                    }
                }
            }
        }
        
        // END_INNER_DEF_LINE をスキップ
        if i < tokens.len() {
            i += 1;
        }
        
        let final_condition_tokens = if is_default_line { Vec::new() } else { condition_tokens };
        
        Ok((ExecutionLine {
            condition_tokens: final_condition_tokens,
            body_tokens,
            repeat_count,
            delay_ms,
        }, i))
    }
    
    fn parse_modifier(&self, modifier: &str, repeat_count: &mut i64, delay_ms: &mut u64) {
        if modifier.ends_with('x') {
            if let Ok(count) = modifier[..modifier.len()-1].parse::<i64>() {
                *repeat_count = count;
            }
        } else if modifier.ends_with("ms") {
            if let Ok(ms) = modifier[..modifier.len()-2].parse::<u64>() {
                *delay_ms = ms;
            }
        } else if modifier.ends_with('s') {
            if let Ok(s) = modifier[..modifier.len()-1].parse::<u64>() {
                *delay_ms = s * 1000;
            }
        }
    }

    // Public methods for lib.rs
    pub fn get_output(&mut self) -> String { std::mem::take(&mut self.output_buffer) }
    pub fn get_workspace(&self) -> &Workspace { &self.workspace }
    pub fn set_workspace(&mut self, workspace: Workspace) { self.workspace = workspace; }
    pub fn get_custom_words_info(&self) -> Vec<(String, Option<String>)> {
        self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, def)| (name.clone(), def.description.clone()))
            .collect()
    }
    pub fn get_word_definition(&self, _name: &str) -> Option<String> {
        None
    }
    pub fn restore_custom_word(&mut self, _name: String, _tokens: Vec<Token>, _description: Option<String>) -> Result<()> {
        Ok(())
    }
    pub fn execute_reset(&mut self) -> Result<()> {
        self.workspace.clear();
        self.dictionary.clear();
        self.output_buffer.clear();
        self.execution_state = None;
        crate::builtins::register_builtins(&mut self.dictionary);
        Ok(())
    }
}

fn is_truthy(value: &Value) -> bool {
    match &value.val_type {
        ValueType::Boolean(b) => *b,
        ValueType::Nil => false,
        ValueType::Number(n) => !n.numerator.is_zero(),
        ValueType::String(s) => !s.is_empty(),
        ValueType::Vector(v, _) => !v.is_empty(),
        _ => true,
    }
}
