// rust/src/interpreter/word_def.rs

use std::collections::HashSet;
use crate::types::{Token};
use super::{Interpreter, WordDefinition, WordProperty, error::{AjisaiError, Result}};
use wasm_bindgen::JsValue;
use web_sys::console;

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

    // rust/src/interpreter/word_def.rs のgenerate_word_nameメソッドを置き換え

pub(super) fn generate_word_name(&self, tokens: &[Token]) -> String {
    console::log_1(&JsValue::from_str("--- generate_word_name ---"));
    console::log_1(&JsValue::from_str(&format!("Input tokens for naming: {:?}", tokens)));

    // 統一的なRPN変換を使用
    let rpn_tokens = self.convert_to_rpn(tokens);
    console::log_1(&JsValue::from_str(&format!("RPN tokens for naming: {:?}", rpn_tokens)));
    
    // Vectorをグループ化して処理
    let grouped_tokens = self.group_vectors_for_naming(&rpn_tokens);
    
    let final_name = grouped_tokens.join("_");

    console::log_1(&JsValue::from_str(&format!("Generated final name: {}", final_name)));
    console::log_1(&JsValue::from_str("--- end generate_word_name ---"));
    
    final_name
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
