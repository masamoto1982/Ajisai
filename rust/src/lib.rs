// rust/src/lib.rs (ParenComment対応完全版 + シンタックスハイライト対応)

use wasm_bindgen::prelude::*;

mod types;
mod tokenizer;
mod interpreter;
mod builtins;

use types::*;
use interpreter::Interpreter;
use tokenizer::TokenWithPosition;

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
                
                let output = self.interpreter.get_output();
                js_sys::Reflect::set(&obj, &"output".into(), &output.into()).unwrap();
                
                js_sys::Reflect::set(&obj, &"autoNamed".into(), &JsValue::from_bool(false)).unwrap();
            }
            Err(e) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                js_sys::Reflect::set(&obj, &"message".into(), &JsValue::from_str(&e.to_string())).unwrap();
                js_sys::Reflect::set(&obj, &"error".into(), &JsValue::from_bool(true)).unwrap();
            }
        }
        
        obj.into()
    }

    #[wasm_bindgen]
    pub fn amnesia(&mut self) -> JsValue {
        let obj = js_sys::Object::new();
        
        match self.interpreter.execute_amnesia() {
            Ok(()) => {
                // インタープリターもリセット
                self.interpreter = Interpreter::new();
                self.step_tokens.clear();
                self.step_position = 0;
                
                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                js_sys::Reflect::set(&obj, &"output".into(), &"All memory cleared. System reset.".into()).unwrap();
            }
            Err(e) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                js_sys::Reflect::set(&obj, &"message".into(), &JsValue::from_str(&e.to_string())).unwrap();
                js_sys::Reflect::set(&obj, &"error".into(), &JsValue::from_bool(true)).unwrap();
            }
        }
        
        obj.into()
    }

    #[wasm_bindgen]
    pub fn tokenize_with_positions(&self, code: &str) -> JsValue {
        let custom_words: std::collections::HashSet<String> = self.interpreter.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect();
            
        match crate::tokenizer::tokenize_with_positions_and_custom_words(code, &custom_words) {
            Ok(tokens) => {
                let arr = js_sys::Array::new();
                for token_with_pos in tokens {
                    let obj = js_sys::Object::new();
                    
                    // トークンの種類
                    let token_type = match &token_with_pos.token {
                        crate::types::Token::Number(_, _) => "number",
                        crate::types::Token::String(_) => "string",
                        crate::types::Token::Boolean(_) => "boolean",
                        crate::types::Token::Symbol(_) => "symbol",
                        crate::types::Token::VectorStart | crate::types::Token::VectorEnd => "vector-delimiter",
                        crate::types::Token::Nil => "nil",
                        crate::types::Token::ParenComment(_) => "comment",
                    };
                    
                    // トークンの値
                    let token_value = match &token_with_pos.token {
                        crate::types::Token::Number(n, d) => {
                            if *d == 1 {
                                n.to_string()
                            } else {
                                format!("{}/{}", n, d)
                            }
                        },
                        crate::types::Token::String(s) => format!("\"{}\"", s),
                        crate::types::Token::Boolean(b) => b.to_string(),
                        crate::types::Token::Symbol(s) => s.clone(),
                        crate::types::Token::VectorStart => "[".to_string(),
                        crate::types::Token::VectorEnd => "]".to_string(),
                        crate::types::Token::Nil => "nil".to_string(),
                        crate::types::Token::ParenComment(c) => format!("({})", c),
                    };
                    
                    js_sys::Reflect::set(&obj, &"type".into(), &token_type.into()).unwrap();
                    js_sys::Reflect::set(&obj, &"value".into(), &token_value.into()).unwrap();
                    js_sys::Reflect::set(&obj, &"start".into(), &JsValue::from_f64(token_with_pos.start as f64)).unwrap();
                    js_sys::Reflect::set(&obj, &"end".into(), &JsValue::from_f64(token_with_pos.end as f64)).unwrap();
                    
                    arr.push(&obj);
                }
                arr.into()
            },
            Err(_e) => {
                js_sys::Array::new().into() // エラー時は空配列を返す
            }
        }
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
    pub fn get_workspace(&self) -> JsValue {
        let workspace_values: Vec<JsValue> = self.interpreter
            .get_workspace()
            .iter()
            .map(|v| value_to_js(v))
            .collect();
        
        let arr = js_sys::Array::new();
        for val in workspace_values {
            arr.push(&val);
        }
        arr.into()
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
    pub fn get_builtin_words_info(&self) -> JsValue {
        let builtin_words_info: Vec<(String, Option<String>)> = self.interpreter.dictionary.iter()
            .filter(|(_, def)| def.is_builtin)
            .map(|(name, def)| (name.clone(), def.description.clone()))
            .collect();
        
        let arr = js_sys::Array::new();
        
        for (name, desc) in builtin_words_info {
            let word_arr = js_sys::Array::new();
            word_arr.push(&JsValue::from_str(&name));
            word_arr.push(&desc.map(|d| JsValue::from_str(&d)).unwrap_or(JsValue::NULL));
            arr.push(&word_arr);
        }
        
        arr.into()
    }

    #[wasm_bindgen]
    pub fn get_builtin_words_by_category(&self) -> JsValue {
        let mut categories: std::collections::BTreeMap<String, Vec<(String, Option<String>)>> = std::collections::BTreeMap::new();
        
        for (name, def) in &self.interpreter.dictionary {
            if def.is_builtin {
                let category = def.category.clone().unwrap_or_else(|| "Other".to_string());
                categories.entry(category)
                    .or_insert_with(Vec::new)
                    .push((name.clone(), def.description.clone()));
            }
        }
        
        let result = js_sys::Object::new();
        for (category, words) in categories {
            let words_array = js_sys::Array::new();
            for (name, desc) in words {
                let word_info = js_sys::Array::new();
                word_info.push(&JsValue::from_str(&name));
                word_info.push(&desc.map(|d| JsValue::from_str(&d)).unwrap_or(JsValue::NULL));
                words_array.push(&word_info);
            }
            js_sys::Reflect::set(&result, &JsValue::from_str(&category), &words_array).unwrap();
        }
        
        result.into()
    }

    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.interpreter = Interpreter::new();
        self.step_tokens.clear();
        self.step_position = 0;
    }
    
    #[wasm_bindgen]
    pub fn save_table(&mut self, _name: String, _schema: JsValue, _records: JsValue) -> Result<(), String> {
        // IndexedDBへの保存処理は現在未実装
        // 将来的にはここでIndexedDBにテーブルデータを保存する
        Ok(())
    }

    #[wasm_bindgen]
    pub fn load_table(&self, _name: String) -> JsValue {
        // IndexedDBからの読み込み処理は現在未実装
        // 将来的にはここでIndexedDBからテーブルデータを読み込む
        JsValue::NULL
    }

    #[wasm_bindgen]
    pub fn get_all_tables(&self) -> Vec<String> {
        // IndexedDBのテーブル一覧取得は現在未実装
        // 将来的にはここでIndexedDB内のテーブル名一覧を返す
        Vec::new()
    }
    
    #[wasm_bindgen]
    pub fn restore_workspace(&mut self, workspace_js: JsValue) -> Result<(), String> {
        if !workspace_js.is_array() {
            return Err("Workspace must be an array".to_string());
        }
        
        let arr = js_sys::Array::from(&workspace_js);
        let mut new_workspace = Vec::new();
        
        for i in 0..arr.length() {
            let item = arr.get(i);
            let value = js_value_to_rust_value(&item)?;
            new_workspace.push(value);
        }
        
        self.interpreter.set_workspace(new_workspace);
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
        let definition = definition.trim();
        if !definition.starts_with('[') || !definition.ends_with(']') {
            return Err("Invalid word definition format".to_string());
        }
        
        let inner = &definition[1..definition.len()-1].trim();
        let tokens = crate::tokenizer::tokenize(inner)
            .map_err(|e| format!("Failed to tokenize definition: {}", e))?;
        
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
        ValueType::Nil => "nil",
    };
    
    js_sys::Reflect::set(&obj, &"type".into(), &type_str.into()).unwrap();
    
    let val = match &value.val_type {
        ValueType::Number(n) => {
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
        ValueType::Nil => JsValue::NULL,
    };
    
    js_sys::Reflect::set(&obj, &"value".into(), &val).unwrap();
    
    obj.into()
}

fn js_value_to_rust_value(js_val: &JsValue) -> Result<Value, String> {
    if js_sys::Reflect::has(js_val, &"type".into()).unwrap_or(false) {
        let type_str = js_sys::Reflect::get(js_val, &"type".into())
            .ok()
            .and_then(|v| v.as_string())
            .ok_or("Invalid type field")?;
        
        let value_field = js_sys::Reflect::get(js_val, &"value".into())
            .map_err(|_| "Missing value field")?;
        
        match type_str.as_str() {
            "number" => {
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
                        .ok_or("Invalid numerator")?;
                    
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
                        .ok_or("Invalid denominator")?;
                    
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
        if let Some(b) = js_val.as_bool() {
            Ok(Value {
                val_type: ValueType::Boolean(b)
            })
        } else if js_val.as_f64().is_some() {
            Err("Direct numeric values are not allowed".to_string())
        } else if let Some(s) = js_val.as_string() {
            Ok(Value {
                val_type: ValueType::String(s)
            })
        } else if js_val.is_null() || js_val.is_undefined() {
            Ok(Value {
                val_type: ValueType::Nil
            })
        } else if js_val.is_array() {
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
