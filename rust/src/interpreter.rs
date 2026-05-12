//! Stack interpreter.
//!
//! The interpreter consumes tokens left-to-right. Numeric tokens are pushed
//! as continued-fraction `Value::Number`. Symbolic tokens are looked up first
//! as core words (built-ins), then as user words defined via `DEF`.
//!
//! There is no return stack: control flow is expressed purely by data stack
//! operations. As a single named auxiliary slot, the interpreter holds one
//! **Register** (the short-term memory of the brain metaphor). The Register
//! follows a caller-clobbers convention: a word that calls another word must
//! not assume the Register is preserved across the call. The Register is
//! initialised to Nil and reset to Nil by `RESET`.

use num_bigint::BigInt;
use num_traits::Zero;
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Truth {
    True,
    False,
    Unknown,
}

impl Truth {
    fn of(v: &Value) -> Self {
        match v {
            Value::Nil => Truth::Unknown,
            Value::Number(cf) => match cf.to_ratio() {
                None => Truth::Unknown,
                Some((p, _)) => {
                    if p.is_zero() {
                        Truth::False
                    } else {
                        Truth::True
                    }
                }
            },
        }
    }

    fn into_value(self) -> Value {
        match self {
            Truth::True => Value::Number(ContinuedFraction::from_int(BigInt::from(1))),
            Truth::False => Value::Number(ContinuedFraction::from_int(BigInt::from(0))),
            Truth::Unknown => Value::Nil,
        }
    }
}

pub struct Interpreter {
    stack: Vec<Value>,
    /// Single-slot Register (short-term memory). Nil represents "empty".
    register: Value,
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
            register: Value::Nil,
            user_words: HashMap::new(),
            user_word_order: Vec::new(),
            output: String::new(),
        }
    }

    pub fn stack(&self) -> &[Value] {
        &self.stack
    }

    pub fn register(&self) -> &Value {
        &self.register
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
        self.register = Value::Nil;
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
            // Arithmetic
            "+" | "ADD" => self.bin_arith(name, cf::add),
            "-" | "SUB" => self.bin_arith(name, cf::sub),
            "*" | "MUL" => self.bin_arith(name, cf::mul),
            "/" | "DIV" => self.bin_arith(name, cf::div),

            // Stack shuffles
            "DUP" => self.dup(),
            "DROP" => self.drop_top(),
            "SWAP" => self.swap(),
            "OVER" => self.over(),

            // Nil
            "NIL" => {
                self.stack.push(Value::Nil);
                Ok(())
            }
            "NIL?" => self.nil_q(),

            // Output
            "." => self.print_top(),

            // Register
            "STORE" | ">R" => self.register_store(),
            "RECALL" | "R>" => self.register_recall(),
            "PEEK" | "R@" => self.register_peek(),

            // Comparison (right-pointing inequalities are intentionally absent;
            // Ajisai mandates the "values increase from left to right" reading).
            "EQ" | "=" => self.cmp(name, |o| o == std::cmp::Ordering::Equal),
            "NE" | "<>" => self.cmp(name, |o| o != std::cmp::Ordering::Equal),
            "LT" | "<" => self.cmp(name, |o| o == std::cmp::Ordering::Less),
            "LE" | "<=" => self.cmp(name, |o| o != std::cmp::Ordering::Greater),

            // Three-valued logic (Kleene K3)
            "AND" | "&" => self.logic_and(name),
            "OR" | "|" => self.logic_or(name),
            "NOT" | "!" => self.logic_not(name),

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

    fn register_store(&mut self) -> Result<(), AjisaiError> {
        let top = self
            .stack
            .pop()
            .ok_or_else(|| AjisaiError::stack_underflow("STORE", 1, 0))?;
        self.register = top;
        Ok(())
    }

    fn register_recall(&mut self) -> Result<(), AjisaiError> {
        let v = std::mem::replace(&mut self.register, Value::Nil);
        self.stack.push(v);
        Ok(())
    }

    fn register_peek(&mut self) -> Result<(), AjisaiError> {
        self.stack.push(self.register.clone());
        Ok(())
    }

    fn cmp<F>(&mut self, name: &str, accept: F) -> Result<(), AjisaiError>
    where
        F: Fn(std::cmp::Ordering) -> bool,
    {
        if self.stack.len() < 2 {
            return Err(AjisaiError::stack_underflow(name, 2, self.stack.len()));
        }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        let result = match (&a, &b) {
            (Value::Number(x), Value::Number(y)) => {
                let (p1, q1) = x.to_ratio().expect("non-Nil CF must yield a ratio");
                let (p2, q2) = y.to_ratio().expect("non-Nil CF must yield a ratio");
                // Compare p1/q1 to p2/q2 as p1*q2 vs p2*q1 (q1, q2 > 0).
                let lhs = &p1 * &q2;
                let rhs = &p2 * &q1;
                let ord = lhs.cmp(&rhs);
                if accept(ord) {
                    Truth::True
                } else {
                    Truth::False
                }
            }
            _ => Truth::Unknown,
        };
        self.stack.push(result.into_value());
        Ok(())
    }

    fn logic_and(&mut self, name: &str) -> Result<(), AjisaiError> {
        if self.stack.len() < 2 {
            return Err(AjisaiError::stack_underflow(name, 2, self.stack.len()));
        }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        let ta = Truth::of(&a);
        let tb = Truth::of(&b);
        // Kleene K3 AND: False dominates Unknown; Unknown dominates True.
        let result = match (ta, tb) {
            (Truth::False, _) | (_, Truth::False) => Truth::False,
            (Truth::Unknown, _) | (_, Truth::Unknown) => Truth::Unknown,
            _ => Truth::True,
        };
        self.stack.push(result.into_value());
        Ok(())
    }

    fn logic_or(&mut self, name: &str) -> Result<(), AjisaiError> {
        if self.stack.len() < 2 {
            return Err(AjisaiError::stack_underflow(name, 2, self.stack.len()));
        }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        let ta = Truth::of(&a);
        let tb = Truth::of(&b);
        // Kleene K3 OR: True dominates Unknown; Unknown dominates False.
        let result = match (ta, tb) {
            (Truth::True, _) | (_, Truth::True) => Truth::True,
            (Truth::Unknown, _) | (_, Truth::Unknown) => Truth::Unknown,
            _ => Truth::False,
        };
        self.stack.push(result.into_value());
        Ok(())
    }

    fn logic_not(&mut self, name: &str) -> Result<(), AjisaiError> {
        let top = self
            .stack
            .pop()
            .ok_or_else(|| AjisaiError::stack_underflow(name, 1, 0))?;
        let result = match Truth::of(&top) {
            Truth::True => Truth::False,
            Truth::False => Truth::True,
            Truth::Unknown => Truth::Unknown,
        };
        self.stack.push(result.into_value());
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
