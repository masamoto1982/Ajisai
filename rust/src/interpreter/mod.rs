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
    pub(crate) dependents: HashMap<String, HashSet<String>>, // キーのワードに依存しているワードのセット
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
            dependents: HashMap::new(),
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
        
        self.output_buffer.push_str(&format!("[DEBUG] Total tokens parsed: {}\n", tokens.len()));
        for (i, token) in tokens.iter().enumerate() {
            self.output_buffer.push_str(&format!("[DEBUG] Token {}: {:?}\n", i, token));
        }
        
        self.execute_tokens(&tokens)
    }

    pub fn execute_tokens(&mut self, tokens: &[Token]) -> Result<()> {
        let mut i = 0;
        while i < tokens.len() {
            let token = &tokens[i];
            self.output_buffer.push_str(&format!("[DEBUG] Processing token {}: {:?}\n", i, token));
            
            match token {
                Token::Number(s) => {
                    let frac = Fraction::from_str(s).map_err(AjisaiError::from)?;
                    self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Number(frac) }], BracketType::Square)});
                },
                Token::String(s) => {
                    self.output_buffer.push_str(&format!("[DEBUG] Pushing string to workspace: {}\n", s));
                    self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::String(s.clone()) }], BracketType::Square)});
                },
                Token::Boolean(b) => self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Boolean(*b) }], BracketType::Square)}),
                Token::Nil => self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Nil }], BracketType::Square)}),
                Token::VectorStart(bt) => {
                    let (values, consumed) = self.collect_vector(tokens, i)?;
                    self.workspace.push(Value { val_type: ValueType::Vector(values, bt.clone()) });
                    i += consumed - 1;
                },
                Token::DefBlockStart => {
                    let (body_tokens, consumed) = self.collect_def_block(tokens, i)?;
                    self.workspace.push(Value { val_type: ValueType::DefinitionBody(body_tokens) });
                    i += consumed - 1;
                },
                Token::Symbol(name) if name.to_uppercase() == "DEF" => {
                    if let Some(Token::String(word_name)) = tokens.get(i + 1) {
                        if let Some(body_val) = self.workspace.pop() {
                             if let ValueType::DefinitionBody(body_tokens) = body_val.val_type {
                                control::op_def_inner(self, &body_tokens, word_name)?;
                                i += 1; 
                             } else {
                                self.workspace.push(body_val); 
                                return Err(AjisaiError::from("DEF must be preceded by a definition block :...;"));
                             }
                        } else {
                            return Err(AjisaiError::from("DEF must be preceded by a definition block :...;"));
                        }
                    } else {
                        if self.workspace.len() >= 2 {
                             self.execute_builtin("DEF")?;
                        } else {
                             return Err(AjisaiError::from("DEF must be followed by a quoted name like 'WORD'"));
                        }
                    }
                },
                Token::Symbol(name) => {
                    self.execute_word(name)?;
                },
                _ => {} 
            }
            i += 1;
        }
        Ok(())
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
    
        let is_conditional = def.lines.iter().any(|line| !line.condition_tokens.is_empty());
    
        let selected_line_index = if is_conditional {
            let value_to_test = self.workspace.pop().ok_or(AjisaiError::WorkspaceUnderflow)?;
            self.output_buffer.push_str(&format!("[DEBUG] Popped value for condition test: {}\n", value_to_test));
            self.select_matching_line_conditional(&def.lines, &value_to_test)?
        } else {
            self.select_matching_line_default(&def.lines)?
        };
        
        if let Some(line_index) = selected_line_index {
            self.output_buffer.push_str(&format!("[DEBUG] Selected line {} for execution\n", line_index + 1));
            self.execute_selected_line(&def.lines[line_index])?;
        } else {
            return Err(AjisaiError::from("No matching condition found and no default line available"));
        }
    
        Ok(())
    }

    fn select_matching_line_conditional(&mut self, lines: &[ExecutionLine], value_to_test: &Value) -> Result<Option<usize>> {
        for (index, line) in lines.iter().enumerate() {
            if !line.condition_tokens.is_empty() {
                if self.evaluate_pattern_condition(&line.condition_tokens, value_to_test)? {
                    return Ok(Some(index));
                }
            }
        }
        Ok(lines.iter().position(|line| line.condition_tokens.is_empty()))
    }
    
    fn select_matching_line_default(&self, lines: &[ExecutionLine]) -> Result<Option<usize>> {
        Ok(lines.iter().position(|_| true))
    }

    fn evaluate_pattern_condition(&mut self, condition_tokens: &[Token], value_to_test: &Value) -> Result<bool> {
        let mut temp_interp = Interpreter {
            workspace: vec![value_to_test.clone()],
            dictionary: self.dictionary.clone(),
            dependents: HashMap::new(), // 評価用なので空で良い
            output_buffer: String::new(),
            execution_state: None,
        };
        
        temp_interp.execute_tokens(condition_tokens)?;

        if let Some(result_val) = temp_interp.workspace.pop() {
            Ok(is_truthy(&result_val))
        } else {
            Ok(false)
        }
    }
    
    fn execute_selected_line(&mut self, line: &ExecutionLine) -> Result<()> {
        self.output_buffer.push_str(&format!("[DEBUG] Executing line with {} repeats, {}ms delay\n", line.repeat_count, line.delay_ms));
        
        for iteration in 0..line.repeat_count {
            if iteration > 0 && line.delay_ms > 0 {
                self.output_buffer.push_str(&format!("[DEBUG] Waiting {}ms...\n", line.delay_ms));
                crate::wasm_sleep(line.delay_ms);
            }
            self.execute_tokens(&line.body_tokens)?;
        }
        
        Ok(())
    }

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        match name.to_uppercase().as_str() {
            "GET" => vector_ops::op_get(self), "INSERT" => vector_ops::op_insert(self),
            "REPLACE" => vector_ops::op_replace(self), "REMOVE" => vector_ops::op_remove(self),
            "LENGTH" => vector_ops::op_length(self), "TAKE" => vector_ops::op_take(self),
            "DROP" => vector_ops::op_drop_vector(self), "SPLIT" => vector_ops::op_split(self),
            "DUP" => vector_ops::op_dup_workspace(self), "SWAP" => vector_ops::op_swap_workspace(self),
            "ROT" => vector_ops::op_rot_workspace(self), "CONCAT" => vector_ops::op_concat(self),
            "REVERSE" => vector_ops::op_reverse(self),
            "+" => arithmetic::op_add(self), "-" => arithmetic::op_sub(self),
            "*" => arithmetic::op_mul(self), "/" => arithmetic::op_div(self),
            "=" => arithmetic::op_eq(self), "<" => arithmetic::op_lt(self),
            "<=" => arithmetic::op_le(self), ">" => arithmetic::op_gt(self),
            ">=" => arithmetic::op_ge(self), "AND" => arithmetic::op_and(self),
            "OR" => arithmetic::op_or(self), "NOT" => arithmetic::op_not(self),
            "PRINT" => io::op_print(self), "DEF" => control::op_def(self),
            "DEL" => control::op_del(self), "RESET" => self.execute_reset(),
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
                    if depth == 0 { return Ok((values, i - start + 1)); }
                },
                Token::Number(s) => values.push(Value { val_type: ValueType::Number(Fraction::from_str(s).map_err(AjisaiError::from)?) }),
                Token::String(s) => values.push(Value { val_type: ValueType::String(s.clone()) }),
                Token::Boolean(b) => values.push(Value { val_type: ValueType::Boolean(*b) }),
                Token::Nil => values.push(Value { val_type: ValueType::Nil }),
                Token::Symbol(name) => values.push(Value { val_type: ValueType::Symbol(name.clone()) }),
                _ => {}
            }
            i += 1;
        }
        Err(AjisaiError::from("Unclosed vector"))
    }

    pub fn get_output(&mut self) -> String { std::mem::take(&mut self.output_buffer) }
    pub fn get_workspace(&self) -> &Workspace { &self.workspace }
    pub fn set_workspace(&mut self, workspace: Workspace) { self.workspace = workspace; }
    pub fn get_custom_words_info(&self) -> Vec<(String, Option<String>)> {
        self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, def)| (name.clone(), def.description.clone()))
            .collect()
    }
    pub fn get_word_definition(&self, _name: &str) -> Option<String> { None }
    pub fn restore_custom_word(&mut self, _name: String, _tokens: Vec<Token>, _description: Option<String>) -> Result<()> { Ok(()) }
    pub fn execute_reset(&mut self) -> Result<()> {
        self.workspace.clear(); 
        self.dictionary.clear();
        self.dependents.clear();
        self.output_buffer.clear(); 
        self.execution_state = None;
        crate::builtins::register_builtins(&mut self.dictionary);
        Ok(())
    }
}

fn is_truthy(value: &Value) -> bool {
    if let ValueType::Vector(v, _) = &value.val_type {
        if v.len() == 1 {
            return match &v[0].val_type {
                ValueType::Boolean(b) => *b, ValueType::Nil => false,
                ValueType::Number(n) => !n.numerator.is_zero(),
                ValueType::String(s) => !s.is_empty(),
                ValueType::Vector(inner_v, _) => !inner_v.is_empty(),
                _ => true,
            }
        }
    }
    false
}
