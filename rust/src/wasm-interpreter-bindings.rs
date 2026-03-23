use crate::builtins;
use crate::interpreter;
use crate::interpreter::Interpreter;
use crate::tokenizer;
use crate::types::fraction::Fraction;
use crate::types::{ExecutionLine, Token, Value, ValueData};
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use std::str::FromStr;
use wasm_bindgen::prelude::*;

fn is_string_value(_val: &Value) -> bool {
    false
}

fn is_boolean_value(_val: &Value) -> bool {
    false
}

fn is_number_value(val: &Value) -> bool {
    val.is_scalar()
}

fn is_datetime_value(_val: &Value) -> bool {
    false
}

fn is_vector_value(val: &Value) -> bool {
    val.is_vector()
}

fn value_as_string(val: &Value) -> String {
    let fractions = val.collect_fractions_flat();
    let bytes: Vec<u8> = fractions
        .iter()
        .filter_map(|f| {
            f.to_i64().and_then(|n| {
                if n >= 0 && n <= 255 {
                    Some(n as u8)
                } else {
                    None
                }
            })
        })
        .collect();

    String::from_utf8_lossy(&bytes).into_owned()
}

fn bracket_chars_for_depth(depth: usize) -> (char, char) {
    match depth % 3 {
        0 => ('{', '}'),
        1 => ('(', ')'),
        2 => ('[', ']'),
        _ => unreachable!(),
    }
}

fn build_bracket_structure_from_shape(shape: &[usize]) -> String {
    fn build_level(shape: &[usize], depth: usize) -> String {
        let (open, close) = bracket_chars_for_depth(depth);
        if shape.len() == 1 {
            let empty = format!("{} {}", open, close);
            (0..shape[0])
                .map(|_| empty.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            let inner = build_level(&shape[1..], depth + 1);
            let one_element = format!("{} {} {}", open, inner, close);
            (0..shape[0])
                .map(|_| one_element.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        }
    }
    if shape.is_empty() {
        return "{ }".to_string();
    }
    build_level(shape, 0)
}

#[derive(Serialize, Deserialize)]
struct CustomWordData {
    dictionary: Option<String>,
    name: String,
    definition: Option<String>,
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
        let mut interp = Interpreter::new();
        interp.gui_mode = true;
        AjisaiInterpreter {
            interpreter: interp,
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

        let trimmed = code.trim();
        let upper_code = trimmed.to_uppercase();

        if upper_code.ends_with("FRAME") {
            let prefix_len = upper_code.len() - 5;
            let is_valid = if prefix_len == 0 {
                true
            } else {
                upper_code
                    .chars()
                    .nth(prefix_len - 1)
                    .map_or(false, |c| c.is_whitespace())
            };

            if is_valid {
                if prefix_len > 0 {
                    let prefix_code = &trimmed[..prefix_len].trim();
                    if !prefix_code.is_empty() {
                        if let Err(e) = self.interpreter.execute(prefix_code).await {
                            js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                            js_sys::Reflect::set(&obj, &"message".into(), &e.to_string().into())
                                .unwrap();
                            js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
                            return Ok(obj.into());
                        }
                    }
                }

                let shape = if let Some(top) = self.interpreter.stack.last() {
                    if is_vector_value(top) && !top.is_nil() {
                        let mut dims = Vec::new();
                        let mut valid = top.len() >= 1 && top.len() <= 9;
                        if valid {
                            if let ValueData::Vector(children) = &top.data {
                                for child in children.iter() {
                                    if let Some(val) = child.as_usize() {
                                        if val >= 1 && val <= 100 {
                                            dims.push(val);
                                        } else {
                                            valid = false;
                                            break;
                                        }
                                    } else {
                                        valid = false;
                                        break;
                                    }
                                }
                            } else {
                                valid = false;
                            }
                        }
                        if valid {
                            Some(dims)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(shape_vec) = shape {
                    self.interpreter.stack.pop();
                    let helper_text = build_bracket_structure_from_shape(&shape_vec);
                    js_sys::Reflect::set(&obj, &"inputHelper".into(), &helper_text.into()).unwrap();
                    js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                    js_sys::Reflect::set(&obj, &"stack".into(), &self.collect_stack()).unwrap();
                    js_sys::Reflect::set(
                        &obj,
                        &"customWords".into(),
                        &self.collect_custom_words_for_state(),
                    )
                    .unwrap();
                    js_sys::Reflect::set(&obj, &"importedModules".into(), &self.collect_imported_modules_array()).unwrap();
                    return Ok(obj.into());
                } else {
                    js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                    js_sys::Reflect::set(&obj, &"message".into(), &"FRAME requires a shape vector [ dim1 dim2 ... ] (1-9 dimensions, values 1-100)".into()).unwrap();
                    js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
                    return Ok(obj.into());
                }
            }
        }

        match self.interpreter.execute(code).await {
            Ok(()) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                let output = self.interpreter.collect_output();
                js_sys::Reflect::set(&obj, &"output".into(), &output.clone().into()).unwrap();
                js_sys::Reflect::set(&obj, &"stack".into(), &self.collect_stack()).unwrap();
                js_sys::Reflect::set(
                    &obj,
                    &"customWords".into(),
                    &self.collect_custom_words_for_state(),
                )
                .unwrap();
                js_sys::Reflect::set(&obj, &"importedModules".into(), &self.collect_imported_modules_array()).unwrap();

                if let Some(def_str) = self.interpreter.definition_to_load.take() {
                    js_sys::Reflect::set(&obj, &"definition_to_load".into(), &def_str.into())
                        .unwrap();
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                js_sys::Reflect::set(&obj, &"message".into(), &error_msg.into()).unwrap();
                js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
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

            match tokenizer::tokenize(code) {
                Ok(tokens) => {
                    self.step_tokens = tokens;
                }
                Err(e) => {
                    self.step_mode = false;
                    js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                    js_sys::Reflect::set(
                        &obj,
                        &"message".into(),
                        &format!("Tokenization error: {}", e).into(),
                    )
                    .unwrap();
                    js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
                    return obj.into();
                }
            }
        }

        if self.step_position >= self.step_tokens.len() {
            self.step_mode = false;
            js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
            js_sys::Reflect::set(&obj, &"output".into(), &"Step execution completed".into())
                .unwrap();
            js_sys::Reflect::set(&obj, &"hasMore".into(), &false.into()).unwrap();
            return obj.into();
        }

        let token = self.step_tokens[self.step_position].clone();

        let line = ExecutionLine {
            body_tokens: vec![token].into(),
        };
        let result = self.interpreter.execute_guard_structure_sync(&[line]);

        match result {
            Ok(()) => {
                let output = self.interpreter.collect_output();
                self.step_position += 1;
                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                js_sys::Reflect::set(&obj, &"output".into(), &output.into()).unwrap();
                js_sys::Reflect::set(
                    &obj,
                    &"hasMore".into(),
                    &(self.step_position < self.step_tokens.len()).into(),
                )
                .unwrap();
                js_sys::Reflect::set(
                    &obj,
                    &"position".into(),
                    &(self.step_position as u32).into(),
                )
                .unwrap();
                js_sys::Reflect::set(
                    &obj,
                    &"total".into(),
                    &(self.step_tokens.len() as u32).into(),
                )
                .unwrap();
                js_sys::Reflect::set(&obj, &"stack".into(), &self.collect_stack()).unwrap();
                js_sys::Reflect::set(
                    &obj,
                    &"customWords".into(),
                    &self.collect_custom_words_for_state(),
                )
                .unwrap();
                js_sys::Reflect::set(&obj, &"importedModules".into(), &self.collect_imported_modules_array()).unwrap();
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
    pub fn reset(&mut self) -> JsValue {
        let obj = js_sys::Object::new();

        self.step_mode = false;
        self.step_tokens.clear();
        self.step_position = 0;
        self.current_step_code.clear();

        match self.interpreter.execute_reset() {
            Ok(()) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                js_sys::Reflect::set(&obj, &"output".into(), &"System reinitialized.".into())
                    .unwrap();
                js_sys::Reflect::set(&obj, &"stack".into(), &self.collect_stack()).unwrap();
                js_sys::Reflect::set(
                    &obj,
                    &"customWords".into(),
                    &self.collect_custom_words_for_state(),
                )
                .unwrap();
                js_sys::Reflect::set(&obj, &"importedModules".into(), &self.collect_imported_modules_array()).unwrap();
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
    pub fn collect_stack(&self) -> JsValue {
        let js_array = js_sys::Array::new();
        for value in self.interpreter.get_stack() {
            js_array.push(&value_to_js_value(value));
        }
        js_array.into()
    }

    #[wasm_bindgen]
    pub fn collect_custom_words_info(&self) -> JsValue {
        let js_array = js_sys::Array::new();

        for dict_name in self.interpreter.custom_dictionary_names() {
            for (name, def) in self.interpreter.custom_dictionary_words(&dict_name) {
                let fq_name = format!("{}::{}", dict_name, name);
                let is_protected = self
                    .interpreter
                    .dependents
                    .get(&fq_name)
                    .map_or(false, |deps| !deps.is_empty());

                let item = js_sys::Array::new();
                item.push(&dict_name.clone().into());
                item.push(&name.clone().into());
                item.push(
                    &def.description
                        .clone()
                        .map(JsValue::from)
                        .unwrap_or(JsValue::NULL),
                );
                item.push(&is_protected.into());

                js_array.push(&item);
            }
        }

        js_array.into()
    }

    fn collect_imported_modules_array(&self) -> JsValue {
        let arr = js_sys::Array::new();
        for name in &self.interpreter.imported_modules {
            arr.push(&JsValue::from_str(name));
        }
        arr.into()
    }

    fn collect_custom_words_for_state(&self) -> JsValue {
        let words_info: Vec<CustomWordData> = self
            .interpreter
            .custom_dictionary_names()
            .into_iter()
            .flat_map(|dict_name| {
                self.interpreter
                    .custom_dictionary_words(&dict_name)
                    .into_iter()
                    .map(move |(name, def)| CustomWordData {
                        dictionary: Some(dict_name.clone()),
                        name: name.clone(),
                        definition: self
                            .interpreter
                            .lookup_word_definition_tokens(&format!("{}::{}", dict_name, name)),
                        description: def.description.clone(),
                    })
            })
            .collect();
        to_value(&words_info).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn collect_core_words_info(&self) -> JsValue {
        to_value(&builtins::collect_builtin_definitions()).unwrap_or(JsValue::NULL)
    }

    /// IMPORT済みモジュール名の一覧を返す。
    /// 例: ["MUSIC", "JSON"]
    #[wasm_bindgen]
    pub fn collect_imported_modules(&self) -> JsValue {
        let arr = js_sys::Array::new();
        for name in &self.interpreter.imported_modules {
            arr.push(&JsValue::from_str(name));
        }
        arr.into()
    }

    /// 指定モジュールのサンプルワード情報を返す。
    /// 返却形式は Array<[name, description]>
    #[wasm_bindgen]
    pub fn collect_module_sample_words_info(&self, module_name: &str) -> JsValue {
        let upper = module_name.to_uppercase();
        let arr = js_sys::Array::new();
        if let Some(module_dict) = self.interpreter.module_samples.get(&upper) {
            for (name, def) in &module_dict.sample_words {
                let item = js_sys::Array::new();
                item.push(&JsValue::from_str(name));
                item.push(
                    &def.description
                        .clone()
                        .map(JsValue::from)
                        .unwrap_or(JsValue::NULL),
                );
                arr.push(&item);
            }
        }
        arr.into()
    }

    /// 指定モジュールが公開するワード情報を返す。
    /// 返却形式は Array<[name, description]>
    #[wasm_bindgen]
    pub fn collect_module_words_info(&self, module_name: &str) -> JsValue {
        let upper = module_name.to_uppercase();
        let prefix = format!("{}::", upper);
        let arr = js_sys::Array::new();
        for (name, def) in &self.interpreter.core_vocabulary {
            if name.starts_with(&prefix) {
                let item = js_sys::Array::new();
                item.push(&JsValue::from_str(name));
                item.push(
                    &def.description
                        .clone()
                        .map(JsValue::from)
                        .unwrap_or(JsValue::NULL),
                );
                arr.push(&item);
            }
        }
        arr.into()
    }

    /// JS側からモジュール状態を復元する。
    /// 配列 ["MUSIC", "JSON"] のような形式で受け取り、各モジュールを再登録する。
    #[wasm_bindgen]
    pub fn restore_imported_modules(&mut self, modules_js: JsValue) {
        let arr = js_sys::Array::from(&modules_js);
        for i in 0..arr.length() {
            if let Some(name) = arr.get(i).as_string() {
                interpreter::modules::restore_module(&mut self.interpreter, &name);
            }
        }
    }

    #[wasm_bindgen]
    pub fn lookup_word_definition(&self, name: &str) -> JsValue {
        let upper_name = name.to_uppercase();
        self.interpreter
            .lookup_word_definition_tokens(&upper_name)
            .map(|def| JsValue::from_str(&def))
            .unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn remove_word(&mut self, name: &str) {
        let upper_name = name.to_uppercase();
        if let Some((dict_name, short_name)) = self.interpreter.split_qualified_name(&upper_name) {
            if let Some(dict) = self.interpreter.custom_dictionaries.get_mut(&dict_name) {
                dict.words.remove(&short_name);
            }
            if let Some(dict) = self.interpreter.module_samples.get_mut(&dict_name) {
                dict.sample_words.remove(&short_name);
            }
            let _ = self.interpreter.rebuild_dependencies();
            return;
        }

        for dict in self.interpreter.custom_dictionaries.values_mut() {
            if dict.words.remove(&upper_name).is_some() {
                let _ = self.interpreter.rebuild_dependencies();
                return;
            }
        }
        for dict in self.interpreter.module_samples.values_mut() {
            if dict.sample_words.remove(&upper_name).is_some() {
                let _ = self.interpreter.rebuild_dependencies();
                return;
            }
        }
    }

    #[wasm_bindgen]
    pub fn restore_stack(&mut self, stack_js: JsValue) -> Result<(), String> {
        let js_array = js_sys::Array::from(&stack_js);
        let mut stack = Vec::new();
        for i in 0..js_array.length() {
            stack.push(js_value_to_value(js_array.get(i))?);
        }
        self.interpreter.update_stack(stack);
        Ok(())
    }

    #[wasm_bindgen]
    pub fn update_input_buffer(&mut self, text: String) {
        self.interpreter.input_buffer = text;
    }

    #[wasm_bindgen]
    pub fn extract_io_output_buffer(&self) -> String {
        self.interpreter.io_output_buffer.clone()
    }

    #[wasm_bindgen]
    pub fn clear_io_output_buffer(&mut self) {
        self.interpreter.io_output_buffer.clear();
    }

    #[wasm_bindgen]
    pub fn push_json_string(&mut self, json_string: &str) -> Result<JsValue, JsValue> {
        use crate::types::json::deserialize_json_to_value;

        let obj = js_sys::Object::new();

        match serde_json::from_str::<serde_json::Value>(json_string) {
            Ok(json_val) => match deserialize_json_to_value(json_val, 1) {
                Ok(parsed) => {
                    self.interpreter.stack.push(parsed);
                    js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                }
                Err(e) => {
                    js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                    js_sys::Reflect::set(&obj, &"message".into(), &format!("{}", e).into())
                        .unwrap();
                }
            },
            Err(e) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                js_sys::Reflect::set(
                    &obj,
                    &"message".into(),
                    &format!("JSON parse error: {}", e).into(),
                )
                .unwrap();
            }
        }
        Ok(obj.into())
    }

    #[wasm_bindgen]
    pub fn restore_custom_words(&mut self, words_js: JsValue) -> Result<(), String> {
        let words: Vec<CustomWordData> = serde_wasm_bindgen::from_value(words_js)
            .map_err(|e| format!("Failed to deserialize words: {}", e))?;

        for word in words {
            self.interpreter.active_custom_dictionary = word
                .dictionary
                .clone()
                .unwrap_or_else(|| "SAMPLE".to_string())
                .to_uppercase();
            let definition = match &word.definition {
                Some(def) if !def.is_empty() => def.clone(),
                _ => continue,
            };

            let tokens = tokenizer::tokenize(&definition)
                .map_err(|e| format!("Failed to tokenize definition for {}: {}", word.name, e))?;

            interpreter::dictionary::op_def_inner(
                &mut self.interpreter,
                &word.name,
                &tokens,
                word.description.clone(),
            )
            .map_err(|e| format!("Failed to restore word {}: {}", word.name, e))?;
        }

        self.interpreter
            .rebuild_dependencies()
            .map_err(|e| e.to_string())?;

        // 復元時の内部メッセージはユーザーに見せない
        let _ = self.interpreter.collect_output();

        Ok(())
    }
}

fn js_value_to_value(js_val: JsValue) -> Result<Value, String> {
    let obj = js_sys::Object::from(js_val);
    let type_str = js_sys::Reflect::get(&obj, &"type".into())
        .map_err(|_| "Failed to get 'type' property".to_string())?
        .as_string()
        .ok_or("Type not string")?;
    let value_js = js_sys::Reflect::get(&obj, &"value".into())
        .map_err(|_| "Failed to get 'value' property".to_string())?;

    match type_str.as_str() {
        "number" => {
            let num_obj = js_sys::Object::from(value_js);
            let num_str = js_sys::Reflect::get(&num_obj, &"numerator".into())
                .map_err(|_| "No numerator".to_string())?
                .as_string()
                .ok_or("Numerator not string")?;
            let den_str = js_sys::Reflect::get(&num_obj, &"denominator".into())
                .map_err(|_| "No denominator".to_string())?
                .as_string()
                .ok_or("Denominator not string")?;
            let fraction = Fraction::new(
                BigInt::from_str(&num_str).map_err(|e| e.to_string())?,
                BigInt::from_str(&den_str).map_err(|e| e.to_string())?,
            );
            Ok(Value::from_fraction(fraction))
        }
        "datetime" => {
            let num_obj = js_sys::Object::from(value_js);
            let num_str = js_sys::Reflect::get(&num_obj, &"numerator".into())
                .map_err(|_| "No numerator".to_string())?
                .as_string()
                .ok_or("Numerator not string")?;
            let den_str = js_sys::Reflect::get(&num_obj, &"denominator".into())
                .map_err(|_| "No denominator".to_string())?
                .as_string()
                .ok_or("Denominator not string")?;
            let fraction = Fraction::new(
                BigInt::from_str(&num_str).map_err(|e| e.to_string())?,
                BigInt::from_str(&den_str).map_err(|e| e.to_string())?,
            );
            Ok(Value::from_datetime(fraction))
        }
        "string" => {
            let s = value_js.as_string().ok_or("Value not string")?;
            Ok(Value::from_string(&s))
        }
        "boolean" => {
            let b = value_js.as_bool().ok_or("Value not boolean")?;
            Ok(Value::from_bool(b))
        }
        "symbol" => {
            let s = value_js.as_string().ok_or("Value not string")?;
            Ok(Value::from_symbol(&s))
        }
        "vector" => {
            let js_array = js_sys::Array::from(&value_js);
            let mut vec = Vec::new();
            for i in 0..js_array.length() {
                vec.push(js_value_to_value(js_array.get(i))?);
            }
            Ok(Value::from_vector(vec))
        }
        "tensor" => {
            let tensor_obj = js_sys::Object::from(value_js);

            let data_js = js_sys::Reflect::get(&tensor_obj, &"data".into())
                .map_err(|_| "No data in tensor".to_string())?;
            let data_array = js_sys::Array::from(&data_js);
            let mut fractions = Vec::new();
            for i in 0..data_array.length() {
                let frac_obj = js_sys::Object::from(data_array.get(i));
                let num_str = js_sys::Reflect::get(&frac_obj, &"numerator".into())
                    .map_err(|_| "No numerator in tensor data".to_string())?
                    .as_string()
                    .ok_or("Numerator not string")?;
                let den_str = js_sys::Reflect::get(&frac_obj, &"denominator".into())
                    .map_err(|_| "No denominator in tensor data".to_string())?
                    .as_string()
                    .ok_or("Denominator not string")?;
                let fraction = Fraction::new(
                    BigInt::from_str(&num_str).map_err(|e| e.to_string())?,
                    BigInt::from_str(&den_str).map_err(|e| e.to_string())?,
                );
                fractions.push(fraction);
            }

            let children: Vec<Value> = fractions.into_iter().map(Value::from_fraction).collect();

            Ok(Value::from_children(children))
        }
        "nil" => Ok(Value::nil()),
        _ => Err(format!("Unknown type: {}", type_str)),
    }
}

fn value_to_js_value(value: &Value) -> JsValue {
    let obj = js_sys::Object::new();

    if value.is_nil() {
        js_sys::Reflect::set(&obj, &"type".into(), &"nil".into()).unwrap();
        js_sys::Reflect::set(&obj, &"value".into(), &JsValue::NULL).unwrap();
        return obj.into();
    }

    let type_str = if is_datetime_value(value) {
        "datetime"
    } else if is_boolean_value(value) {
        "boolean"
    } else if is_string_value(value) {
        "string"
    } else if is_number_value(value) {
        "number"
    } else if is_vector_value(value) {
        "vector"
    } else if value.is_scalar() {
        "number"
    } else {
        "nil"
    };

    js_sys::Reflect::set(&obj, &"type".into(), &type_str.into()).unwrap();

    match type_str {
        "number" | "datetime" => {
            if let Some(f) = value.as_scalar() {
                let num_obj = js_sys::Object::new();
                js_sys::Reflect::set(
                    &num_obj,
                    &"numerator".into(),
                    &f.numerator().to_string().into(),
                )
                .unwrap();
                js_sys::Reflect::set(
                    &num_obj,
                    &"denominator".into(),
                    &f.denominator().to_string().into(),
                )
                .unwrap();
                js_sys::Reflect::set(&obj, &"value".into(), &num_obj).unwrap();
            }
        }
        "string" => {
            let s = value_as_string(value);
            js_sys::Reflect::set(&obj, &"value".into(), &s.into()).unwrap();
        }
        "boolean" => {
            if let Some(f) = value.as_scalar() {
                let b = !f.is_zero();
                js_sys::Reflect::set(&obj, &"value".into(), &b.into()).unwrap();
            }
        }
        "vector" => {
            let js_array = js_sys::Array::new();
            if let ValueData::Vector(children) = &value.data {
                for child in children.iter() {
                    js_array.push(&value_to_js_value(child));
                }
            }
            js_sys::Reflect::set(&obj, &"value".into(), &js_array).unwrap();
        }
        _ => {}
    };

    obj.into()
}

#[cfg(test)]
mod test_input_helper {
    use super::build_bracket_structure_from_shape;

    #[test]
    fn test_build_bracket_structure_from_shape() {
        assert_eq!(build_bracket_structure_from_shape(&[1]), "{ }");
        assert_eq!(build_bracket_structure_from_shape(&[2]), "{ } { }");
        assert_eq!(build_bracket_structure_from_shape(&[3]), "{ } { } { }");

        assert_eq!(build_bracket_structure_from_shape(&[1, 1]), "{ ( ) }");
        assert_eq!(
            build_bracket_structure_from_shape(&[1, 2]),
            "{ ( ) ( ) }"
        );
        assert_eq!(
            build_bracket_structure_from_shape(&[1, 3]),
            "{ ( ) ( ) ( ) }"
        );
        assert_eq!(
            build_bracket_structure_from_shape(&[2, 3]),
            "{ ( ) ( ) ( ) } { ( ) ( ) ( ) }"
        );

        assert_eq!(
            build_bracket_structure_from_shape(&[1, 1, 1]),
            "{ ( [ ] ) }"
        );
        assert_eq!(
            build_bracket_structure_from_shape(&[1, 1, 2]),
            "{ ( [ ] [ ] ) }"
        );
        assert_eq!(
            build_bracket_structure_from_shape(&[1, 2, 3]),
            "{ ( [ ] [ ] [ ] ) ( [ ] [ ] [ ] ) }"
        );
        assert_eq!(
            build_bracket_structure_from_shape(&[2, 2, 3]),
            "{ ( [ ] [ ] [ ] ) ( [ ] [ ] [ ] ) } { ( [ ] [ ] [ ] ) ( [ ] [ ] [ ] ) }"
        );

        // 4D: brackets cycle back to { }
        assert_eq!(
            build_bracket_structure_from_shape(&[1, 1, 1, 1]),
            "{ ( [ { } ] ) }"
        );
    }
}
