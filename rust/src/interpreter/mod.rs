// rust/src/interpreter/mod.rs

pub mod stack_ops;
pub mod arithmetic;
pub mod vector_ops;
pub mod control;
pub mod io;
pub mod error;
pub mod register_ops;
pub mod execute;
pub mod word_def;
pub mod step;
pub mod token_processor;

use std::collections::{HashMap, HashSet};
use crate::types::{Stack, Register};
use self::error::Result;

pub struct Interpreter {
    pub(crate) stack: Stack,
    pub(crate) register: Register,
    pub(crate) dictionary: HashMap<String, WordDefinition>,
    pub(crate) dependencies: HashMap<String, HashSet<String>>,
    pub(crate) call_stack: Vec<String>,
    pub(crate) output_buffer: String,
    pub(crate) word_properties: HashMap<String, WordProperty>,
    // ステップ実行用のフィールド
    pub(crate) step_tokens: Vec<crate::types::Token>,
    pub(crate) step_position: usize,
    pub(crate) step_mode: bool,
    pub(crate) auto_named: bool,
    pub(crate) last_auto_named_word: Option<String>,
    // 実行後削除予定のワードを追跡
    pub(crate) words_to_delete: Vec<String>,
}

#[derive(Clone)]
pub struct WordDefinition {
    pub tokens: Vec<crate::types::Token>,
    pub is_builtin: bool,
    pub is_temporary: bool,  // 追加: 一時的なワードかどうか
    pub description: Option<String>,
}

#[derive(Clone, Debug)]
pub struct WordProperty {
    pub is_value_producer: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Interpreter {
            stack: Vec::new(),
            register: None,
            dictionary: HashMap::new(),
            dependencies: HashMap::new(),
            call_stack: Vec::new(),
            output_buffer: String::new(),
            word_properties: HashMap::new(),
            step_tokens: Vec::new(),
            step_position: 0,
            step_mode: false,
            auto_named: false,
            last_auto_named_word: None,
            words_to_delete: Vec::new(),  // 追加
        };
        
        crate::builtins::register_builtins(&mut interpreter.dictionary);
        interpreter.initialize_word_properties();
        interpreter
    }

    fn initialize_word_properties(&mut self) {
        let value_producers = vec![
            "R>", "R@", "DUP", "OVER", "ROT",
        ];
        
        for name in value_producers {
            self.word_properties.insert(name.to_string(), WordProperty {
                is_value_producer: true,
            });
        }
    }
    
    // 一時的なワードをクリーンアップ
    pub(crate) fn cleanup_temporary_words(&mut self) {
        for word_name in self.words_to_delete.drain(..) {
            // 依存関係もクリーンアップ
            if let Some(deps) = self.dependencies.get(&word_name) {
                for dep in deps.clone() {
                    if let Some(dep_set) = self.dependencies.get_mut(&dep) {
                        dep_set.remove(&word_name);
                    }
                }
            }
            self.dependencies.remove(&word_name);
            self.dictionary.remove(&word_name);
            self.word_properties.remove(&word_name);
        }
    }
    
    // 基本的なアクセサメソッド
    pub fn get_output(&mut self) -> String {
        let output = self.output_buffer.clone();
        self.output_buffer.clear();
        output
    }
    
    pub(crate) fn append_output(&mut self, text: &str) {
        self.output_buffer.push_str(text);
    }
    
    pub fn was_auto_named(&self) -> bool {
        self.auto_named
    }

    pub fn get_last_auto_named_word(&self) -> Option<String> {
        self.last_auto_named_word.clone()
    }
    
    pub fn get_stack(&self) -> &Stack { &self.stack }
    pub fn get_register(&self) -> &Register { &self.register }
    
    pub fn get_custom_words(&self) -> Vec<String> {
        self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)  // 一時的なワードも含める
            .map(|(name, _)| name.clone())
            .collect()
    }
    
    pub fn get_custom_words_with_descriptions(&self) -> Vec<(String, Option<String>)> {
        self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)  // 一時的なワードも含める
            .map(|(name, def)| (name.clone(), def.description.clone()))
            .collect()
    }
   
    pub fn get_custom_words_info(&self) -> Vec<(String, Option<String>, bool)> {
        self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)  // 一時的なワードも含める
            .map(|(name, def)| {
                // 一時的なワードは保護されていないものとして扱う（削除可能）
                let is_protected = if def.is_temporary {
                    false
                } else {
                    self.dependencies.get(name).map_or(false, |deps| !deps.is_empty())
                };
                (name.clone(), def.description.clone(), is_protected)
            })
            .collect()
    }
   
    pub fn set_stack(&mut self, stack: Stack) {
        self.stack = stack;
    }
   
    pub fn set_register(&mut self, register: Register) {
        self.register = register;
    }
}

// AMNESIA操作の実装
pub fn op_amnesia(_interp: &mut Interpreter) -> Result<()> {
    if let Some(window) = web_sys::window() {
        let event = web_sys::CustomEvent::new("ajisai-amnesia")
            .map_err(|_| error::AjisaiError::from("Failed to create amnesia event"))?;
        window.dispatch_event(&event)
            .map_err(|_| error::AjisaiError::from("Failed to dispatch amnesia event"))?;
    }
    Ok(())
}
