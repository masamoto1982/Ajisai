pub mod stack_ops;
pub mod arithmetic;
pub mod vector_ops;
pub mod control;
pub mod io;
pub mod error;

use std::collections::{HashMap, HashSet};
use crate::types::{Value, ValueType, DataStack, ExecStack, ExecFrame, Token};
use crate::tokenizer::tokenize;
use self::error::{AjisaiError, Result};

pub struct Interpreter {
    pub(crate) data_stack: DataStack,    // データスタック（ワーキングメモリ）
    pub(crate) exec_stack: ExecStack,    // 実行スタック（中央実行系）
    pub(crate) dictionary: HashMap<String, WordDefinition>,  // 長期記憶
    pub(crate) dependencies: HashMap<String, HashSet<String>>,
    pub(crate) last_defined_word: Option<String>,  // 最後に定義したワード
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
            data_stack: Vec::new(),
            exec_stack: Vec::new(),
            dictionary: HashMap::new(),
            dependencies: HashMap::new(),
            last_defined_word: None,
            step_tokens: Vec::new(),
            step_position: 0,
            step_mode: false,
            output_buffer: String::new(),
        };
        
        // 基本的なビルトインを登録
        register_core_builtins(&mut interpreter.dictionary);
        interpreter
    }
    
    pub fn execute(&mut self, code: &str) -> Result<()> {
        let tokens = tokenize(code).map_err(AjisaiError::from)?;
        self.execute_tokens_with_context(&tokens)
    }

    pub fn execute_tokens_with_context(&mut self, tokens: &[Token]) -> Result<()> {
        let mut i = 0;

        while i < tokens.len() {
            let token = &tokens[i];
            
            match token {
                Token::Symbol(name) if name.to_uppercase() == "DEF" => {
                    // DEFを実行
                    self.builtin_def()?;
                    
                    // DEFの直後にDescriptionがあれば、最後に定義したワードに追加
                    if i + 1 < tokens.len() {
                        if let Token::Description(text) = &tokens[i + 1] {
                            if let Some(word_name) = &self.last_defined_word {
                                if let Some(def) = self.dictionary.get_mut(word_name) {
                                    def.description = Some(text.clone());
                                }
                            }
                            i += 1; // Descriptionトークンをスキップ
                        }
                    }
                },
                _ => {
                    self.process_single_token(token)?;
                }
            }
            i += 1;
        }
        
        // 実行スタックに残っているものを処理
        self.process_exec_stack()?;
        
        Ok(())
    }

    fn process_single_token(&mut self, token: &Token) -> Result<()> {
        match token {
            Token::Symbol(name) => {
                let name_upper = name.to_uppercase();
                
                match name_upper.as_str() {
                    "DEF" => self.builtin_def()?,
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
                // 説明文は通常は無視（DEFの後で処理される）
                Ok(())
            },
            Token::Number(num, den) => {
                self.data_stack.push(Value {
                    val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)),
                });
                Ok(())
            },
            Token::String(s) => {
                self.data_stack.push(Value {
                    val_type: ValueType::String(s.clone()),
                });
                Ok(())
            },
            Token::Boolean(b) => {
                self.data_stack.push(Value {
                    val_type: ValueType::Boolean(*b),
                });
                Ok(())
            },
            Token::Nil => {
                self.data_stack.push(Value {
                    val_type: ValueType::Nil,
                });
                Ok(())
            },
            Token::VectorStart | Token::VectorEnd | 
            Token::BlockStart | Token::BlockEnd => {
                // 現時点ではデリミタもシンボルとして扱う
                let symbol_name = match token {
                    Token::VectorStart => "[",
                    Token::VectorEnd => "]",
                    Token::BlockStart => "{",
                    Token::BlockEnd => "}",
                    _ => unreachable!(),
                };
                self.data_stack.push(Value {
                    val_type: ValueType::Symbol(symbol_name.to_string()),
                });
                Ok(())
            }
        }
    }

    // 実行スタックの処理
    fn process_exec_stack(&mut self) -> Result<()> {
        while let Some(mut frame) = self.exec_stack.pop() {
            match &mut frame {
                ExecFrame::WordCall { tokens, position, .. } |
                ExecFrame::Quotation { tokens, position } => {
                    if *position < tokens.len() {
                        let token = tokens[*position].clone();
                        *position += 1;
                        
                        // フレームを戻す（まだ実行が残っている場合）
                        if *position < tokens.len() {
                            self.exec_stack.push(frame);
                        }
                        
                        // トークンを実行
                        self.execute_token(&token)?;
                    }
                }
            }
        }
        Ok(())
    }

    // 実行コンテキストでトークンを実行
    fn execute_token(&mut self, token: &Token) -> Result<()> {
        match token {
            Token::Symbol(name) => {
                if let Some(def) = self.dictionary.get(name).cloned() {
                    if def.is_builtin {
                        self.execute_builtin(name)?;
                    } else {
                        // ユーザー定義ワードを実行スタックに積む
                        self.exec_stack.push(ExecFrame::WordCall {
                            name: name.clone(),
                            tokens: def.tokens,
                            position: 0,
                        });
                    }
                } else {
                    // 未定義のシンボルはエラー
                    return Err(AjisaiError::UnknownWord(name.clone()));
                }
            },
            _ => {
                // 実行コンテキストでは、データはそのままデータスタックに積む
                self.process_single_token(token)?;
            }
        }
        Ok(())
    }

    // DEF: スタックから定義を作成
    fn builtin_def(&mut self) -> Result<()> {
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
        
        // 辞書に登録
        self.dictionary.insert(name.clone(), WordDefinition {
            tokens: definition_tokens,
            is_builtin: false,
            description: None,
        });
        
        // 最後に定義したワードを記録
        self.last_defined_word = Some(name);
        
        Ok(())
    }

    // DEL: ワードを削除
    fn builtin_del(&mut self) -> Result<()> {
        let val = self.data_stack.pop()
            .ok_or(AjisaiError::StackUnderflow)?;
        
        let name = match val.val_type {
            ValueType::Symbol(s) | ValueType::String(s) => s.to_uppercase(),
            _ => return Err(AjisaiError::type_error("symbol or string", "other type")),
        };
        
        // ビルトインは削除不可
        if let Some(def) = self.dictionary.get(&name) {
            if def.is_builtin {
                return Err(AjisaiError::from(format!("Cannot delete builtin word: {}", name)));
            }
        }
        
        self.dictionary.remove(&name);
        
        // last_defined_wordもクリア
        if self.last_defined_word.as_ref() == Some(&name) {
            self.last_defined_word = None;
        }
        
        Ok(())
    }

    // EXEC: データスタックからシンボルを取り出して実行
    fn builtin_exec(&mut self) -> Result<()> {
        let val = self.data_stack.pop()
            .ok_or(AjisaiError::StackUnderflow)?;
        
        match val.val_type {
            ValueType::Symbol(name) => {
                self.execute_token(&Token::Symbol(name))?;
                self.process_exec_stack()?;
            },
            _ => return Err(AjisaiError::type_error("symbol", "other type")),
        }
        Ok(())
    }

    // デバッグ用: データスタックを表示
    fn builtin_show_stack(&mut self) -> Result<()> {
        self.append_output("Data Stack: ");
        for (i, val) in self.data_stack.iter().enumerate() {
            if i > 0 { self.append_output(" "); }
            self.append_output(&format!("{}", val));
        }
        self.append_output("\n");
        Ok(())
    }

    // デバッグ用: 実行スタックを表示
    fn builtin_show_exec_stack(&mut self) -> Result<()> {
        self.append_output("Exec Stack: ");
        self.append_output(&format!("{} frames\n", self.exec_stack.len()));
        Ok(())
    }

    // 基本的な算術演算などのビルトイン実行
    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        match name {
            "+" => arithmetic::op_add_data(self),
            "-" => arithmetic::op_sub_data(self),
            "*" => arithmetic::op_mul_data(self),
            "/" => arithmetic::op_div_data(self),
            "DUP" => stack_ops::op_dup_data(self),
            "DROP" => stack_ops::op_drop_data(self),
            "SWAP" => stack_ops::op_swap_data(self),
            "." => io::op_dot_data(self),
            "CR" => io::op_cr_data(self),
            _ => Err(AjisaiError::UnknownBuiltin(name.to_string())),
        }
    }

    // ヘルパーメソッド
    pub fn get_output(&mut self) -> String {
        let output = self.output_buffer.clone();
        self.output_buffer.clear();
        output
    }
    
    pub(crate) fn append_output(&mut self, text: &str) {
        self.output_buffer.push_str(text);
    }

    // getter
    pub fn get_stack(&self) -> &DataStack { &self.data_stack }
    
    pub fn get_custom_words_info(&self) -> Vec<(String, Option<String>, bool)> {
        self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, def)| {
                (name.clone(), def.description.clone(), false)
            })
            .collect()
    }
}

// ValueをTokenに変換するヘルパー関数
fn value_to_token(value: Value) -> Result<Token> {
    match value.val_type {
        ValueType::Number(frac) => Ok(Token::Number(frac.numerator, frac.denominator)),
        ValueType::String(s) => Ok(Token::String(s)),
        ValueType::Boolean(b) => Ok(Token::Boolean(b)),
        ValueType::Symbol(s) => Ok(Token::Symbol(s)),
        ValueType::Nil => Ok(Token::Nil),
        _ => Err(AjisaiError::from("Cannot convert value to token")),
    }
}

// 最小限のビルトインを登録
fn register_core_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    // 定義・実行制御
    register_builtin(dictionary, "DEF", "Define new word from stack");
    register_builtin(dictionary, "DEL", "Delete word from dictionary");
    register_builtin(dictionary, "EXEC", "Execute symbol from data stack");
    
    // 基本演算
    register_builtin(dictionary, "+", "Add two numbers");
    register_builtin(dictionary, "-", "Subtract");
    register_builtin(dictionary, "*", "Multiply");
    register_builtin(dictionary, "/", "Divide");
    
    // スタック操作
    register_builtin(dictionary, "DUP", "Duplicate top of stack");
    register_builtin(dictionary, "DROP", "Drop top of stack");
    register_builtin(dictionary, "SWAP", "Swap top two items");
    
    // 出力
    register_builtin(dictionary, ".", "Print and drop");
    register_builtin(dictionary, "CR", "Print newline");
    register_builtin(dictionary, ".S", "Show data stack");
    register_builtin(dictionary, ".E", "Show execution stack");
}

fn register_builtin(dictionary: &mut HashMap<String, WordDefinition>, name: &str, description: &str) {
    dictionary.insert(name.to_string(), WordDefinition {
        tokens: vec![],
        is_builtin: true,
        description: Some(description.to_string()),
    });
}
