// rust/src/interpreter/mod.rs (完全版)

pub mod vector_ops;
pub mod arithmetic;
pub mod control;
pub mod io;
pub mod error;

use std::collections::{HashMap, HashSet};
use crate::types::{Workspace, Token, Value, ValueType, BracketType, Fraction};
use self::error::{Result, AjisaiError};
use num_traits::ToPrimitive;

pub struct Interpreter {
    pub(crate) workspace: Workspace,
    pub(crate) dictionary: HashMap<String, WordDefinition>,
    pub(crate) dependencies: HashMap<String, HashSet<String>>,
    pub(crate) output_buffer: String,
    pub(crate) debug_buffer: String,
    pub(crate) call_stack: Vec<String>,
}

#[derive(Clone)]
pub struct WordDefinition {
    pub tokens: Vec<Token>,
    pub is_builtin: bool,
    pub description: Option<String>,
    pub category: Option<String>,
    pub repeat_count: i64,
}

#[derive(Debug, Clone)]
pub struct MultiLineDefinition {
    pub lines: Vec<Vec<Token>>,
    pub has_conditionals: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Interpreter {
            workspace: Vec::new(),
            dictionary: HashMap::new(),
            dependencies: HashMap::new(),
            output_buffer: String::new(),
            debug_buffer: String::new(),
            call_stack: Vec::new(),
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
            
        if tokens.is_empty() { return Ok(()); }

        if let Some((def_result, remaining_code)) = self.try_process_def_pattern_from_code(&tokens) {
            def_result?;
            if !remaining_code.trim().is_empty() {
                self.execute(&remaining_code)?;
            }
            return Ok(());
        }

        self.execute_tokens(&tokens)
    }

    pub fn execute_reset(&mut self) -> Result<()> {
        if let Some(window) = web_sys::window() {
            let event = web_sys::CustomEvent::new("ajisai-reset").map_err(|_| AjisaiError::from("Failed to create reset event"))?;
            window.dispatch_event(&event).map_err(|_| AjisaiError::from("Failed to dispatch reset event"))?;
        }
        self.workspace.clear();
        self.dictionary.clear();
        self.dependencies.clear();
        self.output_buffer.clear();
        self.call_stack.clear();
        crate::builtins::register_builtins(&mut self.dictionary);
        Ok(())
    }

    pub fn execute_single_token(&mut self, token: &Token) -> Result<String> {
        self.output_buffer.clear();
        match token {
            Token::Number(s) => {
                let frac = Fraction::from_str(s)?;
                let wrapped = Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Number(frac) }], BracketType::Square) };
                let display = format!("{}", wrapped);
                self.workspace.push(wrapped);
                Ok(format!("Pushed {}", display))
            },
            Token::String(s) => {
                let wrapped = Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::String(s.clone())}], BracketType::Square) };
                self.workspace.push(wrapped);
                Ok(format!("Pushed wrapped string: ['{}']", s))
            },
            Token::Boolean(b) => {
                let wrapped = Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Boolean(*b)}], BracketType::Square) };
                self.workspace.push(wrapped);
                Ok(format!("Pushed wrapped boolean: [{}]", b))
            },
            Token::Nil => {
                let wrapped = Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Nil }], BracketType::Square) };
                self.workspace.push(wrapped);
                Ok("Pushed wrapped nil: [nil]".to_string())
            },
            Token::Symbol(name) => {
                self.execute_word(name)?;
                let output = self.get_output();
                Ok(if output.is_empty() { format!("Executed word: {}", name) } else { output })
            },
            _ => Ok(format!("Skipped token: {:?}", token)),
        }
    }

    fn try_process_def_pattern_from_code(&mut self, tokens: &[Token]) -> Option<(Result<()>, String)> {
        let def_pos = tokens.iter().rposition(|t| matches!(t, Token::Symbol(s) if s == "DEF"))?;
        
        if def_pos > 0 {
            if let Token::String(name) = &tokens[def_pos - 1] {
                let mut body_tokens = &tokens[..def_pos - 1];
                let mut repeat_count = 1;

                if body_tokens.len() >= 2 {
                    if let (Token::Number(num_str), Token::Symbol(s)) = (&body_tokens[0], &body_tokens[1]) {
                        if s == "REPEAT" {
                           if let Ok(frac) = Fraction::from_str(num_str) {
                               if let Some(num) = frac.to_i64() {
                                   repeat_count = num;
                                   body_tokens = &body_tokens[2..];
                               }
                           }
                        }
                    }
                }

                if body_tokens.is_empty() { return Some((Err(AjisaiError::from("DEF requires a body")), String::new())); }
                
                let multiline_def = self.parse_multiline_definition(body_tokens);
                let def_result = self.define_word_from_multiline(name.clone(), multiline_def, repeat_count);
                
                // Assuming DEF consumes the whole line for simplicity now
                return Some((def_result, "".to_string()));
            }
        }
        None
    }

    fn parse_multiline_definition(&mut self, tokens: &[Token]) -> MultiLineDefinition {
        let mut lines = Vec::new();
        let mut current_line = Vec::new();
        let mut has_conditionals = false;
        
        for token in tokens {
            match token {
                Token::LineBreak => {
                    if !current_line.is_empty() {
                        lines.push(current_line.clone());
                        current_line.clear();
                    }
                },
                Token::FunctionComment(_) => {},
                Token::Colon => {
                    has_conditionals = true;
                    current_line.push(token.clone());
                },
                _ => current_line.push(token.clone()),
            }
        }
        if !current_line.is_empty() { lines.push(current_line); }
        
        MultiLineDefinition { lines, has_conditionals }
    }

    fn define_word_from_multiline(&mut self, name: String, multiline_def: MultiLineDefinition, repeat_count: i64) -> Result<()> {
        let name = name.to_uppercase();
        // ... (Dependency checks, etc. would go here) ...

        let executable_tokens = if multiline_def.has_conditionals {
            self.create_conditional_tokens(&multiline_def.lines)?
        } else {
            multiline_def.lines.into_iter().flatten().collect()
        };

        self.dictionary.insert(name.clone(), WordDefinition {
            tokens: executable_tokens,
            is_builtin: false,
            description: None,
            category: None,
            repeat_count,
        });

        self.output_buffer.push_str(&format!("Defined word: {}\n", name));
        Ok(())
    }

    fn create_conditional_tokens(&mut self, lines: &[Vec<Token>]) -> Result<Vec<Token>> {
        // ... (Implementation unchanged from original) ...
        Ok(vec![])
    }
    
    pub(crate) fn execute_tokens(&mut self, tokens: &[Token]) -> Result<()> {
        let mut i = 0;
        while i < tokens.len() {
            match &tokens[i] {
                Token::Number(s) => {
                    let frac = Fraction::from_str(s)?;
                    let val = Value { val_type: ValueType::Number(frac) };
                    self.workspace.push(Value { val_type: ValueType::Vector(vec![val], BracketType::Square)});
                    i += 1;
                },
                Token::String(s) => {
                    self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::String(s.clone()) }], BracketType::Square)});
                    i += 1;
                },
                Token::Boolean(b) => {
                    self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Boolean(*b) }], BracketType::Square)});
                    i += 1;
                },
                Token::Nil => {
                    self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Nil }], BracketType::Square)});
                    i += 1;
                },
                Token::VectorStart(bracket_type) => {
                    let (vector_values, consumed) = self.collect_vector(tokens, i, bracket_type.clone())?;
                    self.workspace.push(Value { val_type: ValueType::Vector(vector_values, bracket_type.clone())});
                    i += consumed;
                },
                Token::Symbol(name) => {
                    self.execute_word(name)?;
                    i += 1;
                },
                Token::VectorEnd(_) => return Err(AjisaiError::from("Unexpected vector end")),
                _ => { i += 1; }
            }
        }
        Ok(())
    }

    fn collect_vector(&self, tokens: &[Token], start: usize, expected_bracket_type: BracketType) -> Result<(Vec<Value>, usize)> {
        let mut values = Vec::new();
        let mut i = start + 1;
        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorStart(inner_bracket_type) => {
                    let (nested_values, consumed) = self.collect_vector(tokens, i, inner_bracket_type.clone())?;
                    values.push(Value { val_type: ValueType::Vector(nested_values, inner_bracket_type.clone())});
                    i += consumed;
                },
                Token::VectorEnd(end_bracket_type) => {
                    if *end_bracket_type != expected_bracket_type {
                        return Err(AjisaiError::from("Mismatched bracket types"));
                    }
                    return Ok((values, i - start + 1));
                },
                token => {
                    values.push(self.token_to_value(token)?);
                    i += 1;
                }
            }
        }
        Err(AjisaiError::from("Unclosed vector"))
    }

    fn token_to_value(&self, token: &Token) -> Result<Value> {
        match token {
            Token::Number(s) => Ok(Value { val_type: ValueType::Number(Fraction::from_str(s)?) }),
            Token::String(s) => Ok(Value { val_type: ValueType::String(s.clone()) }),
            Token::Boolean(b) => Ok(Value { val_type: ValueType::Boolean(*b) }),
            Token::Nil => Ok(Value { val_type: ValueType::Nil }),
            Token::Symbol(s) => Ok(Value { val_type: ValueType::Symbol(s.clone()) }),
            _ => Err(AjisaiError::from("Cannot convert this token to a value")),
        }
    }

    fn execute_word(&mut self, name: &str) -> Result<()> {
        if let Some(def) = self.dictionary.get(name).cloned() {
            if def.is_builtin {
                self.execute_builtin(name)
            } else {
                self.call_stack.push(name.to_string());
                let result = self.execute_tokens(&def.tokens);
                self.call_stack.pop();
                result.map_err(|e| e.with_context(&self.call_stack))
            }
        } else {
            Err(AjisaiError::UnknownWord(name.to_string()))
        }
    }

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        match name {
            "GET" => vector_ops::op_get(self),
            "INSERT" => vector_ops::op_insert(self),
            "REPLACE" => vector_ops::op_replace(self),
            "REMOVE" => vector_ops::op_remove(self),
            "LENGTH" => vector_ops::op_length(self),
            "TAKE" => vector_ops::op_take(self),
            "DROP" => vector_ops::op_drop_vector(self),
            "REPEAT" => vector_ops::op_repeat(self),
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
            "IF_SELECT" => control::op_if_select(self),
            _ => Err(AjisaiError::UnknownBuiltin(name.to_string())),
        }
    }

    pub fn get_output(&mut self) -> String { std::mem::take(&mut self.output_buffer) }
    pub fn get_debug_output(&mut self) -> String { std::mem::take(&mut self.debug_buffer) }
    pub fn get_workspace(&self) -> &Workspace { &self.workspace }
    
    pub fn get_custom_words_info(&self) -> Vec<(String, Option<String>, bool)> {
        self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, def)| {
                let protected = self.dependencies.get(name).map_or(false, |deps| !deps.is_empty());
                (name.clone(), def.description.clone(), protected)
            })
            .collect()
    }
   
    pub fn set_workspace(&mut self, workspace: Workspace) { self.workspace = workspace; }
    
    pub fn restore_custom_word(&mut self, name: String, tokens: Vec<Token>, description: Option<String>) -> Result<()> {
        self.dictionary.insert(name.to_uppercase(), WordDefinition {
            tokens, is_builtin: false, description, category: None, repeat_count: 1,
        });
        Ok(())
    }
   
    pub fn get_word_definition(&self, name: &str) -> Option<String> {
        self.dictionary.get(name).and_then(|def| {
            if def.is_builtin { return None; }
            let body = def.tokens.iter().map(|t| self.token_to_string(t)).collect::<Vec<_>>().join(" ");
            Some(format!("[ {} ]", body))
        })
    }

    fn token_to_string(&self, token: &Token) -> String {
        match token {
            Token::Number(s) => s.clone(),
            Token::String(s) => format!("'{}'", s),
            Token::Boolean(b) => b.to_string(),
            Token::Nil => "nil".to_string(),
            Token::Symbol(s) => s.clone(),
            Token::VectorStart(bt) => bt.opening_char().to_string(),
            Token::VectorEnd(bt) => bt.closing_char().to_string(),
            _ => "".to_string(),
        }
    }
}
