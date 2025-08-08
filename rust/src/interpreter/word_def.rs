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
            is_temporary: false,  // 明示的に命名されたワードは永続的
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
            // 既存のワードがある場合は、それを実行するように変更
            console::log_1(&JsValue::from_str(&format!("Word '{}' already exists, executing it", name)));
            
            // 既存のワードを実行
            let def = self.dictionary.get(&name).cloned().unwrap();
            if def.is_temporary {
                // 一時的なワードの場合は削除予約
                self.words_to_delete.push(name.clone());
            }
            return self.execute_custom_word(&name, &def.tokens);
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

        // デバッグ用に元の式を description に保存
        let description = Some(format!("Auto: {}", 
            tokens.iter()
                .take(20)  // 最初の20トークンまで
                .map(|t| self.token_to_string(t))
                .collect::<Vec<_>>()
                .join(" ")
        ));

        self.dictionary.insert(name.clone(), WordDefinition {
            tokens: storage_tokens.clone(),
            is_builtin: false,
            is_temporary: true,  // 自動命名されたワードは一時的
            description,
        });

        let is_producer = self.check_if_value_producer(&name);
        self.word_properties.insert(name.clone(), WordProperty {
            is_value_producer: is_producer,
        });

        // 自動命名されたワードを即座に実行
        console::log_1(&JsValue::from_str(&format!("Executing auto-named word: {}", name)));
        self.execute_custom_word(&name, &storage_tokens)?;
        
        // 実行後に削除予約
        self.words_to_delete.push(name.clone());
        
        console::log_1(&JsValue::from_str("--- end define_from_tokens ---"));
        Ok(())
    }

    fn execute_custom_word(&mut self, name: &str, tokens: &[Token]) -> Result<()> {
        self.call_stack.push(name.to_string());
        let result = self.execute_tokens_with_context(tokens);
        self.call_stack.pop();
        
        result.map_err(|e| e.with_context(&self.call_stack))
    }

    pub(super) fn generate_word_name(&self, tokens: &[Token]) -> String {
        console::log_1(&JsValue::from_str("--- generate_word_name ---"));
        console::log_1(&JsValue::from_str(&format!("Input tokens for naming: {:?}", tokens)));

        // タイムスタンプベースのユニークな名前を生成
        let timestamp = js_sys::Date::now() as u64;
        let name = format!("W_{:X}", timestamp);
        
        console::log_1(&JsValue::from_str(&format!("Generated name: {}", name)));
        name
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
}
