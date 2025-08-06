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

        // デバッグ用に元の式を description に保存
        let description = Some(format!("Auto: {}", 
            tokens.iter()
                .take(20)  // 最初の20トークンまで
                .map(|t| self.token_to_string(t))
                .collect::<Vec<_>>()
                .join(" ")
        ));

        self.dictionary.insert(name.clone(), WordDefinition {
            tokens: storage_tokens,
            is_builtin: false,
            description,
        });

        let is_producer = self.check_if_value_producer(&name);
        self.word_properties.insert(name.clone(), WordProperty {
            is_value_producer: is_producer,
        });

        self.append_output(&format!("Defined: {}\n", name));
        console::log_1(&JsValue::from_str("--- end define_from_tokens ---"));
        Ok(())
    }

    // rust/src/interpreter/word_def.rs のgenerate_word_nameメソッドを修正

pub(super) fn generate_word_name(&self, tokens: &[Token]) -> String {
    console::log_1(&JsValue::from_str("--- generate_word_name ---"));
    console::log_1(&JsValue::from_str(&format!("Input tokens for naming: {:?}", tokens)));

    // RPN形式に変換
    let rpn_tokens = self.convert_to_rpn(tokens);
    console::log_1(&JsValue::from_str(&format!("RPN tokens for naming: {:?}", rpn_tokens)));
    
    // トークンを文字列化して名前を生成
    let name_parts: Vec<String> = rpn_tokens.iter()
        .map(|token| match token {
            Token::Number(n, d) => {
                if *d == 1 {
                    n.to_string()
                } else {
                    format!("{}D{}", n, d)  // 分数は「/」の代わりに「D」を使用
                }
            },
            Token::String(s) => format!("S_{}", s.replace(" ", "_")),  // 文字列は「S_」プレフィックス
            Token::Boolean(b) => b.to_string().to_uppercase(),
            Token::Symbol(s) => {
                // 演算子記号を安全な文字に置換
                match s.as_str() {
                    "+" => "ADD".to_string(),
                    "-" => "SUB".to_string(),
                    "*" => "MUL".to_string(),
                    "/" => "DIV".to_string(),
                    ">" => "GT".to_string(),
                    ">=" => "GE".to_string(),
                    "=" => "EQ".to_string(),
                    "<" => "LT".to_string(),
                    "<=" => "LE".to_string(),
                    _ => s.clone()
                }
            },
            Token::Nil => "NIL".to_string(),
            Token::VectorStart => "V".to_string(),  // Vectorの開始
            Token::VectorEnd => "V".to_string(),    // Vectorの終了
            Token::BlockStart => "B".to_string(),   // Blockの開始
            Token::BlockEnd => "B".to_string(),     // Blockの終了
        })
        .collect();
    
    let name = name_parts.join("_");

    console::log_1(&JsValue::from_str(&format!("Generated name: {}", name)));
    console::log_1(&JsValue::from_str("--- end generate_word_name ---"));
    
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
