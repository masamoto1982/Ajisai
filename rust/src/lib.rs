// rust/src/lib.rs

use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;
use crate::types::{Value, ValueType, Fraction, BracketType, Token};
use crate::interpreter::Interpreter;
use num_bigint::BigInt;
use std::str::FromStr;

mod types;
mod tokenizer;
mod interpreter;
mod builtins;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    
    #[wasm_bindgen(js_name = "setTimeout")]
    fn set_timeout(closure: &Closure<dyn FnMut()>, time: u32) -> u32;
    
    #[wasm_bindgen(js_name = "performance")]
    type Performance;
    
    #[wasm_bindgen(method, js_name = "now")]
    fn now(this: &Performance) -> f64;
    
    #[wasm_bindgen(js_name = "performance")]
    static performance: Performance;
}

// 同期的な遅延のための実装（1秒制限）
pub fn wasm_sleep(ms: u64) -> String {
    const MAX_SAFE_DELAY_MS: u64 = 1000; // 1秒まで
    
    if ms > MAX_SAFE_DELAY_MS {
        return format!("[ERROR] Delay {}ms exceeds maximum allowed delay ({}ms). Execution aborted.", ms, MAX_SAFE_DELAY_MS);
    }
    
    let start = performance.now();
    let target = start + ms as f64;
    
    // 1秒以下の短時間ビジーウェイト
    while performance.now() < target {
        // 空のループで時間を消費
    }
    
    format!("[DEBUG] Actually waited {}ms", ms)
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
    pub fn execute(&mut self, code: &str) -> JsValue {
        // JavaScriptのconsoleにもログを出力
        web_sys::console::log_1(&format!("Executing code: {:?}", code).into());
        
        let obj = js_sys::Object::new();
        match self.interpreter.execute(code) {
            Ok(()) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                let output = self.interpreter.get_output();
                js_sys::Reflect::set(&obj, &"output".into(), &output.clone().into()).unwrap();
                
                // JavaScriptのconsoleにも出力
                web_sys::console::log_1(&format!("Execution output: {}", output).into());
                
                // デバッグ出力を追加
                if !output.is_empty() {
                    js_sys::Reflect::set(&obj, &"debugOutput".into(), &"Executed successfully".into()).unwrap();
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                web_sys::console::log_1(&format!("Execution error: {}", error_msg).into());
                
                js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                js_sys::Reflect::set(&obj, &"message".into(), &error_msg.into()).unwrap();
                js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
            }
        }
        obj.into()
    }

    // ... 他のメソッドは同じ
    #[wasm_bindgen]
    pub fn execute_step(&mut self, code: &str) -> JsValue {
        let obj = js_sys::Object::new();
        
        // 新しいコードが来た場合または初回実行の場合は初期化
        if !self.step_mode || code != self.current_step_code {
            self.step_mode = true;
            self.step_position = 0;
            self.current_step_code = code.to_string();
            
            let custom_word_names: std::collections::HashSet<String> = self.interpreter.dictionary.iter()
                .filter(|(_, def)| !def.is_builtin)
                .map(|(name, _)| name.clone())
                .collect();
                
            match crate::tokenizer::tokenize_with_custom_words(code, &custom_word_names) {
                Ok(tokens) => {
                    self.step_tokens = tokens;
                }
                Err(e) => {
                    self.step_mode = false;
                    js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                    js_sys::Reflect::set(&obj, &"message".into(), &format!("Tokenization error: {}", e).into()).unwrap();
                    js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
                    return obj.into();
                }
            }
        }

        // ステップ実行完了チェック
        if self.step_position >= self.step_tokens.len() {
            self.step_mode = false;
            js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
            js_sys::Reflect::set(&obj, &"output".into(), &"Step execution completed".into()).unwrap();
            js_sys::Reflect::set(&obj, &"hasMore".into(), &false.into()).unwrap();
            js_sys::Reflect::set(&obj, &"debugOutput".into(), &"All steps completed".into()).unwrap();
            return obj.into();
        }

        // 1ステップ実行
        let token = self.step_tokens[self.step_position].clone();
        let result = self.interpreter.execute_tokens(&[token]);
        
        match result {
            Ok(()) => {
                let output = self.interpreter.get_output();
                self.step_position += 1;
                
                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                js_sys::Reflect::set(&obj, &"output".into(), &output.into()).unwrap();
                js_sys::Reflect::set(&obj, &"hasMore".into(), &(self.step_position < self.step_tokens.len()).into()).unwrap();
                js_sys::Reflect::set(&obj, &"position".into(), &(self.step_position as u32).into()).unwrap();
                js_sys::Reflect::set(&obj, &"total".into(), &(self.step_tokens.len() as u32).into()).unwrap();
                
                // 進捗メッセージ
                let progress_msg = format!("Step {}/{} completed", self.step_position, self.step_tokens.len());
                js_sys::Reflect::set(&obj, &"debugOutput".into(), &progress_msg.into()).unwrap();
            }
            Err(e) => {
                self.step_mode = false;
                js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                js_sys::Reflect::set(&obj, &"message".into(), &e.to_string().into()).unwrap();
                js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
                js_sys::Reflect::set(&obj, &"hasMore".into(), &false.into()).unwrap();
            }
        }
        
        obj.into()
    }

    #[wasm_bindgen]
    pub fn init_step(&mut self, code: &str) -> String {
        self.step_mode = true;
        self.step_position = 0;
        self.current_step_code = code.to_string();
        
        // トークン化
        let custom_word_names: std::collections::HashSet<String> = self.interpreter.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect();
            
        match crate::tokenizer::tokenize_with_custom_words(code, &custom_word_names) {
            Ok(tokens) => {
                self.step_tokens = tokens;
                format!("Step mode initialized. {} tokens to execute.", self.step_tokens.len())
            }
            Err(e) => {
                self.step_mode = false;
                format!("Error initializing step mode: {}", e)
            }
        }
    }

    #[wasm_bindgen]
    pub fn step(&mut self) -> JsValue {
        let obj = js_sys::Object::new();
        
        if !self.step_mode {
            js_sys::Reflect::set(&obj, &"hasMore".into(), &false.into()).unwrap();
            js_sys::Reflect::set(&obj, &"output".into(), &"Step mode not initialized".into()).unwrap();
            js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
            return obj.into();
        }

        if self.step_position >= self.step_tokens.len() {
            self.step_mode = false;
            js_sys::Reflect::set(&obj, &"hasMore".into(), &false.into()).unwrap();
            js_sys::Reflect::set(&obj, &"output".into(), &"Step execution completed".into()).unwrap();
            return obj.into();
        }

        // 1つのトークンを実行
        let token = self.step_tokens[self.step_position].clone();
        let result = self.interpreter.execute_tokens(&[token]);
        
        match result {
            Ok(()) => {
                let output = self.interpreter.get_output();
                self.step_position += 1;
                
                js_sys::Reflect::set(&obj, &"hasMore".into(), &(self.step_position < self.step_tokens.len()).into()).unwrap();
                js_sys::Reflect::set(&obj, &"output".into(), &output.into()).unwrap();
                js_sys::Reflect::set(&obj, &"position".into(), &(self.step_position as u32).into()).unwrap();
                js_sys::Reflect::set(&obj, &"total".into(), &(self.step_tokens.len() as u32).into()).unwrap();
            }
            Err(e) => {
                self.step_mode = false;
                js_sys::Reflect::set(&obj, &"hasMore".into(), &false.into()).unwrap();
                js_sys::Reflect::set(&obj, &"output".into(), &e.to_string().into()).unwrap();
                js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
            }
        }
        
        obj.into()
    }

    #[wasm_bindgen]
    pub fn reset(&mut self) -> JsValue {
        let obj = js_sys::Object::new();
        
        // ステップモードをリセット
        self.step_mode = false;
        self.step_tokens.clear();
        self.step_position = 0;
        self.current_step_code.clear();
        
        match self.interpreter.execute_reset() {
            Ok(()) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                js_sys::Reflect::set(&obj, &"output".into(), &"System reinitialized.".into()).unwrap();
                
                // RESETイベントを発火してデータベースクリア
                let window = web_sys::window().unwrap();
                let event = web_sys::CustomEvent::new("ajisai-reset").unwrap();
                let _ = window.dispatch_event(&event);
            }
            Err(e) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                js_sys::Reflect::set(&obj, &"message".into(), &e.to_string().into()).unwrap();
                js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
            }
        }
        obj.into()
    }
    
    #[wasm_bindgen]
    pub fn get_workspace(&self) -> JsValue {
        let js_array = js_sys::Array::new();
        for value in self.interpreter.get_workspace() {
            js_array.push(&value_to_js_value(value));
        }
        js_array.into()
    }

    #[wasm_bindgen]
    pub fn get_custom_words_info(&self) -> JsValue {
        let words_info: Vec<(String, Option<String>, bool)> = self.interpreter.get_custom_words_info()
            .into_iter()
            .map(|(name, description)| (name, description, false)) // protectedフラグを追加
            .collect();
        to_value(&words_info).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn get_builtin_words_info(&self) -> JsValue {
        to_value(&crate::builtins::get_builtin_definitions()).unwrap_or(JsValue::NULL)
    }
    
    #[wasm_bindgen]
    pub fn get_word_definition(&self, name: &str) -> JsValue {
        match self.interpreter.get_word_definition(name) {
            Some(def) => JsValue::from_str(&def),
            None => JsValue::NULL,
        }
    }
    
    #[wasm_bindgen]
    pub fn restore_workspace(&mut self, workspace_js: JsValue) -> Result<(), String> {
        let js_array = js_sys::Array::from(&workspace_js);
        let mut workspace = Vec::new();
        for i in 0..js_array.length() {
            workspace.push(js_value_to_value(js_array.get(i))?);
        }
        self.interpreter.set_workspace(workspace);
        Ok(())
    }

    #[wasm_bindgen]
    pub fn restore_word(&mut self, name: String, definition: String, description: Option<String>) -> Result<(), String> {
        // カスタムワードの復元（簡易実装）
        let custom_word_names: std::collections::HashSet<String> = self.interpreter.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect();
            
        let tokens = crate::tokenizer::tokenize_with_custom_words(&definition, &custom_word_names)
            .map_err(|e| format!("Failed to tokenize word definition: {}", e))?;
            
        self.interpreter.restore_custom_word(name, tokens, description)
            .map_err(|e| format!("Failed to restore word: {}", e))
    }
}

fn js_value_to_value(js_val: JsValue) -> Result<Value, String> {
    let obj = js_sys::Object::from(js_val);
    let type_str = js_sys::Reflect::get(&obj, &"type".into())
        .map_err(|e| e.as_string().unwrap_or("Unknown error".to_string()))?
        .as_string().ok_or("Type not string")?;
    let value_js = js_sys::Reflect::get(&obj, &"value".into())
        .map_err(|e| e.as_string().unwrap_or("Unknown error".to_string()))?;

    let val_type = match type_str.as_str() {
        "number" => {
            let num_obj = js_sys::Object::from(value_js);
            let num_str = js_sys::Reflect::get(&num_obj, &"numerator".into())
                .map_err(|e| e.as_string().unwrap_or("Unknown error".to_string()))?
                .as_string().ok_or("Numerator not string")?;
            let den_str = js_sys::Reflect::get(&num_obj, &"denominator".into())
                .map_err(|e| e.as_string().unwrap_or("Unknown error".to_string()))?
                .as_string().ok_or("Denominator not string")?;
            ValueType::Number(Fraction::new(
                BigInt::from_str(&num_str).unwrap(), 
                BigInt::from_str(&den_str).unwrap()
            ))
        },
        "string" => ValueType::String(value_js.as_string().ok_or("Value not string")?),
        "boolean" => ValueType::Boolean(value_js.as_bool().ok_or("Value not boolean")?),
        "symbol" => ValueType::Symbol(value_js.as_string().ok_or("Value not string")?),
        "vector" => {
            let bracket_type_str = js_sys::Reflect::get(&obj, &"bracketType".into())
                .map_err(|e| e.as_string().unwrap_or("Unknown error".to_string()))?
                .as_string();
            let bracket_type = match bracket_type_str.as_deref() {
                Some("curly") => BracketType::Curly,
                Some("round") => BracketType::Round,
                _ => BracketType::Square,
            };
            let js_array = js_sys::Array::from(&value_js);
            let mut vec = Vec::new();
            for i in 0..js_array.length() {
                vec.push(js_value_to_value(js_array.get(i))?);
            }
            ValueType::Vector(vec, bracket_type)
        },
        "nil" => ValueType::Nil,
        _ => return Err(format!("Unknown type: {}", type_str)),
    };
    Ok(Value { val_type })
}

fn value_to_js_value(value: &Value) -> JsValue {
    let obj = js_sys::Object::new();
    match &value.val_type {
        ValueType::Number(frac) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"number".into()).unwrap();
            let frac_obj = js_sys::Object::new();
            js_sys::Reflect::set(&frac_obj, &"numerator".into(), &frac.numerator.to_string().into()).unwrap();
            js_sys::Reflect::set(&frac_obj, &"denominator".into(), &frac.denominator.to_string().into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &frac_obj.into()).unwrap();
        },
        ValueType::String(s) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"string".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &s.clone().into()).unwrap();
        },
        ValueType::Boolean(b) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"boolean".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &(*b).into()).unwrap();
        },
        ValueType::Symbol(s) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"symbol".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &s.clone().into()).unwrap();
        },
        ValueType::Vector(vec, bracket_type) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"vector".into()).unwrap();
            let js_array = js_sys::Array::new();
            for elem in vec {
                js_array.push(&value_to_js_value(elem));
            }
            js_sys::Reflect::set(&obj, &"value".into(), &js_array.into()).unwrap();
            let bracket_str = match bracket_type {
                BracketType::Square => "square",
                BracketType::Curly => "curly",
                BracketType::Round => "round",
            };
            js_sys::Reflect::set(&obj, &"bracketType".into(), &bracket_str.into()).unwrap();
        },
        ValueType::DefinitionBody(_) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"definition".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &":...;".into()).unwrap();
        },
        ValueType::Nil => {
            js_sys::Reflect::set(&obj, &"type".into(), &"nil".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &JsValue::NULL).unwrap();
        },
    }
    obj.into()
}
