pub mod error;

use std::collections::HashMap;
use crate::types::{Value, Stack, Register, Token, WordDefinition};
use self::error::{AjisaiError, Result};

pub struct Interpreter {
    pub stack: Stack,
    pub register: Register,
    pub dictionary: HashMap<String, WordDefinition>,
    pub output_buffer: String,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Interpreter {
            stack: Vec::new(),
            register: None,
            dictionary: HashMap::new(),
            output_buffer: String::new(),
        };
        
        // 組み込みワードを登録
        crate::builtins::register_builtins(&mut interpreter.dictionary);
        
        interpreter
    }

    pub fn execute(&mut self, _input: &str) -> Result<()> {
        self.output_buffer.clear();
        self.output_buffer.push_str("OK");
        Ok(())
    }

    pub fn reset_output(&mut self) {
        self.output_buffer.clear();
    }

    pub fn get_output(&self) -> String {
        self.output_buffer.clone()
    }

    pub fn init_step(&mut self, _input: &str) -> Result<()> {
        self.output_buffer.clear();
        Ok(())
    }

    pub fn step(&mut self) -> Result<(String, bool)> {
        Ok((self.output_buffer.clone(), false))
    }

    pub fn get_stack(&self) -> &Stack {
        &self.stack
    }

    pub fn get_register(&self) -> &Register {
        &self.register
    }

    pub fn get_custom_words_info(&self) -> Vec<(String, Option<String>, bool)> {
        self.dictionary
            .iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, def)| {
                (name.clone(), def.description.clone(), false)
            })
            .collect()
    }

    pub fn get_word_definition(&self, name: &str) -> Option<Vec<Token>> {
        self.dictionary.get(name)
            .filter(|def| !def.is_builtin)
            .map(|def| def.tokens.clone())
    }

    pub fn restore_stack(&mut self, stack: Vec<Value>) {
        self.stack = stack;
    }

    pub fn restore_register(&mut self, register: Option<Value>) {
        self.register = register;
    }

    pub fn restore_word(&mut self, name: String, tokens: Vec<Token>, description: Option<String>) {
        self.dictionary.insert(name, WordDefinition {
            tokens,
            is_builtin: false,
            description,
        });
    }
}