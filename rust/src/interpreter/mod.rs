pub mod arithmetic;
pub mod control;
pub mod stack_ops;
// Other modules like io, vector_ops might need updating if used.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::{
    builtins::make_std_dictionary,
    stack::Stack,
    tokenizer::tokenize,
    types::{Token, Type},
};

pub struct Interpreter {
    pub stack: Stack,
    pub dictionary: crate::types::Dictionary,
    tokens: VecDeque<Token>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            stack: Stack::new(),
            dictionary: make_std_dictionary(),
            tokens: VecDeque::new(),
        }
    }

    pub fn run(&mut self, code: &str) -> Result<(), String> {
        let new_tokens = tokenize(code);
        self.tokens.extend(new_tokens);

        while let Some(token) = self.tokens.pop_front() {
            match token {
                Token::BlockStart => {
                    let block_tokens = self.collect_block_tokens();
                    self.stack.push(Type::Quotation(Rc::new(block_tokens)));
                }
                Token::VectorStart => {
                    let vector_body = self.collect_vector_as_data()?;
                    self.stack
                        .push(Type::Vector(Rc::new(RefCell::new(vector_body))));
                }
                Token::Number(val) => {
                    self.stack.push(Type::Number(Rc::new(val)));
                }
                Token::String(val) => {
                    self.stack.push(Type::String(Rc::new(val)));
                }
                Token::Word(word) => {
                    if let Some(word_def) = self.dictionary.get(&word).cloned() {
                        match word_def.as_ref() {
                            crate::types::Word::Builtin(f) => f(self)?,
                            crate::types::Word::UserDefined(tokens) => {
                                self.run_tokens(tokens.clone());
                            }
                        }
                    } else {
                        return Err(format!("Unknown word: {}", word));
                    }
                }
                Token::BlockEnd => return Err("Unexpected '}'".to_string()),
                Token::VectorEnd => return Err("Unexpected ']'".to_string()),
            }
        }
        Ok(())
    }
    
    fn collect_block_tokens(&mut self) -> Vec<Token> {
        let mut block_tokens = Vec::new();
        let mut depth = 1;
        while let Some(token) = self.tokens.pop_front() {
            match token {
                Token::BlockStart => {
                    depth += 1;
                    block_tokens.push(token);
                }
                Token::BlockEnd => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    block_tokens.push(token);
                }
                _ => {
                    block_tokens.push(token);
                }
            }
        }
        block_tokens
    }

    fn collect_vector_as_data(&mut self) -> Result<Vec<Type>, String> {
        let mut vector_contents: Vec<Type> = Vec::new();
        while let Some(token) = self.tokens.pop_front() {
            match token {
                Token::VectorEnd => return Ok(vector_contents),
                Token::VectorStart => {
                    let nested_vector = self.collect_vector_as_data()?;
                    vector_contents.push(Type::Vector(Rc::new(RefCell::new(nested_vector))));
                }
                Token::Number(val) => {
                    vector_contents.push(Type::Number(Rc::new(val)));
                }
                Token::String(val) => {
                    vector_contents.push(Type::String(Rc::new(val)));
                }
                Token::Word(name) => {
                    vector_contents.push(Type::Symbol(Rc::new(name)));
                }
                Token::BlockStart | Token::BlockEnd => {
                    return Err(
                        "Quotation `{}` cannot be placed in a data vector.".to_string(),
                    );
                }
            }
        }
        Err("Unclosed vector literal `[`.".to_string())
    }

    pub fn run_tokens(&mut self, tokens: Rc<Vec<Token>>) {
        for token in tokens.iter().rev() {
            self.tokens.push_front(token.clone());
        }
    }
}
