// rust/src/interpreter/mod.rs (最新完全版)

pub mod vector_ops;
pub mod arithmetic;
pub mod control;
pub mod io;
pub mod error;

use std::collections::{HashMap, HashSet};
use crate::types::{Workspace, Token, Value, ValueType, BracketType};
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

#[derive(Debug, Clone)]
pub struct MultiLineDefinition {
    pub lines: Vec<Vec<Token>>,
    pub has_conditionals: bool,
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
        // まず最初にデバッグ出力
        self.output_buffer.push_str(&format!("DEBUG: execute() called with code: '{}'\n", code));
        
        // 全体を一度にトークン化（改行を保持）
        let custom_word_names: HashSet<String> = self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect();
        
        let tokens = crate::tokenizer::tokenize_with_custom_words(code, &custom_word_names)
            .map_err(error::AjisaiError::from)?;
            
        self.append_output(&format!("DEBUG: All tokens: {:?}\n", tokens));
        
        if tokens.is_empty() {
            return Ok(());
        }

        // DEFパターンを探して処理
        if let Some((def_result, remaining_code)) = self.try_process_def_pattern_from_code(code, &tokens) {
            self.append_output("DEBUG: DEF pattern processing started\n");
            
            // DEF処理を実行
            def_result?;
            
            // 残りのコードがあれば実行
            if !remaining_code.trim().is_empty() {
                self.append_output(&format!("DEBUG: Executing remaining code: '{}'\n", remaining_code));
                // 再帰的にexecuteを呼ぶ（新しいカスタムワードを含めて）
                self.execute(&remaining_code)?;
            } else {
                self.append_output("DEBUG: No remaining code to execute\n");
            }
            
            return Ok(());
        }

        self.append_output("DEBUG: No DEF pattern, executing tokens normally\n");
        // DEFパターンがない場合は通常の実行
        self.execute_tokens(&tokens)
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
                Ok(format!("Pushed string: '{}'", s))
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
            Token::FunctionComment(comment) => {
                // 機能説明コメントは実行時には無視
                Ok(format!("Skipped function comment: \"{}\"", comment))
            },
            Token::Colon => {
                Ok("Colon token (should not be executed alone)".to_string())
            },
            Token::LineBreak => {
                Ok("Line break token (should not be executed alone)".to_string())
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
            Token::VectorStart(_) => {
                Ok("Vector start token (incomplete)".to_string())
            },
            Token::VectorEnd(_) => {
                Ok("Vector end token (incomplete)".to_string())
            },
        }
    }

    fn try_process_def_pattern_from_code(&mut self, code: &str, tokens: &[Token]) -> Option<(Result<()>, String)> {
        // DEFの位置を探す
        let def_position = tokens.iter().rposition(|t| {
            if let Token::Symbol(s) = t {
                s == "DEF"
            } else {
                false
            }
        })?;
        
        // DEF前に文字列（ワード名）があるかチェック
        if def_position >= 1 {
            if let Token::String(name) = &tokens[def_position - 1] {
                let body_tokens = &tokens[..def_position - 1];
                
                if body_tokens.is_empty() {
                    return Some((Err(error::AjisaiError::from("DEF requires a body")), String::new()));
                }
                
                // 複数行かどうかを判定
                let multiline_def = self.parse_multiline_definition(body_tokens);
                
                // 元のコードからDEF後の部分を直接抽出
                let remaining_code = if let Some(def_pos_in_code) = code.rfind("DEF") {
                    let after_def = &code[def_pos_in_code + 3..]; // "DEF"の3文字分スキップ
                    after_def.trim().to_string()
                } else {
                    String::new()
                };
                
                self.append_output(&format!("DEBUG: Remaining code extracted: '{}'\n", remaining_code));
                
                let def_result = self.define_word_from_multiline(
                    name.clone(),
                    multiline_def
                );
                
                return Some((def_result, remaining_code));
            }
        }
        
        None
    }

    fn try_process_multiline_def_pattern(&mut self, tokens: &[Token]) -> Option<(Result<()>, Vec<Token>)> {
        // DEFの位置を探す
        let def_position = tokens.iter().rposition(|t| {
            if let Token::Symbol(s) = t {
                s == "DEF"
            } else {
                false
            }
        })?;
        
        // DEF前に文字列（ワード名）があるかチェック
        if def_position >= 1 {
            if let Token::String(name) = &tokens[def_position - 1] {
                let body_tokens = &tokens[..def_position - 1];
                
                if body_tokens.is_empty() {
                    return Some((Err(error::AjisaiError::from("DEF requires a body")), Vec::new()));
                }
                
                // 複数行かどうかを判定
                let multiline_def = self.parse_multiline_definition(body_tokens);
                
                // 先にワードを定義
                let def_result = self.define_word_from_multiline(
                    name.clone(),
                    multiline_def
                );
                
                // DEF後にトークンがあるかチェック
                let remaining_tokens = if def_position + 1 < tokens.len() {
                    // DEF後のコードを再トークン化（新しいカスタムワードを含めて）
                    let remaining_code = &tokens[def_position + 1..];
                    let remaining_code_str = self.tokens_to_string(remaining_code);
                    self.append_output(&format!("DEBUG: Re-tokenizing remaining code: '{}'\n", remaining_code_str));
                    
                    let custom_word_names: HashSet<String> = self.dictionary.iter()
                        .filter(|(_, def)| !def.is_builtin)
                        .map(|(name, _)| name.clone())
                        .collect();
                    
                    match crate::tokenizer::tokenize_with_custom_words(&remaining_code_str, &custom_word_names) {
                        Ok(retokenized) => {
                            self.append_output(&format!("DEBUG: Re-tokenized result: {:?}\n", retokenized));
                            retokenized
                        },
                        Err(_) => remaining_code.to_vec(),
                    }
                } else {
                    Vec::new()
                };
                
                return Some((def_result, remaining_tokens));
            }
        }
        
        None
    }

    fn parse_multiline_definition(&self, tokens: &[Token]) -> MultiLineDefinition {
        let mut lines = Vec::new();
        let mut current_line = Vec::new();
        let mut has_conditionals = false;
        
        for token in tokens {
            match token {
                Token::LineBreak => {
                    if !current_line.is_empty() {
                        lines.push(current_line.clone());
                        current_line.clear();
                    }
                },
                Token::FunctionComment(_) => {
                    // コメントはスキップ
                },
                _ => {
                    if let Token::Colon = token {
                        has_conditionals = true;
                    }
                    current_line.push(token.clone());
                }
            }
        }
        
        // 最後の行を追加
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        MultiLineDefinition {
            lines,
            has_conditionals,
        }
    }

    fn define_word_from_multiline(&mut self, name: String, multiline_def: MultiLineDefinition) -> Result<()> {
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

        // 処理方式の判定と実行
        let executable_tokens = if multiline_def.lines.len() == 1 {
            // 単一行 → 通常の定義
            multiline_def.lines[0].clone()
        } else if multiline_def.has_conditionals {
            // 複数行 + コロンあり → 条件分岐
            control::create_conditional_execution_tokens(&multiline_def.lines)?
        } else {
            // 複数行 + コロンなし → 順次実行
            self.create_sequential_execution_tokens(&multiline_def.lines)
        };

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

        self.dictionary.insert(name.clone(), WordDefinition {
            tokens: executable_tokens,
            is_builtin: false,
            description: None,
            category: None,
        });

        self.append_output(&format!("Defined word: {}\n", name));
        Ok(())
    }

    fn create_sequential_execution_tokens(&self, lines: &[Vec<Token>]) -> Vec<Token> {
        let mut result = Vec::new();
        
        for line in lines.iter() {
            result.extend(line.iter().cloned());
        }
        
        result
    }

    pub(crate) fn execute_tokens(&mut self, tokens: &[Token]) -> Result<()> {
        self.append_output(&format!("DEBUG: execute_tokens called with {} tokens: {:?}\n", tokens.len(), tokens));
        
        let mut i = 0;
        while i < tokens.len() {
            self.append_output(&format!("DEBUG: Processing token #{}: {:?}\n", i, tokens[i]));
            
            match &tokens[i] {
                Token::Number(num, den) => {
                    self.workspace.push(Value {
                        val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)),
                    });
                    self.append_output(&format!("DEBUG: Pushed number {}/{}, workspace size: {}\n", num, den, self.workspace.len()));
                    i += 1;
                },
                Token::String(s) => {
                    self.workspace.push(Value {
                        val_type: ValueType::String(s.clone()),
                    });
                    self.append_output(&format!("DEBUG: Pushed string '{}', workspace size: {}\n", s, self.workspace.len()));
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
                Token::FunctionComment(_) => {
                    // 機能説明コメントは実行時には無視
                    i += 1;
                },
                Token::Colon => {
                    // コロンは条件分岐処理で既に処理済みのはず
                    i += 1;
                },
                Token::LineBreak => {
                    // 改行は定義処理で既に処理済みのはず
                    i += 1;
                },
                Token::VectorStart(bracket_type) => {
                    self.append_output(&format!("DEBUG: Processing vector start, workspace size before: {}\n", self.workspace.len()));
                    let (vector_values, consumed) = self.collect_vector(tokens, i, bracket_type.clone())?;
                    self.workspace.push(Value {
                        val_type: ValueType::Vector(vector_values, bracket_type.clone()),
                    });
                    self.append_output(&format!("DEBUG: Pushed vector, workspace size after: {}\n", self.workspace.len()));
                    i += consumed;
                },
                Token::Symbol(name) => {
                    self.append_output(&format!("DEBUG: Executing word '{}', workspace size before: {}\n", name, self.workspace.len()));
                    
                    match self.execute_word(name) {
                        Ok(_) => {
                            self.append_output(&format!("DEBUG: Successfully executed '{}', workspace size after: {}\n", name, self.workspace.len()));
                        },
                        Err(e) => {
                            self.append_output(&format!("DEBUG: Error executing '{}': {}\n", name, e));
                            return Err(e);
                        }
                    }
                    i += 1;
                },
                Token::VectorEnd(_) => {
                    return Err(error::AjisaiError::from("Unexpected vector end"));
                },
            }
        }
        
        self.append_output(&format!("DEBUG: execute_tokens completed, final workspace size: {}\n", self.workspace.len()));
        Ok(())
    }

    fn collect_vector(&self, tokens: &[Token], start: usize, expected_bracket_type: BracketType) -> Result<(Vec<Value>, usize)> {
        let mut values = Vec::new();
        let mut i = start + 1; // VectorStart の次から
        
        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorStart(inner_bracket_type) => {
                    let (nested_values, consumed) = self.collect_vector(tokens, i, inner_bracket_type.clone())?;
                    values.push(Value {
                        val_type: ValueType::Vector(nested_values, inner_bracket_type.clone()),
                    });
                    i += consumed;
                },
                Token::VectorEnd(end_bracket_type) => {
                    if *end_bracket_type != expected_bracket_type {
                        return Err(error::AjisaiError::from(format!(
                            "Mismatched bracket types: expected {}, found {}",
                            expected_bracket_type.closing_char(),
                            end_bracket_type.closing_char()
                        )));
                    }
                    return Ok((values, i - start + 1));
                },
                Token::FunctionComment(_) => {
                    i += 1;
                },
                token => {
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
            Token::Colon => Ok(Value {
                val_type: ValueType::Symbol(":".to_string()),
            }),
            Token::LineBreak => {
                Err(error::AjisaiError::from("Cannot convert line break to value"))
            },
            Token::FunctionComment(_) => {
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
            ValueType::Symbol(s) => {
                if s == ":" {
                    Ok(Token::Colon)
                } else {
                    Ok(Token::Symbol(s))
                }
            },
            ValueType::Nil => Ok(Token::Nil),
            ValueType::Vector(_, _) => {
                Err(error::AjisaiError::from("Nested vectors not supported in token conversion"))
            },
        }
    }

    // トークンを文字列に戻すヘルパーメソッド
    fn tokens_to_string(&self, tokens: &[Token]) -> String {
        tokens.iter()
            .map(|token| self.token_to_string(token))
            .collect::<Vec<String>>()
            .join(" ")
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

    fn execute_word(&mut self, name: &str) -> Result<()> {
        if let Some(def) = self.dictionary.get(name).cloned() {
            if def.is_builtin {
                self.execute_builtin(name)
            } else {
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
        // Vector定義の場合は直接実行（要素を個別実行しない）
        self.execute_tokens(tokens)
    }

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
    match name {
        // 算術・論理演算（> と >= を復活）
        "+" => arithmetic::op_add(self),
        "/" => arithmetic::op_div(self),
        "*" => arithmetic::op_mul(self),
        "-" => arithmetic::op_sub(self),
        "=" => arithmetic::op_eq(self),
        "<=" => arithmetic::op_le(self),
        "<" => arithmetic::op_lt(self),
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
        
        // ワード管理
        "DEF" => control::op_def(self),
        "DEL" => control::op_del(self),
        
        // 条件分岐制御（内部使用）
        "CONDITIONAL_BRANCH" => control::op_conditional_branch(self),
        
        "NOP" => control::op_nop(self),
        
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
                let protected = self.dependencies.get(name)
                    .map(|deps| !deps.is_empty())
                    .unwrap_or(false);
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
                        match token {
                            Token::FunctionComment(_) => None,
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
            Token::String(s) => format!("'{}'", s),
            Token::Boolean(b) => b.to_string(),
            Token::Nil => "nil".to_string(),
            Token::Symbol(s) => s.clone(),
            Token::VectorStart(bracket_type) => bracket_type.opening_char().to_string(),
            Token::VectorEnd(bracket_type) => bracket_type.closing_char().to_string(),
            Token::FunctionComment(comment) => format!("\"{}\"", comment),
            Token::Colon => ":".to_string(),
            Token::LineBreak => "\n".to_string(),
        }
    }
}
