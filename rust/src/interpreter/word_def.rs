// rust/src/interpreter/word_def.rs

use std::collections::HashSet;
use crate::types::{Token};
use super::{Interpreter, WordDefinition, WordProperty, error::{AjisaiError, Result}};
use wasm_bindgen::JsValue;
use web_sys::console;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

impl Interpreter {
    pub(super) fn define_named_word(&mut self, name: String, body_tokens: Vec<Token>) -> Result<()> {
        console::log_1(&JsValue::from_str("--- define_named_word ---"));
        console::log_1(&JsValue::from_str(&format!("Defining word: {}", name)));
        console::log_1(&JsValue::from_str(&format!("Body tokens (RPN): {:?}", body_tokens)));
        
        let name = name.to_uppercase();

        if let Some(existing) = self.dictionary.get(&name) {
            if existing.is_builtin {
                return Err(AjisaiError::from(format!("Cannot redefine builtin word: {}", name)));
            }
        }

        if self.dictionary.contains_key(&name) {
            if let Some(dependents) = self.dependencies.get(&name) {
                if !dependents.is_empty() {
                    let dependent_list: Vec<String> = dependents.iter().cloned().collect();
                    return Err(AjisaiError::ProtectedWord {
                        name: name.clone(),
                        dependents: dependent_list,
                    });
                }
            }
        }

        let mut new_dependencies = HashSet::new();
        for token in &body_tokens {
            if let Token::Symbol(s) = token {
                if self.dictionary.contains_key(s) {
                    new_dependencies.insert(s.clone());
                }
            }
        }

        for dep_name in &new_dependencies {
            self.dependencies
                .entry(dep_name.clone())
                .or_insert_with(HashSet::new)
                .insert(name.clone());
        }

        self.dictionary.insert(name.clone(), WordDefinition {
            tokens: body_tokens,
            is_builtin: false,
            description: None,
        });

        let is_producer = self.check_if_value_producer(&name);
        self.word_properties.insert(name.clone(), WordProperty {
            is_value_producer: is_producer,
        });

        self.append_output(&format!("Defined: {}\n", name));
        console::log_1(&JsValue::from_str("--- end define_named_word ---"));

        Ok(())
    }
    
    pub(super) fn define_from_tokens(&mut self, tokens: &[Token]) -> Result<()> {
        console::log_1(&JsValue::from_str("--- define_from_tokens (auto-naming) ---"));
        console::log_1(&JsValue::from_str(&format!("Original tokens: {:?}", tokens)));

        let name = self.generate_word_name(tokens);
        console::log_1(&JsValue::from_str(&format!("Generated name: {}", name)));
        
        if self.dictionary.contains_key(&name) {
            if let Some(dependents) = self.dependencies.get(&name) {
                if !dependents.is_empty() {
                    let dependent_list: Vec<String> = dependents.iter().cloned().collect();
                    return Err(AjisaiError::ProtectedWord {
                        name: name.clone(),
                        dependents: dependent_list,
                    });
                }
            }
            self.append_output(&format!("Word '{}' already exists.\n", name));
            console::log_1(&JsValue::from_str("--- end define_from_tokens (already exists) ---"));
            return Ok(());
        }

        self.auto_named = true;
        self.last_auto_named_word = Some(name.clone());

        let storage_tokens = self.rearrange_tokens(tokens);
        console::log_1(&JsValue::from_str(&format!("Storage tokens (RPN): {:?}", storage_tokens)));

        let mut new_dependencies = HashSet::new();
        for token in &storage_tokens {
            if let Token::Symbol(s) = token {
                if self.dictionary.contains_key(s) {
                    new_dependencies.insert(s.clone());
                }
            }
        }

        for dep_name in &new_dependencies {
            self.dependencies
                .entry(dep_name.clone())
                .or_insert_with(HashSet::new)
                .insert(name.clone());
        }

        self.dictionary.insert(name.clone(), WordDefinition {
            tokens: storage_tokens,
            is_builtin: false,
            description: None,
        });

        let is_producer = self.check_if_value_producer(&name);
        self.word_properties.insert(name.clone(), WordProperty {
            is_value_producer: is_producer,
        });

        self.append_output(&format!("Defined: {}\n", name));
        console::log_1(&JsValue::from_str("--- end define_from_tokens ---"));
        Ok(())
    }

    pub(super) fn generate_word_name(&self, tokens: &[Token]) -> String {
        console::log_1(&JsValue::from_str("--- generate_word_name ---"));
        console::log_1(&JsValue::from_str(&format!("Input tokens for naming: {:?}", tokens)));

        // RPN形式に変換
        let rpn_tokens = self.convert_to_rpn(tokens);
        
        // 基本ハッシュを生成
        let base_hash = self.calculate_token_hash(&rpn_tokens);
        
        // 衝突回避ループ
        let mut attempt = 0u32;
        loop {
            let name = if attempt == 0 {
                self.hash_to_name(base_hash)
            } else {
                // 衝突時は番号を付与して再生成
                let mut hasher = DefaultHasher::new();
                base_hash.hash(&mut hasher);
                attempt.hash(&mut hasher);
                let modified_hash = hasher.finish();
                self.hash_to_name(modified_hash)
            };
            
            // 既存の辞書に存在しないか確認
            if !self.dictionary.contains_key(&name) {
                console::log_1(&JsValue::from_str(&format!("Generated unique name: {} (attempt: {})", name, attempt)));
                console::log_1(&JsValue::from_str("--- end generate_word_name ---"));
                return name;
            }
            
            // 既存の定義と同じトークン列なら同じ名前を返す
            if let Some(existing_def) = self.dictionary.get(&name) {
                if !existing_def.is_builtin && self.tokens_equal(&existing_def.tokens, &rpn_tokens) {
                    console::log_1(&JsValue::from_str(&format!("Found identical definition: {}", name)));
                    console::log_1(&JsValue::from_str("--- end generate_word_name ---"));
                    return name;
                }
            }
            
            console::log_1(&JsValue::from_str(&format!("Collision detected for {}, trying again...", name)));
            attempt += 1;
            
            // 安全装置：1000回試行しても衝突し続ける場合は連番方式にフォールバック
            if attempt >= 1000 {
                let fallback_name = format!("W_FALLBACK_{}", 
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                );
                console::log_1(&JsValue::from_str(&format!("Fallback to timestamp-based name: {}", fallback_name)));
                return fallback_name;
            }
        }
    }
    
    // トークン列からハッシュを計算
    fn calculate_token_hash(&self, tokens: &[Token]) -> u64 {
        let mut hasher = DefaultHasher::new();
        
        for token in tokens {
            match token {
                Token::Number(n, d) => {
                    "NUM".hash(&mut hasher);
                    n.hash(&mut hasher);
                    d.hash(&mut hasher);
                }
                Token::String(s) => {
                    "STR".hash(&mut hasher);
                    s.hash(&mut hasher);
                }
                Token::Boolean(b) => {
                    "BOOL".hash(&mut hasher);
                    b.hash(&mut hasher);
                }
                Token::Symbol(s) => {
                    "SYM".hash(&mut hasher);
                    s.hash(&mut hasher);
                }
                Token::Nil => {
                    "NIL".hash(&mut hasher);
                }
                Token::VectorStart => {
                    "VSTART".hash(&mut hasher);
                }
                Token::VectorEnd => {
                    "VEND".hash(&mut hasher);
                }
                Token::BlockStart => {
                    "BSTART".hash(&mut hasher);
                }
                Token::BlockEnd => {
                    "BEND".hash(&mut hasher);
                }
            }
        }
        
        hasher.finish()
    }
    
    // ハッシュ値を名前に変換
    fn hash_to_name(&self, hash: u64) -> String {
        let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut name = String::with_capacity(8);
        
        // 最初の文字は必ずアルファベット（A-Z）
        let first_idx = ((hash >> 56) as usize) % 26;
        name.push(chars.chars().nth(first_idx).unwrap());
        
        // 残り7文字（A-Z, 0-9）
        for i in 0..7 {
            let shift = i * 8;
            let idx = ((hash >> shift) as usize) % 36;
            name.push(chars.chars().nth(idx).unwrap());
        }
        
        name
    }
    
    // トークン列の等価性を確認
    fn tokens_equal(&self, tokens1: &[Token], tokens2: &[Token]) -> bool {
        if tokens1.len() != tokens2.len() {
            return false;
        }
        
        for (t1, t2) in tokens1.iter().zip(tokens2.iter()) {
            if !self.token_equal(t1, t2) {
                return false;
            }
        }
        
        true
    }
    
    // 個別トークンの等価性を確認
    fn token_equal(&self, t1: &Token, t2: &Token) -> bool {
        match (t1, t2) {
            (Token::Number(n1, d1), Token::Number(n2, d2)) => n1 == n2 && d1 == d2,
            (Token::String(s1), Token::String(s2)) => s1 == s2,
            (Token::Boolean(b1), Token::Boolean(b2)) => b1 == b2,
            (Token::Symbol(s1), Token::Symbol(s2)) => s1 == s2,
            (Token::Nil, Token::Nil) => true,
            (Token::VectorStart, Token::VectorStart) => true,
            (Token::VectorEnd, Token::VectorEnd) => true,
            (Token::BlockStart, Token::BlockStart) => true,
            (Token::BlockEnd, Token::BlockEnd) => true,
            _ => false,
        }
    }
}

// 新しいヘルパーメソッド（word_def.rsに追加）
fn group_vectors_for_naming(&self, tokens: &[Token]) -> Vec<String> {
    let mut result = Vec::new();
    let mut i = 0;
    
    while i < tokens.len() {
        match &tokens[i] {
            Token::VectorStart => {
                // Vectorの開始を検出
                let mut vector_parts = vec!["[".to_string()];
                i += 1;
                
                // VectorEndまでの要素を収集
                let mut depth = 1;
                while i < tokens.len() && depth > 0 {
                    match &tokens[i] {
                        Token::VectorStart => {
                            depth += 1;
                            vector_parts.push("[".to_string());
                        }
                        Token::VectorEnd => {
                            depth -= 1;
                            if depth == 0 {
                                vector_parts.push("]".to_string());
                            } else {
                                vector_parts.push("]".to_string());
                            }
                        }
                        Token::Number(n, d) => {
                            if *d == 1 {
                                vector_parts.push(n.to_string());
                            } else {
                                vector_parts.push(format!("{}_{}", n, d));
                            }
                        }
                        Token::String(s) => {
                            vector_parts.push(format!("STR_{}", s.replace(" ", "_")));
                        }
                        Token::Boolean(b) => {
                            vector_parts.push(b.to_string().to_uppercase());
                        }
                        Token::Symbol(s) => {
                            vector_parts.push(s.clone());
                        }
                        Token::Nil => {
                            vector_parts.push("NIL".to_string());
                        }
                        Token::BlockStart => {
                            vector_parts.push("{".to_string());
                        }
                        Token::BlockEnd => {
                            vector_parts.push("}".to_string());
                        }
                    }
                    i += 1;
                }
                
                // Vector全体を一つの要素として結合
                result.push(vector_parts.join("_"));
            }
            Token::Number(n, d) => {
                if *d == 1 {
                    result.push(n.to_string());
                } else {
                    result.push(format!("{}_{}", n, d));
                }
                i += 1;
            }
            Token::String(s) => {
                result.push(format!("STR_{}", s.replace(" ", "_")));
                i += 1;
            }
            Token::Boolean(b) => {
                result.push(b.to_string().to_uppercase());
                i += 1;
            }
            Token::Symbol(s) => {
                result.push(s.clone());
                i += 1;
            }
            Token::Nil => {
                result.push("NIL".to_string());
                i += 1;
            }
            Token::BlockStart => {
                // ブロックも同様にグループ化可能（必要に応じて）
                result.push("BSTART".to_string());
                i += 1;
            }
            Token::BlockEnd => {
                result.push("BEND".to_string());
                i += 1;
            }
            Token::VectorEnd => {
                // 単独のVectorEnd（エラーケース）はスキップ
                i += 1;
            }
        }
    }
    
    result
}

    pub(super) fn check_if_value_producer(&self, word_name: &str) -> bool {
        let mut dummy = Interpreter::new();
        dummy.dictionary = self.dictionary.clone();
        
        if let Some(def) = self.dictionary.get(word_name) {
            if !def.is_builtin {
                match dummy.execute_tokens_with_context(&def.tokens) {
                    Ok(_) => !dummy.stack.is_empty(),
                    Err(_) => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn restore_custom_word(&mut self, name: String, tokens: Vec<Token>, description: Option<String>) -> Result<()> {
        let name = name.to_uppercase();
        
        if let Some(existing) = self.dictionary.get(&name) {
            if existing.is_builtin {
                return Err(AjisaiError::from(format!("Cannot restore builtin word: {}", name)));
            }
        }

        let mut new_dependencies = HashSet::new();
        for token in &tokens {
            if let Token::Symbol(s) = token {
                if self.dictionary.contains_key(s) {
                    new_dependencies.insert(s.clone());
                }
            }
        }

        for dep_name in &new_dependencies {
            self.dependencies
                .entry(dep_name.clone())
                .or_insert_with(HashSet::new)
                .insert(name.clone());
        }

        self.dictionary.insert(name.clone(), WordDefinition {
            tokens,
            is_builtin: false,
            description,
        });

        let is_producer = self.check_if_value_producer(&name);
        self.word_properties.insert(name.clone(), WordProperty {
            is_value_producer: is_producer,
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
                return Some(format!("{{ {} }}", body_string));
            }
        }
        None
    }
}
