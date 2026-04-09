use super::wasm_value_conversion::{build_bracket_structure_from_shape, is_vector_value};
use super::{set_js_prop, AjisaiInterpreter};
use crate::tokenizer;
use crate::types::{ExecutionLine, ValueData};
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
                            set_js_prop(&obj, "status", &("ERROR".into()));
                            set_js_prop(&obj, "message", &(e.to_string().into()));
                            set_js_prop(&obj, "error", &(true.into()));
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
                    set_js_prop(&obj, "inputHelper", &(helper_text.into()));
                    set_js_prop(&obj, "status", &("OK".into()));
                    set_js_prop(&obj, "stack", &(self.collect_stack()));
                    set_js_prop(&obj, "userWords", &(self.collect_user_words_for_state()));
                    set_js_prop(
                        &obj,
                        "importedModules",
                        &(self.collect_imported_modules_array()),
                    );
                    return Ok(obj.into());
                } else {
                    set_js_prop(&obj, "status", &("ERROR".into()));
                    set_js_prop(&obj, "message", &("FRAME requires a shape vector [ dim1 dim2 ... ] (1-9 dimensions, values 1-100)".into()));
                    set_js_prop(&obj, "error", &(true.into()));
                    return Ok(obj.into());
                }
            }
        }

        match self.interpreter.execute(code).await {
            Ok(()) => {
                set_js_prop(&obj, "status", &("OK".into()));
                let output = self.interpreter.collect_output();
                set_js_prop(&obj, "output", &(output.clone().into()));
                set_js_prop(&obj, "stack", &(self.collect_stack()));
                set_js_prop(&obj, "userWords", &(self.collect_user_words_for_state()));
                set_js_prop(
                    &obj,
                    "importedModules",
                    &(self.collect_imported_modules_array()),
                );

                if let Some(def_str) = self.interpreter.definition_to_load.take() {
                    set_js_prop(&obj, "definition_to_load", &(def_str.into()));
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                set_js_prop(&obj, "status", &("ERROR".into()));
                set_js_prop(&obj, "message", &(error_msg.into()));
                set_js_prop(&obj, "error", &(true.into()));
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
                    set_js_prop(&obj, "status", &("ERROR".into()));
                    set_js_prop(
                        &obj,
                        "message",
                        &(format!("Tokenization error: {}", e).into()),
                    );
                    set_js_prop(&obj, "error", &(true.into()));
                    return obj.into();
                }
            }
        }

        if self.step_position >= self.step_tokens.len() {
            self.step_mode = false;
            set_js_prop(&obj, "status", &("OK".into()));
            set_js_prop(&obj, "output", &("Step execution completed".into()));
            set_js_prop(&obj, "hasMore", &(false.into()));
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
                set_js_prop(&obj, "status", &("OK".into()));
                set_js_prop(&obj, "output", &(output.into()));
                set_js_prop(
                    &obj,
                    "hasMore",
                    &((self.step_position < self.step_tokens.len()).into()),
                );
                set_js_prop(&obj, "position", &((self.step_position as u32).into()));
                set_js_prop(&obj, "total", &((self.step_tokens.len() as u32).into()));
                set_js_prop(&obj, "stack", &(self.collect_stack()));
                set_js_prop(&obj, "userWords", &(self.collect_user_words_for_state()));
                set_js_prop(
                    &obj,
                    "importedModules",
                    &(self.collect_imported_modules_array()),
                );
            }
            Err(e) => {
                self.step_mode = false;
                set_js_prop(&obj, "status", &("ERROR".into()));
                set_js_prop(&obj, "message", &(e.to_string().into()));
                set_js_prop(&obj, "error", &(true.into()));
                set_js_prop(&obj, "hasMore", &(false.into()));
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
                set_js_prop(&obj, "status", &("OK".into()));
                set_js_prop(&obj, "output", &("System reinitialized.".into()));
                set_js_prop(&obj, "stack", &(self.collect_stack()));
                set_js_prop(&obj, "userWords", &(self.collect_user_words_for_state()));
                set_js_prop(
                    &obj,
                    "importedModules",
                    &(self.collect_imported_modules_array()),
                );
            }
            Err(e) => {
                set_js_prop(&obj, "status", &("ERROR".into()));
                set_js_prop(&obj, "message", &(e.to_string().into()));
                set_js_prop(&obj, "error", &(true.into()));
            }
        }
        obj.into()
    }
}
