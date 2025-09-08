// rust/src/interpreter/mod.rs (完全修正版)

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
    pub repeat_count: Option<i64>,
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
        self.append_output(&format!("DEBUG: execute() called with code: '{}'\n", code));
        
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
            
            def_result?;
            
            if !remaining_code.trim().is_empty() {
                self.append_output(&format!("DEBUG: Executing remaining code: '{}'\n", remaining_code));
                self.execute(&remaining_code)?;
            } else {
                self.append_output("DEBUG: No remaining code to execute\n");
            }
            
            return Ok(());
        }

        self.append_output("DEBUG: No DEF pattern, executing tokens normally\n");
        self.execute_tokens(&tokens)
    }

    pub fn execute_reset(&mut self) -> Result<()> {
        if let Some(window) = web_sys::window() {
            let event = web_sys::CustomEvent::new("ajisai-reset")
                .map_err(|_| error::AjisaiError::from("Failed to create reset event"))?;
            window.dispatch_event(&event)
                .map_err(|_| error::AjisaiError::from("Failed to dispatch reset event"))?;
        }
        
        self.workspace.clear();
        self.dictionary.clear();
        self.dependencies.clear();
        self.output_buffer.clear();
        self.call_stack.clear();
        
        crate::builtins::register_builtins(&mut self.dictionary);
        
        Ok(())
    }

    pub fn execute_single_token(&mut self, token: &Token) -> Result<String> {
        self.output_buffer.clear();
        
        match token {
            Token::Number(num, den) => {
                let wrapped_value = Value {
                    val_type: ValueType::Vector(
                        vec![Value {
                            val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)),
                        }],
                        BracketType::Square
                    )
                };
                self.workspace.push(wrapped_value);
                Ok(format!("Pushed wrapped number: [{}]", if *den == 1 { num.to_string() } else { format!("{}/{}", num, den) }))
            },
            Token::String(s) => {
                let wrapped_value = Value {
                    val_type: ValueType::Vector(
                        vec![Value {
                            val_type: ValueType::String(s.clone()),
                        }],
                        BracketType::Square
                    )
                };
                self.workspace.push(wrapped_value);
                Ok(format!("Pushed wrapped string: ['{}']", s))
            },
            Token::Boolean(b) => {
                let wrapped_value = Value {
                    val_type: ValueType::Vector(
                        vec![Value {
                            val_type: ValueType::Boolean(*b),
                        }],
                        BracketType::Square
                    )
                };
                self.workspace.push(wrapped_value);
                Ok(format!("Pushed wrapped boolean: [{}]", b))
            },
            Token::Nil => {
                let wrapped_value = Value {
                    val_type: ValueType::Vector(
                        vec![Value {
                            val_type: ValueType::Nil,
                        }],
                        BracketType::Square
                    )
                };
                self.workspace.push(wrapped_value);
                Ok("Pushed wrapped nil: [nil]".to_string())
            },
            Token::FunctionComment(comment) => {
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
        // 最初のDEFの位置を探す
        let def_position = tokens.iter().position(|t| {
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
                
                // 複数行定義の解析（REPEAT対応）
                let multiline_def = self.parse_multiline_definition(body_tokens);
                
                let remaining_code = if let Some(def_pos_in_code) = code.find("DEF") {
                    let after_first_def = &code[def_pos_in_code + 3..];
                    let lines: Vec<&str> = after_first_def.lines().collect();
                    if lines.len() > 1 {
                        lines[1..].join("\n").trim().to_string()
                    } else {
                        after_first_def.trim().to_string()
                    }
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

    fn parse_multiline_definition(&self, tokens: &[Token]) -> MultiLineDefinition {
        let mut lines = Vec::new();
        let mut current_line = Vec::new();
        let mut has_conditionals = false;
        let mut repeat_count = None;
        
        let mut i = 0;
        
        // 最初に REPEAT 回数指定をチェック（修正版）
        if i < tokens.len() {
            if let Token::Number(count, 1) = &tokens[i] {
                if i + 1 < tokens.len() {
                    if let Token::Symbol(word) = &tokens[i + 1] {
                        if word == "REPEAT" {
                            repeat_count = Some(*count);
                            i += 2; // 数値とREPEATをスキップ
                            self.append_output(&format!("DEBUG: Found REPEAT with count: {}\n", count));
                        }
                    }
                }
            }
        }
        
        self.append_output(&format!("DEBUG: Starting token parsing from index {}\n", i));
        
        // 残りのトークンを行単位で処理
        while i < tokens.len() {
            match &tokens[i] {
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
                    if let Token::Colon = &tokens[i] {
                        has_conditionals = true;
                    }
                    current_line.push(tokens[i].clone());
                }
            }
            i += 1;
        }
        
        // 最後の行を追加
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        self.append_output(&format!("DEBUG: Parsed multiline definition - lines: {}, has_conditionals: {}, repeat_count: {:?}\n", 
            lines.len(), has_conditionals, repeat_count));
        
        MultiLineDefinition {
            lines,
            has_conditionals,
            repeat_count,
        }
    }

    fn define_word_from_multiline(&mut self, name: String, multiline_def: MultiLineDefinition) -> Result<()> {
        let name = name.to_uppercase();
        
        self.append_output(&format!("DEBUG: define_word_from_multiline - name: {}, repeat_count: {:?}, has_conditionals: {}\n", 
            name, multiline_def.repeat_count, multiline_def.has_conditionals));
        
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

        // 処理方式の判定と実行（修正版）
        let executable_tokens = if multiline_def.repeat_count.is_some() {
            // 明示的なREPEAT指定がある場合
            self.append_output("DEBUG: Using REPEAT processing\n");
            if multiline_def.lines.len() == 1 && !multiline_def.has_conditionals {
                // 単一行 + REPEAT → 単純反復
                self.create_simple_repeat_tokens(multiline_def.repeat_count, &multiline_def.lines[0])
            } else {
                // 複数行 or 条件分岐 + REPEAT → 高度なREPEAT処理
                control::create_repeat_execution_tokens(multiline_def.repeat_count, &multiline_def.lines)?
            }
        } else if multiline_def.has_conditionals {
            // 条件分岐ありだがREPEAT指定なし → 従来の分岐処理
            self.append_output("DEBUG: Using traditional conditional processing\n");
            self.create_traditional_conditional_tokens(&multiline_def.lines)?
        } else if multiline_def.lines.len() == 1 {
            // 単一行 → Vector括弧を取り除く
            self.append_output("DEBUG: Using single line processing\n");
            self.extract_vector_content_if_needed(&multiline_def.lines[0])?
        } else {
            // 複数行 + REPEATなし + 条件なし → 順次実行
            self.append_output("DEBUG: Using sequential processing\n");
            self.create_sequential_execution_tokens(&multiline_def.lines)
        };

        self.append_output(&format!("DEBUG: Generated executable tokens: {:?}\n", executable_tokens));

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

    // 従来の分岐処理（新規追加）
    fn create_traditional_conditional_tokens(&self, lines: &[Vec<Token>]) -> Result<Vec<Token>> {
        if lines.is_empty() {
            return Err(error::AjisaiError::from("No lines found"));
        }

        // デフォルト行（条件なし行）の存在チェック
        let has_default = lines.iter().any(|line| {
            !line.iter().any(|token| matches!(token, Token::Colon))
        });
        
        if !has_default {
            return Err(error::AjisaiError::from("Default line (line without condition) is required for safety"));
        }

        // 従来の分岐処理：IF_SELECT構造を構築
        self.build_traditional_conditional_structure(lines)
    }

    fn build_traditional_conditional_structure(&self, lines: &[Vec<Token>]) -> Result<Vec<Token>> {
        let mut conditional_lines = Vec::new();
        let mut default_line = None;

        // 条件行とデフォルト行を分離
        for line in lines {
            if let Some(colon_pos) = line.iter().position(|t| matches!(t, Token::Colon)) {
                // 条件行
                let condition = &line[..colon_pos];
                let action = &line[colon_pos + 1..];
                
                conditional_lines.push((condition.to_vec(), action.to_vec()));
            } else {
                // デフォルト行
                default_line = Some(line.clone());
            }
        }

        if conditional_lines.is_empty() {
            // 条件行がない場合は、デフォルト行のみ実行
            if let Some(default) = default_line {
                return Ok(default);
            } else {
                return Ok(Vec::new());
            }
        }

        // ネストしたIF_SELECT構造を構築
        Ok(self.build_nested_if_select(&conditional_lines, &default_line.unwrap_or_default()))
    }

    fn build_nested_if_select(&self, conditional_lines: &[(Vec<Token>, Vec<Token>)], default_action: &[Token]) -> Vec<Token> {
        if conditional_lines.is_empty() {
            return default_action.to_vec();
        }

        if conditional_lines.len() == 1 {
            let (condition, action) = &conditional_lines[0];
            let mut result = Vec::new();
            
            // 条件
            result.extend(condition.iter().cloned());
            
            // 真の場合のアクション
            result.push(Token::VectorStart(BracketType::Square));
            result.extend(action.iter().cloned());
            result.push(Token::VectorEnd(BracketType::Square));
            
            // 偽の場合のアクション（デフォルト）
            result.push(Token::VectorStart(BracketType::Square));
            result.extend(default_action.iter().cloned());
            result.push(Token::VectorEnd(BracketType::Square));
            
            result.push(Token::Symbol("IF_SELECT".to_string()));
            
            return result;
        }

        // 複数の条件行がある場合、再帰的にネスト
        let (first_condition, first_action) = &conditional_lines[0];
        let remaining_lines = &conditional_lines[1..];
        
        let mut result = Vec::new();
        
        // 最初の条件
        result.extend(first_condition.iter().cloned());
        
        // 真の場合のアクション
        result.push(Token::VectorStart(BracketType::Square));
        result.extend(first_action.iter().cloned());
        result.push(Token::VectorEnd(BracketType::Square));
        
        // 偽の場合のアクション（残りの条件を再帰処理）
        result.push(Token::VectorStart(BracketType::Square));
        let nested = self.build_nested_if_select(remaining_lines, default_action);
        result.extend(nested);
        result.push(Token::VectorEnd(BracketType::Square));
        
        result.push(Token::Symbol("IF_SELECT".to_string()));
        
        result
    }

    fn create_simple_repeat_tokens(&self, repeat_count: Option<i64>, line: &[Token]) -> Vec<Token> {
        let mut result = Vec::new();
        
        // 回数指定
        let count = repeat_count.unwrap_or(1);
        result.push(Token::Number(count, 1));
        
        // アクション（Vectorでラップ）
        result.push(Token::VectorStart(BracketType::Square));
        result.extend(line.iter().cloned());
        result.push(Token::VectorEnd(BracketType::Square));
        
        // 簡単な反復実行ワード
        result.push(Token::Symbol("SIMPLE_REPEAT".to_string()));
        
        result
    }

    fn extract_vector_content_if_needed(&self, tokens: &[Token]) -> Result<Vec<Token>> {
        if tokens.len() >= 2 {
            if let (Token::VectorStart(_), Token::VectorEnd(_)) = (&tokens[0], &tokens[tokens.len() - 1]) {
                return Ok(tokens[1..tokens.len() - 1].to_vec());
            }
        }
        
        Ok(tokens.to_vec())
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
                    let wrapped_value = Value {
                        val_type: ValueType::Vector(
                            vec![Value {
                                val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)),
                            }],
                            BracketType::Square
                        )
                    };
                    self.workspace.push(wrapped_value);
                    self.append_output(&format!("DEBUG: Pushed wrapped number [{}], workspace size: {}\n", 
                        if *den == 1 { num.to_string() } else { format!("{}/{}", num, den) }, 
                        self.workspace.len()));
                    i += 1;
                },
                Token::String(s) => {
                    let wrapped_value = Value {
                        val_type: ValueType::Vector(
                            vec![Value {
                                val_type: ValueType::String(s.clone()),
                            }],
                            BracketType::Square
                        )
                    };
                    self.workspace.push(wrapped_value);
                    self.append_output(&format!("DEBUG: Pushed wrapped string ['{}'], workspace size: {}\n", s, self.workspace.len()));
                    i += 1;
                },
                Token::Boolean(b) => {
                    let wrapped_value = Value {
                        val_type: ValueType::Vector(
                            vec![Value {
                                val_type: ValueType::Boolean(*b),
                            }],
                            BracketType::Square
                        )
                    };
                    self.workspace.push(wrapped_value);
                    i += 1;
                },
                Token::Nil => {
                    let wrapped_value = Value {
                        val_type: ValueType::Vector(
                            vec![Value {
                                val_type: ValueType::Nil,
                            }],
                            BracketType::Square
                        )
                    };
                    self.workspace.push(wrapped_value);
                    i += 1;
                },
                Token::FunctionComment(_) => {
                    i += 1;
                },
                Token::Colon => {
                    i += 1;
                },
                Token::LineBreak => {
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
        self.execute_tokens(tokens)
    }

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        match name {
            // 位置指定操作
            "GET" => vector_ops::op_get(self),
            "INSERT" => vector_ops::op_insert(self),
            "REPLACE" => vector_ops::op_replace(self),
            "REMOVE" => vector_ops::op_remove(self),
            
            // 量指定操作
            "LENGTH" => vector_ops::op_length(self),
            "TAKE" => vector_ops::op_take(self),
            "DROP" => vector_ops::op_drop_vector(self),
            "SPLIT" => vector_ops::op_split(self),
            
            // ワークスペース操作
            "DUP" => vector_ops::op_dup_workspace(self),
            "SWAP" => vector_ops::op_swap_workspace(self),
            "ROT" => vector_ops::op_rot_workspace(self),
            
            // Vector構造操作
            "CONCAT" => vector_ops::op_concat(self),
            "REVERSE" => vector_ops::op_reverse(self),
            
            // 算術演算
            "+" => arithmetic::op_add(self),
            "-" => arithmetic::op_sub(self),
            "*" => arithmetic::op_mul(self),
            "/" => arithmetic::op_div(self),
            
            // 比較演算
            "=" => arithmetic::op_eq(self),
            "<" => arithmetic::op_lt(self),
            "<=" => arithmetic::op_le(self),
            ">" => arithmetic::op_gt(self),
            ">=" => arithmetic::op_ge(self),
            
            // 論理演算
            "AND" => arithmetic::op_and(self),
            "OR" => arithmetic::op_or(self),
            "NOT" => arithmetic::op_not(self),
            
            // 入出力
            "PRINT" => io::op_print(self),
            
            // ワード管理・システム
            "DEF" => control::op_def(self),
            "DEL" => control::op_del(self),
            "RESET" => {
                self.execute_reset()?;
                Ok(())
            },
            
            // 条件分岐制御
            "IF_SELECT" => control::op_if_select(self),
            
            // REPEAT制御
            "EXECUTE_REPEAT" => control::op_execute_repeat(self),
            "SIMPLE_REPEAT" => {
                // 簡単な反復実行
                if self.workspace.len() < 2 {
                    return Err(error::AjisaiError::WorkspaceUnderflow);
                }
                
                let action_val = self.workspace.pop().unwrap();
                let count_val = self.workspace.pop().unwrap();
                
                let count = match count_val.val_type {
                    ValueType::Vector(ref v, _) if v.len() == 1 => {
                        match &v[0].val_type {
                            ValueType::Number(n) if n.denominator == 1 => n.numerator,
                            _ => return Err(error::AjisaiError::type_error("integer count", "other type")),
                        }
                    },
                    _ => return Err(error::AjisaiError::type_error("single-element vector with integer", "other type")),
                };
                
                if count < 0 {
                    return Err(error::AjisaiError::from("Repeat count must be non-negative"));
                }
                
                match action_val.val_type {
                    ValueType::Vector(action_tokens_values, _) => {
                        // Vectorの内容をトークンに変換
                        let tokens = self.vector_content_to_tokens(action_tokens_values)?;
                        
                        // 指定回数だけ実行
                        for _i in 0..count {
                            self.execute_tokens(&tokens)?;
                        }
                        Ok(())
                    },
                    _ => Err(error::AjisaiError::type_error("vector", "other type")),
                }
            },
            
            _ => Err(error::AjisaiError::UnknownBuiltin(name.to_string())),
        }
    }

    fn vector_content_to_tokens(&self, values: Vec<Value>) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        for value in values {
            match value.val_type {
                ValueType::Vector(inner_values, bracket_type) => {
                    tokens.push(Token::VectorStart(bracket_type.clone()));
                    let inner_tokens = self.vector_content_to_tokens(inner_values)?;
                    tokens.extend(inner_tokens);
                    tokens.push(Token::VectorEnd(bracket_type));
                },
                ValueType::Number(frac) => {
                    tokens.push(Token::Number(frac.numerator, frac.denominator));
                },
                ValueType::String(s) => {
                    tokens.push(Token::String(s));
                },
                ValueType::Boolean(b) => {
                    tokens.push(Token::Boolean(b));
                },
                ValueType::Symbol(s) => {
                    tokens.push(Token::Symbol(s));
                },
                ValueType::Nil => {
                    tokens.push(Token::Nil);
                },
            }
        }
        Ok(tokens)
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
