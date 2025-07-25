pub mod stack_ops;
pub mod arithmetic;
pub mod vector_ops;
pub mod control;
pub mod io;
pub mod error;
pub mod register_ops;

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
    current_tokens: Vec<Token>,  // 現在処理中のトークン列を保持
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
            current_tokens: Vec::new(),
        };
        
        crate::builtins::register_builtins(&mut interpreter.dictionary);
        interpreter
    }
    
    pub fn execute(&mut self, code: &str) -> Result<()> {
        let tokens = tokenize(code).map_err(AjisaiError::from)?;
        self.current_tokens = tokens.clone();
        let rearranged = self.rearrange_tokens(&tokens)?;
        self.execute_tokens_with_context(&rearranged)
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
        let tokens = tokenize(code).map_err(AjisaiError::from)?;
        self.current_tokens = tokens.clone();
        self.step_tokens = self.rearrange_tokens(&tokens)?;
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

    // トークンを並び替える（値を先に、ワードを後に）
    fn rearrange_tokens(&self, tokens: &[Token]) -> Result<Vec<Token>> {
        let mut values = Vec::new();
        let mut words = Vec::new();
        let mut i = 0;
        
        while i < tokens.len() {
            match &tokens[i] {
                Token::Symbol(name) => {
                    // DEFは特別扱い
                    if name == "DEF" {
                        // DEFが見つかったら、現在までの値とワードをすべて処理
                        let mut result = values.clone();
                        result.extend(words.clone());
                        result.push(tokens[i].clone());
                        
                        // DEF以降のトークンも処理
                        i += 1;
                        if i < tokens.len() {
                            let remaining = self.rearrange_tokens(&tokens[i..])?;
                            result.extend(remaining);
                        }
                        return Ok(result);
                    }
                    
                    if self.dictionary.contains_key(name) {
                        words.push(tokens[i].clone());
                    } else {
                        // 未知のシンボルは値として扱う
                        values.push(tokens[i].clone());
                    }
                },
                Token::Number(_, _) | Token::String(_) | Token::Boolean(_) | Token::Nil => {
                    values.push(tokens[i].clone());
                },
                Token::VectorStart => {
                    // ベクトル全体を1つの値として扱う
                    let (vector_tokens, consumed) = self.collect_vector_tokens(&tokens[i..])?;
                    values.extend(vector_tokens);
                    i += consumed - 1;
                },
                Token::Description(_) => {
                    // 説明は値として扱う
                    values.push(tokens[i].clone());
                },
                _ => {
                    values.push(tokens[i].clone());
                }
            }
            i += 1;
        }
        
        // 値を先に、ワードを後に配置
        values.extend(words);
        Ok(values)
    }

    fn collect_vector_tokens(&self, tokens: &[Token]) -> Result<(Vec<Token>, usize)> {
        let mut result = Vec::new();
        let mut depth = 0;
        let mut i = 0;
        
        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorStart => {
                    depth += 1;
                    result.push(tokens[i].clone());
                },
                Token::VectorEnd => {
                    depth -= 1;
                    result.push(tokens[i].clone());
                    if depth == 0 {
                        return Ok((result, i + 1));
                    }
                },
                _ => result.push(tokens[i].clone()),
            }
            i += 1;
        }
        
        Err(AjisaiError::from("Unclosed vector"))
    }

    fn execute_single_token(&mut self, token: &Token) -> Result<()> {
        match token {
            Token::Description(_) => Ok(()),
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
            Token::VectorStart => {
                let (vector_values, _) = self.collect_vector_as_data(&self.step_tokens[self.step_position - 1..])?;
                self.stack.push(Value {
                    val_type: ValueType::Vector(vector_values),
                });
                Ok(())
            },
            Token::Symbol(name) => {
                if let Some(def) = self.dictionary.get(name).cloned() {
                    if def.is_builtin {
                        self.execute_builtin(name)?;
                    } else {
                        // カスタムワードも並び替えて実行
                        let rearranged = self.rearrange_tokens(&def.tokens)?;
                        self.execute_tokens_with_context(&rearranged)?;
                    }
                } else {
                    return Err(AjisaiError::UnknownWord(name.clone()));
                }
                Ok(())
            },
            Token::VectorEnd => Err(AjisaiError::from("Unexpected closing delimiter found.")),
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
                    self.stack.push(Value {
                        val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)),
                    });
                },
                Token::String(s) => {
                    self.stack.push(Value {
                        val_type: ValueType::String(s.clone()),
                    });
                },
                Token::Boolean(b) => {
                    self.stack.push(Value {
                        val_type: ValueType::Boolean(*b),
                    });
                },
                Token::Nil => {
                    self.stack.push(Value {
                        val_type: ValueType::Nil,
                    });
                },
                Token::VectorStart => {
                    let (vector_values, consumed) = self.collect_vector_as_data(&tokens[i..])?;
                    self.stack.push(Value {
                        val_type: ValueType::Vector(vector_values),
                    });
                    i += consumed - 1;
                },
                Token::Symbol(name) => {
                    if name == "DEF" {
                        // DEFの新しい実装（ワード定義）
                        control::op_def(self, &self.current_tokens, pending_description.take())?;
                    } else if name == "DEL" {
                        // DELは通常の組み込みワードとして実行
                        control::op_del(self)?;
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
                Token::VectorEnd => return Err(AjisaiError::from("Unexpected closing delimiter found.")),
            }
            i += 1;
        }
        Ok(())
    }

    fn execute_custom_word(&mut self, name: &str, tokens: &[Token]) -> Result<()> {
        self.call_stack.push(name.to_string());
        let rearranged = self.rearrange_tokens(tokens)?;
        let result = self.execute_tokens_with_context(&rearranged);
        self.call_stack.pop();
        
        result.map_err(|e| e.with_context(&self.call_stack))
    }

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        use self::{stack_ops::*, arithmetic::*, vector_ops::*, control::*, io::*, register_ops::*};
        
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
            
            // レジスタ演算
            "R+" => op_r_add(self),
            "R-" => op_r_sub(self),
            "R*" => op_r_mul(self),
            "R/" => op_r_div(self),
            
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
    
    // アクセサメソッド群
    pub fn get_stack(&self) -> &Stack { &self.stack }
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
   
    pub fn set_stack(&mut self, stack: Stack) {
        self.stack = stack;
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
                return Some(body_string);
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
            Token::Description(d) => format!("({})", d),
        }
    }
}
