// rust/src/interpreter/mod.rs (暗黙のGOTO削除版)

pub mod vector_ops;
pub mod arithmetic;
pub mod control;
pub mod io;
pub mod error;

use std::collections::{HashMap, HashSet};
use crate::types::{Workspace, Token, Value, ValueType, BracketType};
use self::error::Result;

pub struct Interpreter {
    pub(crate) workspace: Workspace,
    pub(crate) dictionary: HashMap<String, WordDefinition>,
    pub(crate) dependencies: HashMap<String, HashSet<String>>,
    pub(crate) output_buffer: String,
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
            call_stack: Vec::new(),
        };
        
        crate::builtins::register_builtins(&mut interpreter.dictionary);
        interpreter
    }

    pub fn execute(&mut self, code: &str) -> Result<()> {
        self.append_output(&format!("DEBUG: execute() called with code: '{}'\n", code));
        
        let custom_word_names: HashSet<String> = self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect();
        
        let tokens = crate::tokenizer::tokenize_with_custom_words(code, &custom_word_names)
            .map_err(error::AjisaiError::from)?;
            
        self.append_output(&format!("DEBUG: All tokens: {:?}\n", tokens));
        
        if tokens.is_empty() {
            return Ok(());
        }

        // DEFパターンを探して処理
        if let Some((def_result, remaining_code)) = self.try_process_def_pattern_from_code(code, &tokens) {
            self.append_output("DEBUG: DEF pattern processing started\n");
            
            def_result?;
            
            if !remaining_code.trim().is_empty() {
                self.append_output(&format!("DEBUG: Executing remaining code: '{}'\n", remaining_code));
                self.execute(&remaining_code)?;
            } else {
                self.append_output("DEBUG: No remaining code to execute\n");
            }
            
            return Ok(());
        }

        self.append_output("DEBUG: No DEF pattern, executing tokens normally\n");
        self.execute_tokens(&tokens)
    }

    pub fn execute_reset(&mut self) -> Result<()> {
        self.append_output("*** EXECUTE_RESET CALLED ***\n");
        
        if let Some(window) = web_sys::window() {
            let event = web_sys::CustomEvent::new("ajisai-reset")
                .map_err(|_| error::AjisaiError::from("Failed to create reset event"))?;
            window.dispatch_event(&event)
                .map_err(|_| error::AjisaiError::from("Failed to dispatch reset event"))?;
        }
        
        self.workspace.clear();
        self.dictionary.clear();
        self.dependencies.clear();
        self.output_buffer.clear();
        self.call_stack.clear();
        
        crate::builtins::register_builtins(&mut self.dictionary);
        
        self.append_output("*** EXECUTE_RESET COMPLETED ***\n");
        Ok(())
    }

    pub fn execute_single_token(&mut self, token: &Token) -> Result<String> {
        self.append_output(&format!("*** EXECUTE_SINGLE_TOKEN: {:?} ***\n", token));
        self.output_buffer.clear();
        
        match token {
            Token::Number(num, den) => {
                let wrapped_value = Value {
                    val_type: ValueType::Vector(
                        vec![Value {
                            val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)),
                        }],
                        BracketType::Square
                    )
                };
                self.workspace.push(wrapped_value);
                Ok(format!("Pushed wrapped number: [{}]", if *den == 1 { num.to_string() } else { format!("{}/{}", num, den) }))
            },
            Token::String(s) => {
                let wrapped_value = Value {
                    val_type: ValueType::Vector(
                        vec![Value {
                            val_type: ValueType::String(s.clone()),
                        }],
                        BracketType::Square
                    )
                };
                self.workspace.push(wrapped_value);
                Ok(format!("Pushed wrapped string: ['{}']", s))
            },
            Token::Boolean(b) => {
                let wrapped_value = Value {
                    val_type: ValueType::Vector(
                        vec![Value {
                            val_type: ValueType::Boolean(*b),
                        }],
                        BracketType::Square
                    )
                };
                self.workspace.push(wrapped_value);
                Ok(format!("Pushed wrapped boolean: [{}]", b))
            },
            Token::Nil => {
                let wrapped_value = Value {
                    val_type: ValueType::Vector(
                        vec![Value {
                            val_type: ValueType::Nil,
                        }],
                        BracketType::Square
                    )
                };
                self.workspace.push(wrapped_value);
                Ok("Pushed wrapped nil: [nil]".to_string())
            },
            Token::FunctionComment(comment) => {
                Ok(format!("Skipped function comment: \"{}\"", comment))
            },
            Token::Colon => {
                Ok("Colon token (should not be executed alone)".to_string())
            },
            Token::LineBreak => {
                Ok("Line break token (should not be executed alone)".to_string())
            },
            Token::Symbol(name) => {
                self.execute_word(name)?;
                let output = self.get_output();
                if output.is_empty() {
                    Ok(format!("Executed word: {}", name))
                } else {
                    Ok(output)
                }
            },
            Token::VectorStart(_) => {
                Ok("Vector start token (incomplete)".to_string())
            },
            Token::VectorEnd(_) => {
                Ok("Vector end token (incomplete)".to_string())
            },
        }
    }

    fn try_process_def_pattern_from_code(&mut self, code: &str, tokens: &[Token]) -> Option<(Result<()>, String)> {
        self.append_output("*** TRY_PROCESS_DEF_PATTERN_FROM_CODE ***\n");
        
        let def_position = tokens.iter().rposition(|t| {
            if let Token::Symbol(s) = t {
                s == "DEF"
            } else {
                false
            }
        })?;
        
        self.append_output(&format!("*** DEF found at position: {} ***\n", def_position));
        
        if def_position >= 1 {
            if let Token::String(name) = &tokens[def_position - 1] {
                self.append_output(&format!("*** DEF word name: {} ***\n", name));
                
                let mut body_tokens = &tokens[..def_position - 1];
                
                let mut repeat_count = 1;
                if body_tokens.len() >= 2 {
                    if let (Token::Number(num, 1), Token::Symbol(s)) = (&body_tokens[0], &body_tokens[1]) {
                        if s == "REPEAT" {
                            repeat_count = *num;
                            body_tokens = &body_tokens[2..];
                        }
                    }
                }

                if body_tokens.is_empty() {
                    return Some((Err(error::AjisaiError::from("DEF requires a body")), String::new()));
                }
                
                self.append_output(&format!("*** DEF body tokens: {:?} ***\n", body_tokens));
                
                let multiline_def = self.parse_multiline_definition(body_tokens);
                
                let remaining_code = "".to_string(); // In this model, DEF consumes all code.
                
                let def_result = self.define_word_from_multiline(
                    name.clone(),
                    multiline_def,
                    repeat_count,
                );
                
                return Some((def_result, remaining_code));
            }
        }
        
        self.append_output("*** No DEF pattern found ***\n");
        None
    }

    fn parse_multiline_definition(&mut self, tokens: &[Token]) -> MultiLineDefinition {
        self.append_output("*** PARSE_MULTILINE_DEFINITION ***\n");
        
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
                _ => {
                    if let Token::Colon = token {
                        has_conditionals = true;
                    }
                    current_line.push(token.clone());
                }
            }
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        MultiLineDefinition {
            lines,
            has_conditionals,
        }
    }

    fn define_word_from_multiline(&mut self, name: String, multiline_def: MultiLineDefinition, repeat_count: i64) -> Result<()> {
        let name = name.to_uppercase();
        
        let executable_tokens = if multiline_def.has_conditionals {
            self.create_conditional_tokens(&multiline_def.lines)?
        } else {
            multiline_def.lines.into_iter().flatten().collect()
        };

        if let Some(existing) = self.dictionary.get(&name) {
            if existing.is_builtin {
                return Err(error::AjisaiError::from(format!("Cannot redefine builtin word: {}", name)));
            }
        }

        if self.dictionary.contains_key(&name) {
            if let Some(dependents) = self.dependencies.get(&name) {
                if !dependents.is_empty() {
                    let dependent_list: Vec<String> = dependents.iter().cloned().collect();
                    return Err(error::AjisaiError::ProtectedWord { 
                        name: name.clone(), 
                        dependents: dependent_list 
                    });
                }
            }
        }

        self.dictionary.insert(name.clone(), WordDefinition {
            tokens: executable_tokens,
            is_builtin: false,
            description: None,
            category: None,
            repeat_count,
        });

        self.append_output(&format!("Defined word: {}\n", name));
        Ok(())
    }

    fn create_conditional_tokens(&mut self, lines: &[Vec<Token>]) -> Result<Vec<Token>> {
        let mut conditional_lines = Vec::new();
        let mut default_line: Option<Vec<Token>> = None;

        for line in lines {
            if let Some(colon_pos) = line.iter().position(|t| matches!(t, Token::Colon)) {
                conditional_lines.push((line[..colon_pos].to_vec(), line[colon_pos + 1..].to_vec()));
            } else {
                if default_line.is_some() {
                    return Err(error::AjisaiError::from("Multiple default lines found"));
                }
                default_line = Some(line.clone());
            }
        }

        if let Some(default_action) = default_line {
            Ok(self.build_nested_if_select(&conditional_lines, &default_action))
        } else {
            Err(error::AjisaiError::from("A default line is mandatory in a conditional definition"))
        }
    }

    fn build_nested_if_select(&mut self, conditional_lines: &[(Vec<Token>, Vec<Token>)], default_action: &[Token]) -> Vec<Token> {
        if conditional_lines.is_empty() {
            return default_action.to_vec();
        }

        let (condition, action) = &conditional_lines[0];
        let remaining_lines = &conditional_lines[1..];
        
        let false_branch_action = self.build_nested_if_select(remaining_lines, default_action);
        
        let mut result = Vec::new();
        result.extend(condition.iter().cloned());
        
        result.push(Token::VectorStart(BracketType::Square));
        result.extend(action.iter().cloned());
        result.push(Token::VectorEnd(BracketType::Square));
        
        result.push(Token::VectorStart(BracketType::Square));
        result.extend(false_branch_action);
        result.push(Token::VectorEnd(BracketType::Square));
        
        result.push(Token::Symbol("IF_SELECT".to_string()));
        result
    }

    pub(crate) fn execute_tokens(&mut self, tokens: &[Token]) -> Result<()> {
        let mut i = 0;
        while i < tokens.len() {
            match &tokens[i] {
                Token::Number(num, den) => {
                    self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Number(crate::types::Fraction::new(*num, *den))}], BracketType::Square)});
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
                Token::VectorEnd(_) => return Err(error::AjisaiError::from("Unexpected vector end")),
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
                        return Err(error::AjisaiError::from(format!("Mismatched bracket types")));
                    }
                    return Ok((values, i - start + 1));
                },
                token => {
                    values.push(self.token_to_value(token)?);
                    i += 1;
                }
            }
        }
        
        Err(error::AjisaiError::from("Unclosed vector"))
    }

    fn token_to_value(&self, token: &Token) -> Result<Value> {
        match token {
            Token::Number(num, den) => Ok(Value { val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)) }),
            Token::String(s) => Ok(Value { val_type: ValueType::String(s.clone()) }),
            Token::Boolean(b) => Ok(Value { val_type: ValueType::Boolean(*b) }),
            Token::Nil => Ok(Value { val_type: ValueType::Nil }),
            Token::Symbol(s) => Ok(Value { val_type: ValueType::Symbol(s.clone()) }),
            _ => Err(error::AjisaiError::from("Cannot convert token to value")),
        }
    }

    pub(crate) fn get_word_dependencies(&self, word_name: &str) -> Option<Vec<String>> {
        self.dictionary.get(word_name).map(|def| {
            def.tokens.iter().filter_map(|token| {
                if let Token::Symbol(sym) = token {
                    if self.dictionary.contains_key(sym) && !self.is_builtin_word(sym) {
                        Some(sym.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }).collect()
        })
    }

    pub(crate) fn is_builtin_word(&self, name: &str) -> bool {
        self.dictionary.get(name).map(|def| def.is_builtin).unwrap_or(false)
    }

    fn execute_word(&mut self, name: &str) -> Result<()> {
        if let Some(def) = self.dictionary.get(name).cloned() {
            if def.is_builtin {
                self.execute_builtin(name)
            } else {
                self.call_stack.push(name.to_string());
                for _ in 0..def.repeat_count {
                    if let Err(e) = self.execute_tokens(&def.tokens) {
                        self.call_stack.pop();
                        return Err(e.with_context(&self.call_stack));
                    }
                }
                self.call_stack.pop();
                Ok(())
            }
        } else {
            Err(error::AjisaiError::UnknownWord(name.to_string()))
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
            _ => Err(error::AjisaiError::UnknownBuiltin(name.to_string())),
        }
    }

    pub fn get_output(&mut self) -> String {
        let output = self.output_buffer.clone();
        self.output_buffer.clear();
        output
    }
    
    pub(crate) fn append_output(&mut self, text: &str) {
        self.output_buffer.push_str(text);
    }
    
    pub fn get_workspace(&self) -> &Workspace { &self.workspace }
    
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
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, def)| {
                let protected = self.dependencies.get(name).map(|deps| !deps.is_empty()).unwrap_or(false);
                (name.clone(), def.description.clone(), protected)
            })
            .collect()
    }
   
    pub fn set_workspace(&mut self, workspace: Workspace) {
        self.workspace = workspace;
    }
    
    pub fn restore_custom_word(&mut self, name: String, tokens: Vec<Token>, description: Option<String>) -> Result<()> {
        let name = name.to_uppercase();
        
        if let Some(existing) = self.dictionary.get(&name) {
            if existing.is_builtin {
                return Err(error::AjisaiError::from(format!("Cannot restore builtin word: {}", name)));
            }
        }

        self.dictionary.insert(name, WordDefinition {
            tokens,
            is_builtin: false,
            description,
            category: None,
            repeat_count: 1,
        });

        Ok(())
    }
   
    pub fn get_word_definition(&self, name: &str) -> Option<String> {
        self.dictionary.get(name).and_then(|def| {
            if !def.is_builtin {
                let body = def.tokens.iter().map(|t| self.token_to_string(t)).collect::<Vec<_>>().join(" ");
                Some(format!("[ {} ]", body))
            } else {
                None
            }
        })
    }

    fn token_to_string(&self, token: &Token) -> String {
        match token {
            Token::Number(n, d) if *d == 1 => n.to_string(),
            Token::Number(n, d) => format!("{}/{}", n, d),
            Token::String(s) => format!("'{}'", s),
            Token::Boolean(b) => b.to_string(),
            Token::Nil => "nil".to_string(),
            Token::Symbol(s) => s.clone(),
            Token::VectorStart(bt) => bt.opening_char().to_string(),
            Token::VectorEnd(bt) => bt.closing_char().to_string(),
            Token::FunctionComment(c) => format!("\"{}\"", c),
            Token::Colon => ":".to_string(),
            Token::LineBreak => "\n".to_string(),
        }
    }
}
