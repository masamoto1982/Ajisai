// rust/src/wasm_api.rs

use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;
use std::collections::HashSet; // ★ HashSet をインポート
use std::fmt::Write;           // ★ Write トレイトをインポート
use crate::interpreter::Interpreter;
use crate::types::{Value, ValueType, BracketType, Token};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use crate::interpreter;
use crate::tokenizer;
use crate::builtins;

#[derive(Serialize, Deserialize)]
struct CustomWordData {
    name: String,
    definition: String,
    description: Option<String>,
}

#[wasm_bindgen]
pub struct AjisaiInterpreter {
    interpreter: Interpreter,
    step_tokens: Vec<Token>,
    step_position: usize,
    step_mode: bool,
    current_step_code: String,
}

#[wasm_bindgen]
impl AjisaiInterpreter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        AjisaiInterpreter {
            interpreter: Interpreter::new(),
            step_tokens: Vec::new(),
            step_position: 0,
            step_mode: false,
            current_step_code: String::new(),
        }
    }

    #[wasm_bindgen]
    pub async fn execute(&mut self, code: &str) -> Result<JsValue, JsValue> {
        self.interpreter.definition_to_load = None;
        let obj = js_sys::Object::new();

        match self.interpreter.execute(code).await {
            Ok(()) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();

                // アウトプットとデバッグアウトプットを両方取得
                let output = self.interpreter.get_output();
                let debug_output = self.interpreter.get_debug_output();

                js_sys::Reflect::set(&obj, &"output".into(), &output.into()).unwrap();
                js_sys::Reflect::set(&obj, &"debugOutput".into(), &debug_output.into()).unwrap();

                js_sys::Reflect::set(&obj, &"stack".into(), &self.get_stack()).unwrap();
                js_sys::Reflect::set(&obj, &"customWords".into(), &self.get_custom_words_for_state()).unwrap();

                if let Some(def_str) = self.interpreter.definition_to_load.take() {
                    js_sys::Reflect::set(&obj, &"definition_to_load".into(), &def_str.into()).unwrap();
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                js_sys::Reflect::set(&obj, &"message".into(), &error_msg.into()).unwrap();
                js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();

                // エラー時も、そこまでのデバッグログを返す
                let debug_output = self.interpreter.get_debug_output();
                js_sys::Reflect::set(&obj, &"debugOutput".into(), &debug_output.into()).unwrap();
            }
        }
        Ok(obj.into())
    }

    #[wasm_bindgen]
    pub fn execute_step(&mut self, code: &str) -> JsValue {
        let obj = js_sys::Object::new();

        if !self.step_mode || code != self.current_step_code {
            self.step_mode = true;
            self.step_position = 0;
            self.current_step_code = code.to_string();

            // ステップ実行時もバッファをクリア
            self.interpreter.output_buffer.clear();
            self.interpreter.debug_buffer.clear();
            self.interpreter.call_stack_depth = 0; // call_stack_depth にアクセス

            let custom_word_names: std::collections::HashSet<String> = self.interpreter.dictionary.iter()
                .filter(|(_, def)| !def.is_builtin)
                .map(|(name, _)| name.clone())
                .collect();

            match tokenizer::tokenize_with_custom_words(code, &custom_word_names) {
                Ok(tokens) => { self.step_tokens = tokens; }
                Err(e) => {
                    self.step_mode = false;
                    js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                    js_sys::Reflect::set(&obj, &"message".into(), &format!("Tokenization error: {}", e).into()).unwrap();
                    js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
                    return obj.into();
                }
            }
        }

        if self.step_position >= self.step_tokens.len() {
            self.step_mode = false;
            js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
            js_sys::Reflect::set(&obj, &"output".into(), &"Step execution completed".into()).unwrap();
            js_sys::Reflect::set(&obj, &"debugOutput".into(), &self.interpreter.get_debug_output().into()).unwrap();
            js_sys::Reflect::set(&obj, &"hasMore".into(), &false.into()).unwrap();
            return obj.into();
        }

        let token = self.step_tokens[self.step_position].clone();

        // ステップ実行では execute_tokens_sync を使う
        let result = self.interpreter.execute_tokens_sync(&[token]);

        match result {
            Ok(()) => {
                let output = self.interpreter.get_output();
                let debug_output = self.interpreter.get_debug_output();
                self.step_position += 1;

                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                js_sys::Reflect::set(&obj, &"output".into(), &output.into()).unwrap();
                js_sys::Reflect::set(&obj, &"debugOutput".into(), &debug_output.into()).unwrap();

                js_sys::Reflect::set(&obj, &"hasMore".into(), &(self.step_position < self.step_tokens.len()).into()).unwrap();
                js_sys::Reflect::set(&obj, &"position".into(), &(self.step_position as u32).into()).unwrap();
                js_sys::Reflect::set(&obj, &"total".into(), &(self.step_tokens.len() as u32).into()).unwrap();
                js_sys::Reflect::set(&obj, &"stack".into(), &self.get_stack()).unwrap();
                js_sys::Reflect::set(&obj, &"customWords".into(), &self.get_custom_words_for_state()).unwrap();
            }
            Err(e) => {
                self.step_mode = false;
                let debug_output = self.interpreter.get_debug_output();

                js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                js_sys::Reflect::set(&obj, &"message".into(), &e.to_string().into()).unwrap();
                js_sys::Reflect::set(&obj, &"debugOutput".into(), &debug_output.into()).unwrap();
                js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
                js_sys::Reflect::set(&obj, &"hasMore".into(), &false.into()).unwrap();
            }
        }

        obj.into()
    }

    #[wasm_bindgen]
