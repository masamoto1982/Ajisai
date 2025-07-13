use std::collections::{HashMap, HashSet};
use crate::types::*;
use crate::tokenizer::*;
use crate::builtins;

pub struct Interpreter {
    stack: Stack,
    register: Register,
    dictionary: HashMap<String, WordDefinition>,
    dependencies: HashMap<String, HashSet<String>>, // word -> それを使用しているワードのセット
    // ステップ実行用の状態
    step_tokens: Vec<Token>,
    step_position: usize,
    step_mode: bool,
    // 出力バッファ
    output_buffer: String,
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
            step_tokens: Vec::new(),
            step_position: 0,
            step_mode: false,
            output_buffer: String::new(),
        };
        
        builtins::register_builtins(&mut interpreter.dictionary);
        
        interpreter
    }
    
    pub fn execute(&mut self, code: &str) -> Result<(), String> {
        let tokens = tokenize(code)?;
        self.execute_tokens_with_context(&tokens)?;
        Ok(())
    }

    // 出力バッファを取得してクリア
    pub fn get_output(&mut self) -> String {
        let output = self.output_buffer.clone();
        self.output_buffer.clear();
        output
    }
    
    // 出力バッファに追加
    fn append_output(&mut self, text: &str) {
        self.output_buffer.push_str(text);
    }

    // ステップ実行の初期化
    pub fn init_step_execution(&mut self, code: &str) -> Result<(), String> {
        self.step_tokens = tokenize(code)?;
        self.step_position = 0;
        self.step_mode = true;
        Ok(())
    }

    // 1ステップ実行
    pub fn execute_step(&mut self) -> Result<bool, String> {
        if !self.step_mode || self.step_position >= self.step_tokens.len() {
            self.step_mode = false;
            return Ok(false); // 実行完了
        }

        let token = self.step_tokens[self.step_position].clone();
        self.step_position += 1;

        // トークンを1つ実行
        match self.execute_single_token(&token) {
            Ok(_) => Ok(self.step_position < self.step_tokens.len()),
            Err(e) => {
                self.step_mode = false;
                Err(e)
            }
        }
    }

    // ステップ実行の状態を取得
    pub fn get_step_info(&self) -> Option<(usize, usize)> {
        if self.step_mode {
            Some((self.step_position, self.step_tokens.len()))
        } else {
            None
        }
    }

    // 単一トークンの実行
fn execute_single_token(&mut self, token: &Token) -> Result<(), String> {
    match token {
        Token::Description(_text) => {
            // 説明文はここでは処理しない（execute_tokens_with_contextで処理）
            Ok(())
        },
        Token::Number(num, den) => {
            self.stack.push(Value {
                val_type: ValueType::Number(Fraction::new(*num, *den)),
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
            // ベクタを収集（ステップ実行時は一度に処理）
            let mut depth = 1;
            let mut vector_tokens = vec![Token::VectorStart];
            
            while depth > 0 && self.step_position < self.step_tokens.len() {
                let next_token = self.step_tokens[self.step_position].clone();
                self.step_position += 1;
                
                match &next_token {
                    Token::VectorStart => depth += 1,
                    Token::VectorEnd => depth -= 1,
                    _ => {}
                }
                
                vector_tokens.push(next_token);
            }
            
            // ベクタをデータとして解析
            let (vector_values, _) = self.collect_vector_as_data(&vector_tokens)?;
            self.stack.push(Value {
                val_type: ValueType::Vector(vector_values),
            });
            Ok(())
        },
        Token::Symbol(name) => {
            if matches!(name.as_str(), "+" | "-" | "*" | "/" | ">" | ">=" | "=" | "<" | "<=") {
                self.execute_operator(name)?;
            } else if let Some(def) = self.dictionary.get(name).cloned() {
                if def.is_builtin {
                    self.execute_builtin(name)?;
                } else {
                    // カスタムワードは展開して実行
                    self.execute_tokens_with_context(&def.tokens)?;
                }
            } else {
                return Err(format!("Unknown word: {}", name));
            }
            Ok(())
        },
        Token::VectorEnd => Err("Unexpected ']' found.".to_string()),
    }
}

    /// トークンをデータとして解析し、Valueのベクタに変換する（ネスト対応）
    fn collect_vector_as_data(&self, tokens: &[Token]) -> Result<(Vec<Value>, usize), String> {
        let mut values = Vec::new();
        let mut i = 1; // 開始の'['をスキップ

        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorEnd => {
                    // ベクタの終わり
                    return Ok((values, i + 1)); // 消費したトークン数を返す
                },
                Token::VectorStart => {
                    // ネストしたベクタの開始
                    let (nested_values, consumed) = self.collect_vector_as_data(&tokens[i..])?;
                    values.push(Value { val_type: ValueType::Vector(nested_values) });
                    i += consumed; // ネストしたベクタのトークンをスキップ
                    continue;
                },
                // トークンを直接Valueに変換
                Token::Number(num, den) => values.push(Value { val_type: ValueType::Number(Fraction::new(*num, *den)) }),
                Token::String(s) => values.push(Value { val_type: ValueType::String(s.clone()) }),
                Token::Boolean(b) => values.push(Value { val_type: ValueType::Boolean(*b) }),
                Token::Nil => values.push(Value { val_type: ValueType::Nil }),
                Token::Symbol(s) => values.push(Value { val_type: ValueType::Symbol(s.clone()) }),
                Token::Description(_) => { /* 説明はVectorデータ内では無視 */ },
            }
            i += 1;
        }

        Err("Unclosed vector".to_string())
    }
    
    fn execute_tokens_with_context(&mut self, tokens: &[Token]) -> Result<(), String> {
        let mut i = 0;
        let mut pending_description: Option<String> = None;

        while i < tokens.len() {
            match &tokens[i] {
                Token::Description(text) => {
                    pending_description = Some(text.clone());
                },
                Token::Number(num, den) => {
                    self.stack.push(Value {
                        val_type: ValueType::Number(Fraction::new(*num, *den)),
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
                    // ベクタを「データ」として解析し、スタックに積む
                    let (vector_values, consumed) = self.collect_vector_as_data(&tokens[i..])?;
                    self.stack.push(Value {
                        val_type: ValueType::Vector(vector_values),
                    });
                    i += consumed - 1; // インデックスを調整
                },
                Token::Symbol(name) => {
                    // シンボルの実行ロジック
                    if matches!(name.as_str(), "+" | "-" | "*" | "/" | ">" | ">=" | "=" | "<" | "<=") {
                        self.execute_operator(name)?;
                    } else if let Some(def) = self.dictionary.get(name).cloned() {
                        if def.is_builtin {
                            if name == "DEF" {
                                let desc = pending_description.take();
                                self.op_def_with_comment(desc)?;
                            } else {
                                self.execute_builtin(name)?;
                            }
                        } else {
                            // カスタムワードの実行時に暗黙の反復をチェック
                            self.execute_custom_word(name, &def.tokens)?;
                        }
                    } else {
                        return Err(format!("Unknown word: {}", name));
                    }
                },
                Token::VectorEnd => return Err("Unexpected ']' found.".to_string()),
            }
            
            i += 1;
        }
        
        Ok(())
    }

    // カスタムワードの実行（暗黙の反復対応）
fn execute_custom_word(&mut self, name: &str, tokens: &[Token]) -> Result<(), String> {
    web_sys::console::log_1(&format!("execute_custom_word: {} with stack top: {:?}", 
                                     name, self.stack.last()).into());
    
    // スタックトップがVectorかチェック（クローンして借用を解放）
    if let Some(top) = self.stack.last().cloned() {
        if let ValueType::Vector(v) = top.val_type {
            web_sys::console::log_1(&format!("Applying {} to vector of {} elements", 
                                             name, v.len()).into());
            
            // Vectorの場合、各要素に対してワードを適用
            self.stack.pop(); // Vectorを取り出す
            
            let mut results = Vec::new();
            for (idx, elem) in v.iter().enumerate() {
                web_sys::console::log_1(&format!("Processing element {}: {:?}", idx, elem).into());
                
                // 各要素をスタックに積む
                self.stack.push(elem.clone());
                
                // ワードを実行
                self.execute_tokens_with_context(tokens)?;
                
                // 結果を取得（複数の結果がある可能性を考慮）
                if let Some(result) = self.stack.pop() {
                    web_sys::console::log_1(&format!("Result for element {}: {:?}", idx, result).into());
                    results.push(result);
                }
            }
            
            // 結果をVectorとしてスタックに戻す
            self.stack.push(Value { val_type: ValueType::Vector(results) });
            return Ok(());
        }
    }
    
    // Vectorでない場合は通常の実行
    web_sys::console::log_1(&format!("Normal execution of {}", name).into());
    self.execute_tokens_with_context(tokens)
}

    fn body_vector_to_tokens(
        &self,
        body: &[Value],
    ) -> Result<(Vec<Token>, HashSet<String>), String> {
        let mut tokens = Vec::new();
        let mut dependencies = HashSet::new();

        for val in body {
            self.value_to_tokens_recursive(val, &mut tokens, &mut dependencies)?;
        }

        Ok((tokens, dependencies))
    }

    fn value_to_tokens_recursive(
        &self,
        val: &Value,
        tokens: &mut Vec<Token>,
        dependencies: &mut HashSet<String>,
    ) -> Result<(), String> {
        match &val.val_type {
            ValueType::Number(n) => tokens.push(Token::Number(n.numerator, n.denominator)),
            ValueType::String(s) => tokens.push(Token::String(s.clone())),
            ValueType::Boolean(b) => tokens.push(Token::Boolean(*b)),
            ValueType::Nil => tokens.push(Token::Nil),
            ValueType::Symbol(s) => {
                tokens.push(Token::Symbol(s.clone()));
                if let Some(def) = self.dictionary.get(s) {
                    if !def.is_builtin {
                        dependencies.insert(s.clone());
                    }
                }
            }
            ValueType::Vector(v) => {
                tokens.push(Token::VectorStart);
                for item in v {
                    self.value_to_tokens_recursive(item, tokens, dependencies)?;
                }
                tokens.push(Token::VectorEnd);
            }
        }
        Ok(())
    }
        
    fn execute_builtin(&mut self, name: &str) -> Result<(), String> {
        web_sys::console::log_1(&format!("execute_builtin: {}", name).into());
        
        match name {
            "DUP" => self.op_dup(),
            "DROP" => self.op_drop(),
            "SWAP" => self.op_swap(),
            "OVER" => self.op_over(),
            "ROT" => self.op_rot(),
            "NIP" => self.op_nip(),
            ">R" => self.op_to_r(),
            "R>" => self.op_from_r(),
            "R@" => self.op_r_fetch(),
            "DEF" => self.op_def_with_comment(None),
            "IF" => self.op_if(),
            "LENGTH" => self.op_length(),
            "HEAD" => self.op_head(),
            "TAIL" => self.op_tail(),
            "CONS" => self.op_cons(),
            "APPEND" => self.op_append(),
            "REVERSE" => self.op_reverse(),
            "NTH" => self.op_nth(),
            "UNCONS" => self.op_uncons(),
            "EMPTY?" => self.op_empty(),
            "DEL" => self.op_del(),
            "NOT" => self.op_not(),
            // 出力ワード
            "." => self.op_dot(),
            "PRINT" => self.op_print(),
            "CR" => self.op_cr(),
            "SPACE" => self.op_space(),
            "SPACES" => self.op_spaces(),
            "EMIT" => self.op_emit(),
            _ => Err(format!("Unknown builtin: {}", name)),
        }
    }
    
    fn execute_operator(&mut self, op: &str) -> Result<(), String> {
        web_sys::console::log_1(&format!("execute_operator: {}", op).into());
        
        match op {
            "+" => self.op_add(),
            "-" => self.op_sub(),
            "*" => self.op_mul(),
            "/" => self.op_div(),
            ">" => self.op_gt(),
            ">=" => self.op_ge(),
            "=" => self.op_eq(),
            "<" => self.op_lt(),
            "<=" => self.op_le(),
            _ => Err(format!("Unknown operator: {}", op)),
        }
    }
    
    fn op_def_with_comment(&mut self, description: Option<String>) -> Result<(), String> {
        if self.stack.len() < 2 {
            return Err("Stack underflow for DEF".to_string());
        }
    
        let name_val = self.stack.pop().unwrap();
        let body_val = self.stack.pop().unwrap();
    
        web_sys::console::log_1(&format!("DEF: defining {} with body {:?}", 
                                         name_val, body_val).into());
    
        match (&name_val.val_type, &body_val.val_type) {
            (ValueType::String(name), ValueType::Vector(body)) => {
                let name = name.to_uppercase();
    
                if let Some(existing) = self.dictionary.get(&name) {
                    if existing.is_builtin {
                        return Err(format!("Cannot redefine builtin word: {}", name));
                    }
                }
    
                if self.dictionary.contains_key(&name) {
                    if let Some(dependents) = self.dependencies.get(&name) {
                        if !dependents.is_empty() {
                            let dependent_list: Vec<String> = dependents.iter().cloned().collect();
                            return Err(format!(
                                "Cannot redefine '{}' because it is used by: {}",
                                name,
                                dependent_list.join(", ")
                            ));
                        }
                    }
    
                    if let Some(old_def) = self.dictionary.get(&name) {
                        let mut old_deps = HashSet::new();
                        for token in &old_def.tokens {
                           if let Token::Symbol(s) = token {
                               old_deps.insert(s.clone());
                           }
                        }

                        for dep in old_deps {
                            if let Some(deps) = self.dependencies.get_mut(&dep) {
                                deps.remove(&name);
                            }
                        }
                    }
                }
    
                let (new_tokens, new_dependencies) = self.body_vector_to_tokens(body)?;
    
                for dep_name in &new_dependencies {
                    self.dependencies
                        .entry(dep_name.clone())
                        .or_insert_with(HashSet::new)
                        .insert(name.clone());
                }
    
                self.dictionary.insert(name.clone(), WordDefinition {
                    tokens: new_tokens,
                    is_builtin: false,
                    description,
                });
    
                Ok(())
            }
            _ => Err("Type error: DEF requires a vector and a string".to_string()),
        }
    }

    pub fn delete_word(&mut self, name: &str) -> Result<(), String> {
        if let Some(def) = self.dictionary.get(name) {
            if def.is_builtin {
                return Err(format!("Cannot delete builtin word: {}", name));
            }
        } else {
            return Err(format!("Word not found: {}", name));
        }
        
        if let Some(dependents) = self.dependencies.get(name) {
            if !dependents.is_empty() {
                let dependent_list: Vec<String> = dependents.iter().cloned().collect();
                return Err(format!(
                    "Cannot delete '{}' because it is used by: {}", 
                    name, 
                    dependent_list.join(", ")
                ));
            }
        }
        
        self.dictionary.remove(name);
        
        for (_, deps) in self.dependencies.iter_mut() {
            deps.remove(name);
        }
        
        self.dependencies.remove(name);
        
        Ok(())
    }
    
    fn op_dup(&mut self) -> Result<(), String> {
        if let Some(top) = self.stack.last() {
            self.stack.push(top.clone());
            Ok(())
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_drop(&mut self) -> Result<(), String> {
        if self.stack.pop().is_none() {
            Err("Stack underflow".to_string())
        } else {
            Ok(())
        }
    }
    
    fn op_swap(&mut self) -> Result<(), String> {
        let len = self.stack.len();
        if len < 2 {
            Err("Stack underflow".to_string())
        } else {
            self.stack.swap(len - 1, len - 2);
            Ok(())
        }
    }
    
    fn op_over(&mut self) -> Result<(), String> {
        let len = self.stack.len();
        if len < 2 {
            Err("Stack underflow".to_string())
        } else {
            let item = self.stack[len - 2].clone();
            self.stack.push(item);
            Ok(())
        }
    }
    
    fn op_rot(&mut self) -> Result<(), String> {
        let len = self.stack.len();
        if len < 3 {
            Err("Stack underflow".to_string())
        } else {
            let third = self.stack.remove(len - 3);
            self.stack.push(third);
            Ok(())
        }
    }
    
    fn op_nip(&mut self) -> Result<(), String> {
        let len = self.stack.len();
        if len < 2 {
            Err("Stack underflow".to_string())
        } else {
            self.stack.remove(len - 2);
            Ok(())
        }
    }
    
    fn op_to_r(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            self.register = Some(val);
            Ok(())
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_from_r(&mut self) -> Result<(), String> {
        if let Some(val) = self.register.take() {
            self.stack.push(val);
            Ok(())
        } else {
            Err("Register is empty".to_string())
        }
    }
    
    fn op_r_fetch(&mut self) -> Result<(), String> {
        if let Some(val) = &self.register {
            self.stack.push(val.clone());
            Ok(())
        } else {
            Err("Register is empty".to_string())
        }
    }
    
    // 暗黙の反復を実装した新しい演算子
    fn op_add(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        web_sys::console::log_1(&format!("op_add: a={:?}, b={:?}", a, b).into());
        
        match (&a.val_type, &b.val_type) {
            // スカラー + スカラー（従来通り）
            (ValueType::Number(n1), ValueType::Number(n2)) => {
                self.stack.push(Value { val_type: ValueType::Number(n1.add(n2)) });
                Ok(())
            },
            // Vector + スカラー（ブロードキャスト）
            (ValueType::Vector(v), ValueType::Number(n)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Number(en.add(n))
                        },
                        _ => elem.clone()
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            // スカラー + Vector（ブロードキャスト）
            (ValueType::Number(n), ValueType::Vector(v)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Number(n.add(en))
                        },
                        _ => elem.clone()
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            // Vector + Vector（要素ごと）
            (ValueType::Vector(v1), ValueType::Vector(v2)) => {
                if v1.len() != v2.len() {
                    return Err("Vector length mismatch".to_string());
                }
                let result: Vec<Value> = v1.iter().zip(v2.iter())
                    .map(|(a, b)| match (&a.val_type, &b.val_type) {
                        (ValueType::Number(n1), ValueType::Number(n2)) => Value {
                            val_type: ValueType::Number(n1.add(n2))
                        },
                        _ => a.clone()
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            // その他の型の組み合わせは元のまま返す（エラーにはしない）
            _ => {
                self.stack.push(a);
                Ok(())
            }
        }
    }
    
    fn op_sub(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Number(n1), ValueType::Number(n2)) => {
                self.stack.push(Value { val_type: ValueType::Number(n1.sub(n2)) });
                Ok(())
            },
            (ValueType::Vector(v), ValueType::Number(n)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Number(en.sub(n))
                        },
                        _ => elem.clone()
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            (ValueType::Number(n), ValueType::Vector(v)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Number(n.sub(en))
                        },
                        _ => elem.clone()
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            (ValueType::Vector(v1), ValueType::Vector(v2)) => {
                if v1.len() != v2.len() {
                    return Err("Vector length mismatch".to_string());
                }
                let result: Vec<Value> = v1.iter().zip(v2.iter())
                    .map(|(a, b)| match (&a.val_type, &b.val_type) {
                        (ValueType::Number(n1), ValueType::Number(n2)) => Value {
                            val_type: ValueType::Number(n1.sub(n2))
                        },
                        _ => a.clone()
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            _ => {
                self.stack.push(a);
                Ok(())
            }
        }
    }
    
    fn op_mul(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        web_sys::console::log_1(&format!("op_mul: a={:?}, b={:?}", a, b).into());
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Number(n1), ValueType::Number(n2)) => {
                self.stack.push(Value { val_type: ValueType::Number(n1.mul(n2)) });
                Ok(())
            },
            (ValueType::Vector(v), ValueType::Number(n)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Number(en.mul(n))
                        },
                        _ => elem.clone()
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            (ValueType::Number(n), ValueType::Vector(v)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Number(n.mul(en))
                        },
                        _ => elem.clone()
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            (ValueType::Vector(v1), ValueType::Vector(v2)) => {
                if v1.len() != v2.len() {
                    return Err("Vector length mismatch".to_string());
                }
                let result: Vec<Value> = v1.iter().zip(v2.iter())
                    .map(|(a, b)| match (&a.val_type, &b.val_type) {
                        (ValueType::Number(n1), ValueType::Number(n2)) => Value {
                            val_type: ValueType::Number(n1.mul(n2))
                        },
                        _ => a.clone()
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            _ => {
                self.stack.push(a);
                Ok(())
            }
        }
    }
    
    fn op_div(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Number(n1), ValueType::Number(n2)) => {
                self.stack.push(Value { val_type: ValueType::Number(n1.div(n2)) });
                Ok(())
            },
            (ValueType::Vector(v), ValueType::Number(n)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Number(en.div(n))
                        },
                        _ => elem.clone()
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            (ValueType::Number(n), ValueType::Vector(v)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Number(n.div(en))
                        },
                        _ => elem.clone()
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            (ValueType::Vector(v1), ValueType::Vector(v2)) => {
                if v1.len() != v2.len() {
                    return Err("Vector length mismatch".to_string());
                }
                let result: Vec<Value> = v1.iter().zip(v2.iter())
                    .map(|(a, b)| match (&a.val_type, &b.val_type) {
                        (ValueType::Number(n1), ValueType::Number(n2)) => Value {
                            val_type: ValueType::Number(n1.div(n2))
                        },
                        _ => a.clone()
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            _ => {
                self.stack.push(a);
                Ok(())
            }
        }
    }
    
    // 比較演算子も暗黙の反復に対応
    fn op_gt(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Number(n1), ValueType::Number(n2)) => {
                self.stack.push(Value { val_type: ValueType::Boolean(n1.gt(n2)) });
                Ok(())
            },
            (ValueType::Vector(v), ValueType::Number(n)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Boolean(en.gt(n))
                        },
                        _ => Value { val_type: ValueType::Boolean(false) }
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            (ValueType::Number(n), ValueType::Vector(v)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Boolean(n.gt(en))
                        },
                        _ => Value { val_type: ValueType::Boolean(false) }
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            (ValueType::Vector(v1), ValueType::Vector(v2)) => {
                if v1.len() != v2.len() {
                    return Err("Vector length mismatch".to_string());
                }
                let result: Vec<Value> = v1.iter().zip(v2.iter())
                    .map(|(a, b)| match (&a.val_type, &b.val_type) {
                        (ValueType::Number(n1), ValueType::Number(n2)) => Value {
                            val_type: ValueType::Boolean(n1.gt(n2))
                        },
                        _ => Value { val_type: ValueType::Boolean(false) }
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            _ => {
                self.stack.push(a);
                Ok(())
            }
        }
    }
    
    fn op_ge(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        web_sys::console::log_1(&format!("op_ge: a={:?}, b={:?}", a, b).into());
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Number(n1), ValueType::Number(n2)) => {
                let result = n1.ge(n2);
                web_sys::console::log_1(&format!("op_ge result: {}", result).into());
                self.stack.push(Value { val_type: ValueType::Boolean(result) });
                Ok(())
            },
            (ValueType::Vector(v), ValueType::Number(n)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Boolean(en.ge(n))
                        },
                        _ => Value { val_type: ValueType::Boolean(false) }
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            (ValueType::Number(n), ValueType::Vector(v)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Boolean(n.ge(en))
                        },
                        _ => Value { val_type: ValueType::Boolean(false) }
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            (ValueType::Vector(v1), ValueType::Vector(v2)) => {
                if v1.len() != v2.len() {
                    return Err("Vector length mismatch".to_string());
                }
                let result: Vec<Value> = v1.iter().zip(v2.iter())
                    .map(|(a, b)| match (&a.val_type, &b.val_type) {
                        (ValueType::Number(n1), ValueType::Number(n2)) => Value {
                            val_type: ValueType::Boolean(n1.ge(n2))
                        },
                        _ => Value { val_type: ValueType::Boolean(false) }
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            _ => {
                self.stack.push(a);
                Ok(())
            }
        }
    }
    
    fn op_eq(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        // =演算子はVectorの比較もサポートするが、暗黙の反復も行う
        match (&a.val_type, &b.val_type) {
            // スカラー同士（従来通り）
            (ValueType::Number(n1), ValueType::Number(n2)) => {
                self.stack.push(Value { val_type: ValueType::Boolean(n1.eq(n2)) });
                Ok(())
            },
            (ValueType::String(s1), ValueType::String(s2)) => {
                self.stack.push(Value { val_type: ValueType::Boolean(s1 == s2) });
                Ok(())
            },
            (ValueType::Boolean(b1), ValueType::Boolean(b2)) => {
                self.stack.push(Value { val_type: ValueType::Boolean(b1 == b2) });
                Ok(())
            },
            (ValueType::Symbol(s1), ValueType::Symbol(s2)) => {
                self.stack.push(Value { val_type: ValueType::Boolean(s1 == s2) });
                Ok(())
            },
            (ValueType::Nil, ValueType::Nil) => {
                self.stack.push(Value { val_type: ValueType::Boolean(true) });
                Ok(())
            },
            // Vector全体の比較
            (ValueType::Vector(v1), ValueType::Vector(v2)) => {
                if v1.len() == v2.len() && v1 == v2 {
                    self.stack.push(Value { val_type: ValueType::Boolean(true) });
                } else {
                    self.stack.push(Value { val_type: ValueType::Boolean(false) });
                }
                Ok(())
            },
            // 異なる型の場合はfalse
            _ => {
                self.stack.push(Value { val_type: ValueType::Boolean(false) });
                Ok(())
            },
        }
    }
    
    fn op_lt(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Number(n1), ValueType::Number(n2)) => {
                self.stack.push(Value { val_type: ValueType::Boolean(n1.lt(n2)) });
                Ok(())
            },
            (ValueType::Vector(v), ValueType::Number(n)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Boolean(en.lt(n))
                        },
                        _ => Value { val_type: ValueType::Boolean(false) }
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            (ValueType::Number(n), ValueType::Vector(v)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Boolean(n.lt(en))
                        },
                        _ => Value { val_type: ValueType::Boolean(false) }
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            (ValueType::Vector(v1), ValueType::Vector(v2)) => {
                if v1.len() != v2.len() {
                    return Err("Vector length mismatch".to_string());
                }
                let result: Vec<Value> = v1.iter().zip(v2.iter())
                    .map(|(a, b)| match (&a.val_type, &b.val_type) {
                        (ValueType::Number(n1), ValueType::Number(n2)) => Value {
                            val_type: ValueType::Boolean(n1.lt(n2))
                        },
                        _ => Value { val_type: ValueType::Boolean(false) }
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            _ => {
                self.stack.push(a);
                Ok(())
            }
        }
    }
    
    fn op_le(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Number(n1), ValueType::Number(n2)) => {
                self.stack.push(Value { val_type: ValueType::Boolean(n1.le(n2)) });
                Ok(())
            },
            (ValueType::Vector(v), ValueType::Number(n)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Boolean(en.le(n))
                        },
                        _ => Value { val_type: ValueType::Boolean(false) }
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            (ValueType::Number(n), ValueType::Vector(v)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Boolean(n.le(en))
                        },
                        _ => Value { val_type: ValueType::Boolean(false) }
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            (ValueType::Vector(v1), ValueType::Vector(v2)) => {
                if v1.len() != v2.len() {
                    return Err("Vector length mismatch".to_string());
                }
                let result: Vec<Value> = v1.iter().zip(v2.iter())
                    .map(|(a, b)| match (&a.val_type, &b.val_type) {
                        (ValueType::Number(n1), ValueType::Number(n2)) => Value {
                            val_type: ValueType::Boolean(n1.le(n2))
                        },
                        _ => Value { val_type: ValueType::Boolean(false) }
                    })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(result) });
                Ok(())
            },
            _ => {
                self.stack.push(a);
                Ok(())
            }
        }
    }
    
    fn op_length(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            match val.val_type {
                ValueType::Vector(v) => {
                    self.stack.push(Value { val_type: ValueType::Number(Fraction::new(v.len() as i64, 1)) });
                    Ok(())
                },
                _ => Err("Type error: LENGTH requires a vector".to_string()),
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_head(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            match val.val_type {
                ValueType::Vector(v) => {
                    if let Some(first) = v.first() {
                        self.stack.push(first.clone());
                        Ok(())
                    } else {
                        Err("HEAD of empty vector".to_string())
                    }
                },
                _ => Err("Type error: HEAD requires a vector".to_string()),
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_tail(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            match val.val_type {
                ValueType::Vector(v) => {
                    if v.is_empty() {
                        Err("TAIL of empty vector".to_string())
                    } else {
                        let tail: Vec<Value> = v.into_iter().skip(1).collect();
                        self.stack.push(Value { val_type: ValueType::Vector(tail) });
                        Ok(())
                    }
                },
                _ => Err("Type error: TAIL requires a vector".to_string()),
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_cons(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let vec_val = self.stack.pop().unwrap();
        let elem = self.stack.pop().unwrap();
        match vec_val.val_type {
            ValueType::Vector(mut v) => {
                v.insert(0, elem);
                self.stack.push(Value { val_type: ValueType::Vector(v) });
                Ok(())
            },
            _ => Err("Type error: CONS requires an element and a vector".to_string()),
        }
    }

    fn op_append(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let elem = self.stack.pop().unwrap();
        let vec_val = self.stack.pop().unwrap();
        match vec_val.val_type {
            ValueType::Vector(mut v) => {
                v.push(elem);
                self.stack.push(Value { val_type: ValueType::Vector(v) });
                Ok(())
            },
            _ => Err("Type error: APPEND requires a vector and an element".to_string()),
        }
    }
    
    fn op_reverse(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            match val.val_type {
                ValueType::Vector(mut v) => {
                    v.reverse();
                    self.stack.push(Value { val_type: ValueType::Vector(v) });
                    Ok(())
                },
                _ => Err("Type error: REVERSE requires a vector".to_string()),
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }

    fn op_nth(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let vec_val = self.stack.pop().unwrap();
        let index_val = self.stack.pop().unwrap();
        match (&index_val.val_type, &vec_val.val_type) {
            (ValueType::Number(n), ValueType::Vector(v)) => {
                if n.denominator != 1 { return Err("NTH requires an integer index".to_string()); }
                let mut index = n.numerator;
                let len = v.len() as i64;
                if index < 0 { index = len + index; }
                if index < 0 || index >= len { return Err(format!("Index {} out of bounds for vector of length {}", n.numerator, len)); }
                self.stack.push(v[index as usize].clone());
                Ok(())
            },
            _ => Err("Type error: NTH requires a number and a vector".to_string()),
        }
    }
    
    fn op_uncons(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            match val.val_type {
                ValueType::Vector(v) => {
                    if v.is_empty() { return Err("UNCONS of empty vector".to_string()); }
                    let mut v_mut = v;
                    let head = v_mut.remove(0);
                    self.stack.push(head);
                    self.stack.push(Value { val_type: ValueType::Vector(v_mut) });
                    Ok(())
                },
                _ => Err("Type error: UNCONS requires a vector".to_string()),
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }
        
    fn op_empty(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            match val.val_type {
                ValueType::Vector(v) => {
                    self.stack.push(Value { val_type: ValueType::Boolean(v.is_empty()) });
                    Ok(())
                },
                _ => Err("Type error: EMPTY? requires a vector".to_string()),
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    // IFワードに暗黙の反復を追加
    fn op_if(&mut self) -> Result<(), String> {
        if self.stack.len() < 3 {
            return Err("Stack underflow for IF".to_string());
        }
        
        let else_branch = self.stack.pop().unwrap();
        let then_branch = self.stack.pop().unwrap();
        let condition = self.stack.pop().unwrap();
        
        web_sys::console::log_1(&format!("op_if: condition={:?}, then={:?}, else={:?}", 
                                         condition, then_branch, else_branch).into());
        
        match (&condition.val_type, &then_branch.val_type, &else_branch.val_type) {
            // 通常のIF（スカラーの真偽値）
            (ValueType::Boolean(cond), ValueType::Vector(then_vec), ValueType::Vector(else_vec)) => {
                let vec_to_execute = if *cond { then_vec } else { else_vec };
                let (tokens, _) = self.body_vector_to_tokens(vec_to_execute)?;
                self.execute_tokens_with_context(&tokens)?;
                Ok(())
            },
            // Vectorの真偽値に対する暗黙の反復
            (ValueType::Vector(cond_vec), ValueType::Vector(then_vec), ValueType::Vector(else_vec)) => {
                // 各条件に対してIFを実行
                for cond_val in cond_vec {
                    match &cond_val.val_type {
                        ValueType::Boolean(cond) => {
                            let vec_to_execute = if *cond { then_vec } else { else_vec };
                            let (tokens, _) = self.body_vector_to_tokens(vec_to_execute)?;
                            self.execute_tokens_with_context(&tokens)?;
                        },
                        _ => {
                            // 真偽値でない要素はスキップ
                        }
                    }
                }
                Ok(())
            },
            _ => Err("Type error: IF requires a boolean (or vector of booleans) and two vectors".to_string()),
        }
    }

    fn op_not(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            match val.val_type {
                ValueType::Boolean(b) => {
                    self.stack.push(Value { val_type: ValueType::Boolean(!b) });
                    Ok(())
                },
                // Vectorに対してもNOTを適用（暗黙の反復）
                ValueType::Vector(v) => {
                    let result: Vec<Value> = v.iter()
                        .map(|elem| match &elem.val_type {
                            ValueType::Boolean(b) => Value {
                                val_type: ValueType::Boolean(!b)
                            },
                            _ => elem.clone()
                        })
                        .collect();
                    self.stack.push(Value { val_type: ValueType::Vector(result) });
                    Ok(())
                },
                _ => Err("Type error: NOT requires a boolean or vector of booleans".to_string()),
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_del(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            match val.val_type {
                ValueType::String(name) => {
                    self.delete_word(&name.to_uppercase())
                },
                _ => Err("Type error: DEL requires a string".to_string()),
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    // 出力ワードの実装（修正版）
    fn op_dot(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            match &val.val_type {
                ValueType::Vector(_) => {
                    // Vectorの場合は各要素を出力
                    self.append_output(&val.to_string());
                },
                _ => {
                    // スカラーの場合は通常通り
                    self.append_output(&val.to_string());
                }
            }
            self.append_output(" ");
            Ok(())
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_print(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.last() {
            self.append_output(&val.to_string());
            self.append_output(" ");
            Ok(())
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_cr(&mut self) -> Result<(), String> {
        self.append_output("\n");
        Ok(())
    }
    
    fn op_space(&mut self) -> Result<(), String> {
        self.append_output(" ");
        Ok(())
    }
    
    fn op_spaces(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            match val.val_type {
                ValueType::Number(n) => {
                    if n.denominator == 1 && n.numerator >= 0 {
                        let spaces = " ".repeat(n.numerator as usize);
                        self.append_output(&spaces);
                        Ok(())
                    } else {
                        Err("SPACES requires a non-negative integer".to_string())
                    }
                },
                _ => Err("Type error: SPACES requires a number".to_string()),
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_emit(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            match val.val_type {
                ValueType::Number(n) => {
                    if n.denominator == 1 && n.numerator >= 0 && n.numerator <= 127 {
                        let ch = n.numerator as u8 as char;
                        self.append_output(&ch.to_string());
                        Ok(())
                    } else {
                        Err("EMIT requires an ASCII code (0-127)".to_string())
                    }
                },
                _ => Err("Type error: EMIT requires a number".to_string()),
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    pub fn get_stack(&self) -> &Stack { &self.stack }
    
    pub fn get_register(&self) -> &Register { &self.register }
    
    pub fn get_custom_words(&self) -> Vec<String> {
        let mut words: Vec<String> = self.dictionary
            .iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect();
        words.sort();
        words
    }
    
   pub fn get_custom_words_with_descriptions(&self) -> Vec<(String, Option<String>)> {
       let mut words: Vec<(String, Option<String>)> = self.dictionary
           .iter()
           .filter(|(_, def)| !def.is_builtin)
           .map(|(name, def)| (name.clone(), def.description.clone()))
           .collect();
       words.sort_by(|a, b| a.0.cmp(&b.0));
       words
   }
   
   pub fn get_custom_words_info(&self) -> Vec<(String, Option<String>, bool)> {
       let mut words: Vec<(String, Option<String>, bool)> = self.dictionary
           .iter()
           .filter(|(_, def)| !def.is_builtin)
           .map(|(name, def)| {
               let is_protected = self.dependencies.get(name)
                   .map_or(false, |deps| !deps.is_empty());
               (name.clone(), def.description.clone(), is_protected)
           })
           .collect();
       words.sort_by(|a, b| a.0.cmp(&b.0));
       words
   }
}
