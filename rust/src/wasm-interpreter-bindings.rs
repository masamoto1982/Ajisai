use crate::interpreter::Interpreter;
use crate::types::Token;
use wasm_bindgen::prelude::*;

#[path = "wasm-value-conversion.rs"]
pub(crate) mod wasm_value_conversion;

#[path = "wasm-interpreter-execution.rs"]
mod wasm_interpreter_execution;

#[path = "wasm-interpreter-state.rs"]
mod wasm_interpreter_state;

#[wasm_bindgen]
pub struct AjisaiInterpreter {
    interpreter: Interpreter,
    step_tokens: Vec<Token>,
    step_position: usize,
    step_mode: bool,
    current_step_code: String,
}

pub(crate) fn set_js_prop(obj: &js_sys::Object, key: &str, value: &JsValue) {
    js_sys::Reflect::set(obj, &JsValue::from_str(key), value).unwrap();
}

#[wasm_bindgen]
impl AjisaiInterpreter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let interp = Interpreter::new();
        AjisaiInterpreter {
            interpreter: interp,
            step_tokens: Vec::new(),
            step_position: 0,
            step_mode: false,
            current_step_code: String::new(),
        }
    }
}
