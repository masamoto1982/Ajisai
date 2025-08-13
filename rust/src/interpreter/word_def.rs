// rust/src/interpreter/word_def.rs

use std::collections::{HashSet, VecDeque};
use crate::types::Token;
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
            is_temporary: true,  // 二項演算で生成されたワードは一時的
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
    
    // 一時的なワードとその依存関係を再帰的に削除するメソッド
    pub(super) fn delete_temporary_word_cascade(&mut self, word_name: &str) {
        // 削除対象のワードを収集
        let mut words_to_delete = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(word_name.to_string());
        
        while let Some(current_word) = queue.pop_front() {
            // すでに処理済みならスキップ
            if !words_to_delete.insert(current_word.clone()) {
                continue;
            }
            
            // このワードが使用しているワードを探す
            if let Some(def) = self.dictionary.get(&current_word) {
                // 一時的なワードのみ対象
                if def.is_temporary {
                    // トークンから依存しているワードを抽出
                    for token in &def.tokens {
                        if let Token::Symbol(dep_name) = token {
                            if let Some(dep_def) = self.dictionary.get(dep_name) {
                                // 依存先も一時的なワードなら削除対象に追加
                                if dep_def.is_temporary {
                                    queue.push_back(dep_name.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // 収集したワードをすべて削除
        for word in words_to_delete {
            console::log_1(&JsValue::from_str(&format!("Deleting temporary word: {}", word)));
            
            // 辞書から削除
            self.dictionary.remove(&word);
            self.word_properties.remove(&word);
            
            // 依存関係のクリーンアップ
            self.dependencies.remove(&word);
            for (_, deps) in self.dependencies.iter_mut() {
                deps.remove(&word);
            }
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
            is_temporary: false,  // 復元されたワードは永続的
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

    pub(super) fn token_to_string(&self, token: &Token) -> String {
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
