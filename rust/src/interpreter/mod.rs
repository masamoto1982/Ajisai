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
    fn process_single_token(&mut self, token: &Token) -> Result<()> {
        match token {
            Token::Symbol(name) => {
                let name_upper = name.to_uppercase();
                
                match name_upper.as_str() {
                    "DEF" => {
                        // DEFの直後にDescriptionがあるかチェック
                        let description = if self.step_position < self.step_tokens.len() {
                            if let Token::Description(desc) = &self.step_tokens[self.step_position] {
                                self.step_position += 1; // Descriptionトークンをスキップ
                                Some(desc.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        self.builtin_def(description)?;
                    },
                    "DEL" => self.builtin_del()?,
                    "EXEC" => self.builtin_exec()?,
                    ".S" => self.builtin_show_stack()?,
                    ".E" => self.builtin_show_exec_stack()?,
                    _ => {
                        // 辞書に登録されているか確認
                        if self.dictionary.contains_key(&name_upper) {
                            // 登録されていれば実行
                            self.execute_token(&Token::Symbol(name_upper))?;
                            self.process_exec_stack()?;
                        } else {
                            // 未登録ならデータとしてスタックに積む
                            self.data_stack.push(Value {
                                val_type: ValueType::Symbol(name.clone()),
                            });
                        }
                    }
                }
            },
            Token::Description(_) => {
                // 説明文は通常は無視（DEFの処理で使用される）
                Ok(())
            },
            // 他のトークンの処理は既存のまま
            _ => {
                // 既存の処理
            }
        }
    }

    // DEF: スタックから定義を作成（説明文付き）
    fn builtin_def(&mut self, description: Option<String>) -> Result<()> {
        if self.data_stack.len() < 2 {
            return Err(AjisaiError::StackUnderflow);
        }
        
        // スタックトップから: カスタムワード名
        let name_val = self.data_stack.pop().unwrap();
        let name = match name_val.val_type {
            ValueType::Symbol(s) | ValueType::String(s) => s.to_uppercase(),
            _ => return Err(AjisaiError::type_error("symbol or string", "other type")),
        };
        
        // 残りのスタック全体を定義として使用
        let mut definition_tokens = Vec::new();
        while let Some(val) = self.data_stack.pop() {
            definition_tokens.push(value_to_token(val)?);
        }
        definition_tokens.reverse(); // 正しい順序に戻す
        
        // ビルトインワードは再定義不可
        if let Some(existing) = self.dictionary.get(&name) {
            if existing.is_builtin {
                return Err(AjisaiError::from(format!("Cannot redefine builtin word: {}", name)));
            }
        }
        
        // 辞書に登録（説明文付き）
        self.dictionary.insert(name.clone(), WordDefinition {
            tokens: definition_tokens,
            is_builtin: false,
            description: description.or_else(|| Some("User defined word".to_string())),
        });
        
        Ok(())
    }

    // execute_tokens_with_contextも修正が必要
    pub fn execute_tokens_with_context(&mut self, tokens: &[Token]) -> Result<()> {
        let mut i = 0;
        let mut pending_description: Option<String> = None;

        while i < tokens.len() {
            let token = &tokens[i];
            
            match token {
                Token::Description(text) => {
                    // DEFの前に説明文がある場合のために保持
                    pending_description = Some(text.clone());
                },
                Token::Symbol(name) if name.to_uppercase() == "DEF" => {
                    // DEFの処理
                    let mut description = pending_description.take();
                    
                    // DEFの後の説明文もチェック
                    if description.is_none() && i + 1 < tokens.len() {
                        if let Token::Description(text) = &tokens[i + 1] {
                            description = Some(text.clone());
                            i += 1; // Descriptionトークンをスキップ
                        }
                    }
                    
                    self.builtin_def(description)?;
                },
                _ => {
                    // 通常のトークン処理
                    self.process_single_token(token)?;
                    pending_description = None; // 説明文をリセット
                }
            }
            i += 1;
        }
        Ok(())
    }
}
