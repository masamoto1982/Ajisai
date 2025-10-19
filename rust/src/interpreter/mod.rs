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
    pub(crate) definition_to_load: Option<String>,
    pub(crate) operation_target: OperationTarget,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Interpreter {
            stack: Vec::new(),
            dictionary: HashMap::new(),
            dependents: HashMap::new(),
            output_buffer: String::new(),
            definition_to_load: None,
            operation_target: OperationTarget::StackTop,
        };
        crate::builtins::register_builtins(&mut interpreter.dictionary);
        interpreter
    }

    fn is_bare_stack(&self) -> bool {
        self.stack.iter().any(|v| !matches!(v.val_type, ValueType::Vector(_, _)))
    }

    fn ensure_wrapped_stack(&mut self) {
        if self.is_bare_stack() {
            let items = std::mem::take(&mut self.stack);
            self.stack.push(Value {
                val_type: ValueType::Vector(items, BracketType::Square)
            });
        }
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

    fn collect_def_block(&self, tokens: &[Token], start_index: usize) -> Result<(Vec<Token>, usize)> {
        let mut body_tokens = Vec::new();
        let mut i = start_index + 1;
        let mut depth = 1;

        while i < tokens.len() {
            match &tokens[i] {
                Token::DefBlockStart => {
                    depth += 1;
                    body_tokens.push(tokens[i].clone());
                },
                Token::DefBlockEnd => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok((body_tokens, i - start_index + 1));
                    }
                    body_tokens.push(tokens[i].clone());
                },
                _ => {
                    body_tokens.push(tokens[i].clone());
                }
            }
            i += 1;
        }
        Err(AjisaiError::from("Unclosed definition block"))
    }

    // [削除]
    // fn collect_guard_sections(...)
    // fn is_condition_true(...)
    // fn execute_guard_structure(...)

    pub(crate) fn execute_tokens_sync(&mut self, tokens: &[Token]) -> Result<()> {
        let mut i = 0;
        while i < tokens.len() {
            match &tokens[i] {
                Token::Number(n) => {
                    self.ensure_wrapped_stack();
                    self.stack.push(Value { val_type: ValueType::Number(Fraction::from_str(n).map_err(AjisaiError::from)?) });
                },
                Token::String(s) => {
                    self.ensure_wrapped_stack();
                    self.stack.push(Value { val_type: ValueType::String(s.clone()) });
                },
                Token::Boolean(b) => {
                    self.ensure_wrapped_stack();
                    self.stack.push(Value { val_type: ValueType::Boolean(*b) });
                },
                Token::Nil => {
                    self.ensure_wrapped_stack();
                    self.stack.push(Value { val_type: ValueType::Nil });
                },
                Token::VectorStart(_) => {
                    self.ensure_wrapped_stack();
                    let (values, bracket_type, consumed) = self.collect_vector(tokens, i)?;
                    self.stack.push(Value { val_type: ValueType::Vector(values, bracket_type) });
                    i += consumed - 1;
                },
                Token::DefBlockStart => {
                    self.ensure_wrapped_stack();
                    let (body_tokens, consumed) = self.collect_def_block(tokens, i)?;
                    self.stack.push(Value { val_type: ValueType::DefinitionBody(body_tokens) });
                    i += consumed - 1;
                },
                Token::Symbol(name) => {
                    let upper_name = name.to_uppercase();
                    match upper_name.as_str() {
                        "STACK" => self.operation_target = OperationTarget::Stack,
                        "STACKTOP" => self.operation_target = OperationTarget::StackTop,
                        _ => {
                            self.ensure_wrapped_stack();
                            self.execute_word_sync(&upper_name)?;
                            self.operation_target = OperationTarget::StackTop;
                        }
                    }
                },
                // [削除]
                // Token::GuardSeparator => { ... }
                Token::LineBreak => {
                    // Top-levelでは無視
                },
                _ => {}
            }
            i += 1;
        }
        Ok(())
    }

    pub(crate) fn execute_word_sync(&mut self, name: &str) -> Result<()> {
        let def = self.dictionary.get(name).cloned()
            .ok_or_else(|| AjisaiError::UnknownWord(name.to_string()))?;

        if def.is_builtin {
            let original_target = self.operation_target;
            
            if (self.stack.len() == 1 || self.is_bare_stack()) && self.operation_target == OperationTarget::Stack {
                self.operation_target = OperationTarget::StackTop;
            }
            
            let result = self.execute_builtin(name);
            
            self.operation_target = original_target;
            return result;
        }

        // [修正]
        // ガードのロジック (`if !line.condition_tokens.is_empty() ...`) を削除
        // 複数行の定義が実行されるように `break;` も削除
        for line in &def.lines {
            self.execute_tokens_sync(&line.body_tokens)?;
        }
        Ok(())
    }

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
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
            "LEVEL" => vector_ops::op_level(self),
            
            "+" => arithmetic::op_add(self),
            "-" => arithmetic::op_sub(self),
            "*" => arithmetic::op_mul(self),
            "/" => arithmetic::op_div(self),
            
            "=" => comparison::op_eq(self),
            "<" => comparison::op_lt(self),
            "<=" => comparison::op_le(self),
            ">" => comparison::op_gt(self),
            ">=" => comparison::op_ge(self),
            
            "AND" => comparison::op_and(self),
            "OR" => comparison::op_or(self),
            "NOT" => comparison::op_not(self),
            
            "PRINT" => io::op_print(self),
            "AUDIO" => audio::op_sound(self),
            
            "TIMES" => control::execute_times(self),
            "WAIT" => control::execute_wait(self),
            
            "DEF" => dictionary::op_def(self),
            "DEL" => dictionary::op_del(self),
            "?" => dictionary::op_lookup(self),
            
            "RESET" => self.execute_reset(),
            
            "MAP" => higher_order::op_map(self),
            "FILTER" => higher_order::op_filter(self),
            
            _ => Err(AjisaiError::UnknownWord(name.to_string())),
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
            // Token::GuardSeparator => ":".to_string(), // <--- 削除
            Token::DefBlockStart => "{".to_string(),
            Token::DefBlockEnd => "}".to_string(),
            Token::LineBreak => "\n".to_string(),
        }
    }
    
    pub fn get_word_definition_tokens(&self, name: &str) -> Option<String> {
        if let Some(def) = self.dictionary.get(name) {
            if !def.is_builtin && !def.lines.is_empty() {
                let mut result = String::new();
                for (i, line) in def.lines.iter().enumerate() {
                    if i > 0 { result.push('\n'); }
                    
                    // [修正]
                    // if !line.condition_tokens.is_empty() { ... } のブロックを削除
                    
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
        self.operation_target = OperationTarget::StackTop;
        crate::builtins::register_builtins(&mut self.dictionary);
        Ok(())
    }

    pub async fn execute(&mut self, code: &str) -> Result<()> {
        let custom_word_names: HashSet<String> = self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect();
        let tokens = crate::tokenizer::tokenize_with_custom_words(code, &custom_word_names)?;
        self.execute_tokens_sync(&tokens)
    }

    pub fn get_output(&mut self) -> String { std::mem::take(&mut self.output_buffer) }
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
                // [修正] line.condition_tokens を削除
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
