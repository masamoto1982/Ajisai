mod interpreter;
mod tokenizer;
mod types;
mod builtins;

use wasm_bindgen::prelude::*;
use std::panic;

// panicをコンソールに出力するための設定
pub fn set_panic_hook() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[wasm_bindgen]
pub struct AjisaiInterpreter {
    interpreter: interpreter::Interpreter,
}

#[wasm_bindgen]
impl AjisaiInterpreter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        set_panic_hook();
        AjisaiInterpreter {
            interpreter: interpreter::Interpreter::new(),
        }
    }

    #[wasm_bindgen]
    pub fn execute(&mut self, code: &str) -> JsValue {
        self.interpreter.reset_output();
        
        match self.interpreter.execute(code) {
            Ok(_) => {
                let output = self.interpreter.get_output();
                let result = serde_json::json!({
                    "status": "OK",
                    "output": output
                });
                serde_wasm_bindgen::to_value(&result).unwrap()
            },
            Err(e) => {
                let result = serde_json::json!({
                    "status": "ERROR",
                    "message": format!("{:?}", e)
                });
                serde_wasm_bindgen::to_value(&result).unwrap()
            }
        }
    }

    #[wasm_bindgen]
    pub fn init_step(&mut self, code: &str) -> Result<String, JsValue> {
        match self.interpreter.init_step(code) {
            Ok(_) => Ok("OK".to_string()),
            Err(e) => Err(JsValue::from_str(&e.to_string())),
        }
    }

    #[wasm_bindgen]
    pub fn step(&mut self) -> Result<JsValue, JsValue> {
        match self.interpreter.step() {
            Ok((output, has_more)) => {
                let result = serde_json::json!({
                    "hasMore": has_more,
                    "output": output,
                    "position": 0,
                    "total": 0,
                });
                Ok(serde_wasm_bindgen::to_value(&result).unwrap())
            },
            Err(e) => Err(JsValue::from_str(&format!("{:?}", e))),
        }
    }

    #[wasm_bindgen]
    pub fn get_stack(&self) -> JsValue {
        let stack: Vec<_> = self.interpreter.stack.iter()
            .map(|val| {
                let js_val = match &val.val_type {
                    types::ValueType::Number(n) => {
                        serde_json::json!({
                            "type": "number",
                            "value": {
                                "numerator": n.numerator,
                                "denominator": n.denominator
                            }
                        })
                    },
                    types::ValueType::String(s) => {
                        serde_json::json!({
                            "type": "string",
                            "value": s
                        })
                    },
                    types::ValueType::Boolean(b) => {
                        serde_json::json!({
                            "type": "boolean",
                            "value": b
                        })
                    },
                    types::ValueType::Symbol(s) => {
                        serde_json::json!({
                            "type": "symbol",
                            "value": s
                        })
                    },
                    types::ValueType::Vector(v) => {
                        serde_json::json!({
                            "type": "vector",
                            "value": v
                        })
                    },
                    types::ValueType::Quotation(_) => {
                        serde_json::json!({
                            "type": "quotation",
                            "value": "{ ... }"
                        })
                    },
                    types::ValueType::Nil => {
                        serde_json::json!({
                            "type": "nil",
                            "value": null
                        })
                    },
                };
                js_val
            })
            .collect();
        
        serde_wasm_bindgen::to_value(&stack).unwrap()
    }

    #[wasm_bindgen]
    pub fn get_register(&self) -> JsValue {
        match &self.interpreter.register {
            Some(val) => {
                let js_val = match &val.val_type {
                    types::ValueType::Number(n) => {
                        serde_json::json!({
                            "type": "number",
                            "value": {
                                "numerator": n.numerator,
                                "denominator": n.denominator
                            }
                        })
                    },
                    types::ValueType::String(s) => {
                        serde_json::json!({
                            "type": "string",
                            "value": s
                        })
                    },
                    types::ValueType::Boolean(b) => {
                        serde_json::json!({
                            "type": "boolean",
                            "value": b
                        })
                    },
                    types::ValueType::Symbol(s) => {
                        serde_json::json!({
                            "type": "symbol",
                            "value": s
                        })
                    },
                    types::ValueType::Vector(v) => {
                        serde_json::json!({
                            "type": "vector",
                            "value": v
                        })
                    },
                    types::ValueType::Quotation(_) => {
                        serde_json::json!({
                            "type": "quotation",
                            "value": "{ ... }"
                        })
                    },
                    types::ValueType::Nil => {
                        serde_json::json!({
                            "type": "nil",
                            "value": null
                        })
                    },
                };
                serde_wasm_bindgen::to_value(&js_val).unwrap()
            },
            None => JsValue::NULL,
        }
    }

    #[wasm_bindgen]
    pub fn get_custom_words(&self) -> js_sys::Array {
        let words = js_sys::Array::new();
        
        for (name, def) in &self.interpreter.dictionary {
            if !def.is_builtin {
                words.push(&JsValue::from_str(name));
            }
        }
        
        words
    }

    #[wasm_bindgen]
    pub fn get_custom_words_with_descriptions(&self) -> JsValue {
        let mut words = Vec::new();
        
        for (name, def) in &self.interpreter.dictionary {
            if !def.is_builtin {
                let word_info = serde_json::json!({
                    "name": name,
                    "description": def.description,
                });
                words.push(word_info);
            }
        }
        
        serde_wasm_bindgen::to_value(&words).unwrap()
    }

    #[wasm_bindgen]
    pub fn get_custom_words_info(&self) -> JsValue {
        let words = self.interpreter.get_custom_words_info();
        serde_wasm_bindgen::to_value(&words).unwrap()
    }

    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.interpreter = interpreter::Interpreter::new();
    }

    #[wasm_bindgen]
    pub fn save_table(&mut self, name: &str, schema: JsValue, records: JsValue) -> Result<(), JsValue> {
        Err(JsValue::from_str("Table feature is not implemented"))
    }

    #[wasm_bindgen]
    pub fn load_table(&self, name: &str) -> JsValue {
        JsValue::NULL
    }

    #[wasm_bindgen]
    pub fn get_all_tables(&self) -> js_sys::Array {
        js_sys::Array::new()
    }

    #[wasm_bindgen]
    pub fn restore_stack(&mut self, stack_js: JsValue) -> Result<(), JsValue> {
        let stack_data: Vec<serde_json::Value> = serde_wasm_bindgen::from_value(stack_js)?;
        self.interpreter.stack.clear();
        
        for item in stack_data {
            if let Some(val) = Self::js_value_to_value(&item) {
                self.interpreter.stack.push(val);
            }
        }
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn restore_register(&mut self, register_js: JsValue) -> Result<(), JsValue> {
        if register_js.is_null() || register_js.is_undefined() {
            self.interpreter.register = None;
        } else {
            let reg_data: serde_json::Value = serde_wasm_bindgen::from_value(register_js)?;
            self.interpreter.register = Self::js_value_to_value(&reg_data);
        }
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn get_word_definition(&self, name: &str) -> JsValue {
        if let Some(tokens) = self.interpreter.get_word_definition(name) {
            let tokens_str: Vec<String> = tokens.iter().map(|t| format!("{:?}", t)).collect();
            JsValue::from_str(&tokens_str.join(" "))
        } else {
            JsValue::NULL
        }
    }

    #[wasm_bindgen]
    pub fn restore_word(&mut self, name: &str, definition: &str, description: Option<String>) -> Result<(), JsValue> {
        // 簡易的な実装：定義文字列からトークンを復元
        match tokenizer::tokenize(definition) {
            Ok(tokens) => {
                self.interpreter.restore_word(name.to_string(), tokens, description);
                Ok(())
            },
            Err(e) => Err(JsValue::from_str(&e)),
        }
    }

    // ヘルパー関数
    fn js_value_to_value(js_val: &serde_json::Value) -> Option<types::Value> {
        let obj = js_val.as_object()?;
        let type_str = obj.get("type")?.as_str()?;
        
        match type_str {
            "number" => {
                let value = obj.get("value")?;
                if let Some(value_obj) = value.as_object() {
                    let numerator = value_obj.get("numerator")?.as_i64()?;
                    let denominator = value_obj.get("denominator")?.as_i64()?;
                    Some(types::Value {
                        val_type: types::ValueType::Number(types::Fraction::new(numerator, denominator))
                    })
                } else {
                    None
                }
            },
            "string" => {
                let value = obj.get("value")?.as_str()?;
                Some(types::Value {
                    val_type: types::ValueType::String(value.to_string())
                })
            },
            "boolean" => {
                let value = obj.get("value")?.as_bool()?;
                Some(types::Value {
                    val_type: types::ValueType::Boolean(value)
                })
            },
            "symbol" => {
                let value = obj.get("value")?.as_str()?;
                Some(types::Value {
                    val_type: types::ValueType::Symbol(value.to_string())
                })
            },
            "nil" => {
                Some(types::Value {
                    val_type: types::ValueType::Nil
                })
            },
            _ => None,
        }
    }
}
