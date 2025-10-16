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

    // HELPER: Checks if the stack contains non-Vector values at the top level ("bare stack").
    fn is_bare_stack(&self) -> bool {
        self.stack.iter().any(|v| !matches!(v.val_type, ValueType::Vector(_, _)))
    }

    // HELPER: Wraps the stack in a single vector if it's bare.
    fn ensure_wrapped_stack(&mut self) {
        if self.is_bare_stack() {
            let items = std::mem::take(&mut self.stack);
            self.stack.push(Value {
                val_type: ValueType::Vector(items, BracketType::Square)
            });
        }
    }

    // [修正点1] ネストされた異なる括弧を正しく解釈できるように修正
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
                    // ベクトルリテラル内に現れるべきでないトークンはエラーとするか、無視する
                    // ここでは単純に進める
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

    // ガード構造のセクション収集
    fn collect_guard_sections(&self, tokens: &[Token], start: usize) -> Result<(Vec<Vec<Token>>, usize)> {
        let mut sections = Vec::new();
        let mut current_section = Vec::new();
        let mut i = start;
    
        while i < tokens.len() {
            match &tokens[i] {
                Token::GuardSeparator => {
                    sections.push(current_section);
                    current_section = Vec::new();
                }
                Token::LineBreak => {
                    // LineBreakはガード構造を終了させる
                    break;
                }
                _ => {
                    current_section.push(tokens[i].clone());
                }
            }
            i += 1;
        }
    
        if !current_section.is_empty() {
            sections.push(current_section);
        }
    
        Ok((sections, i - start))
    }

    // [修正点2] ガードが偽の条件を正しくスキップするように修正
    fn execute_guard_structure(&mut self, first_condition: Value, sections: &[Vec<Token>]) -> Result<()> {
        let is_true = match &first_condition.val_type {
            ValueType::Vector(v, _) if v.len() == 1 => match &v[0].val_type {
                ValueType::Boolean(b) => *b,
                _ => {
                    self.stack.push(first_condition);
                    return Err(AjisaiError::type_error("boolean condition", "other type"));
                }
            },
            _ => {
                self.stack.push(first_condition);
                return Err(AjisaiError::type_error("single-element vector with boolean", "other type"));
            }
        };

        if is_true {
            if !sections.is_empty() {
                self.execute_tokens_sync(&sections[0])?;
            }
            return Ok(());
        }

        // 最初の条件が偽の場合、残りのセクションを評価
        // sections: [action1, condition2, action2, condition3, action3, default_action]
        let mut section_idx = 1;
        while section_idx < sections.len() {
            if section_idx + 1 < sections.len() {
                // `condition, action` のペアがある場合
                self.execute_tokens_sync(&sections[section_idx])?;
                let cond = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
                
                let is_true = match &cond.val_type {
                    ValueType::Vector(v, _) if v.len() == 1 => match &v[0].val_type {
                        ValueType::Boolean(b) => *b,
                        _ => { self.stack.push(cond); return Err(AjisaiError::type_error("boolean condition", "other type")); }
                    },
                    _ => { self.stack.push(cond); return Err(AjisaiError::type_error("single-element vector with boolean", "other type")); }
                };

                if is_true {
                    self.execute_tokens_sync(&sections[section_idx + 1])?;
                    return Ok(());
                }
                section_idx += 2; // 次の `condition, action` ペアへ
            } else {
                // デフォルトアクション (else節)
                self.execute_tokens_sync(&sections[section_idx])?;
                return Ok(());
            }
        }
        
        Ok(())
    }

    pub(crate) fn execute_tokens_sync(&mut self, tokens: &[Token]) -> Result<()> {
        let mut i = 0;
        while i < tokens.len() {
            match &tokens[i] {
                Token::Number(n) => {
                    self.ensure_wrapped_stack(); // AUTO-NEST
                    self.stack.push(Value { val_type: ValueType::Number(Fraction::from_str(n).map_err(AjisaiError::from)?) });
                },
                Token::String(s) => {
                    self.ensure_wrapped_stack(); // AUTO-NEST
                    self.stack.push(Value { val_type: ValueType::String(s.clone()) });
                },
                Token::Boolean(b) => {
                    self.ensure_wrapped_stack(); // AUTO-NEST
                    self.stack.push(Value { val_type: ValueType::Boolean(*b) });
                },
                Token::Nil => {
                    self.ensure_wrapped_stack(); // AUTO-NEST
                    self.stack.push(Value { val_type: ValueType::Nil });
                },
                Token::VectorStart(_) => {
                    self.ensure_wrapped_stack(); // AUTO-NEST
                    let (values, bracket_type, consumed) = self.collect_vector(tokens, i)?;
                    self.stack.push(Value { val_type: ValueType::Vector(values, bracket_type) });
                    i += consumed - 1;
                },
                Token::DefBlockStart => {
                    self.ensure_wrapped_stack(); // AUTO-NEST
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
                            // MODIFIED: AUTO-NEST before executing a word on a bare stack
                            self.ensure_wrapped_stack();
                            self.execute_word_sync(&upper_name)?;
                            // Reset operation target after a word is executed
                            self.operation_target = OperationTarget::StackTop;
                        }
                    }
                },
                Token::GuardSeparator => {
                    let first_condition = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
                    let (sections, consumed) = self.collect_guard_sections(tokens, i + 1)?;
                    self.execute_guard_structure(first_condition, &sections)?;
                    i += consumed;
                },
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

        for line in &def.lines {
            if !line.condition_tokens.is_empty() {
                self.execute_tokens_sync(&line.condition_tokens)?;
                let condition_result = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
                
                let is_true = match &condition_result.val_type {
                    ValueType::Vector(v, _) if v.len() == 1 => match &v[0].val_type {
                        ValueType::Boolean(b) => *b,
                        _ => {
                            self.stack.push(condition_result);
                            return Err(AjisaiError::type_error("boolean condition", "other type"));
                        }
                    },
                    _ => {
                        self.stack.push(condition_result);
                        return Err(AjisaiError::type_error("single-element vector with boolean", "other type"));
                    }
                };
                
                if is_true {
                    self.execute_tokens_sync(&line.body_tokens)?;
                    break;
                } else {
                    continue;
                }
            } else {
                self.execute_tokens_sync(&line.body_tokens)?;
                break;
            }
        }
        Ok(())
    }

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        match name {
            // ベクトル操作
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
            
            // オーディオ
            "AUDIO" => audio::op_sound(self),
            
            // 制御構造
            "TIMES" => control::execute_times(self),
            "WAIT" => control::execute_wait(self),
            
            // ワード定義・管理
            "DEF" => dictionary::op_def(self),
            "DEL" => dictionary::op_del(self),
            "?" => dictionary::op_lookup(self),
            
            // システム
            "RESET" => self.execute_reset(),
            
            // 高階関数
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
            Token::GuardSeparator => ":".to_string(),
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
                    
                    if !line.condition_tokens.is_empty() {
                        for token in &line.condition_tokens {
                            result.push_str(&self.token_to_string(token));
                            result.push(' ');
                        }
                        result.push_str(": ");
                    }
                    
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
                for token in line.condition_tokens.iter().chain(line.body_tokens.iter()) {
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
}
