use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;
use crate::interpreter::Interpreter;
use crate::types::{Value, ValueData, DisplayHint, Token, ExecutionLine};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use crate::interpreter;
use crate::tokenizer;
use crate::builtins;

fn is_string_value(val: &Value) -> bool {
    val.display_hint == DisplayHint::String && val.is_vector()
}

fn is_boolean_value(val: &Value) -> bool {
    val.display_hint == DisplayHint::Boolean && val.is_scalar()
}

fn is_number_value(val: &Value) -> bool {
    matches!(val.display_hint, DisplayHint::Number | DisplayHint::Auto) && val.is_scalar()
}

fn is_datetime_value(val: &Value) -> bool {
    val.display_hint == DisplayHint::DateTime && val.is_scalar()
}

fn is_vector_value(val: &Value) -> bool {
    val.is_vector()
}

fn value_as_string(val: &Value) -> String {
    let fractions = val.flatten_fractions();
    let bytes: Vec<u8> = fractions.iter()
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

fn generate_bracket_structure_from_shape(shape: &[usize]) -> String {
    fn build_level(shape: &[usize], depth: usize) -> String {
        let (open, close) = bracket_chars_for_depth(depth);
        if shape.len() == 1 {
            let empty = format!("{} {}", open, close);
            (0..shape[0]).map(|_| empty.as_str()).collect::<Vec<_>>().join(" ")
        } else {
            let inner = build_level(&shape[1..], depth + 1);
            let one_element = format!("{} {} {}", open, inner, close);
            (0..shape[0]).map(|_| one_element.as_str()).collect::<Vec<_>>().join(" ")
        }
    }
    if shape.is_empty() {
        return "{ }".to_string();
    }
    build_level(shape, 0)
}

#[derive(Serialize, Deserialize)]
struct CustomWordData {
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
        AjisaiInterpreter {
            interpreter: Interpreter::new(),
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
                upper_code.chars().nth(prefix_len - 1).map_or(false, |c| c.is_whitespace())
            };

            if is_valid {
                if prefix_len > 0 {
                    let prefix_code = &trimmed[..prefix_len].trim();
                    if !prefix_code.is_empty() {
                        if let Err(e) = self.interpreter.execute(prefix_code).await {
                            js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                            js_sys::Reflect::set(&obj, &"message".into(), &e.to_string().into()).unwrap();
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
                                for child in children {
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
                        if valid { Some(dims) } else { None }
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(shape_vec) = shape {
                    self.interpreter.stack.pop();
                    let helper_text = generate_bracket_structure_from_shape(&shape_vec);
                    js_sys::Reflect::set(&obj, &"inputHelper".into(), &helper_text.into()).unwrap();
                    js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                    js_sys::Reflect::set(&obj, &"stack".into(), &self.get_stack()).unwrap();
                    js_sys::Reflect::set(&obj, &"customWords".into(), &self.get_custom_words_for_state()).unwrap();
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
                let output = self.interpreter.get_output();
                js_sys::Reflect::set(&obj, &"output".into(), &output.clone().into()).unwrap();
                js_sys::Reflect::set(&obj, &"stack".into(), &self.get_stack()).unwrap();
                js_sys::Reflect::set(&obj, &"customWords".into(), &self.get_custom_words_for_state()).unwrap();

                if let Some(def_str) = self.interpreter.definition_to_load.take() {
                    js_sys::Reflect::set(&obj, &"definition_to_load".into(), &def_str.into()).unwrap();
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
                Ok(tokens) => { self.step_tokens = tokens; }
                Err(e) => {
                    self.step_mode = false;
                    js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                    js_sys::Reflect::set(&obj, &"message".into(), &format!("Tokenization error: {}", e).into()).unwrap();
                    js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
                    return obj.into();
                }
            }
        }

        if self.step_position >= self.step_tokens.len() {
            self.step_mode = false;
            js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
            js_sys::Reflect::set(&obj, &"output".into(), &"Step execution completed".into()).unwrap();
            js_sys::Reflect::set(&obj, &"hasMore".into(), &false.into()).unwrap();
            return obj.into();
        }

        let token = self.step_tokens[self.step_position].clone();

        let line = ExecutionLine {
            body_tokens: vec![token],
        };
        let result = self.interpreter.execute_guard_structure_sync(&[line]);

        match result {
            Ok(()) => {
                let output = self.interpreter.get_output();
                self.step_position += 1;
                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                js_sys::Reflect::set(&obj, &"output".into(), &output.into()).unwrap();
                js_sys::Reflect::set(&obj, &"hasMore".into(), &(self.step_position < self.step_tokens.len()).into()).unwrap();
                js_sys::Reflect::set(&obj, &"position".into(), &(self.step_position as u32).into()).unwrap();
                js_sys::Reflect::set(&obj, &"total".into(), &(self.step_tokens.len() as u32).into()).unwrap();
                js_sys::Reflect::set(&obj, &"stack".into(), &self.get_stack()).unwrap();
                js_sys::Reflect::set(&obj, &"customWords".into(), &self.get_custom_words_for_state()).unwrap();
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
                js_sys::Reflect::set(&obj, &"output".into(), &"System reinitialized.".into()).unwrap();
                js_sys::Reflect::set(&obj, &"stack".into(), &self.get_stack()).unwrap();
                js_sys::Reflect::set(&obj, &"customWords".into(), &self.get_custom_words_for_state()).unwrap();
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
    pub fn get_stack(&self) -> JsValue {
        let js_array = js_sys::Array::new();
        for value in self.interpreter.get_stack() {
            js_array.push(&value_to_js_value(value));
        }
        js_array.into()
    }

    #[wasm_bindgen]
    pub fn get_custom_words_info(&self) -> JsValue {
        let js_array = js_sys::Array::new();

        for (name, def) in self.interpreter.dictionary.iter() {
            if def.is_builtin { continue; }

            let is_protected = self.interpreter.dependents.get(name)
                .map_or(false, |deps| !deps.is_empty());

            let item = js_sys::Array::new();
            item.push(&name.clone().into());
            item.push(&def.description.clone().map(JsValue::from).unwrap_or(JsValue::NULL));
            item.push(&is_protected.into());

            js_array.push(&item);
        }

        js_array.into()
    }

    fn get_custom_words_for_state(&self) -> JsValue {
        let words_info: Vec<CustomWordData> = self.interpreter.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, def)| {
                CustomWordData {
                    name: name.clone(),
                    definition: self.interpreter.get_word_definition_tokens(name),
                    description: def.description.clone(),
                }
            })
            .collect();
        to_value(&words_info).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn get_builtin_words_info(&self) -> JsValue {
        to_value(&builtins::get_builtin_definitions()).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn get_word_definition(&self, name: &str) -> JsValue {
        let upper_name = name.to_uppercase();
        self.interpreter.get_word_definition_tokens(&upper_name)
            .map(|def| JsValue::from_str(&def))
            .unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn remove_word(&mut self, name: &str) {
        let upper_name = name.to_uppercase();
        if let Some(removed_def) = self.interpreter.dictionary.remove(&upper_name) {
            for dep_name in &removed_def.dependencies {
                if let Some(deps) = self.interpreter.dependents.get_mut(dep_name) {
                    deps.remove(&upper_name);
                }
            }
            self.interpreter.dependents.remove(&upper_name);
            for deps in self.interpreter.dependents.values_mut() {
                deps.remove(&upper_name);
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
        self.interpreter.set_stack(stack);
        Ok(())
    }

    #[wasm_bindgen]
    pub fn set_input_buffer(&mut self, text: String) {
        self.interpreter.input_buffer = text;
    }

    #[wasm_bindgen]
    pub fn get_io_output_buffer(&self) -> String {
        self.interpreter.io_output_buffer.clone()
    }

    #[wasm_bindgen]
    pub fn clear_io_output_buffer(&mut self) {
        self.interpreter.io_output_buffer.clear();
    }

    #[wasm_bindgen]
    pub fn restore_custom_words(&mut self, words_js: JsValue) -> Result<(), String> {
        let words: Vec<CustomWordData> = serde_wasm_bindgen::from_value(words_js)
            .map_err(|e| format!("Failed to deserialize words: {}", e))?;

        for word in words {
            let definition = match &word.definition {
                Some(def) if !def.is_empty() => def.clone(),
                _ => continue,
            };

            let tokens = tokenizer::tokenize(&definition)
                .map_err(|e| format!("Failed to tokenize definition for {}: {}", word.name, e))?;

            interpreter::dictionary::op_def_inner(&mut self.interpreter, &word.name, &tokens, word.description.clone())
                .map_err(|e| format!("Failed to restore word {}: {}", word.name, e))?;
        }

        self.interpreter.rebuild_dependencies().map_err(|e| e.to_string())?;

        // 復元時の内部メッセージはユーザーに見せない
        let _ = self.interpreter.get_output();

        Ok(())
    }
}

fn js_value_to_value(js_val: JsValue) -> Result<Value, String> {
    let obj = js_sys::Object::from(js_val);
    let type_str = js_sys::Reflect::get(&obj, &"type".into())
        .map_err(|_| "Failed to get 'type' property".to_string())?
        .as_string().ok_or("Type not string")?;
    let value_js = js_sys::Reflect::get(&obj, &"value".into())
        .map_err(|_| "Failed to get 'value' property".to_string())?;

    match type_str.as_str() {
        "number" => {
            let num_obj = js_sys::Object::from(value_js);
            let num_str = js_sys::Reflect::get(&num_obj, &"numerator".into()).map_err(|_| "No numerator".to_string())?.as_string().ok_or("Numerator not string")?;
            let den_str = js_sys::Reflect::get(&num_obj, &"denominator".into()).map_err(|_| "No denominator".to_string())?.as_string().ok_or("Denominator not string")?;
            let fraction = Fraction::new(
                BigInt::from_str(&num_str).map_err(|e| e.to_string())?,
                BigInt::from_str(&den_str).map_err(|e| e.to_string())?
            );
            Ok(Value::from_fraction(fraction))
        },
        "datetime" => {
            let num_obj = js_sys::Object::from(value_js);
            let num_str = js_sys::Reflect::get(&num_obj, &"numerator".into()).map_err(|_| "No numerator".to_string())?.as_string().ok_or("Numerator not string")?;
            let den_str = js_sys::Reflect::get(&num_obj, &"denominator".into()).map_err(|_| "No denominator".to_string())?.as_string().ok_or("Denominator not string")?;
            let fraction = Fraction::new(
                BigInt::from_str(&num_str).map_err(|e| e.to_string())?,
                BigInt::from_str(&den_str).map_err(|e| e.to_string())?
            );
            Ok(Value::from_datetime(fraction))
        },
        "string" => {
            let s = value_js.as_string().ok_or("Value not string")?;
            Ok(Value::from_string(&s))
        },
        "boolean" => {
            let b = value_js.as_bool().ok_or("Value not boolean")?;
            Ok(Value::from_bool(b))
        },
        "symbol" => {
            let s = value_js.as_string().ok_or("Value not string")?;
            Ok(Value::from_symbol(&s))
        },
        "vector" => {
            let js_array = js_sys::Array::from(&value_js);
            let mut vec = Vec::new();
            for i in 0..js_array.length() {
                vec.push(js_value_to_value(js_array.get(i))?);
            }
            Ok(Value::from_vector(vec))
        },
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
                    .as_string().ok_or("Numerator not string")?;
                let den_str = js_sys::Reflect::get(&frac_obj, &"denominator".into())
                    .map_err(|_| "No denominator in tensor data".to_string())?
                    .as_string().ok_or("Denominator not string")?;
                let fraction = Fraction::new(
                    BigInt::from_str(&num_str).map_err(|e| e.to_string())?,
                    BigInt::from_str(&den_str).map_err(|e| e.to_string())?
                );
                fractions.push(fraction);
            }

            let children: Vec<Value> = fractions.into_iter()
                .map(Value::from_fraction)
                .collect();

            Ok(Value::from_children(children))
        },
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

    if value.display_hint == DisplayHint::String && value.is_vector() {
        js_sys::Reflect::set(&obj, &"type".into(), &"string".into()).unwrap();
        let s = value_as_string(value);
        js_sys::Reflect::set(&obj, &"value".into(), &s.into()).unwrap();
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
                js_sys::Reflect::set(&num_obj, &"numerator".into(), &f.numerator.to_string().into()).unwrap();
                js_sys::Reflect::set(&num_obj, &"denominator".into(), &f.denominator.to_string().into()).unwrap();
                js_sys::Reflect::set(&obj, &"value".into(), &num_obj).unwrap();
            }
        },
        "string" => {
            let s = value_as_string(value);
            js_sys::Reflect::set(&obj, &"value".into(), &s.into()).unwrap();
        },
        "boolean" => {
            if let Some(f) = value.as_scalar() {
                let b = !f.is_zero();
                js_sys::Reflect::set(&obj, &"value".into(), &b.into()).unwrap();
            }
        },
        "vector" => {
            let js_array = js_sys::Array::new();
            if let ValueData::Vector(children) = &value.data {
                for child in children {
                    js_array.push(&value_to_js_value(child));
                }
            }
            js_sys::Reflect::set(&obj, &"value".into(), &js_array).unwrap();
        },
        _ => {}
    };

    obj.into()
}

#[cfg(test)]
mod test_input_helper {
    use super::generate_bracket_structure_from_shape;

    #[test]
    fn test_generate_bracket_structure_from_shape() {
        assert_eq!(generate_bracket_structure_from_shape(&[1]), "{ }");
        assert_eq!(generate_bracket_structure_from_shape(&[2]), "{ } { }");
        assert_eq!(generate_bracket_structure_from_shape(&[3]), "{ } { } { }");

        assert_eq!(generate_bracket_structure_from_shape(&[1, 1]), "{ ( ) }");
        assert_eq!(generate_bracket_structure_from_shape(&[1, 2]), "{ ( ) ( ) }");
        assert_eq!(generate_bracket_structure_from_shape(&[1, 3]), "{ ( ) ( ) ( ) }");
        assert_eq!(generate_bracket_structure_from_shape(&[2, 3]), "{ ( ) ( ) ( ) } { ( ) ( ) ( ) }");

        assert_eq!(generate_bracket_structure_from_shape(&[1, 1, 1]), "{ ( [ ] ) }");
        assert_eq!(generate_bracket_structure_from_shape(&[1, 1, 2]), "{ ( [ ] [ ] ) }");
        assert_eq!(generate_bracket_structure_from_shape(&[1, 2, 3]), "{ ( [ ] [ ] [ ] ) ( [ ] [ ] [ ] ) }");
        assert_eq!(generate_bracket_structure_from_shape(&[2, 2, 3]), "{ ( [ ] [ ] [ ] ) ( [ ] [ ] [ ] ) } { ( [ ] [ ] [ ] ) ( [ ] [ ] [ ] ) }");

        // 4D: brackets cycle back to { }
        assert_eq!(generate_bracket_structure_from_shape(&[1, 1, 1, 1]), "{ ( [ { } ] ) }");
    }
}
