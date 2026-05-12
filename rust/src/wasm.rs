//! WASM bindings for the Phase 1 interpreter.
//!
//! The TypeScript shell expects a minimal protocol surface defined in
//! `src/wasm-interpreter-types.ts`. Phase 1 implements the subset required
//! to run a Stack/CF interpreter with DEF/DEL and Nil; richer subsystems
//! (modules, hedged execution, etc.) will return empty/no-op responses
//! pending Phase 2.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::error::AjisaiError;
use crate::interpreter::Interpreter;
use crate::value::Value;

#[derive(Serialize)]
struct ProtocolValueSemantics {
    #[serde(rename = "semanticKind")]
    semantic_kind: &'static str,
    shape: &'static str,
    capabilities: Vec<&'static str>,
    origin: &'static str,
}

#[derive(Serialize)]
struct Fraction {
    numerator: String,
    denominator: String,
}

#[derive(Serialize)]
#[serde(untagged)]
enum ProtocolValuePayload {
    Number(Fraction),
    Nil(String),
}

#[derive(Serialize)]
struct ProtocolValue {
    #[serde(rename = "type")]
    kind: &'static str,
    value: ProtocolValuePayload,
    /// Canonical nested continued-fraction representation.
    #[serde(rename = "continuedFraction")]
    continued_fraction: String,
    #[serde(rename = "displayHint")]
    display_hint: &'static str,
    semantics: ProtocolValueSemantics,
}

#[derive(Serialize)]
struct ExecuteResult {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnosis: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<bool>,
}

#[derive(Deserialize)]
struct RestoreUserWord {
    name: String,
    #[serde(default)]
    definition: Option<String>,
}

#[wasm_bindgen]
pub struct AjisaiInterpreter {
    inner: Interpreter,
}

#[wasm_bindgen]
impl AjisaiInterpreter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Interpreter::new(),
        }
    }

    pub fn execute(&mut self, code: &str) -> JsValue {
        let result = self.inner.execute(code);
        let payload = match result {
            Ok(()) => {
                let out = self.inner.take_output();
                ExecuteResult {
                    status: "OK",
                    output: if out.is_empty() { None } else { Some(out) },
                    message: None,
                    detail: None,
                    diagnosis: None,
                    error: None,
                }
            }
            Err(e) => error_payload(e),
        };
        serde_wasm_bindgen::to_value(&payload).unwrap_or(JsValue::NULL)
    }

    pub fn execute_step(&mut self, code: &str) -> JsValue {
        // Phase 1 has no concept of "step"; execute_step is an alias of execute.
        self.execute(code)
    }

    pub fn reset(&mut self) -> JsValue {
        self.inner.reset();
        serde_wasm_bindgen::to_value(&ExecuteResult {
            status: "OK",
            output: None,
            message: None,
            detail: None,
            diagnosis: None,
            error: None,
        })
        .unwrap_or(JsValue::NULL)
    }

    pub fn collect_stack(&self) -> JsValue {
        let items: Vec<ProtocolValue> = self.inner.stack().iter().map(protocol_value).collect();
        serde_wasm_bindgen::to_value(&items).unwrap_or(JsValue::NULL)
    }

    pub fn collect_user_words_info(&self) -> JsValue {
        // [name, definition, description-or-null, is-builtin-shadow]
        let rows: Vec<(String, String, Option<String>, bool)> = self
            .inner
            .user_words()
            .map(|w| (w.name.clone(), w.definition.clone(), None, false))
            .collect();
        serde_wasm_bindgen::to_value(&rows).unwrap_or(JsValue::NULL)
    }

    pub fn collect_core_words_info(&self) -> JsValue {
        let rows: Vec<(&'static str, &'static str, &'static str)> = vec![
            ("+", "Add the top two numbers.", "a b +"),
            ("-", "Subtract the top from the next.", "a b -"),
            ("*", "Multiply the top two numbers.", "a b *"),
            ("/", "Divide the next by the top.", "a b /"),
            ("DUP", "Duplicate the top of the stack.", "a DUP"),
            ("DROP", "Discard the top of the stack.", "a DROP"),
            ("SWAP", "Swap the top two stack items.", "a b SWAP"),
            ("OVER", "Copy the second item on top.", "a b OVER"),
            ("NIL", "Push a Nil bubble.", "NIL"),
            ("NIL?", "Test the top for Nil (1 or 0).", "x NIL?"),
            (".", "Print the top value to output.", "x ."),
            ("DEF", "Define a user word from the rest of the line.", "DEF NAME body"),
            ("DEL", "Delete a previously defined user word.", "DEL NAME"),
        ];
        serde_wasm_bindgen::to_value(&rows).unwrap_or(JsValue::NULL)
    }

    pub fn collect_core_word_aliases_info(&self) -> JsValue {
        let rows: Vec<(&'static str, &'static str, &'static str, &'static str)> = vec![
            ("ADD", "+", "Alias for +.", "a b ADD"),
            ("SUB", "-", "Alias for -.", "a b SUB"),
            ("MUL", "*", "Alias for *.", "a b MUL"),
            ("DIV", "/", "Alias for /.", "a b DIV"),
        ];
        serde_wasm_bindgen::to_value(&rows).unwrap_or(JsValue::NULL)
    }

    pub fn collect_input_helper_words_info(&self) -> JsValue {
        let rows: Vec<(&'static str, &'static str)> = vec![
            ("+", "+ "),
            ("-", "- "),
            ("*", "* "),
            ("/", "/ "),
            ("DUP", "DUP "),
            ("DROP", "DROP "),
            ("SWAP", "SWAP "),
            ("OVER", "OVER "),
            ("NIL", "NIL "),
            ("NIL?", "NIL? "),
            (".", ". "),
            ("DEF", "DEF "),
            ("DEL", "DEL "),
        ];
        serde_wasm_bindgen::to_value(&rows).unwrap_or(JsValue::NULL)
    }

    pub fn lookup_word_definition(&self, name: &str) -> Option<String> {
        let upper = name.to_ascii_uppercase();
        self.inner
            .user_words()
            .find(|w| w.name == upper)
            .map(|w| w.definition.clone())
    }

    pub fn restore_stack(&mut self, _stack_js: JsValue) {
        // Phase 1: stack persistence is delegated to the TS shell's snapshot;
        // restoring is a no-op pending a stable protocol encoding.
    }

    pub fn restore_user_words(&mut self, words: JsValue) {
        let parsed: Vec<RestoreUserWord> = serde_wasm_bindgen::from_value(words).unwrap_or_default();
        for w in parsed {
            if let Some(def) = w.definition {
                let _ = self.inner.execute(&format!("DEF {} {}", w.name, def));
            }
        }
    }

    pub fn remove_word(&mut self, name: &str) {
        let _ = self.inner.execute(&format!("DEL {}", name));
    }

    pub fn push_json_string(&mut self, _json: &str) -> JsValue {
        let payload = serde_json::json!({
            "status": "ERROR",
            "message": "push_json_string is not available in Phase 1",
        });
        serde_wasm_bindgen::to_value(&payload).unwrap_or(JsValue::NULL)
    }

    pub fn collect_imported_modules(&self) -> JsValue {
        let empty: Vec<String> = Vec::new();
        serde_wasm_bindgen::to_value(&empty).unwrap_or(JsValue::NULL)
    }

    pub fn collect_module_words_info(&self, _module_name: &str) -> JsValue {
        let empty: Vec<(String, Option<String>)> = Vec::new();
        serde_wasm_bindgen::to_value(&empty).unwrap_or(JsValue::NULL)
    }

    pub fn collect_module_sample_words_info(&self, _module_name: &str) -> JsValue {
        let empty: Vec<(String, Option<String>)> = Vec::new();
        serde_wasm_bindgen::to_value(&empty).unwrap_or(JsValue::NULL)
    }

    pub fn collect_dictionary_dependencies(&self) -> JsValue {
        let empty: Vec<(String, Vec<String>, Vec<String>)> = Vec::new();
        serde_wasm_bindgen::to_value(&empty).unwrap_or(JsValue::NULL)
    }

    pub fn restore_imported_modules(&mut self, _modules: JsValue) {}

    pub fn set_execution_mode(&mut self, _mode: &str) {}

    pub fn get_execution_mode(&self) -> JsValue {
        JsValue::from_str("greedy")
    }

    pub fn collect_hedged_trace(&self) -> JsValue {
        let empty: Vec<String> = Vec::new();
        serde_wasm_bindgen::to_value(&empty).unwrap_or(JsValue::NULL)
    }
}

impl Default for AjisaiInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

fn protocol_value(v: &Value) -> ProtocolValue {
    match v {
        Value::Nil => ProtocolValue {
            kind: "nil",
            value: ProtocolValuePayload::Nil("Nil".to_string()),
            continued_fraction: "Nil".to_string(),
            display_hint: "nil",
            semantics: ProtocolValueSemantics {
                semantic_kind: "absence",
                shape: "absence",
                capabilities: vec!["nilPassthrough", "displayable", "aiExplainable"],
                origin: "literal",
            },
        },
        Value::Number(cf) => {
            let (p, q) = cf.to_ratio().expect("non-Nil CF must yield a ratio");
            ProtocolValue {
                kind: "number",
                value: ProtocolValuePayload::Number(Fraction {
                    numerator: p.to_string(),
                    denominator: q.to_string(),
                }),
                continued_fraction: cf.nested_display(),
                display_hint: "number",
                semantics: ProtocolValueSemantics {
                    semantic_kind: "number",
                    shape: "scalar",
                    capabilities: vec!["numeric", "exactNumeric", "stackItem", "displayable", "aiExplainable"],
                    origin: "literal",
                },
            }
        }
    }
}

fn error_payload(e: AjisaiError) -> ExecuteResult {
    ExecuteResult {
        status: "ERROR",
        output: None,
        message: Some(e.summary),
        detail: Some(e.detail),
        diagnosis: Some(e.diagnosis),
        error: Some(true),
    }
}
