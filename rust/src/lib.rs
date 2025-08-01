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
                
                // 出力を取得
                let output = self.interpreter.get_output();
                js_sys::Reflect::set(&obj, &"output".into(), &output.into()).unwrap();
                
                // 自動命名フラグと名前を追加
                let auto_named = self.interpreter.was_auto_named();
                js_sys::Reflect::set(&obj, &"autoNamed".into(), &JsValue::from_bool(auto_named)).unwrap();
                
                if auto_named {
                    if let Some(name) = self.interpreter.get_last_auto_named_word() {
                        js_sys::Reflect::set(&obj, &"autoNamedWord".into(), &JsValue::from_str(&name)).unwrap();
                    }
                }
                
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
                
                // 出力を取得
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
    // DEFコマンドを使わず、直接辞書に登録する
    
    // 定義文字列（"{ ... }"形式）からトークンを抽出
    let definition = definition.trim();
    if !definition.starts_with('{') || !definition.ends_with('}') {
        return Err("Invalid word definition format".to_string());
    }
    
    // 中身のトークンを取得
    let inner = &definition[1..definition.len()-1].trim();
    let tokens = crate::tokenizer::tokenize(inner)
        .map_err(|e| format!("Failed to tokenize definition: {}", e))?;
    
    // 直接辞書に登録
    self.interpreter.restore_custom_word(name, tokens, description)
        .map_err(|e| e.to_string())
}
}

fn value_to_js(value: &Value) -> JsValue {
    let obj = js_sys::Object::new();
    
    let type_str = match &value.val_type {
        ValueType::Number(_) => "number",
        ValueType::String(_) => "string",
        ValueType::Boolean(_) => "boolean",
        ValueType::Symbol(_) => "symbol",
        ValueType::Vector(_) => "vector",
        ValueType::Quotation(_) => "quotation",
        ValueType::Nil => "nil",
    };
    
    js_sys::Reflect::set(&obj, &"type".into(), &type_str.into()).unwrap();
    
    let val = match &value.val_type {
        ValueType::Number(n) => {
            // 分数オブジェクトとして出力
            let frac_obj = js_sys::Object::new();
            js_sys::Reflect::set(&frac_obj, &"numerator".into(), &JsValue::from_f64(n.numerator as f64)).unwrap();
            js_sys::Reflect::set(&frac_obj, &"denominator".into(), &JsValue::from_f64(n.denominator as f64)).unwrap();
            frac_obj.into()
        },
        ValueType::String(s) => JsValue::from_str(s),
        ValueType::Boolean(b) => JsValue::from_bool(*b),
        ValueType::Symbol(s) => JsValue::from_str(s),
        ValueType::Vector(v) => {
            let arr = js_sys::Array::new();
            for item in v.iter() {
                arr.push(&value_to_js(item));
            }
            arr.into()
        },
        ValueType::Quotation(tokens) => {
            // Quotationをオブジェクトとして表現
            let quot_obj = js_sys::Object::new();
            js_sys::Reflect::set(&quot_obj, &"type".into(), &"quotation".into()).unwrap();
            js_sys::Reflect::set(&quot_obj, &"length".into(), &JsValue::from_f64(tokens.len() as f64)).unwrap();
            quot_obj.into()
        },
        ValueType::Nil => JsValue::NULL,
    };
    
    js_sys::Reflect::set(&obj, &"value".into(), &val).unwrap();
    
    obj.into()
}

fn js_value_to_rust_value(js_val: &JsValue) -> Result<Value, String> {
    // JavaScriptオブジェクトの場合（{type: ..., value: ...}形式）
    if js_sys::Reflect::has(js_val, &"type".into()).unwrap_or(false) {
        let type_str = js_sys::Reflect::get(js_val, &"type".into())
            .ok()
            .and_then(|v| v.as_string())
            .ok_or("Invalid type field")?;
        
        let value_field = js_sys::Reflect::get(js_val, &"value".into())
            .map_err(|_| "Missing value field")?;
        
        match type_str.as_str() {
            "number" => {
                // valueフィールドは分数を表すオブジェクト
                if js_sys::Reflect::has(&value_field, &"numerator".into()).unwrap_or(false) &&
                   js_sys::Reflect::has(&value_field, &"denominator".into()).unwrap_or(false) {
                    
                    let num = js_sys::Reflect::get(&value_field, &"numerator".into())
                        .ok()
                        .and_then(|v| v.as_f64())
                        .and_then(|n| {
                            if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
                                Some(n as i64)
                            } else {
                                None
                            }
                        })
                        .ok_or("Invalid numerator: must be an integer within i64 range")?;
                    
                    let den = js_sys::Reflect::get(&value_field, &"denominator".into())
                        .ok()
                        .and_then(|v| v.as_f64())
                        .and_then(|n| {
                            if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
                                Some(n as i64)
                            } else {
                                None
                            }
                        })
                        .ok_or("Invalid denominator: must be an integer within i64 range")?;
                    
                    if den == 0 {
                        return Err("Division by zero in fraction".to_string());
                    }
                    
                    Ok(Value {
                        val_type: ValueType::Number(Fraction::new(num, den))
                    })
                } else {
                    Err("Number value must be an object with 'numerator' and 'denominator' fields".to_string())
                }
            },
            "string" => {
                let s = value_field.as_string()
                    .ok_or("Invalid string value")?;
                Ok(Value {
                    val_type: ValueType::String(s)
                })
            },
            "boolean" => {
                let b = value_field.as_bool()
                    .ok_or("Invalid boolean value")?;
                Ok(Value {
                    val_type: ValueType::Boolean(b)
                })
            },
            "symbol" => {
                let s = value_field.as_string()
                    .ok_or("Invalid symbol value")?;
                Ok(Value {
                    val_type: ValueType::Symbol(s)
                })
            },
            "vector" => {
                if value_field.is_array() {
                    let arr = js_sys::Array::from(&value_field);
                    let mut values = Vec::new();
                    for i in 0..arr.length() {
                        let elem = arr.get(i);
                        values.push(js_value_to_rust_value(&elem)?);
                    }
                    Ok(Value {
                        val_type: ValueType::Vector(values)
                    })
                } else {
                    Err("Invalid vector value".to_string())
                }
            },
            "nil" => {
                Ok(Value {
                    val_type: ValueType::Nil
                })
            },
            _ => Err(format!("Unknown type: {}", type_str)),
        }
    } else {
        // 単純な値の場合（後方互換性のため）
        if let Some(b) = js_val.as_bool() {
            Ok(Value {
                val_type: ValueType::Boolean(b)
            })
        } else if js_val.as_f64().is_some() {
            // 浮動小数点数は受け付けない
            Err("Direct numeric values are not allowed to preserve precision. Numbers must be passed as objects with 'numerator' and 'denominator' fields.".to_string())
        } else if let Some(s) = js_val.as_string() {
            Ok(Value {
                val_type: ValueType::String(s)
            })
        } else if js_val.is_null() || js_val.is_undefined() {
            Ok(Value {
                val_type: ValueType::Nil
            })
        } else if js_val.is_array() {
            // 配列の場合はVectorとして処理
            let arr = js_sys::Array::from(js_val);
            let mut values = Vec::new();
            for i in 0..arr.length() {
                let elem = arr.get(i);
                values.push(js_value_to_rust_value(&elem)?);
            }
            Ok(Value {
                val_type: ValueType::Vector(values)
            })
        } else {
            Err("Unsupported value type".to_string())
        }
    }
}
