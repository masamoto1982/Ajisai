use crate::interpreter::Interpreter;
use crate::types::Token;
use wasm_bindgen::prelude::*;

mod wasm_interpreter_execution;
mod wasm_interpreter_state;
mod wasm_runtime_metrics;
pub(crate) mod wasm_value_conversion;

/// Install console_error_panic_hook so any panic on the WASM side
/// surfaces in the browser console with a JS-friendly stack trace
/// instead of an opaque `RuntimeError: unreachable executed` trap.
/// Idempotent (`set_once`). Called from the TS loader exactly once
/// right after wasm-bindgen `init`.
#[wasm_bindgen]
pub fn init_panic_hook() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
}

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
