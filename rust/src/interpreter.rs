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
    // データベース関連
    current_table: Option<String>,
    tables: HashMap<String, TableData>,
}

#[derive(Clone)]
pub struct WordDefinition {
    pub tokens: Vec<Token>,
    pub is_builtin: bool,
    pub description: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TableData {
    pub schema: Vec<String>,
    pub records: Vec<Vec<Value>>,
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
            current_table: None,
            tables: HashMap::new(),
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
            "AND" => self.op_and(),
            "OR" => self.op_or(),
            // Nil関連
            "NIL?" => self.op_nil_check(),
            "NOT-NIL?" => self.op_not_nil_check(),
            "KNOWN?" => self.op_not_nil_check(), // エイリアス
            "DEFAULT" => self.op_default(),
            // データベース関連
            "TABLE" => self.op_table(),
            "TABLE-CREATE" => self.op_table_create(),
            "FILTER" => self.op_filter(),
            "PROJECT" => self.op_project(),
            "INSERT" => self.op_insert(),
            "UPDATE" => self.op_update(),
            "DELETE" => self.op_delete(),
            "TABLES" => self.op_tables(),
            "SAVE-DB" => self.op_save_db(),
            "LOAD-DB" => self.op_load_db(),
            // ワイルドカード
            "MATCH?" => self.op_match(),
            "WILDCARD" => self.op_wildcard(),
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
    
    // 比較演算子も暗黙の反復に対応（Nil対応も追加）
    fn op_gt(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Number(n1), ValueType::Number(n2)) => {
                self.stack.push(Value { val_type: ValueType::Boolean(n1.gt(n2)) });
                Ok(())
            },
            // Nilとの比較
            (ValueType::Number(_), ValueType::Nil) |
            (ValueType::Nil, ValueType::Number(_)) |
            (ValueType::Nil, ValueType::Nil) => {
                self.stack.push(Value { val_type: ValueType::Nil });
                Ok(())
            },
            (ValueType::Vector(v), ValueType::Number(n)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Boolean(en.gt(n))
                        },
                        ValueType::Nil => Value { val_type: ValueType::Nil },
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
                        ValueType::Nil => Value { val_type: ValueType::Nil },
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
                        (ValueType::Nil, _) | (_, ValueType::Nil) => Value { val_type: ValueType::Nil },
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
            // Nilとの比較
            (ValueType::Number(_), ValueType::Nil) |
            (ValueType::Nil, ValueType::Number(_)) |
            (ValueType::Nil, ValueType::Nil) => {
                self.stack.push(Value { val_type: ValueType::Nil });
                Ok(())
            },
            (ValueType::Vector(v), ValueType::Number(n)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Boolean(en.ge(n))
                        },
                        ValueType::Nil => Value { val_type: ValueType::Nil },
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
                        ValueType::Nil => Value { val_type: ValueType::Nil },
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
                        (ValueType::Nil, _) | (_, ValueType::Nil) => Value { val_type: ValueType::Nil },
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
            // Nilとの比較
            (ValueType::Number(_), ValueType::Nil) |
            (ValueType::Nil, ValueType::Number(_)) |
            (ValueType::Nil, ValueType::Nil) => {
                self.stack.push(Value { val_type: ValueType::Nil });
                Ok(())
            },
            (ValueType::Vector(v), ValueType::Number(n)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Boolean(en.lt(n))
                        },
                        ValueType::Nil => Value { val_type: ValueType::Nil },
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
                        ValueType::Nil => Value { val_type: ValueType::Nil },
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
                        (ValueType::Nil, _) | (_, ValueType::Nil) => Value { val_type: ValueType::Nil },
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
            // Nilとの比較
            (ValueType::Number(_), ValueType::Nil) |
            (ValueType::Nil, ValueType::Number(_)) |
            (ValueType::Nil, ValueType::Nil) => {
                self.stack.push(Value { val_type: ValueType::Nil });
                Ok(())
            },
            (ValueType::Vector(v), ValueType::Number(n)) => {
                let result: Vec<Value> = v.iter()
                    .map(|elem| match &elem.val_type {
                        ValueType::Number(en) => Value {
                            val_type: ValueType::Boolean(en.le(n))
                        },
                        ValueType::Nil => Value { val_type: ValueType::Nil },
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
                        ValueType::Nil => Value { val_type: ValueType::Nil },
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
                        (ValueType::Nil, _) | (_, ValueType::Nil) => Value { val_type: ValueType::Nil },
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

    // 三値論理対応のAND演算
    fn op_and(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Boolean(a_val), ValueType::Boolean(b_val)) => {
                self.stack.push(Value { 
                    val_type: ValueType::Boolean(*a_val && *b_val) 
                });
            },
            // falseが確定的
            (ValueType::Boolean(false), ValueType::Nil) |
            (ValueType::Nil, ValueType::Boolean(false)) => {
                self.stack.push(Value { 
                    val_type: ValueType::Boolean(false) 
                });
            },
            // 結果は不明
            (ValueType::Boolean(_), ValueType::Nil) |
            (ValueType::Nil, ValueType::Boolean(_)) |
            (ValueType::Nil, ValueType::Nil) => {
                self.stack.push(Value { 
                    val_type: ValueType::Nil 
                });
            },
            // Vector対応（暗黙の反復）
            (ValueType::Vector(v), other) => {
                let other_value = Value { val_type: other.clone() };
                let results: Vec<Value> = v.iter()
                    .map(|elem| {
                        let mut temp_stack = vec![elem.clone(), other_value.clone()];
                        match self.apply_and_3vl(&mut temp_stack) {
                            Ok(result) => result,
                            Err(_) => Value { val_type: ValueType::Nil },
                        }
                    })
                    .collect();
                self.stack.push(Value { 
                    val_type: ValueType::Vector(results) 
                });
            },
            (other, ValueType::Vector(v)) => {
                let other_value = Value { val_type: other.clone() };
                let results: Vec<Value> = v.iter()
                    .map(|elem| {
                        let mut temp_stack = vec![other_value.clone(), elem.clone()];
                        match self.apply_and_3vl(&mut temp_stack) {
                            Ok(result) => result,
                            Err(_) => Value { val_type: ValueType::Nil },
                        }
                    })
                    .collect();
                self.stack.push(Value { 
                    val_type: ValueType::Vector(results) 
                });
            },
            _ => return Err("Type error in AND".to_string()),
        }
        Ok(())
    }

    // ヘルパー関数：3値論理のAND
    fn apply_and_3vl(&self, stack: &mut Vec<Value>) -> Result<Value, String> {
        let b = stack.pop().unwrap();
        let a = stack.pop().unwrap();
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Boolean(a_val), ValueType::Boolean(b_val)) => {
                Ok(Value { val_type: ValueType::Boolean(*a_val && *b_val) })
            },
            (ValueType::Boolean(false), ValueType::Nil) |
            (ValueType::Nil, ValueType::Boolean(false)) => {
                Ok(Value { val_type: ValueType::Boolean(false) })
            },
            (ValueType::Boolean(_), ValueType::Nil) |
            (ValueType::Nil, ValueType::Boolean(_)) |
            (ValueType::Nil, ValueType::Nil) => {
                Ok(Value { val_type: ValueType::Nil })
            },
            _ => Err("Type error in AND".to_string()),
        }
    }

    // 三値論理対応のOR演算
    fn op_or(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Boolean(a_val), ValueType::Boolean(b_val)) => {
                self.stack.push(Value { 
                    val_type: ValueType::Boolean(*a_val || *b_val) 
                });
            },
            // trueが確定的
            (ValueType::Boolean(true), ValueType::Nil) |
            (ValueType::Nil, ValueType::Boolean(true)) => {
                self.stack.push(Value { 
                    val_type: ValueType::Boolean(true) 
                });
            },
            // 結果は不明
            (ValueType::Boolean(_), ValueType::Nil) |
            (ValueType::Nil, ValueType::Boolean(_)) |
            (ValueType::Nil, ValueType::Nil) => {
                self.stack.push(Value { 
                    val_type: ValueType::Nil 
                });
            },
            // Vector対応（暗黙の反復）
            (ValueType::Vector(v), other) => {
                let other_value = Value { val_type: other.clone() };
                let results: Vec<Value> = v.iter()
                    .map(|elem| {
                        let mut temp_stack = vec![elem.clone(), other_value.clone()];
                        match self.apply_or_3vl(&mut temp_stack) {
                            Ok(result) => result,
                            Err(_) => Value { val_type: ValueType::Nil },
                        }
                    })
                    .collect();
                self.stack.push(Value { 
                    val_type: ValueType::Vector(results) 
                });
            },
            (other, ValueType::Vector(v)) => {
                let other_value = Value { val_type: other.clone() };
                let results: Vec<Value> = v.iter()
                    .map(|elem| {
                        let mut temp_stack = vec![other_value.clone(), elem.clone()];
                        match self.apply_or_3vl(&mut temp_stack) {
                            Ok(result) => result,
                            Err(_) => Value { val_type: ValueType::Nil },
                        }
                    })
                    .collect();
                self.stack.push(Value { 
                    val_type: ValueType::Vector(results) 
                });
            },
            _ => return Err("Type error in OR".to_string()),
        }
        Ok(())
    }

    // ヘルパー関数：3値論理のOR
    fn apply_or_3vl(&self, stack: &mut Vec<Value>) -> Result<Value, String> {
        let b = stack.pop().unwrap();
        let a = stack.pop().unwrap();
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Boolean(a_val), ValueType::Boolean(b_val)) => {
                Ok(Value { val_type: ValueType::Boolean(*a_val || *b_val) })
            },
            (ValueType::Boolean(true), ValueType::Nil) |
            (ValueType::Nil, ValueType::Boolean(true)) => {
                Ok(Value { val_type: ValueType::Boolean(true) })
            },
            (ValueType::Boolean(_), ValueType::Nil) |
            (ValueType::Nil, ValueType::Boolean(_)) |
            (ValueType::Nil, ValueType::Nil) => {
                Ok(Value { val_type: ValueType::Nil })
            },
            _ => Err("Type error in OR".to_string()),
        }
    }

    // Nil関連操作
    fn op_nil_check(&mut self) -> Result<(), String> {
        web_sys::console::log_1(&format!("NIL?: checking if stack top is nil, stack size: {}", self.stack.len()).into());
        
        if let Some(val) = self.stack.pop() {
            let is_nil = matches!(val.val_type, ValueType::Nil);
            web_sys::console::log_1(&format!("NIL?: value = {:?}, result = {}", val, is_nil).into());
            
            self.stack.push(Value { val_type: ValueType::Boolean(is_nil) });
            Ok(())
        } else {
            Err("Stack underflow".to_string())
        }
    }

    fn op_not_nil_check(&mut self) -> Result<(), String> {
        web_sys::console::log_1(&format!("NOT-NIL?/KNOWN?: checking if stack top is not nil, stack size: {}", self.stack.len()).into());
        
        if let Some(val) = self.stack.pop() {
            let not_nil = !matches!(val.val_type, ValueType::Nil);
            web_sys::console::log_1(&format!("NOT-NIL?/KNOWN?: value = {:?}, result = {}", val, not_nil).into());
            
            self.stack.push(Value { val_type: ValueType::Boolean(not_nil) });
            Ok(())
        } else {
            Err("Stack underflow".to_string())
        }
    }

    fn op_default(&mut self) -> Result<(), String> {
        web_sys::console::log_1(&format!("DEFAULT: stack size = {}", self.stack.len()).into());
        
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let default_val = self.stack.pop().unwrap();
        let val = self.stack.pop().unwrap();
        
        web_sys::console::log_1(&format!("DEFAULT: value = {:?}, default = {:?}", val, default_val).into());
        
        match val.val_type {
            ValueType::Nil => {
                web_sys::console::log_1(&"DEFAULT: using default value".into());
                self.stack.push(default_val)
            },
            _ => {
                web_sys::console::log_1(&"DEFAULT: using original value".into());
                self.stack.push(val)
            },
        }
        Ok(())
    }

    // データベース操作
    fn op_table(&mut self) -> Result<(), String> {
        web_sys::console::log_1(&format!("TABLE: loading table, stack size = {}", self.stack.len()).into());
        
        if let Some(val) = self.stack.pop() {
            match val.val_type {
                ValueType::String(name) => {
                    web_sys::console::log_1(&format!("TABLE: looking for table '{}'", name).into());
                    
                    if let Some(table) = self.tables.get(&name) {
                        web_sys::console::log_1(&format!("TABLE: found table '{}' with {} records", name, table.records.len()).into());
                        
                        let table_vec = self.table_to_vector(table);
                        self.stack.push(table_vec);
                        self.current_table = Some(name);
                        Ok(())
                    } else {
                        web_sys::console::log_1(&format!("TABLE: table '{}' not found", name).into());
                        Err(format!("Table '{}' not found", name))
                    }
                },
                _ => Err("TABLE requires a string".to_string()),
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }

    fn op_table_create(&mut self) -> Result<(), String> {
        web_sys::console::log_1(&format!("TABLE-CREATE: creating table, stack size = {}", self.stack.len()).into());
        
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let name_val = self.stack.pop().unwrap();
        let data_val = self.stack.pop().unwrap();
        
        match (&name_val.val_type, &data_val.val_type) {
            (ValueType::String(name), ValueType::Vector(records)) => {
                web_sys::console::log_1(&format!("TABLE-CREATE: creating table '{}' with {} records", name, records.len()).into());
                
                // recordsは各レコード（Vector）を含むVector
                let mut table_records: Vec<Vec<Value>> = Vec::new();
                
                for record in records {
                    if let ValueType::Vector(fields) = &record.val_type {
                        table_records.push(fields.clone());
                    } else {
                        return Err("Each record must be a vector".to_string());
                    }
                }
                
                // 最初のレコードからスキーマを推測
                if let Some(first_record) = table_records.first() {
                    let schema: Vec<String> = first_record.iter()
                        .step_by(2)
                        .filter_map(|v| match &v.val_type {
                            ValueType::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect();
                    
                    web_sys::console::log_1(&format!("TABLE-CREATE: schema = {:?}", schema).into());
                    
                    let table_data = TableData {
                        schema,
                        records: table_records,
                    };
                    
                    self.tables.insert(name.clone(), table_data);
                    Ok(())
                } else {
                    Err("Cannot create empty table".to_string())
                }
            },
            _ => Err("TABLE-CREATE requires a vector and a string".to_string()),
        }
    }

    fn op_filter(&mut self) -> Result<(), String> {
        web_sys::console::log_1(&format!("FILTER: filtering table, stack size = {}", self.stack.len()).into());
        
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let filter_val = self.stack.pop().unwrap();
        let table_val = self.stack.pop().unwrap();
        
        match (&table_val.val_type, &filter_val.val_type) {
            (ValueType::Vector(records), ValueType::Vector(filter_expr)) => {
                web_sys::console::log_1(&format!("FILTER: filtering {} records", records.len()).into());
                
                let mut filtered_records = Vec::new();
                
                for (idx, record) in records.iter().enumerate() {
                    // 各レコードに対してフィルタ式を評価
                    self.stack.push(record.clone());
                    let (tokens, _) = self.body_vector_to_tokens(filter_expr)?;
                    self.execute_tokens_with_context(&tokens)?;
                    
                    if let Some(result) = self.stack.pop() {
                        match result.val_type {
                            ValueType::Boolean(true) => {
                                web_sys::console::log_1(&format!("FILTER: record {} passed", idx).into());
                                filtered_records.push(record.clone());
                            },
                            _ => {
                                web_sys::console::log_1(&format!("FILTER: record {} filtered out", idx).into());
                            },
                        }
                    }
                }
                
                web_sys::console::log_1(&format!("FILTER: {} records passed filter", filtered_records.len()).into());
                
                self.stack.push(Value { val_type: ValueType::Vector(filtered_records) });
                Ok(())
            },
            _ => Err("FILTER requires a table and a filter expression".to_string()),
        }
    }

    fn op_project(&mut self) -> Result<(), String> {
        web_sys::console::log_1(&format!("PROJECT: projecting columns, stack size = {}", self.stack.len()).into());
        
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let columns_val = self.stack.pop().unwrap();
        let table_val = self.stack.pop().unwrap();
        
        match (&table_val.val_type, &columns_val.val_type) {
            (ValueType::Vector(records), ValueType::Vector(columns)) => {
                web_sys::console::log_1(&format!("PROJECT: projecting {} columns from {} records", columns.len(), records.len()).into());
                
                let mut projected_records = Vec::new();
                
                for record in records {
                    if let ValueType::Vector(fields) = &record.val_type {
                        let mut new_fields = Vec::new();
                        
                        for col in columns {
                            if let ValueType::String(col_name) = &col.val_type {
                                // レコードから指定されたカラムを探す
                                for i in (0..fields.len()).step_by(2) {
                                    if let ValueType::String(field_name) = &fields[i].val_type {
                                        if field_name == col_name && i + 1 < fields.len() {
                                            new_fields.push(fields[i].clone());
                                            new_fields.push(fields[i + 1].clone());
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        
                        if !new_fields.is_empty() {
                            projected_records.push(Value { val_type: ValueType::Vector(new_fields) });
                        }
                    }
                }
                
                web_sys::console::log_1(&format!("PROJECT: produced {} records", projected_records.len()).into());
                
                self.stack.push(Value { val_type: ValueType::Vector(projected_records) });
                Ok(())
            },
            _ => Err("PROJECT requires a table and column names".to_string()),
        }
    }

    fn op_insert(&mut self) -> Result<(), String> {
        web_sys::console::log_1(&format!("INSERT: inserting record, stack size = {}", self.stack.len()).into());
        
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let table_name_val = self.stack.pop().unwrap();
        let record_val = self.stack.pop().unwrap();
        
        match (&table_name_val.val_type, &record_val.val_type) {
            (ValueType::String(name), ValueType::Vector(fields)) => {
                web_sys::console::log_1(&format!("INSERT: inserting into table '{}'", name).into());
                
                if let Some(table) = self.tables.get_mut(name) {
                    web_sys::console::log_1(&format!("INSERT: table had {} records", table.records.len()).into());
                    table.records.push(fields.clone());
                    web_sys::console::log_1(&format!("INSERT: table now has {} records", table.records.len()).into());
                    Ok(())
                } else {
                    Err(format!("Table '{}' not found", name))
                }
            },
            _ => Err("INSERT requires a record vector and table name".to_string()),
        }
    }

    fn op_update(&mut self) -> Result<(), String> {
        web_sys::console::log_1(&"UPDATE: not fully implemented yet".into());
        // TODO: 完全な実装を後で追加
        Ok(())
    }

    fn op_delete(&mut self) -> Result<(), String> {
        web_sys::console::log_1(&"DELETE: not fully implemented yet".into());
        // TODO: 完全な実装を後で追加
        Ok(())
    }

    fn op_tables(&mut self) -> Result<(), String> {
        web_sys::console::log_1(&format!("TABLES: listing tables, stack size = {}", self.stack.len()).into());
        
        if let Some(pattern_val) = self.stack.pop() {
            match pattern_val.val_type {
                ValueType::String(pattern) => {
                    web_sys::console::log_1(&format!("TABLES: searching with pattern '{}'", pattern).into());
                    
                    let table_names: Vec<Value> = self.tables.keys()
                        .filter(|name| {
                            let matches = self.wildcard_match(name, &pattern);
                            web_sys::console::log_1(&format!("TABLES: '{}' matches '{}': {}", name, pattern, matches).into());
                            matches
                        })
                        .map(|name| Value { val_type: ValueType::String(name.clone()) })
                        .collect();
                    
                    web_sys::console::log_1(&format!("TABLES: found {} matching tables", table_names.len()).into());
                    
                    self.stack.push(Value { val_type: ValueType::Vector(table_names) });
                    Ok(())
                },
                _ => Err("TABLES requires a pattern string".to_string()),
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }

    fn op_save_db(&mut self) -> Result<(), String> {
        web_sys::console::log_1(&"SAVE-DB: Saving database to IndexedDB".into());
        
        // JavaScript側にイベントを送信
        if let Some(window) = web_sys::window() {
            let event = web_sys::CustomEvent::new("ajisai-save-db")
                .map_err(|_| "Failed to create save event")?;
            window.dispatch_event(&event)
                .map_err(|_| "Failed to dispatch save event")?;
        }
        
        web_sys::console::log_1(&format!("SAVE-DB: {} tables to save", self.tables.len()).into());
        Ok(())
    }

    fn op_load_db(&mut self) -> Result<(), String> {
        web_sys::console::log_1(&"LOAD-DB: Loading database from IndexedDB".into());
        
        // JavaScript側にイベントを送信
        if let Some(window) = web_sys::window() {
            let event = web_sys::CustomEvent::new("ajisai-load-db")
                .map_err(|_| "Failed to create load event")?;
            window.dispatch_event(&event)
                .map_err(|_| "Failed to dispatch load event")?;
        }
        
        Ok(())
    }

    // ワイルドカード操作
    fn op_match(&mut self) -> Result<(), String> {
        web_sys::console::log_1(&format!("MATCH?: wildcard matching, stack size = {}", self.stack.len()).into());
        
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let pattern = self.stack.pop().unwrap();
        let value = self.stack.pop().unwrap();
        
        match (&value.val_type, &pattern.val_type) {
            (ValueType::String(s), ValueType::String(p)) => {
                let result = self.wildcard_match(s, p);
                web_sys::console::log_1(&format!("MATCH?: '{}' matches '{}': {}", s, p, result).into());
                
                self.stack.push(Value { 
                    val_type: ValueType::Boolean(result) 
                });
            },
            (ValueType::Vector(v), ValueType::String(p)) => {
                web_sys::console::log_1(&format!("MATCH?: matching {} items against '{}'", v.len(), p).into());
                
                // 暗黙の反復
                let results: Vec<Value> = v.iter()
                    .map(|item| match &item.val_type {
                        ValueType::String(s) => {
                            let matches = self.wildcard_match(s, p);
                            web_sys::console::log_1(&format!("MATCH?: '{}' matches '{}': {}", s, p, matches).into());
                            Value {
                                val_type: ValueType::Boolean(matches)
                            }
                        },
                        _ => Value { val_type: ValueType::Boolean(false) }
                    })
                    .collect();
                self.stack.push(Value { 
                    val_type: ValueType::Vector(results) 
                });
            },
            _ => return Err("Type error in MATCH?".to_string()),
        }
        Ok(())
    }

    fn op_wildcard(&mut self) -> Result<(), String> {
        web_sys::console::log_1(&"WILDCARD: pattern creation (no-op for now)".into());
        // パターンをスタックに載せる（将来の拡張用）
        Ok(())
    }

    // ヘルパー関数
    fn wildcard_match(&self, text: &str, pattern: &str) -> bool {
        let mut text_chars = text.chars().peekable();
        let mut pattern_chars = pattern.chars().peekable();
        
        while let Some(&p) = pattern_chars.peek() {
            match p {
                '*' => {
                    pattern_chars.next();
                    if pattern_chars.peek().is_none() {
                        return true; // パターンが*で終わる
                    }
                    // 次のパターン文字が見つかるまでテキストを進める
                    while text_chars.peek().is_some() {
                        if self.wildcard_match(
                            &text_chars.clone().collect::<String>(),
                            &pattern_chars.clone().collect::<String>()
                        ) {
                            return true;
                        }
                        text_chars.next();
                    }
                    return false;
                },
                '?' => {
                    pattern_chars.next();
                    if text_chars.next().is_none() {
                        return false;
                    }
                },
                _ => {
                    pattern_chars.next();
                    if text_chars.next() != Some(p) {
                        return false;
                    }
                }
            }
        }
        
        text_chars.peek().is_none()
    }

    fn table_to_vector(&self, table: &TableData) -> Value {
        // 各レコードをValueに変換
        let record_values: Vec<Value> = table.records.iter()
            .map(|record| Value { 
                val_type: ValueType::Vector(record.clone()) 
            })
            .collect();
        
        Value { val_type: ValueType::Vector(record_values) }
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
            // Nilの場合はelse分岐
            (ValueType::Nil, ValueType::Vector(_), ValueType::Vector(else_vec)) => {
                let (tokens, _) = self.body_vector_to_tokens(else_vec)?;
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
                        ValueType::Nil => {
                            let (tokens, _) = self.body_vector_to_tokens(else_vec)?;
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
                ValueType::Nil => {
                    // Nilの否定はNil
                    self.stack.push(Value { val_type: ValueType::Nil });
                    Ok(())
                },
                // Vectorに対してもNOTを適用（暗黙の反復）
                ValueType::Vector(v) => {
                    let result: Vec<Value> = v.iter()
                        .map(|elem| match &elem.val_type {
                            ValueType::Boolean(b) => Value {
                                val_type: ValueType::Boolean(!b)
                            },
                            ValueType::Nil => Value { val_type: ValueType::Nil },
                            _ => elem.clone()
                        })
                        .collect();
                    self.stack.push(Value { val_type: ValueType::Vector(result) });
                    Ok(())
                },
                _ => Err("Type error: NOT requires a boolean, nil, or vector".to_string()),
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
   
   // データベース操作用のpublicメソッド
   pub fn save_table(&mut self, name: String, schema: Vec<String>, records: Vec<Vec<Value>>) {
       let table_data = TableData { schema, records };
       self.tables.insert(name, table_data);
   }
   
   pub fn load_table(&self, name: &str) -> Option<(Vec<String>, Vec<Vec<Value>>)> {
       self.tables.get(name).map(|t| (t.schema.clone(), t.records.clone()))
   }
   
   pub fn get_all_tables(&self) -> Vec<String> {
       self.tables.keys().cloned().collect()
   }

    // スタックを設定
    pub fn set_stack(&mut self, stack: Stack) {
        self.stack = stack;
    }
    
    // レジスタを設定
    pub fn set_register(&mut self, register: Register) {
        self.register = register;
    }
    
    // カスタムワードの定義を取得（復元用）
    pub fn get_word_definition(&self, name: &str) -> Option<String> {
        if let Some(def) = self.dictionary.get(name) {
            if !def.is_builtin {
                // トークンを文字列に変換
                let mut result = String::new();
                for token in &def.tokens {
                    match token {
                        Token::Number(n, d) => {
                            if *d == 1 {
                                result.push_str(&n.to_string());
                            } else {
                                result.push_str(&format!("{}/{}", n, d));
                            }
                        },
                        Token::String(s) => result.push_str(&format!("\"{}\"", s)),
                        Token::Boolean(b) => result.push_str(if *b { "true" } else { "false" }),
                        Token::Nil => result.push_str("nil"),
                        Token::Symbol(s) => result.push_str(s),
                        Token::VectorStart => result.push_str("["),
                        Token::VectorEnd => result.push_str("]"),
                        Token::Description(d) => result.push_str(&format!("({})", d)),
                    }
                    result.push(' ');
                }
                Some(result.trim().to_string())
            } else {
                None
            }
        } else {
            None
        }
    }


    #[wasm_bindgen]
    pub fn restore_stack(&mut self, stack_js: JsValue) -> Result<(), String> {
        if !stack_js.is_array() {
            return Err("Stack must be an array".to_string());
        }
        
        let arr = js_sys::Array::from(&stack_js);
        let mut new_stack = Vec::new();
        
        for i in 0..arr.length() {
            let item = arr.get(i);
            let value = js_value_to_rust_value(&item)?;
            new_stack.push(value);
        }
        
        self.interpreter.set_stack(new_stack);
        Ok(())
    }
    
    #[wasm_bindgen]
    pub fn restore_register(&mut self, register_js: JsValue) -> Result<(), String> {
        if register_js.is_null() || register_js.is_undefined() {
            self.interpreter.set_register(None);
        } else {
            let value = js_value_to_rust_value(&register_js)?;
            self.interpreter.set_register(Some(value));
        }
        Ok(())
    }
    
    #[wasm_bindgen]
    pub fn get_word_definition(&self, name: &str) -> JsValue {
        match self.interpreter.get_word_definition(name) {
            Some(def) => JsValue::from_str(&def),
            None => JsValue::NULL,
        }
    }
    
    #[wasm_bindgen]
    pub fn restore_word(&mut self, name: String, definition: String, description: Option<String>) -> Result<(), String> {
        // 説明があれば先に追加
        let code = if let Some(desc) = description {
            format!("({}) {} \"{}\" DEF", desc, definition, name)
        } else {
            format!("{} \"{}\" DEF", definition, name)
        };
        
        self.interpreter.execute(&code)?;
        Ok(())
    }
}
