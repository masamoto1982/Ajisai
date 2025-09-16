// rust/src/lib.rs - ビルドエラー修正版

use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;
use crate::types::{Value, ValueType, Fraction};
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
        
        web_sys::console::log_1(&JsValue::from_str(&format!("Executing code: {}", code)));
        
        match self.interpreter.execute(code) {
            Ok(()) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                js_sys::Reflect::set(&obj, &"output".into(), &self.interpreter.get_output().into()).unwrap();
                js_sys::Reflect::set(&obj, &"debugOutput".into(), &self.interpreter.get_debug_output().into()).unwrap();
            }
            Err(e) => {
                web_sys::console::error_1(&JsValue::from_str(&format!("Execution error: {}", e)));
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
    pub fn get_workspace(&self) -> JsValue {
        let js_array = js_sys::Array::new();
        
        for value in self.interpreter.get_workspace() {
            let js_value = value_to_js_value(value);
            js_array.push(&js_value);
        }
        
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Workspace has {} items", 
            js_array.length()
        )));
        
        js_array.into()
    }

    #[wasm_bindgen]
    pub fn init_step(&mut self, code: &str) -> Result<String, String> {
        let tokens = crate::tokenizer::tokenize(code)
            .map_err(|e| format!("Tokenization error: {}", e))?;
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
    
    #[wasm_bindgen]
    pub fn restore_workspace(&mut self, workspace_js: JsValue) -> Result<(), String> {
        let js_array = js_sys::Array::from(&workspace_js);
        let mut workspace = Vec::new();
        
        for i in 0..js_array.length() {
            let js_val = js_array.get(i);
            let value = js_value_to_value(js_val)?;
            workspace.push(value);
        }
        
        self.interpreter.set_workspace(workspace);
        Ok(())
    }
}

fn js_value_to_value(js_val: JsValue) -> Result<Value, String> {
    let obj = js_sys::Object::from(js_val);
    
    let type_str = js_sys::Reflect::get(&obj, &"type".into())
        .map_err(|_| "Missing type field")?
        .as_string()
        .ok_or("type is not a string")?;
    
    let value_js = js_sys::Reflect::get(&obj, &"value".into())
        .map_err(|_| "Missing value field")?;
    
    let val_type = match type_str.as_str() {
        "number" => {
            let num_obj = js_sys::Object::from(value_js);
            let num_str = js_sys::Reflect::get(&num_obj, &"numerator".into())
                .map_err(|_| "Missing numerator")?
                .as_string()
                .ok_or("numerator is not a string")?;
            let den_str = js_sys::Reflect::get(&num_obj, &"denominator".into())
                .map_err(|_| "Missing denominator")?
                .as_string()
                .ok_or("denominator is not a string")?;
            
            let num = BigInt::from_str(&num_str).map_err(|e| e.to_string())?;
            let den = BigInt::from_str(&den_str).map_err(|e| e.to_string())?;
            
            ValueType::Number(Fraction::new(num, den))
        },
        "string" => {
            ValueType::String(value_js.as_string().ok_or("value is not a string")?)
        },
        "boolean" => {
            ValueType::Boolean(value_js.as_bool().ok_or("value is not a boolean")?)
        },
        "symbol" => {
            ValueType::Symbol(value_js.as_string().ok_or("value is not a string")?)
        },
        "vector" => {
            let js_array = js_sys::Array::from(&value_js);
            let mut vec = Vec::new();
            
            for i in 0..js_array.length() {
                let elem = js_value_to_value(js_array.get(i))?;
                vec.push(elem);
            }
            
            ValueType::Vector(vec)
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
        ValueType::Vector(vec) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"vector".into()).unwrap();
            
            let js_array = js_sys::Array::new();
            for elem in vec {
                js_array.push(&value_to_js_value(elem));
            }
            js_sys::Reflect::set(&obj, &"value".into(), &js_array.into()).unwrap();
        },
        ValueType::Nil => {
            js_sys::Reflect::set(&obj, &"type".into(), &"nil".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &JsValue::NULL).unwrap();
        },
        ValueType::ExecutionLine(_) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"execution_line".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &JsValue::NULL).unwrap();
        },
        ValueType::WordDefinition(_) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"word_definition".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &JsValue::NULL).unwrap();
        },
    }
    
    obj.into()
}
