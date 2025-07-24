use wasm_bindgen::prelude::*;

mod types;
mod tokenizer;
mod interpreter;
mod builtins;

use types::*;
use interpreter::Interpreter;

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
    pub fn execute(&mut self, code: &str) -> Result<JsValue, String> {
        match self.interpreter.execute(code) {
            Ok(()) => {
                let obj = js_sys::Object::new();
                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                
                let output = self.interpreter.get_output();
                js_sys::Reflect::set(&obj, &"output".into(), &output.into()).unwrap();
                
                Ok(obj.into())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    #[wasm_bindgen]
    pub fn init_step(&mut self, code: &str) -> Result<String, String> {
        match self.interpreter.init_step_execution(code) {
            Ok(()) => Ok("OK".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    #[wasm_bindgen]
    pub fn step(&mut self) -> Result<JsValue, String> {
        match self.interpreter.execute_step() {
            Ok(has_more) => {
                let obj = js_sys::Object::new();
                js_sys::Reflect::set(&obj, &"hasMore".into(), &JsValue::from_bool(has_more)).unwrap();
                
                if let Some((position, total)) = self.interpreter.get_step_info() {
                    js_sys::Reflect::set(&obj, &"position".into(), &JsValue::from_f64(position as f64)).unwrap();
                    js_sys::Reflect::set(&obj, &"total".into(), &JsValue::from_f64(total as f64)).unwrap();
                }
                
                let output = self.interpreter.get_output();
                js_sys::Reflect::set(&obj, &"output".into(), &output.into()).unwrap();
                
                Ok(obj.into())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    #[wasm_bindgen]
    pub fn get_stack(&self) -> JsValue {
        let stack_values: Vec<JsValue> = self.interpreter
            .get_stack()
            .iter()
            .map(|v| value_to_js(v))
            .collect();
        
        let arr = js_sys::Array::new();
        for val in stack_values {
            arr.push(&val);
        }
        arr.into()
    }

    #[wasm_bindgen]
    pub fn get_register(&self) -> JsValue {
        match self.interpreter.get_register() {
            Some(v) => value_to_js(v),
            None => JsValue::NULL,
        }
    }

    #[wasm_bindgen]
    pub fn get_custom_words(&self) -> Vec<String> {
        self.interpreter.get_custom_words()
    }

    #[wasm_bindgen]
    pub fn get_custom_words_with_descriptions(&self) -> JsValue {
        let words = self.interpreter.get_custom_words_with_descriptions();
        let arr = js_sys::Array::new();
        
        for (name, desc) in words {
            let word_arr = js_sys::Array::new();
            word_arr.push(&JsValue::from_str(&name));
            word_arr.push(&desc.map(|d| JsValue::from_str(&d)).unwrap_or(JsValue::NULL));
            arr.push(&word_arr);
        }
        
        arr.into()
    }

    #[wasm_bindgen]
    pub fn get_custom_words_info(&self) -> JsValue {
        let words_info = self.interpreter.get_custom_words_info();
        let arr = js_sys::Array::new();
        
        for (name, desc, protected) in words_info {
            let word_arr = js_sys::Array::new();
            word_arr.push(&JsValue::from_str(&name));
            word_arr.push(&desc.map(|d| JsValue::from_str(&d)).unwrap_or(JsValue::NULL));
            word_arr.push(&JsValue::from_bool(protected));
            arr.push(&word_arr);
        }
        
        arr.into()
    }

    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.interpreter = Interpreter::new();
    }
    
    #[wasm_bindgen]
    pub fn restore_stack(&mut self, stack_js: JsValue) -> Result<(), String> {
        if !stack_js.is_array() {
            return Err("Stack must be an array".to_string());
        }
        
        let arr = js_sys::Array::from(&stack_js);
        let mut new_stack = Vec::new();
        
        for i in 0..arr.length() {
            let item = arr.get(i);
            let value = js_value_to_rust_value(&item)?;
            new_stack.push(value);
        }
        
        self.interpreter.set_stack(new_stack);
        Ok(())
    }
    
    #[wasm_bindgen]
    pub fn restore_register(&mut self, register_js: JsValue) -> Result<(), String> {
        if register_js.is_null() || register_js.is_undefined() {
            self.interpreter.set_register(None);
        } else {
            let value = js_value_to_rust_value(&register_js)?;
            self.interpreter.set_register(Some(value));
        }
        Ok(())
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
        let code = if let Some(desc) = description {
            format!("({}) {} \"{}\" DEF", desc, definition, name)
        } else {
            format!("{} \"{}\" DEF", definition, name)
        };
        
        web_sys::console::log_1(&format!("Restoring word with code: {}", code).into());
        
        self.interpreter.execute(&code).map_err(|e| e.to_string())?;
        Ok(())
    }
}

// 以下のvalue_to_js, js_value_to_rust_value関数は変更なし
