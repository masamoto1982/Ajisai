//! Stack interpreter.
//!
//! The interpreter consumes tokens left-to-right. Numeric tokens are pushed
//! as continued-fraction `Value::Number`. Symbolic tokens are looked up first
//! as core words (built-ins), then as user words defined via `DEF`.
//!
//! The interpreter intentionally has *no* return stack: control flow is
//! expressed purely by data stack operations. This supports the VTU goal of
//! avoiding mid-computation memos.

use num_bigint::BigInt;
use std::collections::HashMap;

use crate::cf::{self, ContinuedFraction};
use crate::error::AjisaiError;
use crate::tokenizer::{tokenize, Token};
use crate::value::Value;

#[derive(Clone, Debug)]
pub struct UserWord {
    pub name: String,
    pub definition: String,
}

pub struct Interpreter {
    stack: Vec<Value>,
    user_words: HashMap<String, UserWord>,
    user_word_order: Vec<String>,
    output: String,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            user_words: HashMap::new(),
            user_word_order: Vec::new(),
            output: String::new(),
        }
    }

    pub fn stack(&self) -> &[Value] {
        &self.stack
    }

    pub fn user_words(&self) -> impl Iterator<Item = &UserWord> {
        self.user_word_order
            .iter()
            .filter_map(|n| self.user_words.get(n))
    }

    pub fn take_output(&mut self) -> String {
        std::mem::take(&mut self.output)
    }

    pub fn reset(&mut self) {
        self.stack.clear();
        self.output.clear();
    }

    pub fn execute(&mut self, code: &str) -> Result<(), AjisaiError> {
        let tokens = tokenize(code);
        let mut iter = tokens.into_iter().peekable();
        while let Some(tok) = iter.next() {
            match tok {
                Token::Integer(s) => self.push_int(&s)?,
                Token::Fraction(n, d) => self.push_fraction(&n, &d)?,
                Token::Decimal(s) => self.push_decimal(&s)?,
                Token::Symbol(sym) => {
                    let upper = sym.to_ascii_uppercase();
                    if upper == "DEF" {
                        // DEF expects: ... <name-symbol> <body-tokens-until-end-of-input>
                        // Phase 1 minimal: capture next symbol as the name, consume the
                        // rest of the token stream as the body source.
                        let name = match iter.next() {
                            Some(Token::Symbol(n)) => n,
                            _ => {
                                return Err(AjisaiError::new(
                                    "DEF requires a name symbol",
                                    "DEF must be followed by a symbol naming the new word.",
                                    "Write `DEF NEW-WORD body-tokens` on a single execution line.",
                                ))
                            }
                        };
                        let mut body = String::new();
                        for rest in iter.by_ref() {
                            if !body.is_empty() {
                                body.push(' ');
                            }
                            body.push_str(&token_source(&rest));
                        }
                        let name_up = name.to_ascii_uppercase();
                        let user = UserWord {
                            name: name_up.clone(),
                            definition: body,
                        };
                        if !self.user_words.contains_key(&name_up) {
                            self.user_word_order.push(name_up.clone());
                        }
                        self.user_words.insert(name_up, user);
                        return Ok(());
                    }
                    if upper == "DEL" {
                        let name = match iter.next() {
                            Some(Token::Symbol(n)) => n,
                            _ => {
                                return Err(AjisaiError::new(
                                    "DEL requires a name symbol",
                                    "DEL must be followed by the symbol of the word to remove.",
                                    "Write `DEL WORD-NAME`.",
                                ))
                            }
                        };
                        let name_up = name.to_ascii_uppercase();
                        self.user_words.remove(&name_up);
                        self.user_word_order.retain(|n| n != &name_up);
                        continue;
                    }
                    self.dispatch(&upper)?;
                }
            }
        }
        Ok(())
    }

    fn dispatch(&mut self, name: &str) -> Result<(), AjisaiError> {
        match name {
            "+" | "ADD" => self.bin_arith(name, cf::add),
            "-" | "SUB" => self.bin_arith(name, cf::sub),
            "*" | "MUL" => self.bin_arith(name, cf::mul),
            "/" | "DIV" => self.bin_arith(name, cf::div),
            "DUP" => self.dup(),
            "DROP" => self.drop_top(),
            "SWAP" => self.swap(),
            "OVER" => self.over(),
            "NIL" => {
                self.stack.push(Value::Nil);
                Ok(())
            }
            "NIL?" => self.nil_q(),
            "." => self.print_top(),
            other => {
                if let Some(uw) = self.user_words.get(other).cloned() {
                    return self.execute(&uw.definition);
                }
                Err(AjisaiError::unknown_word(other))
            }
        }
    }

    fn push_int(&mut self, s: &str) -> Result<(), AjisaiError> {
        let n: BigInt = s
            .parse()
            .map_err(|_| AjisaiError::parse_error(s))?;
        self.stack.push(Value::Number(ContinuedFraction::from_int(n)));
        Ok(())
    }

    fn push_fraction(&mut self, num: &str, den: &str) -> Result<(), AjisaiError> {
        let n: BigInt = num.parse().map_err(|_| AjisaiError::parse_error(num))?;
        let d: BigInt = den.parse().map_err(|_| AjisaiError::parse_error(den))?;
        let cf = ContinuedFraction::from_ratio(n, d);
        let v = if cf.is_nil() {
            Value::Nil
        } else {
            Value::Number(cf)
        };
        self.stack.push(v);
        Ok(())
    }

    fn push_decimal(&mut self, s: &str) -> Result<(), AjisaiError> {
        let cf = ContinuedFraction::from_decimal_str(s).ok_or_else(|| AjisaiError::parse_error(s))?;
        self.stack.push(Value::Number(cf));
        Ok(())
    }

    fn bin_arith<F>(&mut self, name: &str, op: F) -> Result<(), AjisaiError>
    where
        F: Fn(&ContinuedFraction, &ContinuedFraction) -> ContinuedFraction,
    {
        if self.stack.len() < 2 {
            return Err(AjisaiError::stack_underflow(name, 2, self.stack.len()));
        }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        match (a, b) {
            (Value::Number(x), Value::Number(y)) => {
                let r = op(&x, &y);
                if r.is_nil() {
                    self.stack.push(Value::Nil);
                } else {
                    self.stack.push(Value::Number(r));
                }
            }
            _ => {
                self.stack.push(Value::Nil);
            }
        }
        Ok(())
    }

    fn dup(&mut self) -> Result<(), AjisaiError> {
        if let Some(top) = self.stack.last().cloned() {
            self.stack.push(top);
            Ok(())
        } else {
            Err(AjisaiError::stack_underflow("DUP", 1, 0))
        }
    }

    fn drop_top(&mut self) -> Result<(), AjisaiError> {
        if self.stack.pop().is_some() {
            Ok(())
        } else {
            Err(AjisaiError::stack_underflow("DROP", 1, 0))
        }
    }

    fn swap(&mut self) -> Result<(), AjisaiError> {
        if self.stack.len() < 2 {
            return Err(AjisaiError::stack_underflow("SWAP", 2, self.stack.len()));
        }
        let n = self.stack.len();
        self.stack.swap(n - 1, n - 2);
        Ok(())
    }

    fn over(&mut self) -> Result<(), AjisaiError> {
        if self.stack.len() < 2 {
            return Err(AjisaiError::stack_underflow("OVER", 2, self.stack.len()));
        }
        let v = self.stack[self.stack.len() - 2].clone();
        self.stack.push(v);
        Ok(())
    }

    fn nil_q(&mut self) -> Result<(), AjisaiError> {
        let top = self
            .stack
            .pop()
            .ok_or_else(|| AjisaiError::stack_underflow("NIL?", 1, 0))?;
        let result = if top.is_nil() {
            ContinuedFraction::from_int(BigInt::from(1))
        } else {
            ContinuedFraction::from_int(BigInt::from(0))
        };
        self.stack.push(Value::Number(result));
        Ok(())
    }

    fn print_top(&mut self) -> Result<(), AjisaiError> {
        let top = self
            .stack
            .pop()
            .ok_or_else(|| AjisaiError::stack_underflow(".", 1, 0))?;
        if !self.output.is_empty() {
            self.output.push('\n');
        }
        self.output.push_str(&top.rational_display());
        Ok(())
    }
}

fn token_source(t: &Token) -> String {
    match t {
        Token::Integer(s) => s.clone(),
        Token::Fraction(n, d) => format!("{}/{}", n, d),
        Token::Decimal(s) => s.clone(),
        Token::Symbol(s) => s.clone(),
    }
}
