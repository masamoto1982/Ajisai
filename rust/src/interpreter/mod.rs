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

#[derive(Debug, Clone)]
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
        
        // 組み込みワードを登録
        crate::builtins::register_builtins(&mut interpreter.dictionary);
        
        interpreter
    }

    pub fn execute(&mut self, input: &str) -> Result<String> {
        self.output_buffer.clear();
        let tokens = tokenize(input)?;
        self.execute_tokens(&tokens)?;
        Ok(self.output_buffer.clone())
    }

    pub fn init_step(&mut self, input: &str) -> Result<()> {
        self.output_buffer.clear();
        self.step_tokens = tokenize(input)?;
        self.step_position = 0;
        self.step_mode = true;
        Ok(())
    }

    pub fn step(&mut self) -> Result<(String, bool)> {
        if !self.step_mode || self.step_position >= self.step_tokens.len() {
            self.step_mode = false;
            return Ok((self.output_buffer.clone(), false));
        }

        let token = &self.step_tokens[self.step_position].clone();
        self.execute_token(token)?;
        self.step_position += 1;

        let has_more = self.step_position < self.step_tokens.len();
        if !has_more {
            self.step_mode = false;
        }

        Ok((self.output_buffer.clone(), has_more))
    }

    pub fn reset_step(&mut self) {
        self.step_mode = false;
        self.step_tokens.clear();
        self.step_position = 0;
    }

    fn execute_tokens(&mut self, tokens: &[Token]) -> Result<()> {
        for token in tokens {
            self.execute_token(token)?;
        }
        Ok(())
    }

    pub fn execute_tokens_with_context(&mut self, tokens: &[Token]) -> Result<()> {
        self.execute_tokens(tokens)
    }

    fn execute_token(&mut self, token: &Token) -> Result<()> {
        match token {
            Token::Number(n) => {
                self.stack.push(Value { val_type: ValueType::Number(n.clone()) });
            },
            Token::String(s) => {
                self.stack.push(Value { val_type: ValueType::String(s.clone()) });
            },
            Token::Symbol(s) => {
                self.stack.push(Value { val_type: ValueType::Symbol(s.clone()) });
            },
            Token::Vector(tokens) => {
                let mut elements = Vec::new();
                for token in tokens {
                    match token {
                        Token::Number(n) => elements.push(Value { val_type: ValueType::Number(n.clone()) }),
                        Token::String(s) => elements.push(Value { val_type: ValueType::String(s.clone()) }),
                        Token::Symbol(s) => elements.push(Value { val_type: ValueType::Symbol(s.clone()) }),
                        Token::Vector(_) => {
                            // ネストしたベクトルの処理
                            self.execute_token(token)?;
                            if let Some(val) = self.stack.pop() {
                                elements.push(val);
                            }
                        },
                        Token::Word(w) => {
                            // ベクトル内のワードは実行せずにQuotationとして扱う
                            let quotation_tokens = vec![token.clone()];
                            elements.push(Value { val_type: ValueType::Quotation(quotation_tokens) });
                        },
                        Token::Nil => elements.push(Value { val_type: ValueType::Nil }),
                    }
                }
                self.stack.push(Value { val_type: ValueType::Vector(elements) });
            },
            Token::Word(word) => {
                if let Some(definition) = self.dictionary.get(word).cloned() {
                    if definition.is_builtin {
                        self.execute_builtin(word)?;
                    } else {
                        self.call_stack.push(word.clone());
                        let result = self.execute_tokens(&definition.tokens);
                        self.call_stack.pop();
                        result?;
                    }
                } else {
                    return Err(AjisaiError::from(format!("Unknown word: {}", word)));
                }
            },
            Token::Nil => {
                self.stack.push(Value { val_type: ValueType::Nil });
            },
        }
        Ok(())
    }

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        use self::{stack_ops::*, arithmetic::*, vector_ops::*, control::*, io::*};

        match name {
            // スタック操作
            "DUP" => op_dup(self),
            "DROP" => op_drop(self),
            "SWAP" => op_swap(self),
            "OVER" => op_over(self),
            "ROT" => op_rot(self),
            "NIP" => op_nip(self),
            
            // レジスタ操作
            ">R" => op_to_r(self),
            "R>" => op_r_from(self),
            "R@" => op_r_fetch(self),
            
            // 算術演算
            "+" => op_add(self),
            "-" => op_sub(self),
            "*" => op_mul(self),
            "/" => op_div(self),
            
            // 比較演算
            "=" => op_eq(self),
            ">" => op_gt(self),
            ">=" => op_gte(self),
            "<" => op_lt(self),
            "<=" => op_lte(self),
            
            // 論理演算
            "NOT" => op_not(self),
            "AND" => op_and(self),
            "OR" => op_or(self),
            
            // ベクトル操作
            "LENGTH" => op_length(self),
            "HEAD" => op_head(self),
            "TAIL" => op_tail(self),
            "CONS" => op_cons(self),
            "APPEND" => op_append(self),
            "REVERSE" => op_reverse(self),
            "NTH" => op_nth(self),
            "UNCONS" => op_uncons(self),
            "EMPTY?" => op_empty_check(self),
            
            // 制御構造
            "DEF" => op_def(self),
            "IF" => op_if(self),
            "CALL" => op_call(self),
            "DEL" => op_del(self),
            
            // Nil関連
            "NIL?" => op_nil_check(self),
            "NOT-NIL?" => op_not_nil_check(self),
            "KNOWN?" => op_not_nil_check(self),
            "DEFAULT" => op_default(self),
            
            "MATCH?" => op_match(self),
            "WILDCARD" => op_wildcard(self),
            
            // 入出力
            "." => op_dot(self),
            "PRINT" => op_print(self),
            "CR" => op_cr(self),
            "SPACE" => op_space(self),
            "SPACES" => op_spaces(self),
            "EMIT" => op_emit(self),
            
            _ => Err(AjisaiError::from(format!("Unknown builtin: {}", name))),
        }
    }

    pub fn append_output(&mut self, text: &str) {
        self.output_buffer.push_str(text);
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
                let is_protected = self.dependencies.values()
                    .any(|deps| deps.contains(name));
                (name.clone(), def.description.clone(), is_protected)
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