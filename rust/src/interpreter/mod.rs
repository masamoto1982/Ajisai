// rust/src/interpreter/mod.rs (完全修正版)

pub mod vector_ops;
pub mod arithmetic;
pub mod control;
pub mod io;
pub mod error;

use std::collections::{HashMap, HashSet};
use crate::types::{Workspace, Token, Value, ValueType};
use self::error::Result;

pub struct Interpreter {
    pub(crate) workspace: Workspace,
    pub(crate) dictionary: HashMap<String, WordDefinition>,
    pub(crate) dependencies: HashMap<String, HashSet<String>>,
    pub(crate) output_buffer: String,
    pub(crate) call_stack: Vec<String>,
}

#[derive(Clone)]
pub struct WordDefinition {
    pub tokens: Vec<Token>,
    pub is_builtin: bool,
    pub description: Option<String>,
    pub category: Option<String>,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Interpreter {
            workspace: Vec::new(),
            dictionary: HashMap::new(),
            dependencies: HashMap::new(),
            output_buffer: String::new(),
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

    pub fn execute_amnesia(&mut self) -> Result<()> {
        // IndexedDBクリアのイベントを発火
        if let Some(window) = web_sys::window() {
            let event = web_sys::CustomEvent::new("ajisai-amnesia")
                .map_err(|_| error::AjisaiError::from("Failed to create amnesia event"))?;
            window.dispatch_event(&event)
                .map_err(|_| error::AjisaiError::from("Failed to dispatch amnesia event"))?;
        }
        
        // インタープリター内部状態もクリア
        self.workspace.clear();
        self.dictionary.clear();
        self.dependencies.clear();
        self.output_buffer.clear();
        self.call_stack.clear();
        
        // 組み込みワードを再登録
        crate::builtins::register_builtins(&mut self.dictionary);
        
        Ok(())
    }

    pub fn execute_single_token(&mut self, token: &Token) -> Result<String> {
        self.output_buffer.clear();
        
        match token {
            Token::Number(num, den) => {
                self.workspace.push(Value {
                    val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)),
                });
                Ok(format!("Pushed number: {}/{}", num, den))
            },
            Token::String(s) => {
                self.workspace.push(Value {
                    val_type: ValueType::String(s.clone()),
                });
                Ok(format!("Pushed string: \"{}\"", s))
            },
            Token::Boolean(b) => {
                self.workspace.push(Value {
                    val_type: ValueType::Boolean(*b),
                });
                Ok(format!("Pushed boolean: {}", b))
            },
            Token::Nil => {
                self.workspace.push(Value {
                    val_type: ValueType::Nil,
                });
                Ok("Pushed nil".to_string())
            },
            Token::ParenComment(comment) => {
                // 丸括弧コメントは実行時には無視
                Ok(format!("Skipped comment: ({})", comment))
            },
            Token::Symbol(name) => {
                self.execute_word(name)?;
                let output = self.get_output();
                if output.is_empty() {
                    Ok(format!("Executed word: {}", name))
                } else {
                    Ok(output)
                }
            },
            Token::VectorStart => {
                // ベクトルの開始だけでは処理できない
                Ok("Vector start token (incomplete)".to_string())
            },
            Token::VectorEnd => {
                // ベクトルの終了だけでは処理できない
                Ok("Vector end token (incomplete)".to_string())
            },
        }
    }

    fn process_line(&mut self, line: &str) -> Result<()> {
        let custom_word_names: HashSet<String> = self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect();
        
        let tokens = crate::tokenizer::tokenize_with_custom_words(line, &custom_word_names)
            .map_err(error::AjisaiError::from)?;
            
        if tokens.is_empty() {
            return Ok(());
        }

        // DEFパターンのチェック
        if let Some(def_result) = self.try_process_def_pattern(&tokens) {
            return def_result;
        }

        // 通常のトークン実行
        self.execute_tokens(&tokens)
    }

    fn try_process_def_pattern(&mut self, tokens: &[Token]) -> Option<Result<()>> {
        // DEFパターンのみをチェック
        let def_position = tokens.iter().rposition(|t| {
            if let Token::Symbol(s) = t {
                s == "DEF"
            } else {
                false
            }
        })?;
        
        if def_position >= 1 {
            if let Token::String(name) = &tokens[def_position - 1] {
                let body_tokens = &tokens[..def_position - 1];
                
                return Some(self.define_word_with_description(
                    name.clone(),
                    body_tokens.to_vec(),
                    None
                ));
            }
        }
        
        None
    }

    fn define_word_with_description(&mut self, name: String, body_tokens: Vec<Token>, description: Option<String>) -> Result<()> {
        let name = name.to_uppercase();
        
        // 既存のワードチェック
        if let Some(existing) = self.dictionary.get(&name) {
            if existing.is_builtin {
                return Err(error::AjisaiError::from(format!("Cannot redefine builtin word: {}", name)));
            }
        }

        // 依存関係チェック
        if self.dictionary.contains_key(&name) {
            if let Some(dependents) = self.dependencies.get(&name) {
                if !dependents.is_empty() {
                    let dependent_list: Vec<String> = dependents.iter().cloned().collect();
                    return Err(error::AjisaiError::ProtectedWord { 
                        name: name.clone(), 
                        dependents: dependent_list 
                    });
                }
            }
        }

        // ベクトルリテラルから実行可能なトークンを抽出
        let executable_tokens = self.extract_executable_tokens(&body_tokens)?;

        // 古い依存関係をクリア
        if let Some(old_deps) = self.get_word_dependencies(&name) {
            for dep in old_deps {
                if let Some(reverse_deps) = self.dependencies.get_mut(&dep) {
                    reverse_deps.remove(&name);
                }
            }
        }

        // 新しい依存関係を登録
        for token in &executable_tokens {
            if let Token::Symbol(sym) = token {
                if self.dictionary.contains_key(sym) && !self.is_builtin_word(sym) {
                    self.dependencies.entry(sym.clone())
                        .or_insert_with(HashSet::new)
                        .insert(name.clone());
                }
            }
        }

        // ワードを登録
        self.dictionary.insert(name.clone(), WordDefinition {
            tokens: executable_tokens,
            is_builtin: false,
            description,
            category: None,
        });

        self.append_output(&format!("Defined word: {}\n", name));
        Ok(())
    }

    // ベクトルリテラルから実行可能なトークンを抽出するメソッド（修正版）
    fn extract_executable_tokens(&self, tokens: &[Token]) -> Result<Vec<Token>> {
        // ベクトルリテラルはそのまま保持する
        // 内部のトークンに展開しない
        Ok(tokens.to_vec())
    }

    pub(crate) fn execute_tokens(&mut self, tokens: &[Token]) -> Result<()> {
        let mut i = 0;
        while i < tokens.len() {
            match &tokens[i] {
                Token::Number(num, den) => {
                    self.workspace.push(Value {
                        val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)),
                    });
                    i += 1;
                },
                Token::String(s) => {
                    self.workspace.push(Value {
                        val_type: ValueType::String(s.clone()),
                    });
                    i += 1;
                },
                Token::Boolean(b) => {
                    self.workspace.push(Value {
                        val_type: ValueType::Boolean(*b),
                    });
                    i += 1;
                },
                Token::Nil => {
                    self.workspace.push(Value {
                        val_type: ValueType::Nil,
                    });
                    i += 1;
                },
                Token::ParenComment(_) => {
                    // 丸括弧コメントは実行時には無視
                    i += 1;
                },
                Token::VectorStart => {
                    let (vector_values, consumed) = self.collect_vector(tokens, i)?;
                    self.workspace.push(Value {
                        val_type: ValueType::Vector(vector_values),
                    });
                    i += consumed;
                },
                Token::Symbol(name) => {
                    self.execute_word(name)?;
                    i += 1;
                },
                Token::VectorEnd => {
                    return Err(error::AjisaiError::from("Unexpected vector end"));
                },
            }
        }
        
        Ok(())
    }

    fn collect_vector(&self, tokens: &[Token], start: usize) -> Result<(Vec<Value>, usize)> {
        let mut values = Vec::new();
        let mut i = start + 1; // VectorStart の次から
        
        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorStart => {
                    // ネストしたVectorを再帰的に処理
                    let (nested_values, consumed) = self.collect_vector(tokens, i)?;
                    values.push(Value {
                        val_type: ValueType::Vector(nested_values),
                    });
                    i += consumed;
                },
                Token::VectorEnd => {
                    // このVectorの終了
                    return Ok((values, i - start + 1));
                },
                Token::ParenComment(_) => {
                    // コメントは無視
                    i += 1;
                },
                token => {
                    // 通常の値をVector内に追加
                    values.push(self.token_to_value(token)?);
                    i += 1;
                }
            }
        }
        
        Err(error::AjisaiError::from("Unclosed vector"))
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
            Token::ParenComment(_) => {
                // コメントはValueにはならない
                Err(error::AjisaiError::from("Cannot convert comment to value"))
            },
            _ => Err(error::AjisaiError::from("Cannot convert token to value")),
        }
    }

    pub fn vector_to_tokens(&self, vector: Vec<Value>) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        for value in vector.iter() {
            let token = self.value_to_token(value.clone())?;
            tokens.push(token);
        }
        
        Ok(tokens)
    }

    fn value_to_token(&self, value: Value) -> Result<Token> {
        match value.val_type {
            ValueType::Number(frac) => Ok(Token::Number(frac.numerator, frac.denominator)),
            ValueType::String(s) => Ok(Token::String(s)),
            ValueType::Boolean(b) => Ok(Token::Boolean(b)),
            ValueType::Symbol(s) => Ok(Token::Symbol(s)),
            ValueType::Nil => Ok(Token::Nil),
            ValueType::Vector(_) => Err(error::AjisaiError::from("Nested vectors not supported in token conversion")),
        }
    }

    pub(crate) fn get_word_dependencies(&self, word_name: &str) -> Option<Vec<String>> {
        if let Some(def) = self.dictionary.get(word_name) {
            let mut deps = Vec::new();
            for token in &def.tokens {
                if let Token::Symbol(sym) = token {
                    if self.dictionary.contains_key(sym) && !self.is_builtin_word(sym) {
                        deps.push(sym.clone());
                    }
                }
            }
            Some(deps)
        } else {
            None
        }
    }

    pub(crate) fn is_builtin_word(&self, name: &str) -> bool {
        self.dictionary.get(name)
            .map(|def| def.is_builtin)
            .unwrap_or(false)
    }

    fn is_protected(&self, name: &str) -> bool {
        self.dependencies.get(name)
            .map(|deps| !deps.is_empty())
            .unwrap_or(false)
    }

    fn execute_word(&mut self, name: &str) -> Result<()> {
        if let Some(def) = self.dictionary.get(name).cloned() {
            if def.is_builtin {
                self.execute_builtin(name)
            } else {
                // カスタムワードは即座実行（EVALなしで）
                self.call_stack.push(name.to_string());
                let result = self.execute_custom_word_immediate(&def.tokens);
                self.call_stack.pop();
                result.map_err(|e| e.with_context(&self.call_stack))
            }
        } else {
            Err(error::AjisaiError::UnknownWord(name.to_string()))
        }
    }

    fn execute_custom_word_immediate(&mut self, tokens: &[Token]) -> Result<()> {
        // ベクトル記号を除いて中身を実行
        if tokens.len() >= 2 && 
           tokens.first() == Some(&Token::VectorStart) && 
           tokens.last() == Some(&Token::VectorEnd) {
            // [ 内容 ] → 内容を直接実行
            let inner_tokens = &tokens[1..tokens.len()-1];
            self.execute_tokens(inner_tokens)
        } else {
            // 通常のトークン列をそのまま実行
            self.execute_tokens(tokens)
        }
    }

    // JUMP用のメソッド（追加）
    pub(crate) fn execute_word_leap(&mut self, name: &str, current_word: Option<&str>) -> Result<()> {
        if let Some(current) = current_word {
            if name != current {
                return Err(error::AjisaiError::from(format!(
                    "JUMP can only jump within the same word. Cannot jump from '{}' to '{}'", 
                    current, name
                )));
            }
        } else {
            return Err(error::AjisaiError::from(format!(
                "JUMP can only be used within custom words. Cannot jump to '{}' from main program", 
                name
            )));
        }

        if let Some(def) = self.dictionary.get(name).cloned() {
            if def.is_builtin {
                return Err(error::AjisaiError::from("Cannot jump to builtin word"));
            } else {
                // カスタムワードを即座実行
                self.execute_custom_word_immediate(&def.tokens)
            }
        } else {
            Err(error::AjisaiError::UnknownWord(name.to_string()))
        }
    }

    // 既存のexecute_custom_wordは内部用として残す
    fn execute_custom_word(&mut self, tokens: &[Token]) -> Result<()> {
        let mut i = 0;
        while i < tokens.len() {
            match &tokens[i] {
                Token::Number(num, den) => {
                    self.workspace.push(Value {
                        val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)),
                    });
                    i += 1;
                },
                Token::String(s) => {
                    self.workspace.push(Value {
                        val_type: ValueType::String(s.clone()),
                    });
                    i += 1;
                },
                Token::Boolean(b) => {
                    self.workspace.push(Value {
                        val_type: ValueType::Boolean(*b),
                    });
                    i += 1;
                },
                Token::Nil => {
                    self.workspace.push(Value {
                        val_type: ValueType::Nil,
                    });
                    i += 1;
                },
                Token::ParenComment(_) => {
                    // カスタムワード内のコメントは無視
                    i += 1;
                },
                Token::Symbol(name) => {
                    self.execute_word(name)?;
                    i += 1;
                },
                Token::VectorStart => {
                    // ベクトルを収集してワークスペースにプッシュ
                    let (vector_values, consumed) = self.collect_vector(tokens, i)?;
                    self.workspace.push(Value {
                        val_type: ValueType::Vector(vector_values),
                    });
                    i += consumed;
                },
                Token::VectorEnd => {
                    return Err(error::AjisaiError::from("Unexpected vector end"));
                },
            }
        }
        
        Ok(())
    }

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        match name {
            // 算術・論理演算
            "+" => arithmetic::op_add(self),
            "/" => arithmetic::op_div(self),
            "*" => arithmetic::op_mul(self),
            "-" => arithmetic::op_sub(self),
            "=" => arithmetic::op_eq(self),
            ">=" => arithmetic::op_ge(self),
            ">" => arithmetic::op_gt(self),
            "AND" => arithmetic::op_and(self),
            "OR" => arithmetic::op_or(self),
            "NOT" => arithmetic::op_not(self),
            
            // 位置指定操作（0オリジン）
            "NTH" => vector_ops::op_get(self),
            "INSERT" => vector_ops::op_insert(self),
            "REPLACE" => vector_ops::op_replace(self),
            "REMOVE" => vector_ops::op_remove(self),
            
            // 量指定操作（1オリジン）
            "LENGTH" => vector_ops::op_length(self),
            "TAKE" => vector_ops::op_take(self),
            "DROP" => vector_ops::op_drop(self),
            "REPEAT" => vector_ops::op_repeat(self),
            "SPLIT" => vector_ops::op_split(self),
            
            // Vector操作
            "CONCAT" => vector_ops::op_concat(self),
            "JUMP" => control::op_jump(self),
            
            // ワード管理
            "DEF" => control::op_def(self),
            "DEL" => control::op_del(self),
            "EVAL" => control::op_eval(self),
            
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
    
    pub fn get_workspace(&self) -> &Workspace { &self.workspace }
    
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
                let protected = self.is_protected(name);
                (name.clone(), def.description.clone(), protected)
            })
            .collect()
    }
   
    pub fn set_workspace(&mut self, workspace: Workspace) {
        self.workspace = workspace;
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
            category: None,
        });

        Ok(())
    }
   
    pub fn get_word_definition(&self, name: &str) -> Option<String> {
        if let Some(def) = self.dictionary.get(name) {
            if !def.is_builtin {
                let body_string = def.tokens.iter()
                    .filter_map(|token| {
                        // ParenCommentはワード定義文字列には含めない
                        match token {
                            Token::ParenComment(_) => None,
                            _ => Some(self.token_to_string(token))
                        }
                    })
                    .collect::<Vec<String>>()
                    .join(" ");
                return Some(format!("[ {} ]", body_string));
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
            Token::ParenComment(comment) => format!("({})", comment),
        }
    }
}
