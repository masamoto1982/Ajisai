// rust/src/interpreter/step.rs

use crate::types::Token;
use crate::tokenizer::tokenize;
use super::{Interpreter, error::{AjisaiError, Result}};

impl Interpreter {
    pub fn init_step_execution(&mut self, code: &str) -> Result<()> {
        let lines: Vec<&str> = code.split('\n')
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();

        self.step_tokens.clear();
        for line in lines {
            let tokens = tokenize(line).map_err(AjisaiError::from)?;
            if !tokens.is_empty() {
                if !self.step_tokens.is_empty() {
                    self.step_tokens.push(Token::Symbol("__LINE_BREAK__".to_string()));
                }
                self.step_tokens.extend(tokens);
            }
        }
        
        self.step_position = 0;
        self.step_mode = true;
        Ok(())
    }

    pub fn execute_step(&mut self) -> Result<bool> {
        if !self.step_mode || self.step_position >= self.step_tokens.len() {
            self.step_mode = false;
            return Ok(false);
        }

        let mut line_tokens = Vec::new();
        while self.step_position < self.step_tokens.len() {
            let token = &self.step_tokens[self.step_position];
            if let Token::Symbol(name) = token {
                if name == "__LINE_BREAK__" {
                    self.step_position += 1;
                    break;
                }
            }
            line_tokens.push(token.clone());
            self.step_position += 1;
        }

        if line_tokens.is_empty() && self.step_position >= self.step_tokens.len() {
            self.step_mode = false;
            return Ok(false);
        }

        if !line_tokens.is_empty() {
            self.process_line_from_tokens(&line_tokens)?;
        }

        Ok(self.step_position < self.step_tokens.len())
    }

    pub fn get_step_info(&self) -> Option<(usize, usize)> {
        if self.step_mode {
            let total_lines = self.step_tokens.iter()
                .filter(|t| matches!(t, Token::Symbol(s) if s == "__LINE_BREAK__"))
                .count() + 1;
            
            let current_line = self.step_tokens[..self.step_position.min(self.step_tokens.len())]
                .iter()
                .filter(|t| matches!(t, Token::Symbol(s) if s == "__LINE_BREAK__"))
                .count() + 1;
            
            Some((current_line, total_lines))
        } else {
            None
        }
    }
}
