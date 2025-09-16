// rust/src/interpreter/mod.rs - デバッグ出力修正版

pub mod vector_ops;
pub mod arithmetic;
pub mod control;
pub mod io;
pub mod error;

use std::collections::{HashMap, HashSet};
use crate::types::{Workspace, Token, Value, ValueType, ExecutionLine, RepeatControl, TimeControl, Fraction};
use self::error::{Result, AjisaiError};
use web_sys::console;
use wasm_bindgen::JsValue;

pub struct Interpreter {
    pub(crate) workspace: Workspace,
    pub(crate) dictionary: HashMap<String, InterpreterWordDefinition>,
    pub(crate) dependencies: HashMap<String, HashSet<String>>,
    pub(crate) output_buffer: String,
    pub(crate) debug_buffer: String,
    pub(crate) call_stack: Vec<String>,
}

#[derive(Clone)]
pub struct InterpreterWordDefinition {
    pub lines: Vec<ExecutionLine>,
    pub is_builtin: bool,
    pub description: Option<String>,
}

impl Interpreter {
    pub fn new() -> Self {
        console::log_1(&JsValue::from_str("=== INTERPRETER NEW ==="));
        let mut interpreter = Interpreter {
            workspace: Vec::new(),
            dictionary: HashMap::new(),
            dependencies: HashMap::new(),
            output_buffer: String::new(),
            debug_buffer: String::new(),
            call_stack: Vec::new(),
        };
        crate::builtins::register_builtins(&mut interpreter.dictionary);
        console::log_1(&JsValue::from_str(&format!("Interpreter created with {} builtin words", interpreter.dictionary.len())));
        interpreter
    }

    pub fn execute(&mut self, code: &str) -> Result<()> {
        console::log_1(&JsValue::from_str(&format!("=== EXECUTE START ===\nCode: '{}'", code)));
        
        let custom_word_names: HashSet<String> = self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect();
        
        console::log_1(&JsValue::from_str(&format!("Custom words available: {:?}", custom_word_names)));
        
        let tokens = crate::tokenizer::tokenize_with_custom_words(code, &custom_word_names)?;
        
        console::log_1(&JsValue::from_str(&format!("Parsed tokens: {:?}", tokens)));
        
        if tokens.is_empty() { 
            console::log_1(&JsValue::from_str("No tokens to execute"));
            return Ok(()); 
        }

        // ワード定義またはワード削除の検出
        if self.is_word_definition(&tokens) {
            console::log_1(&JsValue::from_str("Detected word definition"));
            return self.process_word_definition(&tokens);
        } else if self.is_word_deletion(&tokens) {
            console::log_1(&JsValue::from_str("Detected word deletion"));
            return self.process_word_deletion(&tokens);
        }

        // 通常の実行
        console::log_1(&JsValue::from_str("Processing as normal execution"));
        self.execute_tokens(&tokens)
    }

    fn is_word_definition(&self, tokens: &[Token]) -> bool {
        console::log_1(&JsValue::from_str("=== is_word_definition ==="));
        
        let result = tokens.len() >= 3 && 
               matches!(tokens[0], Token::VectorStart) &&
               matches!(tokens[1], Token::Symbol(ref s) if s == "DEF");
        
        console::log_1(&JsValue::from_str(&format!("Is word definition: {}", result)));
        result
    }

    fn is_word_deletion(&self, tokens: &[Token]) -> bool {
        console::log_1(&JsValue::from_str("=== is_word_deletion ==="));
        
        // [ DEL [ WORD_NAME ] ] の形式をチェック
        let result = tokens.len() >= 6 && 
               matches!(tokens[0], Token::VectorStart) &&
               matches!(tokens[1], Token::Symbol(ref s) if s == "DEL") &&
               matches!(tokens[2], Token::VectorStart) &&
               matches!(tokens[4], Token::VectorEnd) &&
               matches!(tokens[5], Token::VectorEnd);
        
        console::log_1(&JsValue::from_str(&format!("Is word deletion: {}", result)));
        result
    }

    fn process_word_deletion(&mut self, tokens: &[Token]) -> Result<()> {
        console::log_1(&JsValue::from_str("=== process_word_deletion ==="));
        console::log_1(&JsValue::from_str(&format!("Input tokens: {:?}", tokens)));
        
        // [ DEL [ WORD_NAME ] ] の構造を解析
        if tokens.len() != 6 {
            return Err(AjisaiError::from("Invalid DEL format. Expected: [ DEL [ WORD_NAME ] ]"));
        }
        
        let word_name = match &tokens[3] {
            Token::Symbol(name) => name.clone(),
            _ => return Err(AjisaiError::from("Expected word name symbol in DEL command")),
        };
        
        console::log_1(&JsValue::from_str(&format!("Deleting word: '{}'", word_name)));
        
        control::op_del_word(self, &word_name)
    }

    fn process_word_definition(&mut self, tokens: &[Token]) -> Result<()> {
        console::log_1(&JsValue::from_str("=== process_word_definition ==="));
        console::log_1(&JsValue::from_str(&format!("Input tokens: {:?}", tokens)));
        
        // [ DEF [ ワード名 ] 実行行... ] の構造を解析
        if tokens.len() < 6 {
            return Err(AjisaiError::from("Word definition too short"));
        }
        
        let mut i = 2; // "[ DEF" の次から
        
        // ワード名の抽出 [ ワード名 ]
        if !matches!(tokens[i], Token::VectorStart) {
            return Err(AjisaiError::from("Expected word name vector"));
        }
        i += 1;
        
        let word_name = match &tokens[i] {
            Token::Symbol(name) => name.clone(),
            _ => return Err(AjisaiError::from("Expected word name symbol")),
        };
        i += 1;
        
        if !matches!(tokens[i], Token::VectorEnd) {
            return Err(AjisaiError::from("Expected end of word name vector"));
        }
        i += 1;
        
        console::log_1(&JsValue::from_str(&format!("Defining word: '{}'", word_name)));
        
        // 実行行の解析
        let mut execution_lines = Vec::new();
        
        while i < tokens.len() - 1 { // 最後の ] を除く
            if matches!(tokens[i], Token::VectorStart) {
                console::log_1(&JsValue::from_str(&format!("Parsing execution line starting at token {}", i)));
                let (line, consumed) = self.parse_execution_line(&tokens[i..])?;
                console::log_1(&JsValue::from_str(&format!("Parsed execution line: {:?}", line)));
                execution_lines.push(line);
                i += consumed;
            } else {
                i += 1;
            }
        }
        
        console::log_1(&JsValue::from_str(&format!("Total execution lines: {}", execution_lines.len())));
        
        // ワード登録
        self.dictionary.insert(word_name.clone(), InterpreterWordDefinition {
            lines: execution_lines,
            is_builtin: false,
            description: None,
        });
        
        self.output_buffer.push_str(&format!("Defined word: {}\n", word_name));
        console::log_1(&JsValue::from_str(&format!("Word '{}' defined successfully", word_name)));
        
        Ok(())
    }

    fn parse_execution_line(&self, tokens: &[Token]) -> Result<(ExecutionLine, usize)> {
        console::log_1(&JsValue::from_str("=== parse_execution_line ==="));
        console::log_1(&JsValue::from_str(&format!("Input tokens: {:?}", tokens)));
        
        if !matches!(tokens[0], Token::VectorStart) {
            return Err(AjisaiError::from("Expected vector start"));
        }
        
        let mut i = 1;
        let mut repeat = RepeatControl::default();
        let mut timing = TimeControl::default();
        let mut condition: Option<Vec<Value>> = None;
        let mut action = Vec::new();
        
        // ネストレベル0で要素を解析
        let mut nesting_level = 0;
        let mut current_element_start = i;
        let mut element_count = 0;
        
        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorStart => {
                    if nesting_level == 0 {
                        // 新しい要素の開始
                        current_element_start = i;
                    }
                    nesting_level += 1;
                },
                Token::VectorEnd => {
                    nesting_level -= 1;
                    if nesting_level == 0 {
                        // 要素の終了
                        let element_tokens = &tokens[current_element_start..=i];
                        console::log_1(&JsValue::from_str(&format!("Element {}: {:?}", element_count, element_tokens)));
                        
                        let element_values = self.tokens_to_values(element_tokens)?;
                        
                        match element_count {
                            0 => { 
                                // 最初の要素：時間指定
                                console::log_1(&JsValue::from_str("Processing timing element"));
                                if element_values.len() == 1 {
                                    if let ValueType::Symbol(ref s) = element_values[0].val_type {
                                        timing = self.parse_time_from_string(s)?;
                                    }
                                }
                            },
                            1 => { 
                                // 2番目の要素：条件
                                console::log_1(&JsValue::from_str("Processing condition element"));
                                if !element_values.is_empty() {
                                    condition = Some(element_values);
                                }
                            },
                            2 => { 
                                // 3番目の要素：実行内容
                                console::log_1(&JsValue::from_str("Processing action element"));
                                action = element_values;
                            },
                            _ => {
                                console::log_1(&JsValue::from_str(&format!("Unexpected element at position {}", element_count)));
                            }
                        }
                        
                        element_count += 1;
                        i += 1;
                        current_element_start = i;
                        continue;
                    }
                    if nesting_level < 0 {
                        // 実行行の終了
                        console::log_1(&JsValue::from_str(&format!("Execution line ended. Consumed {} tokens", i + 1)));
                        break;
                    }
                },
                Token::RepeatUnit(ref r) => {
                    if nesting_level == 0 {
                        console::log_1(&JsValue::from_str(&format!("Found repeat unit: {:?}", r)));
                        repeat = r.clone();
                        element_count += 1;
                    }
                },
                Token::TimeUnit(ref t) => {
                    if nesting_level == 0 {
                        console::log_1(&JsValue::from_str(&format!("Found time unit: {:?}", t)));
                        timing = t.clone();
                        element_count += 1;
                    }
                },
                _ => {}
            }
            
            i += 1;
        }
        
        let line = ExecutionLine {
            repeat,
            timing,
            condition,
            action,
        };
        
        // Display トレイトを使用してデバッグ出力を修正
        console::log_1(&JsValue::from_str(&format!("Final execution line: repeat={}, timing={}, has_condition={}, action_count={}", 
            line.repeat, line.timing, line.condition.is_some(), line.action.len())));
        
        Ok((line, i))
    }

    fn tokens_to_values(&self, tokens: &[Token]) -> Result<Vec<Value>> {
        console::log_1(&JsValue::from_str(&format!("=== tokens_to_values ===\nTokens: {:?}", tokens)));
        
        let mut values = Vec::new();
        let mut i = 0;
        
        // 外側のベクトル記号をスキップ
        if tokens.len() >= 2 && matches!(tokens[0], Token::VectorStart) && matches!(tokens[tokens.len()-1], Token::VectorEnd) {
            i = 1;
            let end = tokens.len() - 1;
            
            while i < end {
                let value = self.token_to_value(&tokens[i])?;
                console::log_1(&JsValue::from_str(&format!("Converted token {:?} to value {:?}", tokens[i], value)));
                values.push(value);
                i += 1;
            }
        }
        
        console::log_1(&JsValue::from_str(&format!("Final values: {:?}", values)));
        Ok(values)
    }

    fn token_to_value(&self, token: &Token) -> Result<Value> {
        match token {
            Token::Number(s) => {
                let frac = Fraction::from_str(s)?;
                Ok(Value { val_type: ValueType::Number(frac) })
            },
            Token::String(s) => Ok(Value { val_type: ValueType::String(s.clone()) }),
            Token::Boolean(b) => Ok(Value { val_type: ValueType::Boolean(*b) }),
            Token::Nil => Ok(Value { val_type: ValueType::Nil }),
            Token::Symbol(s) => Ok(Value { val_type: ValueType::Symbol(s.clone()) }),
            _ => Err(AjisaiError::from("Cannot convert this token to a value")),
        }
    }

    fn parse_time_from_string(&self, _s: &str) -> Result<TimeControl> {
        // 簡易実装
        Ok(TimeControl::Immediate)
    }

    pub(crate) fn execute_tokens(&mut self, tokens: &[Token]) -> Result<()> {
        console::log_1(&JsValue::from_str(&format!("=== execute_tokens ===\nTokens: {:?}", tokens)));
        
        let mut i = 0;
        while i < tokens.len() {
            console::log_1(&JsValue::from_str(&format!("Processing token[{}]: {:?}", i, tokens[i])));
            
            match &tokens[i] {
                Token::Number(s) => {
                    console::log_1(&JsValue::from_str(&format!("Number token: {}", s)));
                    
                    let frac = match Fraction::from_str(s) {
                        Ok(f) => {
                            console::log_1(&JsValue::from_str(&format!("Parsed to fraction: {}/{}", f.numerator, f.denominator)));
                            f
                        },
                        Err(e) => {
                            console::error_1(&JsValue::from_str(&format!("Failed to parse number: {}", e)));
                            return Err(AjisaiError::from(format!("Failed to parse number: {}", e)));
                        }
                    };
                    
                    let val = Value { val_type: ValueType::Number(frac) };
                    let wrapped = Value { val_type: ValueType::Vector(vec![val])};
                    
                    console::log_1(&JsValue::from_str(&format!("Pushing wrapped number: {}", wrapped)));
                    self.workspace.push(wrapped);
                    
                    console::log_1(&JsValue::from_str(&format!("Workspace size: {}", self.workspace.len())));
                    i += 1;
                },
                Token::String(s) => {
                    self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::String(s.clone()) }])});
                    i += 1;
                },
                Token::Boolean(b) => {
                    self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Boolean(*b) }])});
                    i += 1;
                },
                Token::Nil => {
                    self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Nil }])});
                    i += 1;
                },
                Token::VectorStart => {
                    console::log_1(&JsValue::from_str("Vector start"));
                    let (vector_values, consumed) = self.collect_vector(tokens, i)?;
                    console::log_1(&JsValue::from_str(&format!("Collected vector with {} elements", vector_values.len())));
                    self.workspace.push(Value { val_type: ValueType::Vector(vector_values)});
                    i += consumed;
                },
                Token::Symbol(name) => {
                    console::log_1(&JsValue::from_str(&format!("Executing symbol: {}", name)));
                    self.execute_word(name)?;
                    i += 1;
                },
                Token::VectorEnd => return Err(AjisaiError::from("Unexpected vector end")),
                _ => { i += 1; }
            }
            
            if !self.workspace.is_empty() {
                console::log_1(&JsValue::from_str(&format!("Workspace top: {}", self.workspace.last().unwrap())));
            }
        }
        
        console::log_1(&JsValue::from_str(&format!("=== execute_tokens complete. Final workspace size: {} ===", self.workspace.len())));
        Ok(())
    }

    fn collect_vector(&self, tokens: &[Token], start: usize) -> Result<(Vec<Value>, usize)> {
        console::log_1(&JsValue::from_str(&format!("Collecting vector starting at {}", start)));
        
        let mut values = Vec::new();
        let mut i = start + 1;
        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorStart => {
                    let (nested_values, consumed) = self.collect_vector(tokens, i)?;
                    values.push(Value { val_type: ValueType::Vector(nested_values)});
                    i += consumed;
                },
                Token::VectorEnd => {
                    console::log_1(&JsValue::from_str(&format!("Vector complete with {} elements", values.len())));
                    return Ok((values, i - start + 1));
                },
                token => {
                    console::log_1(&JsValue::from_str(&format!("Adding token to vector: {:?}", token)));
                    values.push(self.token_to_value(token)?);
                    i += 1;
                }
            }
        }
        Err(AjisaiError::from("Unclosed vector"))
    }

    fn execute_word(&mut self, name: &str) -> Result<()> {
        console::log_1(&JsValue::from_str(&format!("execute_word: {}", name)));
        console::log_1(&JsValue::from_str(&format!("Workspace before: {} items", self.workspace.len())));
        
        if let Some(def) = self.dictionary.get(name).cloned() {
            if def.is_builtin {
                let result = self.execute_builtin(name);
                console::log_1(&JsValue::from_str(&format!("Workspace after builtin {}: {} items", name, self.workspace.len())));
                result
            } else {
                self.call_stack.push(name.to_string());
                let result = self.execute_custom_word(name, &def);
                self.call_stack.pop();
                result.map_err(|e| e.with_context(&self.call_stack))
            }
        } else {
            Err(AjisaiError::UnknownWord(name.to_string()))
        }
    }

    fn execute_custom_word(&mut self, name: &str, def: &InterpreterWordDefinition) -> Result<()> {
        console::log_1(&JsValue::from_str(&format!("=== execute_custom_word: {} ===", name)));
        console::log_1(&JsValue::from_str(&format!("Word has {} execution lines", def.lines.len())));
        
        for (line_idx, line) in def.lines.iter().enumerate() {
            console::log_1(&JsValue::from_str(&format!("Executing line {}: {:?}", line_idx, line)));
            
            // 条件チェック
            if let Some(ref condition) = line.condition {
                console::log_1(&JsValue::from_str(&format!("Checking condition: {:?}", condition)));
                // TODO: 条件評価の実装
            }
            
            // アクション実行
            console::log_1(&JsValue::from_str(&format!("Executing action: {:?}", line.action)));
            for action in &line.action {
                match &action.val_type {
                    ValueType::Symbol(sym_name) => {
                        self.execute_word(sym_name)?;
                    },
                    _ => {
                        self.workspace.push(action.clone());
                    }
                }
            }
            
            // 反復制御
            match &line.repeat {
                RepeatControl::Times(n) => {
                    console::log_1(&JsValue::from_str(&format!("Repeat {} times (not implemented yet)", n)));
                },
                RepeatControl::Once => {
                    console::log_1(&JsValue::from_str("Execute once"));
                },
                _ => {
                    console::log_1(&JsValue::from_str(&format!("Repeat control {:?} not implemented yet", line.repeat)));
                }
            }
        }
        
        Ok(())
    }

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        console::log_1(&JsValue::from_str(&format!("execute_builtin: {}", name)));
        
        match name {
            "GET" => vector_ops::op_get(self),
            "DUP" => vector_ops::op_dup_workspace(self),
            "SWAP" => vector_ops::op_swap_workspace(self),
            "+" => arithmetic::op_add(self),
            "-" => arithmetic::op_sub(self),
            "*" => arithmetic::op_mul(self),
            "/" => arithmetic::op_div(self),
            "PRINT" => io::op_print(self),
            "RESET" => self.execute_reset(),
            _ => Err(AjisaiError::UnknownBuiltin(name.to_string())),
        }
    }

    pub fn execute_reset(&mut self) -> Result<()> {
        if let Some(window) = web_sys::window() {
            let event = web_sys::CustomEvent::new("ajisai-reset").map_err(|_| AjisaiError::from("Failed to create reset event"))?;
            window.dispatch_event(&event).map_err(|_| AjisaiError::from("Failed to dispatch reset event"))?;
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
        console::log_1(&JsValue::from_str(&format!("Execute single token: {:?}", token)));
        
        self.output_buffer.clear();
        match token {
            Token::Number(s) => {
                console::log_1(&JsValue::from_str(&format!("Parsing number string: {}", s)));
                
                let frac = Fraction::from_str(s)?;
                
                console::log_1(&JsValue::from_str(&format!("Parsed fraction: {}/{}", frac.numerator, frac.denominator)));
                
                let wrapped = Value { 
                    val_type: ValueType::Vector(vec![Value { val_type: ValueType::Number(frac) }])
                };
                
                let display = format!("{}", wrapped);
                console::log_1(&JsValue::from_str(&format!("Pushing to workspace: {}", display)));
                
                self.workspace.push(wrapped);
                
                console::log_1(&JsValue::from_str(&format!("Workspace size after push: {}", self.workspace.len())));
                
                Ok(format!("Pushed {}", display))
            },
            Token::String(s) => {
                let wrapped = Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::String(s.clone())}])};
                self.workspace.push(wrapped);
                Ok(format!("Pushed wrapped string: ['{}']", s))
            },
            Token::Boolean(b) => {
                let wrapped = Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Boolean(*b)}])};
                self.workspace.push(wrapped);
                Ok(format!("Pushed wrapped boolean: [{}]", b))
            },
            Token::Nil => {
                let wrapped = Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Nil }])};
                self.workspace.push(wrapped);
                Ok("Pushed wrapped nil: [nil]".to_string())
            },
            Token::Symbol(name) => {
                console::log_1(&JsValue::from_str(&format!("Executing word: {}", name)));
                self.execute_word(name)?;
                let output = self.get_output();
                Ok(if output.is_empty() { format!("Executed word: {}", name) } else { output })
            },
            _ => Ok(format!("Skipped token: {:?}", token)),
        }
    }

    pub fn get_word_definition(&self, name: &str) -> Option<String> {
        self.dictionary.get(name).and_then(|def| {
            if def.is_builtin { return None; }
            // 簡易実装
            Some(format!("Word definition for {}", name))
        })
    }

    pub fn restore_custom_word(&mut self, name: String, _tokens: Vec<Token>, description: Option<String>) -> Result<()> {
        // 簡易実装
        self.dictionary.insert(name.to_uppercase(), InterpreterWordDefinition {
            lines: vec![],
            is_builtin: false,
            description,
        });
        Ok(())
    }

    pub fn get_output(&mut self) -> String { std::mem::take(&mut self.output_buffer) }
    pub fn get_debug_output(&mut self) -> String { std::mem::take(&mut self.debug_buffer) }
    pub fn get_workspace(&self) -> &Workspace { &self.workspace }
    
    pub fn get_custom_words_info(&self) -> Vec<(String, Option<String>, bool)> {
        self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, def)| {
                let protected = self.dependencies.get(name).map_or(false, |deps| !deps.is_empty());
                (name.clone(), def.description.clone(), protected)
            })
            .collect()
    }
   
    pub fn set_workspace(&mut self, workspace: Workspace) { self.workspace = workspace; }
}
