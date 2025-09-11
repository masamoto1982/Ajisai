// rust/src/lib.rs (BigInt対応・数値文字列変換版)

use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;
use crate::types::{Value, ValueType, Fraction, BracketType};
use crate::interpreter::Interpreter;
use num_bigint::BigInt;
use std::str::FromStr;

mod types;
mod tokenizer;
mod interpreter;
mod builtins;

#[wasm_bindgen]
pub struct AjisaiInterpreter {
    interpreter: Interpreter,
    step_tokens: Vec<types::Token>,
    step_position: usize,
}

#[wasm_bindgen]
impl AjisaiInterpreter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        AjisaiInterpreter {
            interpreter: Interpreter::new(),
            step_tokens: Vec::new(),
            step_position: 0,
        }
    }

    #[wasm_bindgen]
    pub fn execute(&mut self, code: &str) -> JsValue {
        let obj = js_sys::Object::new();
        match self.interpreter.execute(code) {
            Ok(()) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                js_sys::Reflect::set(&obj, &"output".into(), &self.interpreter.get_output().into()).unwrap();
                js_sys::Reflect::set(&obj, &"debugOutput".into(), &self.interpreter.get_debug_output().into()).unwrap();
            }
            Err(e) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                js_sys::Reflect::set(&obj, &"message".into(), &e.to_string().into()).unwrap();
                js_sys::Reflect::set(&obj, &"error".into(), &JsValue::from_bool(true)).unwrap();
            }
        }
        obj.into()
    }

    #[wasm_bindgen]
    pub fn reset(&mut self) -> JsValue {
        let obj = js_sys::Object::new();
        match self.interpreter.execute_reset() {
            Ok(()) => {
                self.interpreter = Interpreter::new();
                self.step_tokens.clear();
                self.step_position = 0;
                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                js_sys::Reflect::set(&obj, &"output".into(), &"All memory reset. System reinitialized.".into()).unwrap();
            }
            Err(e) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                js_sys::Reflect::set(&obj, &"message".into(), &e.to_string().into()).unwrap();
                js_sys::Reflect::set(&obj, &"error".into(), &JsValue::from_bool(true)).unwrap();
            }
        }
        obj.into()
    }

    #[wasm_bindgen]
    pub fn init_step(&mut self, code: &str) -> Result<String, String> {
        let tokens = crate::tokenizer::tokenize(code).map_err(|e| format!("Tokenization error: {}", e))?;
        self.step_tokens = tokens;
        self.step_position = 0;
        Ok(format!("Step mode initialized. {} tokens to execute.", self.step_tokens.len()))
    }

    #[wasm_bindgen]
    pub fn step(&mut self) -> JsValue {
        let result_obj = js_sys::Object::new();
        if self.step_position >= self.step_tokens.len() {
            js_sys::Reflect::set(&result_obj, &"hasMore".into(), &JsValue::from_bool(false)).unwrap();
            js_sys::Reflect::set(&result_obj, &"output".into(), &"Step execution completed.".into()).unwrap();
            return result_obj.into();
        }
        
        let token = &self.step_tokens[self.step_position];
        match self.interpreter.execute_single_token(token) {
            Ok(output) => {
                self.step_position += 1;
                js_sys::Reflect::set(&result_obj, &"hasMore".into(), &JsValue::from_bool(self.step_position < self.step_tokens.len())).unwrap();
                js_sys::Reflect::set(&result_obj, &"output".into(), &output.into()).unwrap();
                js_sys::Reflect::set(&result_obj, &"position".into(), &JsValue::from_f64(self.step_position as f64)).unwrap();
                js_sys::Reflect::set(&result_obj, &"total".into(), &JsValue::from_f64(self.step_tokens.len() as f64)).unwrap();
            }
            Err(e) => {
                js_sys::Reflect::set(&result_obj, &"hasMore".into(), &JsValue::from_bool(false)).unwrap();
                js_sys::Reflect::set(&result_obj, &"output".into(), &format!("Error: {}", e).into()).unwrap();
                js_sys::Reflect::set(&result_obj, &"error".into(), &JsValue::from_bool(true)).unwrap();
            }
        }
        result_obj.into()
    }

    #[wasm_bindgen]
    pub fn get_workspace(&self) -> JsValue {
        to_value(&self.interpreter.get_workspace().iter().map(value_to_serializable).collect::<Vec<_>>()).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn get_custom_words_info(&self) -> JsValue {
        to_value(&self.interpreter.get_custom_words_info()).unwrap_or(JsValue::NULL)
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
        let workspace_serializable: Vec<SerializableValue> = serde_wasm_bindgen::from_value(workspace_js)
            .map_err(|e| format!("Failed to deserialize workspace: {}", e))?;
        
        let new_workspace = workspace_serializable.into_iter()
            .map(serializable_to_value)
            .collect::<Result<Vec<_>,_>>()?;

        self.interpreter.set_workspace(new_workspace);
        Ok(())
    }

    #[wasm_bindgen]
    pub fn restore_word(&mut self, name: String, definition: String, description: Option<String>) -> Result<(), String> {
        let definition = definition.trim();
        if !definition.starts_with('[') || !definition.ends_with(']') {
            return Err("Invalid word definition format".to_string());
        }
        
        let inner = &definition[1..definition.len()-1].trim();
        let tokens = crate::tokenizer::tokenize(inner)?;
        
        self.interpreter.restore_custom_word(name, tokens, description)
            .map_err(|e| e.to_string())
    }

    // Other functions (get_custom_words, etc.) can be added here if needed
}

// --- Serialization ---

#[derive(serde::Serialize, serde::Deserialize)]
struct SerializableValue {
    #[serde(rename = "type")]
    val_type: String,
    value: serde_json::Value,
    #[serde(rename = "bracketType", skip_serializing_if = "Option::is_none")]
    bracket_type: Option<String>,
}

fn value_to_serializable(value: &Value) -> SerializableValue {
    let (val_type, json_value, bracket_type_str) = match &value.val_type {
        ValueType::Number(f) => ("number", serde_json::json!({
            "numerator": f.numerator.to_string(),
            "denominator": f.denominator.to_string(),
        }), None),
        ValueType::String(s) => ("string", serde_json::json!(s), None),
        ValueType::Boolean(b) => ("boolean", serde_json::json!(b), None),
        ValueType::Symbol(s) => ("symbol", serde_json::json!(s), None),
        ValueType::Vector(v, bt) => ("vector", serde_json::json!(v.iter().map(value_to_serializable).collect::<Vec<_>>()), Some(match bt {
            BracketType::Square => "square",
            BracketType::Curly => "curly",
            BracketType::Round => "round",
        }.to_string())),
        ValueType::Nil => ("nil", serde_json::Value::Null, None),
    };
    SerializableValue { val_type: val_type.to_string(), value: json_value, bracket_type: bracket_type_str }
}

fn serializable_to_value(s_val: SerializableValue) -> Result<Value, String> {
    let val_type = match s_val.val_type.as_str() {
        "number" => {
            let num_str = s_val.value.get("numerator").and_then(|v| v.as_str()).ok_or("Invalid numerator")?;
            let den_str = s_val.value.get("denominator").and_then(|v| v.as_str()).ok_or("Invalid denominator")?;
            let num = BigInt::from_str(num_str).map_err(|e| e.to_string())?;
            let den = BigInt::from_str(den_str).map_err(|e| e.to_string())?;
            ValueType::Number(Fraction::new(num, den))
        },
        "string" => ValueType::String(s_val.value.as_str().ok_or("Invalid string")?.to_string()),
        "boolean" => ValueType::Boolean(s_val.value.as_bool().ok_or("Invalid boolean")?),
        "symbol" => ValueType::Symbol(s_val.value.as_str().ok_or("Invalid symbol")?.to_string()),
        "vector" => {
            let s_vec: Vec<SerializableValue> = serde_json::from_value(s_val.value).map_err(|e| e.to_string())?;
            let vec = s_vec.into_iter().map(serializable_to_value).collect::<Result<_,_>>()?;
            let bt = match s_val.bracket_type.as_deref() {
                Some("curly") => BracketType::Curly,
                Some("round") => BracketType::Round,
                _ => BracketType::Square,
            };
            ValueType::Vector(vec, bt)
        },
        "nil" => ValueType::Nil,
        _ => return Err(format!("Unknown type: {}", s_val.val_type)),
    };
    Ok(Value { val_type })
}
