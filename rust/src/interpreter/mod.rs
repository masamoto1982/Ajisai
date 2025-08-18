// rust/src/interpreter/mod.rs

pub mod stack_ops;
pub mod arithmetic;
pub mod vector_ops;
pub mod control;
pub mod io;
pub mod error;
pub mod quotation;
pub mod goto;

use std::collections::HashMap;
use crate::types::{Stack, Token, Value, ValueType};
use self::error::Result;

pub struct Interpreter {
    pub(crate) stack: Stack,
    pub(crate) dictionary: HashMap<String, WordDefinition>,
    pub(crate) output_buffer: String,
    pub(crate) labels: HashMap<String, usize>,
    pub(crate) program: Vec<Token>,
    pub(crate) pc: usize,
    pub(crate) call_stack: Vec<String>,
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
            dictionary: HashMap::new(),
            output_buffer: String::new(),
            labels: HashMap::new(),
            program: Vec::new(),
            pc: 0,
            call_stack: Vec::new(),
        };
        
        crate::builtins::register_builtins(&mut interpreter.dictionary);
        interpreter
    }

    pub fn execute(&mut self, code: &str) -> Result<()> {
        self.output_buffer.clear();
        
        let lines: Vec<&str> = code.split('\n')
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();

        for line in lines {
            self.process_line(line)?;
        }
        
        Ok(())
    }

    fn process_line(&mut self, line: &str) -> Result<()> {
        let tokens = crate::tokenizer::tokenize(line).map_err(error::AjisaiError::from)?;
        if tokens.is_empty() {
            return Ok(());
        }

        self.labels.clear();
        self.program = tokens.clone();
        
        // 第1パス: ラベル位置の記録
        for (i, token) in tokens.iter().enumerate() {
            if let Token::Label(label) = token {
                self.labels.insert(label.clone(), i);
            }
        }

        // 第2パス: 実行
        self.pc = 0;
        while self.pc < self.program.len() {
            self.execute_token()?;
        }

        Ok(())
    }

    fn execute_token(&mut self) -> Result<()> {
        let token = self.program[self.pc].clone();
        
        match token {
            Token::Number(num, den) => {
                self.stack.push(Value {
                    val_type: ValueType::Number(crate::types::Fraction::new(num, den)),
                });
                self.pc += 1;
            },
            Token::String(s) => {
                self.stack.push(Value {
                    val_type: ValueType::String(s),
                });
                self.pc += 1;
            },
            Token::Boolean(b) => {
                self.stack.push(Value {
                    val_type: ValueType::Boolean(b),
                });
                self.pc += 1;
            },
            Token::Nil => {
                self.stack.push(Value {
                    val_type: ValueType::Nil,
                });
                self.pc += 1;
            },
            Token::VectorStart => {
                let (vector_values, consumed) = self.collect_vector()?;
                self.stack.push(Value {
                    val_type: ValueType::Vector(vector_values),
                });
                self.pc += consumed;
            },
            Token::QuotationStart => {
                let (quotation_tokens, consumed) = self.collect_quotation()?;
                self.stack.push(Value {
                    val_type: ValueType::Quotation(quotation_tokens),
                });
                self.pc += consumed;
            },
            Token::Symbol(name) => {
                if name == "DEF" {
                    self.handle_def()?;
                } else {
                    self.execute_word(&name)?;
                }
                self.pc += 1;
            },
            Token::Label(_) => {
                self.pc += 1;
            },
            _ => {
                return Err(error::AjisaiError::from("Unexpected token"));
            }
        }

        Ok(())
    }

    fn collect_vector(&mut self) -> Result<(Vec<Value>, usize)> {
        let mut values = Vec::new();
        let mut i = self.pc + 1;
        let mut depth = 1;

        while i < self.program.len() && depth > 0 {
            match &self.program[i] {
                Token::VectorStart => depth += 1,
                Token::VectorEnd => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok((values, i - self.pc + 1));
                    }
                },
                token if depth == 1 => {
                    values.push(self.token_to_value(token)?);
                }
                _ => {} // ネストしたベクターの内部は別途処理
            }
            i += 1;
        }

        Err(error::AjisaiError::from("Unclosed vector"))
    }

    fn collect_quotation(&mut self) -> Result<(Vec<Token>, usize)> {
        let mut tokens = Vec::new();
        let mut i = self.pc + 1;
        let mut depth = 1;

        while i < self.program.len() && depth > 0 {
            match &self.program[i] {
                Token::QuotationStart => depth += 1,
                Token::QuotationEnd => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok((tokens, i - self.pc + 1));
                    }
                },
                token => {
                    tokens.push(token.clone());
                }
            }
            i += 1;
        }

        Err(error::AjisaiError::from("Unclosed quotation"))
    }

    fn token_to_value(&self, token: &Token) -> Result<Value> {
        match token {
            Token::Number(num, den) => Ok(Value {
                val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)),
            }),
            Token::String(s) => Ok(Value {
                val_type: ValueType::String(s.clone()),
            }),
            Token::Boolean(b) => Ok(Value {
                val_type: ValueType::Boolean(*b),
            }),
            Token::Nil => Ok(Value {
                val_type: ValueType::Nil,
            }),
            Token::Symbol(s) => Ok(Value {
                val_type: ValueType::Symbol(s.clone()),
            }),
            _ => Err(error::AjisaiError::from("Cannot convert token to value")),
        }
    }

    fn handle_def(&mut self) -> Result<()> {
        if self.stack.len() < 2 {
            return Err(error::AjisaiError::from("DEF requires quotation and name"));
        }

        let name_val = self.stack.pop().unwrap();
        let quotation_val = self.stack.pop().unwrap();

        let name = match name_val.val_type {
            ValueType::String(s) => s.to_uppercase(),
            _ => return Err(error::AjisaiError::from("DEF requires string name")),
        };

        let tokens = match quotation_val.val_type {
            ValueType::Quotation(t) => t,
            _ => return Err(error::AjisaiError::from("DEF requires quotation")),
        };

        let description = if self.pc + 1 < self.program.len() {
            if let Token::String(desc) = &self.program[self.pc + 1] {
                self.pc += 1;
                Some(desc.clone())
            } else {
                None
            }
        } else {
            None
        };

        if let Some(existing) = self.dictionary.get(&name) {
            if existing.is_builtin {
                return Err(error::AjisaiError::from(format!("Cannot redefine builtin word: {}", name)));
            }
        }

        self.dictionary.insert(name.clone(), WordDefinition {
            tokens,
            is_builtin: false,
            description: description.clone(),
        });

        if let Some(desc) = description {
            self.append_output(&format!("Defined: {} - {}\n", name, desc));
        } else {
            self.append_output(&format!("Defined: {}\n", name));
        }

        Ok(())
    }

    fn execute_word(&mut self, name: &str) -> Result<()> {
        if let Some(def) = self.dictionary.get(name).cloned() {
            if def.is_builtin {
                self.execute_builtin(name)
            } else {
                self.execute_custom_word(&def.tokens)
            }
        } else {
            Err(error::AjisaiError::UnknownWord(name.to_string()))
        }
    }

    fn execute_custom_word(&mut self, tokens: &[Token]) -> Result<()> {
        let old_program = self.program.clone();
        let old_pc = self.pc;
        let old_labels = self.labels.clone();

        self.program = tokens.to_vec();
        self.labels.clear();
        
        for (i, token) in tokens.iter().enumerate() {
            if let Token::Label(label) = token {
                self.labels.insert(label.clone(), i);
            }
        }

        self.pc = 0;
        while self.pc < self.program.len() {
            self.execute_token()?;
        }

        self.program = old_program;
        self.pc = old_pc;
        self.labels = old_labels;

        Ok(())
    }

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        match name {
            // スタック操作（レジスタ削除）
            "DUP" => stack_ops::op_dup(self),
            "DROP" => stack_ops::op_drop(self),
            "SWAP" => stack_ops::op_swap(self),
            "OVER" => stack_ops::op_over(self),
            "ROT" => stack_ops::op_rot(self),
            "NIP" => stack_ops::op_nip(self),
            
            // 算術・比較・論理
            "+" => arithmetic::op_add(self),
            "-" => arithmetic::op_sub(self),
            "*" => arithmetic::op_mul(self),
            "/" => arithmetic::op_div(self),
            ">" => arithmetic::op_gt(self),
            ">=" => arithmetic::op_ge(self),
            "=" => arithmetic::op_eq(self),
            "<" => arithmetic::op_lt(self),
            "<=" => arithmetic::op_le(self),
            "NOT" => arithmetic::op_not(self),
            "AND" => arithmetic::op_and(self),
            "OR" => arithmetic::op_or(self),
            
            // ベクトル操作
            "LENGTH" => vector_ops::op_length(self),
            "HEAD" => vector_ops::op_head(self),
            "TAIL" => vector_ops::op_tail(self),
            "CONS" => vector_ops::op_cons(self),
            "APPEND" => vector_ops::op_append(self),
            "REVERSE" => vector_ops::op_reverse(self),
            "NTH" => vector_ops::op_nth(self),
            "UNCONS" => vector_ops::op_uncons(self),
            "EMPTY?" => vector_ops::op_empty(self),
            
            // クオーテーション操作
            "CALL" => quotation::op_call(self),
            
            // GOTO操作
            "GOTO" => goto::op_goto(self),
            "J" => goto::op_jump_if(self),
            
            // 制御構造
            "DEL" => control::op_del(self),
            
            // Nil関連
            "NIL?" => arithmetic::op_nil_check(self),
            "NOT-NIL?" => arithmetic::op_not_nil_check(self),
            "KNOWN?" => arithmetic::op_not_nil_check(self),
            "DEFAULT" => arithmetic::op_default(self),
            
            // 入出力
            "." => io::op_dot(self),
            "PRINT" => io::op_print(self),
            "CR" => io::op_cr(self),
            "SPACE" => io::op_space(self),
            "SPACES" => io::op_spaces(self),
            "EMIT" => io::op_emit(self),
            
            // データベース操作
            "AMNESIA" => op_amnesia(self),
            
            _ => Err(error::AjisaiError::UnknownBuiltin(name.to_string())),
        }
    }

    pub fn get_output(&mut self) -> String {
        let output = self.output_buffer.clone();
        self.output_buffer.clear();
        output
    }
    
    pub(crate) fn append_output(&mut self, text: &str) {
        self.output_buffer.push_str(text);
    }
    
    pub fn get_stack(&self) -> &Stack { &self.stack }
    
    pub fn get_custom_words(&self) -> Vec<String> {
        self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect()
    }
    
    pub fn get_custom_words_with_descriptions(&self) -> Vec<(String, Option<String>)> {
        self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, def)| (name.clone(), def.description.clone()))
            .collect()
    }
   
    pub fn get_custom_words_info(&self) -> Vec<(String, Option<String>, bool)> {
        self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, def)| (name.clone(), def.description.clone(), false))
            .collect()
    }
   
    pub fn set_stack(&mut self, stack: Stack) {
        self.stack = stack;
    }

    pub fn restore_custom_word(&mut self, name: String, tokens: Vec<Token>, description: Option<String>) -> Result<()> {
        let name = name.to_uppercase();
        
        if let Some(existing) = self.dictionary.get(&name) {
            if existing.is_builtin {
                return Err(error::AjisaiError::from(format!("Cannot restore builtin word: {}", name)));
            }
        }

        self.dictionary.insert(name, WordDefinition {
            tokens,
            is_builtin: false,
            description,
        });

        Ok(())
    }
   
    pub fn get_word_definition(&self, name: &str) -> Option<String> {
        if let Some(def) = self.dictionary.get(name) {
            if !def.is_builtin {
                let body_string = def.tokens.iter()
                    .map(|token| self.token_to_string(token))
                    .collect::<Vec<String>>()
                    .join(" ");
                return Some(format!("{{ {} }}", body_string));
            }
        }
        None
    }

    fn token_to_string(&self, token: &Token) -> String {
        match token {
            Token::Number(n, d) => if *d == 1 { n.to_string() } else { format!("{}/{}", n, d) },
            Token::String(s) => format!("\"{}\"", s),
            Token::Boolean(b) => b.to_string(),
            Token::Nil => "nil".to_string(),
            Token::Symbol(s) => s.clone(),
            Token::VectorStart => "[".to_string(),
            Token::VectorEnd => "]".to_string(),
            Token::QuotationStart => "{".to_string(),
            Token::QuotationEnd => "}".to_string(),
            Token::Label(s) => format!("{}:", s),
        }
    }
}

pub fn op_amnesia(_interp: &mut Interpreter) -> Result<()> {
    if let Some(window) = web_sys::window() {
        let event = web_sys::CustomEvent::new("ajisai-amnesia")
            .map_err(|_| error::AjisaiError::from("Failed to create amnesia event"))?;
        window.dispatch_event(&event)
            .map_err(|_| error::AjisaiError::from("Failed to dispatch amnesia event"))?;
    }
    Ok(())
}
