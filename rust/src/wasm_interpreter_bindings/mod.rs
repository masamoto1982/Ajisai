// `wasm-bindgen` expands `#[wasm_bindgen]` items into generated glue that
// contains `unsafe`, so this module re-permits `unsafe_code` over the crate-root
// `#![deny(unsafe_code)]` (structural-memory-safety roadmap Phase 4). No
// hand-written `unsafe` lives here; the allow only covers macro-generated code.
#![allow(unsafe_code)]

use crate::interpreter::Interpreter;
use crate::types::Token;
use wasm_bindgen::prelude::*;

mod wasm_agent;
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

impl Default for AjisaiInterpreter {
    fn default() -> Self {
        Self::new()
    }
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

    /// The resource ceilings this interpreter is actually running under, as
    /// JSON, under the same names every other Ajisai host publishes them by.
    ///
    /// SPEC §2.5 makes limits a host safety control rather than value
    /// semantics, so two conforming hosts legitimately disagree about them —
    /// and they do: the playground runs the interpreter defaults while the MCP
    /// agent profile is an order of magnitude tighter. That is only a trap for
    /// someone who prototypes here and runs there while neither host says what
    /// it applies. Read from the live interpreter rather than from a constant,
    /// so what is displayed is what is enforced.
    #[wasm_bindgen]
    pub fn host_profile(&self) -> String {
        let limits = self.interpreter.runtime_limits();
        serde_json::json!({
            "profile": "browser-playground",
            "limits": {
                "sourceBytes": limits.max_source_bytes,
                "executionSteps": self.interpreter.max_execution_steps(),
                "materializedElements": limits.max_materialized_elements,
                "numericLiteralDigits": limits.max_numeric_literal_digits,
                "numericWork": limits.max_numeric_work,
                "collectionWork": limits.max_collection_work,
                "bigintBits": limits.max_bigint_bits,
                "algebraicTerms": limits.max_algebraic_terms,
            },
        })
        .to_string()
    }
}
