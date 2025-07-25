pub mod stack_ops;
pub mod arithmetic;
pub mod vector_ops;
pub mod control;
// pub mod database; // テーブル機能完成後に再有効化予定
pub mod io;
pub mod error;

use std::collections::{HashMap, HashSet};
use crate::types::{Value, ValueType, Stack, StackEntry, Register, Token}; // StackEntry を追加
use crate::tokenizer::tokenize;
use self::error::{AjisaiError, Result};

const STACK_TIMEOUT_SECONDS: u64 = 200; // タイムアウト時間の定数

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
        
        crate::builtins::register_builtins(&mut interpreter.dictionary);
        interpreter
    }
    
    // タイムアウトした要素を削除
    pub fn cleanup_expired_stack_entries(&mut self) {
        self.stack.retain(|entry| !entry.is_expired(STACK_TIMEOUT_SECONDS));
    }
    
    // スタックに値をプッシュ（タイムスタンプ付き）
    pub(crate) fn push_value(&mut self, value: Value) {
        self.stack.push(StackEntry::new(value));
    }
    
    // スタックから値をポップ
    pub(crate) fn pop_value(&mut self) -> Option<Value> {
        self.stack.pop().map(|entry| entry.value)
    }
    
    // スタックのトップを参照
    pub(crate) fn peek_value(&self) -> Option<&Value> {
        self.stack.last().map(|entry| &entry.value)
    }
    
    pub fn execute(&mut self, code: &str) -> Result<()> {
        let tokens = tokenize(code).map_err(AjisaiError::from)?;
        self.execute_tokens_with_context(&tokens)
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

        match self.execute_single_token(&token) {
            Ok(_) => Ok(self.step_position < self.step_tokens.len()),
            Err(e) => {
                self.step_mode = false;
                Err(e)
            }
        }
    }

    pub fn get_step_info(&self) -> Option<(usize, usize)> {
        if self.step_mode {
            Some((self.step_position, self.step_tokens.len()))
        } else {
            None
        }
    }

    fn execute_single_token(&mut self, token: &Token) -> Result<()> {
        match token {
            Token::Description(_) => Ok(()),
            Token::Number(num, den) => {
                self.push_value(Value {
                    val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)),
                });
                Ok(())
            },
            Token::String(s) => {
                self.push_value(Value {
                    val_type: ValueType::String(s.clone()),
                });
                Ok(())
            },
            Token::Boolean(b) => {
                self.push_value(Value {
                    val_type: ValueType::Boolean(*b),
                });
                Ok(())
            },
            Token::Nil => {
                self.push_value(Value {
                    val_type: ValueType::Nil,
                });
                Ok(())
            },
            Token::VectorStart => {
                let (vector_values, _) = self.collect_vector_as_data(&self.step_tokens[self.step_position - 1..])?;
                self.push_value(Value {
                    val_type: ValueType::Vector(vector_values),
                });
                Ok(())
            },
            Token::BlockStart => {
                let (block_tokens, _) = self.collect_block_tokens(&self.step_tokens, self.step_position - 1)?;
                self.push_value(Value {
                    val_type: ValueType::Quotation(block_tokens),
                });
                Ok(())
            }
            Token::Symbol(name) => {
                if let Some(def) = self.dictionary.get(name).cloned() {
                    if def.is_builtin {
                        self.execute_builtin(name)?;
                    } else {
                        self.execute_tokens_with_context(&def.tokens)?;
                    }
                } else {
                    return Err(AjisaiError::UnknownWord(name.clone()));
                }
                Ok(())
            },
            Token::VectorEnd | Token::BlockEnd => Err(AjisaiError::from("Unexpected closing delimiter found.")),
        }
    }

    fn collect_vector_as_data(&self, tokens: &[Token]) -> Result<(Vec<Value>, usize)> {
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
                Token::Number(num, den) => values.push(Value { val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)) }),
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

    fn collect_block_tokens(&self, tokens: &[Token], start_index: usize) -> Result<(Vec<Token>, usize)> {
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
    
    pub fn execute_tokens_with_context(&mut self, tokens: &[Token]) -> Result<()> {
        let mut i = 0;
        let mut pending_description: Option<String> = None;

        while i < tokens.len() {
            let token = &tokens[i];
            match token {
                Token::Description(text) => {
                    pending_description = Some(text.clone());
                },
                Token::Number(num, den) => {
                    self.push_value(Value {
                        val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)),
                    });
                },
                Token::String(s) => {
                    self.push_value(Value {
                        val_type: ValueType::String(s.clone()),
                    });
                },
                Token::Boolean(b) => {
                    self.push_value(Value {
                        val_type: ValueType::Boolean(*b),
                    });
                },
                Token::Nil => {
                    self.push_value(Value {
                        val_type: ValueType::Nil,
                    });
                },
                Token::VectorStart => {
                    let (vector_values, consumed) = self.collect_vector_as_data(&tokens[i..])?;
                    self.push_value(Value {
                        val_type: ValueType::Vector(vector_values),
                    });
                    i += consumed - 1;
                },
                Token::BlockStart => {
                    let (block_tokens, next_index) = self.collect_block_tokens(tokens, i)?;
                    self.push_value(Value {
                        val_type: ValueType::Quotation(block_tokens),
                    });
                    i = next_index -1;
                },
                Token::Symbol(name) => {
                    if name == "DEF" {
                        // DEFの後のDescriptionトークンを探す
                        let mut description = pending_description.take();
                        if description.is_none() && i + 1 < tokens.len() {
                            if let Token::Description(text) = &tokens[i + 1] {
                                description = Some(text.clone());
                                i += 1; // Descriptionトークンをスキップ
                            }
                        }
                        control::op_def(self, description)?;
                    } else if let Some(def) = self.dictionary.get(name).cloned() {
                        if def.is_builtin {
                            self.execute_builtin(name)?;
                        } else {
                            self.execute_custom_word(name, &def.tokens)?;
                        }
                    } else {
                        return Err(AjisaiError::UnknownWord(name.clone()));
                    }
                },
                Token::VectorEnd | Token::BlockEnd => return Err(AjisaiError::from("Unexpected closing delimiter found.")),
            }
            i += 1;
        }
        Ok(())
    }

    fn execute_custom_word(&mut self, name: &str, tokens: &[Token]) -> Result<()> {
        self.call_stack.push(name.to_string());
        let result = self.execute_tokens_with_context(tokens);
        self.call_stack.pop();
        
        result.map_err(|e| e.with_context(&self.call_stack))
    }

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        use self::{stack_ops::*, arithmetic::*, vector_ops::*, control::*, /*database::*,*/ io::*};
        
        match name {
            // スタック操作
            "DUP" => op_dup(self),
            "DROP" => op_drop(self),
            "SWAP" => op_swap(self),
            "OVER" => op_over(self),
            "ROT" => op_rot(self),
            "NIP" => op_nip(self),
            ">R" => op_to_r(self),
            "R>" => op_from_r(self),
            "R@" => op_r_fetch(self),
            
            // 算術・比較・論理
            "+" => op_add(self),
            "-" => op_sub(self),
            "*" => op_mul(self),
            "/" => op_div(self),
            ">" => op_gt(self),
            ">=" => op_ge(self),
            "=" => op_eq(self),
            "<" => op_lt(self),
            "<=" => op_le(self),
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
            "EMPTY?" => op_empty(self),
            
            // 制御構造
            "IF" => op_if(self),
            "DEL" => op_del(self),
            "CALL" => op_call(self),
            
            // Nil関連
            "NIL?" => op_nil_check(self),
            "NOT-NIL?" => op_not_nil_check(self),
            "KNOWN?" => op_not_nil_check(self),
            "DEFAULT" => op_default(self),
            
            // 入出力
            "." => op_dot(self),
            "PRINT" => op_print(self),
            "CR" => op_cr(self),
            "SPACE" => op_space(self),
            "SPACES" => op_spaces(self),
            "EMIT" => op_emit(self),
            
            _ => Err(AjisaiError::UnknownBuiltin(name.to_string())),
        }
    }
    
    // アクセサメソッド群（値のみを返すように修正）
    pub fn get_stack(&self) -> Vec<Value> {
        self.stack.iter().map(|entry| entry.value.clone()).collect()
    }
    
    pub fn get_register(&self) -> &Register { &self.register }
    
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
            .map(|(name, def)| {
                let is_protected = self.dependencies.get(name).map_or(false, |deps| !deps.is_empty());
                (name.clone(), def.description.clone(), is_protected)
            })
            .collect()
    }
   
    pub fn set_stack(&mut self, values: Vec<Value>) {
        self.stack = values.into_iter().map(StackEntry::new).collect();
    }
   
    pub fn set_register(&mut self, register: Register) {
        self.register = register;
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
            Token::BlockStart => "{".to_string(),
            Token::BlockEnd => "}".to_string(),
            Token::Description(d) => format!("({})", d),
        }
    }
}
