// rust/src/interpreter/mod.rs

pub mod vector_ops;
pub mod arithmetic;
pub mod control;
pub mod io;
pub mod error;
pub mod leap;

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

    self.execute_tokens(&tokens)
}

    fn execute_tokens(&mut self, tokens: &[Token]) -> Result<()> {
        use web_sys::console;
        console::log_1(&format!("=== EXECUTE_TOKENS START ===").into());
        console::log_1(&format!("Tokens: {:?}", tokens).into());
        console::log_1(&format!("Initial workspace: {} items", self.workspace.len()).into());
        
        let mut i = 0;
        while i < tokens.len() {
            console::log_1(&format!("--- Processing token[{}]: {:?} ---", i, tokens[i]).into());
            console::log_1(&format!("Workspace before: {} items", self.workspace.len()).into());
            
            match &tokens[i] {
                Token::Number(num, den) => {
    let num_val = *num;
    let den_val = *den;
    self.workspace.push(Value {
        val_type: ValueType::Number(crate::types::Fraction::new(num_val, den_val)),
    });
    console::log_1(&format!("Added number: {}/{}", num_val, den_val).into());
    i += 1;
},
Token::String(s) => {
    let string_val = s.clone();
    self.workspace.push(Value {
        val_type: ValueType::String(string_val.clone()),
    });
    console::log_1(&format!("Added string: {}", string_val).into());
    i += 1;
},
Token::Boolean(b) => {
    let bool_val = *b;
    self.workspace.push(Value {
        val_type: ValueType::Boolean(bool_val),
    });
    console::log_1(&format!("Added boolean: {}", bool_val).into());
    i += 1;
},
                Token::Nil => {
                    self.workspace.push(Value {
                        val_type: ValueType::Nil,
                    });
                    console::log_1(&format!("Added nil").into());
                    i += 1;
                },
                Token::VectorStart => {
    console::log_1(&format!("Processing vector...").into());
    let (vector_values, consumed) = self.collect_vector(tokens, i)?;
    let vector_len = vector_values.len(); // 長さを先に記録
    self.workspace.push(Value {
        val_type: ValueType::Vector(vector_values),
    });
    console::log_1(&format!("Added vector with {} elements, consumed {} tokens", vector_len, consumed).into());
    i += consumed;
},
                Token::Symbol(name) => {
                    console::log_1(&format!("Processing symbol: {}", name).into());
                    
                    if name == "定" || name == "DEF" {
                        console::log_1(&format!("Handling DEF").into());
                        self.handle_def()?;
                    } else {
                        console::log_1(&format!("Executing word: {}", name).into());
                        match self.execute_word(name) {
                            Ok(()) => {
                                console::log_1(&format!("Word '{}' executed successfully", name).into());
                            },
                            Err(e) => {
                                console::log_1(&format!("Error executing word '{}': {}", name, e).into());
                                return Err(e);
                            }
                        }
                    }
                    i += 1;
                },
                Token::VectorEnd => {
                    console::log_1(&format!("Unexpected vector end").into());
                    return Err(error::AjisaiError::from("Unexpected vector end"));
                },
            }
            
            console::log_1(&format!("Workspace after: {} items", self.workspace.len()).into());
            if !self.workspace.is_empty() {
                console::log_1(&format!("Top value: {:?}", self.workspace.last().unwrap()).into());
            }
        }
        
        console::log_1(&format!("=== EXECUTE_TOKENS END ===").into());
        Ok(())
    }

    fn collect_vector(&self, tokens: &[Token], start: usize) -> Result<(Vec<Value>, usize)> {
        let mut values = Vec::new();
        let mut i = start + 1;
        let mut depth = 1;

        while i < tokens.len() && depth > 0 {
            match &tokens[i] {
                Token::VectorStart => {
                    depth += 1;
                },
                Token::VectorEnd => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok((values, i - start + 1));
                    }
                },
                token if depth == 1 => {
                    values.push(self.token_to_value(token)?);
                }
                _ => {}
            }
            i += 1;
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
            _ => Err(error::AjisaiError::from("Cannot convert token to value")),
        }
    }

    fn handle_def(&mut self) -> Result<()> {
    use web_sys::console;
    console::log_1(&format!("=== HANDLE_DEF START ===").into());
    console::log_1(&format!("Workspace size: {}", self.workspace.len()).into());
    
    if self.workspace.len() < 2 {
        return Err(error::AjisaiError::from("定 requires vector and name"));
    }

    let name_val = self.workspace.pop().unwrap();
    let code_val = self.workspace.pop().unwrap();
    
    console::log_1(&format!("Name value: {:?}", name_val).into());
    console::log_1(&format!("Code value: {:?}", code_val).into());

    let name = match name_val.val_type {
        ValueType::String(s) => s.to_uppercase(),
        _ => return Err(error::AjisaiError::from("定 requires string name")),
    };

    let tokens = match code_val.val_type {
        ValueType::Vector(v) => {
            console::log_1(&format!("Converting vector to tokens: {:?}", v).into());
            let result = self.vector_to_tokens(v)?;
            console::log_1(&format!("Converted to tokens: {:?}", result).into());
            result
        },
        _ => return Err(error::AjisaiError::from("定 requires vector")),
    };

    if let Some(existing) = self.dictionary.get(&name) {
        if existing.is_builtin {
            return Err(error::AjisaiError::from(format!("Cannot redefine builtin word: {}", name)));
        }
    }

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

    if let Some(old_deps) = self.get_word_dependencies(&name) {
        for dep in old_deps {
            if let Some(reverse_deps) = self.dependencies.get_mut(&dep) {
                reverse_deps.remove(&name);
            }
        }
    }

    for token in &tokens {
        if let Token::Symbol(sym) = token {
            if self.dictionary.contains_key(sym) && !self.is_builtin_word(sym) {
                self.dependencies.entry(sym.clone())
                    .or_insert_with(HashSet::new)
                    .insert(name.clone());
            }
        }
    }

    self.dictionary.insert(name.clone(), WordDefinition {
        tokens,
        is_builtin: false,
        description: None,
        category: None,
    });

    // この行を削除：crate::tokenizer::register_custom_word(&name);

    console::log_1(&format!("Word '{}' defined successfully", name).into());
    console::log_1(&format!("=== HANDLE_DEF END ===").into());

    self.append_output(&format!("Defined: {}\n", name));
    Ok(())
}
    pub fn vector_to_tokens(&self, vector: Vec<Value>) -> Result<Vec<Token>> {
        use web_sys::console;
        console::log_1(&format!("=== VECTOR_TO_TOKENS START ===").into());
        console::log_1(&format!("Input vector: {:?}", vector).into());
        
        let mut tokens = Vec::new();
        for (i, value) in vector.iter().enumerate() {
            console::log_1(&format!("Converting value[{}]: {:?}", i, value).into());
            let token = self.value_to_token(value.clone())?;
            console::log_1(&format!("Converted to token[{}]: {:?}", i, token).into());
            tokens.push(token);
        }
        
        console::log_1(&format!("Final tokens: {:?}", tokens).into());
        console::log_1(&format!("=== VECTOR_TO_TOKENS END ===").into());
        Ok(tokens)
    }

    fn value_to_token(&self, value: Value) -> Result<Token> {
        match value.val_type {
            ValueType::Number(frac) => Ok(Token::Number(frac.numerator, frac.denominator)),
            ValueType::String(s) => Ok(Token::String(s)),
            ValueType::Boolean(b) => Ok(Token::Boolean(b)),
            ValueType::Symbol(s) => Ok(Token::Symbol(s)),
            ValueType::Nil => Ok(Token::Nil),
            ValueType::Vector(_) => Err(error::AjisaiError::from("Nested vectors not supported in DEF")),
        }
    }

    fn get_word_dependencies(&self, word_name: &str) -> Option<Vec<String>> {
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

    fn is_builtin_word(&self, name: &str) -> bool {
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
        use web_sys::console;
        console::log_1(&format!("=== EXECUTE_WORD START: {} ===", name).into());
        
        if let Some(def) = self.dictionary.get(name).cloned() {
            if def.is_builtin {
                console::log_1(&format!("Executing builtin word: {}", name).into());
                let result = self.execute_builtin(name);
                match &result {
                    Ok(()) => console::log_1(&format!("Builtin '{}' completed successfully", name).into()),
                    Err(e) => console::log_1(&format!("Builtin '{}' failed: {}", name, e).into()),
                }
                console::log_1(&format!("=== EXECUTE_WORD END: {} ===", name).into());
                result
            } else {
                console::log_1(&format!("Executing custom word: {}", name).into());
                console::log_1(&format!("Custom word tokens: {:?}", def.tokens).into());
                self.call_stack.push(name.to_string());
                let result = self.execute_custom_word(&def.tokens);
                self.call_stack.pop();
                match &result {
                    Ok(()) => console::log_1(&format!("Custom word '{}' completed successfully", name).into()),
                    Err(e) => console::log_1(&format!("Custom word '{}' failed: {}", name, e).into()),
                }
                console::log_1(&format!("=== EXECUTE_WORD END: {} ===", name).into());
                result.map_err(|e| e.with_context(&self.call_stack))
            }
        } else {
            let error = error::AjisaiError::UnknownWord(name.to_string());
            console::log_1(&format!("Unknown word: {}", name).into());
            console::log_1(&format!("=== EXECUTE_WORD END: {} ===", name).into());
            Err(error)
        }
    }

    pub(crate) fn execute_word_leap(&mut self, name: &str, current_word: Option<&str>) -> Result<()> {
        if let Some(current) = current_word {
            if name != current {
                return Err(error::AjisaiError::from(format!(
                    "LEAP can only jump within the same word. Cannot jump from '{}' to '{}'", 
                    current, name
                )));
            }
        } else {
            return Err(error::AjisaiError::from(format!(
                "LEAP can only be used within custom words. Cannot jump to '{}' from main program", 
                name
            )));
        }

        if let Some(def) = self.dictionary.get(name).cloned() {
            if def.is_builtin {
                return Err(error::AjisaiError::from("Cannot LEAP to builtin word"));
            } else {
                self.execute_custom_word(&def.tokens)
            }
        } else {
            Err(error::AjisaiError::UnknownWord(name.to_string()))
        }
    }

    fn execute_custom_word(&mut self, tokens: &[Token]) -> Result<()> {
    use web_sys::console;
    console::log_1(&format!("=== CUSTOM_WORD EXECUTION START ===").into());
    console::log_1(&format!("Tokens to execute: {:?}", tokens).into());
    console::log_1(&format!("Workspace before: {} items", self.workspace.len()).into());
    
    // カスタムワードは execute_tokens を再利用するのではなく、独自実装
    let mut i = 0;
    while i < tokens.len() {
        console::log_1(&format!("--- Custom word token[{}]: {:?} ---", i, tokens[i]).into());
        console::log_1(&format!("Workspace before token[{}]: {} items", i, self.workspace.len()).into());
        
        match &tokens[i] {
            Token::Number(num, den) => {
                let num_val = *num; // 値をコピー
                let den_val = *den; // 値をコピー
                self.workspace.push(Value {
                    val_type: ValueType::Number(crate::types::Fraction::new(num_val, den_val)),
                });
                console::log_1(&format!("Added number: {}/{}", num_val, den_val).into());
            },
            Token::String(s) => {
                let string_val = s.clone(); // クローン
                self.workspace.push(Value {
                    val_type: ValueType::String(string_val.clone()),
                });
                console::log_1(&format!("Added string: {}", string_val).into());
            },
            Token::Boolean(b) => {
                let bool_val = *b; // 値をコピー
                self.workspace.push(Value {
                    val_type: ValueType::Boolean(bool_val),
                });
                console::log_1(&format!("Added boolean: {}", bool_val).into());
            },
            Token::Nil => {
                self.workspace.push(Value {
                    val_type: ValueType::Nil,
                });
                console::log_1(&format!("Added nil").into());
            },
            Token::Symbol(name) => {
                let symbol_name = name.clone(); // クローン
                console::log_1(&format!("Executing symbol in custom word: {}", symbol_name).into());
                match self.execute_word(&symbol_name) {
                    Ok(()) => {
                        console::log_1(&format!("Symbol '{}' executed successfully", symbol_name).into());
                    },
                    Err(e) => {
                        console::log_1(&format!("Error executing symbol '{}': {}", symbol_name, e).into());
                        return Err(e);
                    }
                }
            },
            Token::VectorStart => {
                console::log_1(&format!("Vector processing not implemented in custom words").into());
                return Err(error::AjisaiError::from("Vector literals not supported in custom words"));
            },
            Token::VectorEnd => {
                console::log_1(&format!("Unexpected vector end").into());
                return Err(error::AjisaiError::from("Unexpected vector end"));
            },
        }
        
        console::log_1(&format!("Workspace after token[{}]: {} items", i, self.workspace.len()).into());
        if let Some(top) = self.workspace.last() {
            console::log_1(&format!("Current top value: {:?}", top).into());
        }
        
        i += 1;
    }
    
    console::log_1(&format!("=== CUSTOM_WORD EXECUTION END ===").into());
    console::log_1(&format!("Final workspace: {} items", self.workspace.len()).into());
    
    Ok(())
}

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        match name {
            // 算術演算（記号）
            "+" => arithmetic::op_add(self),
            "-" => arithmetic::op_sub(self),
            "*" => arithmetic::op_mul(self),
            "/" => arithmetic::op_div(self),
            ">" => arithmetic::op_gt(self),
            ">=" => arithmetic::op_ge(self),
            "=" => arithmetic::op_eq(self),
            
            // 論理・存在（漢字）
            "否" => arithmetic::op_not(self),
            "且" => arithmetic::op_and(self),
            "或" => arithmetic::op_or(self),
            "無" => arithmetic::op_nil_check(self),
            "有" => arithmetic::op_some_check(self),
            
            // Vector操作（既存・漢字）
            "頭" => vector_ops::op_head(self),
            "尾" => vector_ops::op_tail(self),
            "接" => vector_ops::op_cons(self),
            "離" => vector_ops::op_uncons(self),
            "追" => vector_ops::op_append(self),
            "除" => vector_ops::op_remove_last(self),
            "複" => vector_ops::op_clone(self),
            "選" => vector_ops::op_select(self),
            "数" => vector_ops::op_count(self),
            "在" => vector_ops::op_at(self),
            "行" => vector_ops::op_do(self),
            
            // Vector操作（新機能・漢字）
            "結" => vector_ops::op_join(self),
            "切" => vector_ops::op_split(self),
            "反" => vector_ops::op_reverse(self),
            "挿" => vector_ops::op_insert(self),
            "消" => vector_ops::op_delete(self),
            "探" => vector_ops::op_find(self),
            "含" => vector_ops::op_contains(self),
            "換" => vector_ops::op_replace(self),
            "抽" => vector_ops::op_filter(self),
            "変" => vector_ops::op_map(self),
            "畳" => vector_ops::op_fold(self),
            "並" => vector_ops::op_sort(self),
            "空" => vector_ops::op_empty(self),
            
            // 制御・システム（漢字）
            "定" => {
                Err(error::AjisaiError::from("定 should be handled separately"))
            },
            "削" => control::op_del(self),
            "成" => leap::op_leap(self),
            "忘" => op_amnesia(self),
            
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
                    .map(|token| self.token_to_string(token))
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
