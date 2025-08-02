// rust/src/interpreter/token_processor.rs

use crate::types::{Value, ValueType, Token};
use super::{Interpreter, error::{AjisaiError, Result}};
use wasm_bindgen::JsValue;
use web_sys::console;

impl Interpreter {
    pub(super) fn collect_vector_as_data(&self, tokens: &[Token]) -> Result<(Vec<Value>, usize)> {
        let mut values = Vec::new();
        let mut i = 1;

        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorEnd => return Ok((values, i + 1)),
                Token::VectorStart => {
                    let (nested_values, consumed) = self.collect_vector_as_data(&tokens[i..])?;
                    values.push(Value { val_type: ValueType::Vector(nested_values) });
                    i += consumed;
                    continue;
                },
                Token::Number(num, den) => values.push(Value { 
                    val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)) 
                }),
                Token::String(s) => values.push(Value { val_type: ValueType::String(s.clone()) }),
                Token::Boolean(b) => values.push(Value { val_type: ValueType::Boolean(*b) }),
                Token::Nil => values.push(Value { val_type: ValueType::Nil }),
                Token::Symbol(s) => values.push(Value { val_type: ValueType::Symbol(s.clone()) }),
                _ => {}
            }
            i += 1;
        }

        Err(AjisaiError::from("Unclosed vector"))
    }

    pub(super) fn collect_block_tokens(&self, tokens: &[Token], start_index: usize) -> Result<(Vec<Token>, usize)> {
        let mut block_tokens = Vec::new();
        let mut depth = 1;
        let mut i = start_index + 1;

        while i < tokens.len() {
            match &tokens[i] {
                Token::BlockStart => depth += 1,
                Token::BlockEnd => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok((block_tokens, i + 1));
                    }
                },
                _ => {}
            }
            block_tokens.push(tokens[i].clone());
            i += 1;
        }

        Err(AjisaiError::from("Unclosed block"))
    }

    pub(super) fn rearrange_tokens(&self, tokens: &[Token]) -> Vec<Token> {
        console::log_1(&JsValue::from_str("--- rearrange_tokens ---"));
        console::log_1(&JsValue::from_str(&format!("Input tokens: {:?}", tokens)));

        let mut literals = Vec::new();
        let mut value_producers = Vec::new();
        let mut value_consumers = Vec::new();
        let mut others = Vec::new();

        for token in tokens {
            match token {
                Token::Number(_, _) | Token::String(_) | Token::Boolean(_) | 
                Token::Nil | Token::VectorStart | Token::VectorEnd |
                Token::BlockStart | Token::BlockEnd => {
                    literals.push(token.clone());
                },
                Token::Symbol(name) => {
                    if let Some(prop) = self.word_properties.get(name) {
                        if prop.is_value_producer {
                            value_producers.push(token.clone());
                        } else {
                            value_consumers.push(token.clone());
                        }
                    } else if self.dictionary.contains_key(name) {
                        if self.check_if_value_producer(name) {
                            value_producers.push(token.clone());
                        } else {
                            value_consumers.push(token.clone());
                        }
                    } else {
                        others.push(token.clone());
                    }
                },
            }
        }

        let mut result = Vec::new();
        result.extend(literals);
        result.extend(value_producers);
        result.extend(value_consumers);
        result.extend(others);
        
        console::log_1(&JsValue::from_str(&format!("Output tokens (RPN): {:?}", result)));
        console::log_1(&JsValue::from_str("--- end rearrange_tokens ---"));
        result
    }

    pub(super) fn convert_to_rpn_structure(&self, tokens: &[Token]) -> Vec<Token> {
        let mut operator_positions = Vec::new();
        for (i, token) in tokens.iter().enumerate() {
            if let Token::Symbol(name) = token {
                if self.is_operator(name) {
                    operator_positions.push(i);
                }
            }
        }
        
        if operator_positions.is_empty() {
            return tokens.to_vec();
        }
        
        if operator_positions.len() == 1 {
            let op_pos = operator_positions[0];
            let op = &tokens[op_pos];
            
            // 前置記法: + a b
            if op_pos == 0 && tokens.len() >= 3 {
                if let Token::Symbol(op_name) = op {
                    if self.is_commutative_operator(op_name) {
                        if let Token::Symbol(name) = &tokens[2] {
                            if self.dictionary.contains_key(name) && !self.dictionary.get(name).unwrap().is_builtin {
                                return vec![tokens[2].clone(), tokens[1].clone(), op.clone()];
                            }
                        }
                    }
                }
                return vec![tokens[1].clone(), tokens[2].clone(), op.clone()];
            }
            // 中置記法: a + b
            else if op_pos == 1 && tokens.len() == 3 {
                if let Token::Symbol(op_name) = op {
                    if self.is_commutative_operator(op_name) {
                        if let Token::Symbol(name) = &tokens[2] {
                            if self.dictionary.contains_key(name) && !self.dictionary.get(name).unwrap().is_builtin {
                                match &tokens[0] {
                                    Token::Number(_, _) | Token::String(_) | Token::Boolean(_) | Token::Nil => {
                                        return vec![tokens[2].clone(), tokens[0].clone(), op.clone()];
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                return vec![tokens[0].clone(), tokens[2].clone(), op.clone()];
            }
            // 後置記法: a b +
            else if op_pos == 2 && tokens.len() == 3 {
                return tokens.to_vec();
            }
        }
        
        tokens.to_vec()
    }

    fn is_operator(&self, name: &str) -> bool {
        matches!(name, "+" | "-" | "*" | "/" | ">" | ">=" | "=" | "<" | "<=")
    }

    fn is_commutative_operator(&self, name: &str) -> bool {
        matches!(name, "+" | "*" | "=")
    }

    pub(super) fn token_to_string(&self, token: &Token) -> String {
        match token {
            Token::Number(n, d) => if *d == 1 { n.to_string() } else { format!("{}/{}", n, d) },
            Token::String(s) => format!("\"{}\"", s),
            Token::Boolean(b) => b.to_string(),
            Token::Nil => "nil".to_string(),
            Token::Symbol(s) => s.clone(),
            Token::VectorStart => "[".to_string(),
            Token::VectorEnd => "]".to_string(),
            Token::BlockStart => "{".to_string(),
            Token::BlockEnd => "}".to_string(),
        }
    }
}
