// rust/src/interpreter/mod.rs

pub mod vector_ops;
pub mod arithmetic;
pub mod comparison;
pub mod control;
pub mod dictionary;
pub mod io;
pub mod error;
pub mod audio;
pub mod higher_order;

use std::collections::{HashMap, HashSet};
use crate::types::{Stack, Token, Value, ValueType, BracketType, WordDefinition};
use crate::types::fraction::Fraction;
use self::error::{Result, AjisaiError};
use std::fmt::Write; // for write!

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperationTarget {
    Stack,
    StackTop,
}

pub struct Interpreter {
    pub(crate) stack: Stack,
    pub(crate) dictionary: HashMap<String, WordDefinition>,
    pub(crate) dependents: HashMap<String, HashSet<String>>,
    pub(crate) output_buffer: String,
    pub(crate) debug_buffer: String, // ★ デバッグログ専用バッファ
    pub(crate) definition_to_load: Option<String>,
    pub(crate) operation_target: OperationTarget,
    call_stack_depth: usize, // ★ デバッグ用の呼び出し深度
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Interpreter {
            stack: Vec::new(),
            dictionary: HashMap::new(),
            dependents: HashMap::new(),
            output_buffer: String::new(),
            debug_buffer: String::new(), // ★ 初期化
            definition_to_load: None,
            operation_target: OperationTarget::StackTop,
            call_stack_depth: 0, // ★ 初期化
        };
        crate::builtins::register_builtins(&mut interpreter.dictionary);
        interpreter
    }

    // ★ デバッグログ用のスタック状態フォーマッタ
    fn format_stack_for_debug(&self) -> String {
        if self.stack.is_empty() {
            return "(empty)".to_string();
        }
        self.stack.iter()
            .map(|v| v.to_string())
            .collect::<Vec<String>>()
            .join(" ")
    }

    // ★ デバッグログ用のインデント
    fn get_indent(&self) -> String {
        "  ".repeat(self.call_stack_depth)
    }

    fn collect_vector(&self, tokens: &[Token], start_index: usize) -> Result<(Vec<Value>, BracketType, usize)> {
        let bracket_type = match &tokens[start_index] {
            Token::VectorStart(bt) => bt.clone(),
            _ => return Err(AjisaiError::from("Expected vector start")),
        };

        let mut values = Vec::new();
        let mut i = start_index + 1;

        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorStart(_) => {
                    let (nested_values, nested_bracket_type, consumed) = self.collect_vector(tokens, i)?;
                    values.push(Value { val_type: ValueType::Vector(nested_values, nested_bracket_type) });
                    i += consumed;
                },
                Token::VectorEnd(bt) if *bt == bracket_type => {
                    return Ok((values, bracket_type.clone(), i - start_index + 1));
                },
                Token::Number(n) => {
                    values.push(Value { val_type: ValueType::Number(Fraction::from_str(n).map_err(AjisaiError::from)?) });
                    i += 1;
                },
                Token::String(s) => {
                    values.push(Value { val_type: ValueType::String(s.clone()) });
                    i += 1;
                },
                Token::Boolean(b) => {
                    values.push(Value { val_type: ValueType::Boolean(*b) });
                    i += 1;
                },
                Token::Nil => {
                    values.push(Value { val_type: ValueType::Nil });
                    i += 1;
                },
                Token::Symbol(s) => {
                    values.push(Value { val_type: ValueType::Symbol(s.clone()) });
                    i += 1;
                },
                _ => {
                    i += 1;
                }
            }
        }
        Err(AjisaiError::from(format!("Unclosed vector starting with {}", bracket_type.opening_char())))
    }

    fn execute_guard_structure(&mut self, tokens: &[Token]) -> Result<()> {
        let sections = self.split_by_guard_separator(tokens);
        
        if sections.is_empty() {
            return Ok(());
        }
        
        let indent = self.get_indent();
        writeln!(self.debug_buffer, "{}[GUARD] Found {} sections", indent, sections.len()).unwrap();

        // 条件が1つしかない場合（: のみ）
        if sections.len() == 2 {
            // 条件部を評価
            writeln!(self.debug_buffer, "{}[GUARD] Evaluating condition: {:?}", indent, sections[0]).unwrap();
            self.execute_tokens_sync(&sections[0])?;
            
            let condition = self.is_condition_true()?;
            writeln!(self.debug_buffer, "{}[GUARD] Condition result: {}", indent, condition).unwrap();
            
            if condition {
                // 真の場合：処理部を評価
                writeln!(self.debug_buffer, "{}[GUARD] Executing action: {:?}", indent, sections[1]).unwrap();
                self.execute_tokens_sync(&sections[1])?;
            }
            return Ok(());
        }
    
        // 複数条件の場合 - 各ペアをチェック
        let mut i = 0;
        while i + 1 < sections.len() {
            // 条件部を評価
            writeln!(self.debug_buffer, "{}[GUARD] Evaluating condition {}: {:?}", indent, i/2, sections[i]).unwrap();
            self.execute_tokens_sync(&sections[i])?;
            
            // スタックトップを評価
            let condition = self.is_condition_true()?;
            writeln!(self.debug_buffer, "{}[GUARD] Condition {} result: {}", indent, i/2, condition).unwrap();

            if condition {
                // 次のセクション（処理部）を評価
                if i + 1 < sections.len() {
                    writeln!(self.debug_buffer, "{}[GUARD] Executing action {}: {:?}", indent, i/2, sections[i+1]).unwrap();
                    self.execute_tokens_sync(&sections[i + 1])?;
                }
                return Ok(());
            }
            
            i += 2; // 条件と処理のペアをスキップ
        }
    
        // すべての条件が偽だった場合、最後のセクション（デフォルト処理）を評価
        if i < sections.len() {
            let default_tokens = sections.last().unwrap();
            writeln!(self.debug_buffer, "{}[GUARD] Executing default action: {:?}", indent, default_tokens).unwrap();
            self.execute_tokens_sync(default_tokens)?;
        } else {
             writeln!(self.debug_buffer, "{}[GUARD] No conditions met, no default action", indent).unwrap();
        }
    
        Ok(())
    }

    fn split_by_guard_separator(&self, tokens: &[Token]) -> Vec<Vec<Token>> {
        let mut sections = Vec::new();
        let mut current_section = Vec::new();
        
        for token in tokens {
            if matches!(token, Token::GuardSeparator) {
                sections.push(current_section);
                current_section = Vec::new();
            } else {
                current_section.push(token.clone());
            }
        }
        
        // 最後のセクションを追加（: がない場合や、: の後のセクション）
        sections.push(current_section);
        
        sections
    }


    fn is_condition_true(&mut self) -> Result<bool> {
        if self.stack.is_empty() {
            writeln!(self.debug_buffer, "{}  (Guard check: Stack empty -> FALSE)", self.get_indent()).unwrap();
            return Ok(false);
        }
        
        let top = self.stack.pop().unwrap();
        
        let result = match &top.val_type {
            ValueType::Boolean(b) => *b,
            ValueType::Vector(v, _) => {
                if v.len() == 1 {
                    if let ValueType::Boolean(b) = v[0].val_type {
                        b
                    } else {
                        true // ベクタが存在するなら真
                    }
                } else {
                    !v.is_empty()
                }
            },
            ValueType::Nil => false,
            _ => true,
        };
        
        writeln!(self.debug_buffer, "{}  (Guard check: Popped '{}' -> {})", self.get_indent(), top, result).unwrap();
        Ok(result)
    }

    pub(crate) fn execute_tokens_sync(&mut self, tokens: &[Token]) -> Result<()> {
        // ガードセパレータが含まれているかチェック
        if tokens.iter().any(|t| matches!(t, Token::GuardSeparator)) {
            return self.execute_guard_structure(tokens);
        }
        
        let indent = self.get_indent();
        
        let mut i = 0;
        while i < tokens.len() {
            let token = &tokens[i];
            
            // ★ トークン実行をデバッグログに出力
            writeln!(self.debug_buffer, "{}[EXEC] Token: {:?}", indent, token).unwrap();

            match token {
                Token::Number(n) => {
                    let val = Value { val_type: ValueType::Number(Fraction::from_str(n).map_err(AjisaiError::from)?) };
                    let vec_val = Value { val_type: ValueType::Vector(vec![val], BracketType::Square) };
                    writeln!(self.debug_buffer, "{}  -> Pushing literal: {}", indent, vec_val).unwrap();
                    self.stack.push(vec_val);
                },
                Token::String(s) => {
                    let val = Value { val_type: ValueType::String(s.clone()) };
                    writeln!(self.debug_buffer, "{}  -> Pushing literal: {}", indent, val).unwrap();
                    self.stack.push(val);
                },
                Token::Boolean(b) => {
                    let val = Value { val_type: ValueType::Boolean(*b) };
                    let vec_val = Value { val_type: ValueType::Vector(vec![val], BracketType::Square) };
                    writeln!(self.debug_buffer, "{}  -> Pushing literal: {}", indent, vec_val).unwrap();
                    self.stack.push(vec_val);
                },
                Token::Nil => {
                    let val = Value { val_type: ValueType::Nil };
                    let vec_val = Value { val_type: ValueType::Vector(vec![val], BracketType::Square) };
                    writeln!(self.debug_buffer, "{}  -> Pushing literal: {}", indent, vec_val).unwrap();
                    self.stack.push(vec_val);
                },
                Token::VectorStart(_) => {
                    let (values, bracket_type, consumed) = self.collect_vector(tokens, i)?;
                    let val = Value { val_type: ValueType::Vector(values, bracket_type) };
                    writeln!(self.debug_buffer, "{}  -> Pushing literal: {}", indent, val).unwrap();
                    self.stack.push(val);
                    i += consumed - 1;
                },
                Token::Symbol(name) => {
                    let upper_name = name.to_uppercase();
                    match upper_name.as_str() {
                        "STACK" => {
                            writeln!(self.debug_buffer, "{}  -> Setting target: STACK", indent).unwrap();
                            self.operation_target = OperationTarget::Stack;
                        }
                        "STACKTOP" => {
                             writeln!(self.debug_buffer, "{}  -> Setting target: STACKTOP", indent).unwrap();
                            self.operation_target = OperationTarget::StackTop;
                        }
                        _ => {
                            self.execute_word_sync(&upper_name)?;
                            // リセットは execute_word_sync の *後* で行う
                            self.operation_target = OperationTarget::StackTop;
                        }
                    }
                },
                Token::GuardSeparator => {
                    // 単独の場合は無視（execute_guard_structure で処理されるため、ここには来ないはず）
                },
                Token::LineBreak => {
                    // Top-levelでは無視
                },
                Token::VectorEnd(_) => {
                     // collect_vector で処理されるため、ここに来たら構文エラー
                    return Err(AjisaiError::from(format!("Unexpected vector end: {:?}", token)));
                }
            }
            i += 1;
        }
        Ok(())
    }

    pub(crate) fn execute_word_sync(&mut self, name: &str) -> Result<()> {
        let def = self.dictionary.get(name).cloned()
            .ok_or_else(|| AjisaiError::UnknownWord(name.to_string()))?;

        let indent = self.get_indent();
        
        writeln!(self.debug_buffer, "{}[CALL] --- Entering Word: {} ---", indent, name).unwrap();
        writeln!(self.debug_buffer, "{}  Stack before: {}", indent, self.format_stack_for_debug()).unwrap();
        
        self.call_stack_depth += 1;
        let result = if def.is_builtin {
            self.execute_builtin(name)
        } else {
            // カスタムワードの実行
            let mut execution_error: Option<AjisaiError> = None;
            for (i, line) in def.lines.iter().enumerate() {
                writeln!(self.debug_buffer, "{}[CUSTOM] Executing line {}: {:?}", self.get_indent(), i, line.body_tokens).unwrap();
                if let Err(e) = self.execute_tokens_sync(&line.body_tokens) {
                    execution_error = Some(e);
                    break; // エラーが発生したら即座にワードの実行を停止
                }
            }
            execution_error.map_or(Ok(()), Err)
        };
        self.call_stack_depth -= 1;
        
        // 実行結果をログに出力
        match &result {
            Ok(_) => {
                writeln!(self.debug_buffer, "{}  Stack after:  {}", indent, self.format_stack_for_debug()).unwrap();
                writeln!(self.debug_buffer, "{}[CALL] --- Exiting Word: {} (OK) ---", indent, name).unwrap();
            },
            Err(e) => {
                 writeln!(self.debug_buffer, "{}  Stack at error: {}", indent, self.format_stack_for_debug()).unwrap();
                 writeln!(self.debug_buffer, "{}[CALL] --- Exiting Word: {} (ERROR: {}) ---", indent, name, e).unwrap();
            }
        }
        
        // STACK/STACKTOP はワード実行後に必ず STACKTOP にリセット
        self.operation_target = OperationTarget::StackTop;

        result
    }

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
    match name {
        // 位置指定操作(0オリジン)
        "GET" => vector_ops::op_get(self),
        "INSERT" => vector_ops::op_insert(self),
        "REPLACE" => vector_ops::op_replace(self),
        "REMOVE" => vector_ops::op_remove(self),
        
        // 量指定操作(1オリジン)
        "LENGTH" => vector_ops::op_length(self),
        "TAKE" => vector_ops::op_take(self),
        
        // Vector構造操作
        "SPLIT" => vector_ops::op_split(self),
        "CONCAT" => vector_ops::op_concat(self),
        "REVERSE" => vector_ops::op_reverse(self),
        "LEVEL" => vector_ops::op_level(self),

        // 算術演算
        "+" => arithmetic::op_add(self),
        "-" => arithmetic::op_sub(self),
        "*" => arithmetic::op_mul(self),
        "/" => arithmetic::op_div(self),
        
        // 比較演算
        "=" => comparison::op_eq(self),
        "<" => comparison::op_lt(self),
        "<=" => comparison::op_le(self),
        ">" => comparison::op_gt(self),
        ">=" => comparison::op_ge(self),
        
        // 論理演算
        "AND" => comparison::op_and(self),
        "OR" => comparison::op_or(self),
        "NOT" => comparison::op_not(self),
        
        // 入出力
        "PRINT" => io::op_print(self),
        
        // カスタムワード管理
        "DEF" => dictionary::op_def(self),
        "DEL" => dictionary::op_del(self),
        "?" => dictionary::op_lookup(self),
        
        "RESET" => self.execute_reset(),
        
        "MAP" => higher_order::op_map(self),
        "FILTER" => higher_order::op_filter(self),
        
        // 制御
        "TIMES" => control::execute_times(self),
        "WAIT" => control::execute_wait(self),

        // 未定義の組み込みワード (get_builtin_definitions にはあるがここに追加し忘れたもの)
        _ => Err(AjisaiError::UnknownBuiltin(name.to_string())),
    }
}

    pub fn token_to_string(&self, token: &Token) -> String {
        match token {
            Token::Number(n) => n.clone(),
            Token::String(s) => format!("'{}'", s),
            Token::Boolean(true) => "TRUE".to_string(),
            Token::Boolean(false) => "FALSE".to_string(),
            Token::Symbol(s) => s.clone(),
            Token::Nil => "NIL".to_string(),
            Token::VectorStart(bt) => bt.opening_char().to_string(),
            Token::VectorEnd(bt) => bt.closing_char().to_string(),
            Token::GuardSeparator => ":".to_string(),
            Token::LineBreak => "\n".to_string(),
        }
    }
    
    pub fn get_word_definition_tokens(&self, name: &str) -> Option<String> {
        if let Some(def) = self.dictionary.get(name) {
            if !def.is_builtin && !def.lines.is_empty() {
                let mut result = String::new();
                for (i, line) in def.lines.iter().enumerate() {
                    if i > 0 { result.push('\n'); }
                    
                    let line_str = line.body_tokens.iter()
                        .map(|token| self.token_to_string(token))
                        .collect::<Vec<String>>()
                        .join(" ");
                    result.push_str(&line_str);
                }
                return Some(result.trim().to_string());
            } else if !def.is_builtin {
                 return Some("".to_string()); // 空の定義 [ ] の場合
            }
        }
        None
    }
    
    pub fn execute_reset(&mut self) -> Result<()> {
        self.stack.clear(); 
        self.dictionary.clear();
        self.dependents.clear();
        self.output_buffer.clear(); 
        self.debug_buffer.clear(); // ★ クリア
        self.definition_to_load = None;
        self.operation_target = OperationTarget::StackTop;
        self.call_stack_depth = 0; // ★ リセット
        crate::builtins::register_builtins(&mut self.dictionary);
        writeln!(self.debug_buffer, "[RESET] Interpreter reset to initial state.").unwrap();
        Ok(())
    }

    pub async fn execute(&mut self, code: &str) -> Result<()> {
        // ★ 実行前にバッファをクリア
        self.output_buffer.clear();
        self.debug_buffer.clear();
        self.call_stack_depth = 0;
        
        writeln!(self.debug_buffer, "[START] Executing code: {}", code).unwrap();
        
        let custom_word_names: HashSet<String> = self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect();
            
        let tokens = match crate::tokenizer::tokenize_with_custom_words(code, &custom_word_names) {
            Ok(t) => t,
            Err(e) => {
                writeln!(self.debug_buffer, "[ERROR] Tokenization failed: {}", e).unwrap();
                return Err(AjisaiError::from(e));
            }
        };
        
        writeln!(self.debug_buffer, "[TOKENS] {:?}", tokens).unwrap();
        
        let result = self.execute_tokens_sync(&tokens);
        
        match &result {
             Ok(_) => writeln!(self.debug_buffer, "[END] Execution finished OK.").unwrap(),
             Err(e) => writeln!(self.debug_buffer, "[END] Execution finished with ERROR: {}", e).unwrap(),
        },
        
        result
    }

    pub fn get_output(&mut self) -> String { std::mem::take(&mut self.output_buffer) }
    pub fn get_debug_output(&mut self) -> String { std::mem::take(&mut self.debug_buffer) } // ★ ゲッター
    pub fn get_stack(&self) -> &Stack { &self.stack }
    pub fn set_stack(&mut self, stack: Stack) { self.stack = stack; }

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

    pub fn init_step_execution(&mut self, code: &str) -> Result<()> {
        let custom_word_names: HashSet<String> = self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect();
        let _tokens = crate::tokenizer::tokenize_with_custom_words(code, &custom_word_names)?;
        Ok(())
    }

    pub fn execute_step(&mut self) -> Result<bool> {
        Ok(false)
    }

    pub fn get_step_info(&self) -> Option<(usize, usize)> {
        None
    }

    pub fn get_register(&self) -> Option<&Value> {
        None
    }

    pub fn set_register(&mut self, _value: Option<Value>) {
    }

    pub fn get_custom_words(&self) -> Vec<String> {
        self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect()
    }

    pub fn get_custom_words_with_descriptions(&self) -> Vec<(String, Option<String>)> {
        self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, def)| (name.clone(), def.description.clone()))
            .collect()
    }

    pub fn get_custom_words_info(&self) -> Vec<(String, Option<String>, bool)> {
        self.dictionary.iter()
            .map(|(name, def)| (name.clone(), def.description.clone(), def.is_builtin))
            .collect()
    }

    pub fn get_word_definition(&self, name: &str) -> Option<String> {
        self.get_word_definition_tokens(name)
    }
}
