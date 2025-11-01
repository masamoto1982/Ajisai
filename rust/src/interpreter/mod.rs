// rust/src/interpreter/mod.rs

pub mod error;
pub mod arithmetic;
pub mod comparison;
pub mod vector_ops;
pub mod higher_order;
pub mod io;
pub mod dictionary;
pub mod control;
pub mod audio;
pub mod bloom;  // ★ 新規：BLOOM機能

use std::collections::{HashMap, HashSet};
use crate::types::{Value, ValueType, WordDefinition, Token, ExecutionLine, GuardClause, GuardBranch, Stack};
use crate::builtins;
use error::{AjisaiError, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperationTarget {
    StackTop,
    Stack,
}

pub struct Interpreter {
    pub stack: Stack,
    pub dictionary: HashMap<String, WordDefinition>,
    pub dependents: HashMap<String, HashSet<String>>,
    pub operation_target: OperationTarget,
    pub output_buffer: String,
    pub debug_buffer: String,
    pub definition_to_load: Option<String>,
    call_stack: Vec<String>,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interp = Interpreter {
            stack: Vec::new(),
            dictionary: HashMap::new(),
            dependents: HashMap::new(),
            operation_target: OperationTarget::StackTop,
            output_buffer: String::new(),
            debug_buffer: String::new(),
            definition_to_load: None,
            call_stack: Vec::new(),
        };
        builtins::register_builtins(&mut interp.dictionary);
        interp
    }

    pub fn get_custom_word_names(&self) -> HashSet<String> {
        self.dictionary
            .iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// トークン列をValueに変換してスタックに積む（実行はしない）
    pub fn eval(&mut self, line: &str) -> Result<Vec<String>> {
        self.output_buffer.clear();
        self.debug_buffer.clear();
        
        let tokens = crate::tokenizer::tokenize_with_custom_words(
            line,
            &self.get_custom_word_names()
        ).map_err(|e| AjisaiError::ParseError(e))?;
        
        // トークン列をValueに変換
        let code_value = self.tokens_to_value(&tokens)?;
        
        // Vectorとしてスタックに積む（保護された状態）
        self.stack.push(code_value);
        
        Ok(vec![])
    }

    /// REPL/GUI用：自動的に1層BLOOMする
    pub fn eval_interactive(&mut self, line: &str) -> Result<Vec<String>> {
        self.eval(line)?;
        
        // 自動的にBLOOM
        bloom::op_bloom(self)?;
        
        let mut result = Vec::new();
        if !self.output_buffer.is_empty() {
            result.push(self.output_buffer.clone());
        }
        Ok(result)
    }

    /// トークン列をValueに変換
    fn tokens_to_value(&mut self, tokens: &[Token]) -> Result<Value> {
        let mut values = Vec::new();
        let mut i = 0;
        
        while i < tokens.len() {
            match &tokens[i] {
                Token::Number(s) => {
                    let frac = crate::types::fraction::Fraction::from_str(s)
                        .map_err(|e| AjisaiError::ParseError(e))?;
                    values.push(Value {
                        val_type: ValueType::Number(frac)
                    });
                }
                Token::String(s) => {
                    values.push(Value {
                        val_type: ValueType::String(s.clone())
                    });
                }
                Token::Boolean(b) => {
                    values.push(Value {
                        val_type: ValueType::Boolean(*b)
                    });
                }
                Token::Symbol(s) => {
                    values.push(Value {
                        val_type: ValueType::Symbol(s.clone())
                    });
                }
                Token::Nil => {
                    values.push(Value {
                        val_type: ValueType::Nil
                    });
                }
                Token::VectorStart(bracket_type) => {
                    let (nested_value, consumed) = self.parse_nested_vector(&tokens[i..], bracket_type.clone())?;
                    values.push(nested_value);
                    i += consumed;
                    continue;
                }
                Token::GuardSeparator => {
                    values.push(Value {
                        val_type: ValueType::GuardSeparator
                    });
                }
                Token::LineBreak => {
                    values.push(Value {
                        val_type: ValueType::LineBreak
                    });
                }
                _ => {}
            }
            i += 1;
        }
        
        Ok(Value {
            val_type: ValueType::Vector(values, crate::types::BracketType::Square)
        })
    }

    /// ネストしたVectorをパース
    fn parse_nested_vector(&mut self, tokens: &[Token], bracket_type: crate::types::BracketType) -> Result<(Value, usize)> {
        let mut values = Vec::new();
        let mut i = 1; // VectorStartの次から
        let mut depth = 1;
        
        while i < tokens.len() && depth > 0 {
            match &tokens[i] {
                Token::VectorStart(bt) if bt == &bracket_type => {
                    depth += 1;
                    let (nested, consumed) = self.parse_nested_vector(&tokens[i..], bt.clone())?;
                    values.push(nested);
                    i += consumed;
                    continue;
                }
                Token::VectorEnd(bt) if bt == &bracket_type => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                Token::Number(s) => {
                    let frac = crate::types::fraction::Fraction::from_str(s)
                        .map_err(|e| AjisaiError::ParseError(e))?;
                    values.push(Value { val_type: ValueType::Number(frac) });
                }
                Token::String(s) => {
                    values.push(Value { val_type: ValueType::String(s.clone()) });
                }
                Token::Boolean(b) => {
                    values.push(Value { val_type: ValueType::Boolean(*b) });
                }
                Token::Symbol(s) => {
                    values.push(Value { val_type: ValueType::Symbol(s.clone()) });
                }
                Token::Nil => {
                    values.push(Value { val_type: ValueType::Nil });
                }
                Token::GuardSeparator => {
                    values.push(Value { val_type: ValueType::GuardSeparator });
                }
                Token::LineBreak => {
                    values.push(Value { val_type: ValueType::LineBreak });
                }
                _ => {}
            }
            i += 1;
        }
        
        Ok((Value { val_type: ValueType::Vector(values, bracket_type) }, i + 1))
    }

    /// Valueの列を実行
    pub fn execute_values(&mut self, values: &[Value]) -> Result<()> {
        for value in values {
            bloom::process_value(self, value.clone())?;
        }
        Ok(())
    }

    /// ガード節を実行
    pub fn execute_guard_clause(&mut self, guard: &GuardClause) -> Result<()> {
        // 各条件分岐を評価
        for branch in &guard.branches {
            if self.evaluate_condition(&branch.condition)? {
                self.execute_values(&branch.action)?;
                return Ok(());
            }
        }
        
        // デフォルト行を実行
        self.execute_values(&guard.default)
    }

    /// 条件を評価して真偽値を返す
    fn evaluate_condition(&mut self, values: &[Value]) -> Result<bool> {
        let stack_size_before = self.stack.len();
        self.execute_values(values)?;
        
        if self.stack.len() <= stack_size_before {
            return Err(AjisaiError::from("Condition must produce a value"));
        }
        
        let result = self.stack.pop().unwrap();
        
        match result.val_type {
            ValueType::Vector(v, _) if v.len() == 1 => {
                match &v[0].val_type {
                    ValueType::Boolean(b) => Ok(*b),
                    ValueType::Nil => Ok(false),
                    _ => Ok(true),
                }
            }
            ValueType::Boolean(b) => Ok(b),
            ValueType::Nil => Ok(false),
            _ => Ok(true),
        }
    }

    pub fn execute_word_sync(&mut self, name: &str) -> Result<()> {
        let upper_name = name.to_uppercase();
        
        self.call_stack.push(upper_name.clone());
        
        let result = self.execute_word_internal(&upper_name);
        
        self.call_stack.pop();
        
        result.map_err(|e| e.with_context(&self.call_stack))
    }

    fn execute_word_internal(&mut self, name: &str) -> Result<()> {
        // 組み込みワードの実行
        match name {
            "BLOOM" => bloom::op_bloom(self),
            "STACKTOP" => {
                self.operation_target = OperationTarget::StackTop;
                Ok(())
            }
            "STACK" => {
                self.operation_target = OperationTarget::Stack;
                Ok(())
            }
            "+" => arithmetic::op_add(self),
            "-" => arithmetic::op_sub(self),
            "*" => arithmetic::op_mul(self),
            "/" => arithmetic::op_div(self),
            "=" => comparison::op_eq(self),
            "<" => comparison::op_lt(self),
            "<=" => comparison::op_le(self),
            ">" => comparison::op_gt(self),
            ">=" => comparison::op_ge(self),
            "NOT" => comparison::op_not(self),
            "AND" => comparison::op_and(self),
            "OR" => comparison::op_or(self),
            "GET" => vector_ops::op_get(self),
            "INSERT" => vector_ops::op_insert(self),
            "REPLACE" => vector_ops::op_replace(self),
            "REMOVE" => vector_ops::op_remove(self),
            "LENGTH" => vector_ops::op_length(self),
            "TAKE" => vector_ops::op_take(self),
            "SPLIT" => vector_ops::op_split(self),
            "CONCAT" => vector_ops::op_concat(self),
            "REVERSE" => vector_ops::op_reverse(self),
            "LEVEL" => vector_ops::op_level(self),
            "MAP" => higher_order::op_map(self),
            "FILTER" => higher_order::op_filter(self),
            "PRINT" => io::op_print(self),
            "CR" => io::op_cr(self),
            "SPACE" => io::op_space(self),
            "SPACES" => io::op_spaces(self),
            "EMIT" => io::op_emit(self),
            "SOUND" => audio::op_sound(self),
            "DEF" => dictionary::op_def(self),
            "DEL" => dictionary::op_del(self),
            "?" => dictionary::op_lookup(self),
            "TIMES" => control::execute_times(self),
            "WAIT" => control::execute_wait(self),
            _ => {
                // カスタムワードの実行
                if let Some(def) = self.dictionary.get(name).cloned() {
                    if def.is_builtin {
                        return Err(AjisaiError::UnknownBuiltin(name.to_string()));
                    }
                    
                    for line in &def.lines {
                        // ExecutionLineをValueに変換して実行
                        let values = self.tokens_to_values(&line.body_tokens)?;
                        self.execute_values(&values)?;
                    }
                    Ok(())
                } else {
                    Err(AjisaiError::UnknownWord(name.to_string()))
                }
            }
        }
    }

    fn tokens_to_values(&mut self, tokens: &[Token]) -> Result<Vec<Value>> {
        let mut values = Vec::new();
        let mut i = 0;
        
        while i < tokens.len() {
            match &tokens[i] {
                Token::Number(s) => {
                    let frac = crate::types::fraction::Fraction::from_str(s)
                        .map_err(|e| AjisaiError::ParseError(e))?;
                    values.push(Value { val_type: ValueType::Number(frac) });
                }
                Token::String(s) => {
                    values.push(Value { val_type: ValueType::String(s.clone()) });
                }
                Token::Boolean(b) => {
                    values.push(Value { val_type: ValueType::Boolean(*b) });
                }
                Token::Symbol(s) => {
                    values.push(Value { val_type: ValueType::Symbol(s.clone()) });
                }
                Token::Nil => {
                    values.push(Value { val_type: ValueType::Nil });
                }
                Token::VectorStart(bracket_type) => {
                    let (nested_value, consumed) = self.parse_nested_vector(&tokens[i..], bracket_type.clone())?;
                    values.push(nested_value);
                    i += consumed;
                    continue;
                }
                Token::GuardSeparator => {
                    values.push(Value { val_type: ValueType::GuardSeparator });
                }
                Token::LineBreak => {
                    values.push(Value { val_type: ValueType::LineBreak });
                }
                _ => {}
            }
            i += 1;
        }
        
        Ok(values)
    }

    pub fn get_word_definition_tokens(&self, name: &str) -> Option<String> {
        self.dictionary.get(name).map(|def| {
            def.lines
                .iter()
                .map(|line| {
                    line.body_tokens
                        .iter()
                        .map(|token| format!("{:?}", token))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}
