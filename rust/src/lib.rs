use wasm_bindgen::prelude::*;

mod types;
mod tokenizer;
mod interpreter;
mod builtins;

use types::*;
use interpreter::*;

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
}

// 既存のメソッドの後に追加
    
    #[wasm_bindgen]
    pub fn save_table(&mut self, name: String, schema: JsValue, records: JsValue) -> Result<(), String> {
        // JavaScriptの配列をRustのVecに変換
        let schema_vec: Vec<String> = serde_wasm_bindgen::from_value(schema)
            .map_err(|e| format!("Failed to parse schema: {:?}", e))?;
            
        let records_js: Vec<Vec<JsValue>> = serde_wasm_bindgen::from_value(records)
            .map_err(|e| format!("Failed to parse records: {:?}", e))?;
            
        // JsValueをValueに変換
        let mut records_vec: Vec<Vec<Value>> = Vec::new();
        for record_js in records_js {
            let mut record: Vec<Value> = Vec::new();
            for value_js in record_js {
                // 簡略化された変換（完全な実装が必要）
                if let Ok(num) = value_js.as_f64() {
                    record.push(Value {
                        val_type: ValueType::Number(Fraction::new(num as i64, 1))
                    });
                } else if let Some(s) = value_js.as_string() {
                    record.push(Value {
                        val_type: ValueType::String(s)
                    });
                } else if let Some(b) = value_js.as_bool() {
                    record.push(Value {
                        val_type: ValueType::Boolean(b)
                    });
                } else if value_js.is_null() || value_js.is_undefined() {
                    record.push(Value {
                        val_type: ValueType::Nil
                    });
                }
            }
            records_vec.push(record);
        }
        
        self.interpreter.save_table(name, schema_vec, records_vec);
        Ok(())
    }
    
    #[wasm_bindgen]
    pub fn load_table(&self, name: &str) -> JsValue {
        match self.interpreter.load_table(name) {
            Some((schema, records)) => {
                let obj = js_sys::Object::new();
                
                // スキーマを変換
                let schema_arr = js_sys::Array::new();
                for s in schema {
                    schema_arr.push(&JsValue::from_str(&s));
                }
                js_sys::Reflect::set(&obj, &"schema".into(), &schema_arr).unwrap();
                
                // レコードを変換
                let records_arr = js_sys::Array::new();
                for record in records {
                    let record_arr = js_sys::Array::new();
                    for value in record {
                        record_arr.push(&value_to_js(&value));
                    }
                    records_arr.push(&record_arr);
                }
                js_sys::Reflect::set(&obj, &"records".into(), &records_arr).unwrap();
                
                obj.into()
            },
            None => JsValue::NULL,
        }
    }
    
    #[wasm_bindgen]
    pub fn get_all_tables(&self) -> Vec<String> {
        self.interpreter.get_all_tables()
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
            if n.denominator == 1 {
                // i64をf64に変換してからJsValueに変換
                // JavaScriptの数値として安全に扱える範囲内であることを確認
                if n.numerator >= -(1i64 << 53) && n.numerator <= (1i64 << 53) {
                    JsValue::from_f64(n.numerator as f64)
                } else {
                    // 大きすぎる数値は文字列として返す
                    JsValue::from_str(&n.numerator.to_string())
                }
            } else {
                JsValue::from_str(&format!("{}/{}", n.numerator, n.denominator))
            }
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
