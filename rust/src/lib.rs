// rust/src/lib.rs

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
}

#[wasm_bindgen]
impl AjisaiInterpreter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        AjisaiInterpreter {
            interpreter: Interpreter::new(),
        }
    }

    #[wasm_bindgen]
    pub fn execute(&mut self, code: &str) -> JsValue {
        let obj = js_sys::Object::new();
        match self.interpreter.execute(code) {
            Ok(()) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                js_sys::Reflect::set(&obj, &"output".into(), &self.interpreter.get_output().into()).unwrap();
            }
            Err(e) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                js_sys::Reflect::set(&obj, &"message".into(), &e.to_string().into()).unwrap();
            }
        }
        obj.into()
    }

    #[wasm_bindgen]
    pub fn reset(&mut self) -> JsValue {
        let obj = js_sys::Object::new();
        match self.interpreter.execute_reset() {
            Ok(()) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                js_sys::Reflect::set(&obj, &"output".into(), &"System reinitialized.".into()).unwrap();
            }
            Err(e) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                js_sys::Reflect::set(&obj, &"message".into(), &e.to_string().into()).unwrap();
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
        let js_array = js_sys::Array::from(&workspace_js);
        let mut workspace = Vec::new();
        for i in 0..js_array.length() {
            workspace.push(js_value_to_value(js_array.get(i))?);
        }
        self.interpreter.set_workspace(workspace);
        Ok(())
    }

    #[wasm_bindgen]
    pub fn init_step(&mut self, code: &str) -> String {
        // ステップ実行の初期化（簡易実装）
        format!("Step mode initialized for: {}", code)
    }

    #[wasm_bindgen]
    pub fn step(&mut self) -> JsValue {
        let obj = js_sys::Object::new();
        // ステップ実行（簡易実装）
        js_sys::Reflect::set(&obj, &"hasMore".into(), &false.into()).unwrap();
        js_sys::Reflect::set(&obj, &"output".into(), &"Step completed".into()).unwrap();
        obj.into()
    }
}

fn js_value_to_value(js_val: JsValue) -> Result<Value, String> {
    let obj = js_sys::Object::from(js_val);
    let type_str = js_sys::Reflect::get(&obj, &"type".into()).map_err(|e| e.as_string().unwrap_or("Unknown error".to_string()))?.as_string().ok_or("Type not string")?;
    let value_js = js_sys::Reflect::get(&obj, &"value".into()).map_err(|e| e.as_string().unwrap_or("Unknown error".to_string()))?;

    let val_type = match type_str.as_str() {
        "number" => {
            let num_obj = js_sys::Object::from(value_js);
            let num_str = js_sys::Reflect::get(&num_obj, &"numerator".into()).map_err(|e| e.as_string().unwrap_or("Unknown error".to_string()))?.as_string().ok_or("Numerator not string")?;
            let den_str = js_sys::Reflect::get(&num_obj, &"denominator".into()).map_err(|e| e.as_string().unwrap_or("Unknown error".to_string()))?.as_string().ok_or("Denominator not string")?;
            ValueType::Number(Fraction::new(BigInt::from_str(&num_str).unwrap(), BigInt::from_str(&den_str).unwrap()))
        },
        "string" => ValueType::String(value_js.as_string().ok_or("Value not string")?),
        "boolean" => ValueType::Boolean(value_js.as_bool().ok_or("Value not boolean")?),
        "symbol" => ValueType::Symbol(value_js.as_string().ok_or("Value not string")?),
        "vector" => {
            let bracket_type_str = js_sys::Reflect::get(&obj, &"bracketType".into()).map_err(|e| e.as_string().unwrap_or("Unknown error".to_string()))?.as_string();
            let bracket_type = match bracket_type_str.as_deref() {
                Some("curly") => BracketType::Curly, Some("round") => BracketType::Round, _ => BracketType::Square,
            };
            let js_array = js_sys::Array::from(&value_js);
            let mut vec = Vec::new();
            for i in 0..js_array.length() { vec.push(js_value_to_value(js_array.get(i))?); }
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
            for elem in vec { js_array.push(&value_to_js_value(elem)); }
            js_sys::Reflect::set(&obj, &"value".into(), &js_array.into()).unwrap();
            let bracket_str = match bracket_type {
                BracketType::Square => "square", BracketType::Curly => "curly", BracketType::Round => "round",
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
