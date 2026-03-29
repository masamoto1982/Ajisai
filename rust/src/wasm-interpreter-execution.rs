use crate::tokenizer;
use crate::types::{ExecutionLine, Token, ValueData};
use super::wasm_value_conversion::{
    build_bracket_structure_from_shape, is_vector_value, value_to_js_value,
};
use super::AjisaiInterpreter;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl AjisaiInterpreter {
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
                        &"userWords".into(),
                        &self.collect_user_words_for_state(),
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
                    &"userWords".into(),
                    &self.collect_user_words_for_state(),
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
                    &"userWords".into(),
                    &self.collect_user_words_for_state(),
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
                    &"userWords".into(),
                    &self.collect_user_words_for_state(),
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
}
