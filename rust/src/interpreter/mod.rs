// rust/src/interpreter/mod.rs

pub mod vector_ops;
pub mod arithmetic;
pub mod control;
pub mod io;
pub mod error;
pub mod flow_control;

use std::collections::{HashMap, HashSet};
use crate::types::{Workspace, Token, Value, ValueType, BracketType, Fraction, WordDefinition};
use self::error::{Result, AjisaiError};
use std::thread;
use std::time::Duration;
use num_traits::Zero;

pub struct Interpreter {
    pub(crate) workspace: Workspace,
    pub(crate) dictionary: HashMap<String, WordDefinition>,
    pub(crate) output_buffer: String,
    pub(crate) execution_state: Option<WordExecutionState>, // For GOTO
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
                    let (body_tokens, consumed) = self.collect_def_block(tokens, i)?;
                    self.workspace.push(Value { val_type: ValueType::DefinitionBody(body_tokens) });
                    i += consumed - 1;
                }
                Token::Symbol(name) => self.execute_word(name)?,
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

        let mut state = WordExecutionState {
            program_counter: 0,
            repeat_counters: def.lines.iter().map(|line| line.repeat_count).collect(),
            word_name: name.to_string(),
            continue_loop: false,
        };

        while state.program_counter < def.lines.len() {
            let pc = state.program_counter;
            
            if state.repeat_counters[pc] <= 0 {
                state.program_counter += 1;
                continue;
            }
            state.repeat_counters[pc] -= 1;
            
            let line = &def.lines[pc];

            if !line.condition_tokens.is_empty() {
                self.execute_tokens(&line.condition_tokens)?;
                let condition_val = self.workspace.pop().ok_or(AjisaiError::WorkspaceUnderflow)?;
                if !is_truthy(&condition_val) {
                    state.program_counter += 1;
                    continue; 
                }
            }

            self.execution_state = Some(state);
            self.execute_tokens(&line.body_tokens)?;
            state = self.execution_state.take().unwrap();

            if line.delay_ms > 0 {
                thread::sleep(Duration::from_millis(line.delay_ms));
            }
            
            if state.continue_loop {
                state.continue_loop = false;
            } else if state.repeat_counters[pc] > 0 {
                // Stay on same line
            } else {
                state.program_counter += 1;
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
        let mut values = Vec::new(); let mut i = start + 1;
        let mut depth = 1;
        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorStart(_) => depth += 1,
                Token::VectorEnd(_) => {
                    depth -= 1;
                    if depth == 0 { return Ok((values, i - start + 1)); }
                },
                _ => { 
                    // This is a simplified logic that doesn't parse tokens into values for now
                 }
            }
            i += 1;
        }
        Err(AjisaiError::from("Unclosed vector"))
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
        // This needs to be re-implemented based on the new WordDefinition structure
        None
    }
    pub fn restore_custom_word(&mut self, _name: String, _tokens: Vec<Token>, _description: Option<String>) -> Result<()> {
        // This needs to be re-implemented based on the new WordDefinition structure
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
