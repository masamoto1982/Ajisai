pub mod stack_ops;
pub mod arithmetic;
pub mod vector_ops;
pub mod control;
pub mod io;
pub mod error;

use std::collections::{HashMap, HashSet};
use crate::types::{Value, ValueType, Stack, Register, Token};
use crate::tokenizer::tokenize;
use self::error::{AjisaiError, Result};

pub struct Interpreter {
    pub(crate) stack: Stack,
    pub(crate) register: Register,
    pub(crate) dictionary: HashMap<String, WordDefinition>,
    pub(crate) dependencies: HashMap<String, HashSet<String>>,
    pub(crate) call_stack: Vec<String>,
    step_tokens: Vec<Token>,
    step_position: usize,
    step_mode: bool,
    pub(crate) output_buffer: String,
}

#[derive(Clone)]
pub struct WordDefinition {
    pub tokens: Vec<Token>,
    pub is_builtin: bool,
    pub description: Option<String>,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Interpreter {
            stack: Vec::new(),
            register: None,
            dictionary: HashMap::new(),
            dependencies: HashMap::new(),
            call_stack: Vec::new(),
            step_tokens: Vec::new(),
            step_position: 0,
            step_mode: false,
            output_buffer: String::new(),
        };
        
        // ビルトイン登録を無効化（純粋なデータスタックとして動作）
        // crate::builtins::register_builtins(&mut interpreter.dictionary);
        interpreter
    }
    
    pub fn execute(&mut self, code: &str) -> Result<()> {
        let tokens = tokenize(code).map_err(AjisaiError::from)?;
        self.push_tokens_as_data(&tokens)
    }

    pub fn get_output(&mut self) -> String {
        let output = self.output_buffer.clone();
        self.output_buffer.clear();
        output
    }
    
    pub(crate) fn append_output(&mut self, text: &str) {
        self.output_buffer.push_str(text);
    }

    pub fn init_step_execution(&mut self, code: &str) -> Result<()> {
        self.step_tokens = tokenize(code).map_err(AjisaiError::from)?;
        self.step_position = 0;
        self.step_mode = true;
        Ok(())
    }

    pub fn execute_step(&mut self) -> Result<bool> {
        if !self.step_mode || self.step_position >= self.step_tokens.len() {
            self.step_mode = false;
            return Ok(false);
        }

        let token = self.step_tokens[self.step_position].clone();
        self.step_position += 1;

        self.push_single_token_as_data(&token)?;
        Ok(self.step_position < self.step_tokens.len())
    }

    pub fn get_step_info(&self) -> Option<(usize, usize)> {
        if self.step_mode {
            Some((self.step_position, self.step_tokens.len()))
        } else {
            None
        }
    }

    // トークンをデータとしてスタックに積む
    fn push_tokens_as_data(&mut self, tokens: &[Token]) -> Result<()> {
        for token in tokens {
            self.push_single_token_as_data(token)?;
        }
        Ok(())
    }

    fn push_single_token_as_data(&mut self, token: &Token) -> Result<()> {
        match token {
            Token::Description(_) => Ok(()), // コメントは無視
            Token::Number(num, den) => {
                self.stack.push(Value {
                    val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)),
                });
                Ok(())
            },
            Token::String(s) => {
                self.stack.push(Value {
                    val_type: ValueType::String(s.clone()),
                });
                Ok(())
            },
            Token::Boolean(b) => {
                self.stack.push(Value {
                    val_type: ValueType::Boolean(*b),
                });
                Ok(())
            },
            Token::Nil => {
                self.stack.push(Value {
                    val_type: ValueType::Nil,
                });
                Ok(())
            },
            Token::Symbol(s) => {
                // シンボルもデータとしてスタックに積む
                self.stack.push(Value {
                    val_type: ValueType::Symbol(s.clone()),
                });
                Ok(())
            },
            Token::VectorStart | Token::VectorEnd | 
            Token::BlockStart | Token::BlockEnd => {
                // デリミタもシンボルとして扱う
                let symbol_name = match token {
                    Token::VectorStart => "[",
                    Token::VectorEnd => "]",
                    Token::BlockStart => "{",
                    Token::BlockEnd => "}",
                    _ => unreachable!(),
                };
                self.stack.push(Value {
                    val_type: ValueType::Symbol(symbol_name.to_string()),
                });
                Ok(())
            }
        }
    }

    // アクセサメソッド群
    pub fn get_stack(&self) -> &Stack { &self.stack }
    pub fn get_register(&self) -> &Register { &self.register }
    
    pub fn get_custom_words(&self) -> Vec<String> {
        Vec::new() // カスタムワードは使用しない
    }
    
    pub fn get_custom_words_with_descriptions(&self) -> Vec<(String, Option<String>)> {
        Vec::new()
    }
   
    pub fn get_custom_words_info(&self) -> Vec<(String, Option<String>, bool)> {
        Vec::new()
    }

    pub fn set_stack(&mut self, stack: Stack) {
        self.stack = stack;
    }
   
    pub fn set_register(&mut self, register: Register) {
        self.register = register;
    }
   
    pub fn get_word_definition(&self, _name: &str) -> Option<String> {
        None // ワード定義は使用しない
    }
}
