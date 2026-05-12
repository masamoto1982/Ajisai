//! WASM bindings for the Ajisai interpreter.
//!
//! The TypeScript shell expects a minimal protocol surface defined in
//! `src/wasm-interpreter-types.ts`. The bindings expose Stack, Register,
//! and DEF/DEL-managed user words; richer subsystems (modules, hedged
//! execution, etc.) return empty/no-op responses pending later phases.

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
struct TensorPayload {
    shape: Vec<usize>,
    data: Vec<Fraction>,
    #[serde(rename = "displayHint", skip_serializing_if = "Option::is_none")]
    display_hint: Option<String>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum ProtocolValuePayload {
    Number(Fraction),
    Nil(String),
    Tensor(TensorPayload),
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
    display_hint: String,
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

    pub fn collect_register(&self) -> JsValue {
        let v = protocol_value(self.inner.register());
        serde_wasm_bindgen::to_value(&v).unwrap_or(JsValue::NULL)
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
            ("ADD", "Add the top two numbers.", "a b ADD"),
            ("SUB", "Subtract the top from the next.", "a b SUB"),
            ("MUL", "Multiply the top two numbers.", "a b MUL"),
            ("DIV", "Divide the next by the top.", "a b DIV"),
            ("DUP", "Duplicate the top of the stack.", "a DUP"),
            ("DROP", "Discard the top of the stack.", "a DROP"),
            ("SWAP", "Swap the top two stack items.", "a b SWAP"),
            ("OVER", "Copy the second item on top.", "a b OVER"),
            ("STORE", "Move the top into the Register.", "a STORE"),
            ("RECALL", "Push the Register onto the stack and clear it.", "RECALL"),
            ("PEEK", "Copy the Register onto the stack.", "PEEK"),
            ("EQ", "Push 1 when the top two are equal, else 0; Nil if either is Nil.", "a b EQ"),
            ("NE", "Push 1 when unequal, else 0.", "a b NE"),
            ("LT", "Push 1 when the next is less than the top.", "a b LT"),
            ("LE", "Push 1 when the next is less than or equal to the top.", "a b LE"),
            ("GT", "Push 1 when the next is greater than the top.", "a b GT"),
            ("GE", "Push 1 when the next is greater than or equal to the top.", "a b GE"),
            ("AND", "Three-valued AND on the top two.", "a b AND"),
            ("OR", "Three-valued OR on the top two.", "a b OR"),
            ("NOT", "Three-valued NOT on the top.", "a NOT"),
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
            ("+", "ADD", "Symbolic sugar for ADD.", "a b +"),
            ("-", "SUB", "Symbolic sugar for SUB.", "a b -"),
            ("*", "MUL", "Symbolic sugar for MUL.", "a b *"),
            ("/", "DIV", "Symbolic sugar for DIV.", "a b /"),
            (">R", "STORE", "Symbolic sugar for STORE.", "a >R"),
            ("R>", "RECALL", "Symbolic sugar for RECALL.", "R>"),
            ("R@", "PEEK", "Symbolic sugar for PEEK.", "R@"),
            ("=", "EQ", "Symbolic sugar for EQ.", "a b ="),
            ("<>", "NE", "Symbolic sugar for NE.", "a b <>"),
            ("<", "LT", "Symbolic sugar for LT.", "a b <"),
            ("<=", "LE", "Symbolic sugar for LE.", "a b <="),
            (">", "GT", "Symbolic sugar for GT.", "a b >"),
            (">=", "GE", "Symbolic sugar for GE.", "a b >="),
            ("&", "AND", "Symbolic sugar for AND.", "a b &"),
            ("|", "OR", "Symbolic sugar for OR.", "a b |"),
            ("!", "NOT", "Symbolic sugar for NOT.", "a !"),
        ];
        serde_wasm_bindgen::to_value(&rows).unwrap_or(JsValue::NULL)
    }

    pub fn collect_input_helper_words_info(&self) -> JsValue {
        let rows: Vec<(&'static str, &'static str)> = vec![
            ("ADD", "ADD "),
            ("SUB", "SUB "),
            ("MUL", "MUL "),
            ("DIV", "DIV "),
            ("DUP", "DUP "),
            ("DROP", "DROP "),
            ("SWAP", "SWAP "),
            ("OVER", "OVER "),
            ("STORE", "STORE "),
            ("RECALL", "RECALL "),
            ("PEEK", "PEEK "),
            ("EQ", "EQ "),
            ("NE", "NE "),
            ("LT", "LT "),
            ("LE", "LE "),
            ("GT", "GT "),
            ("GE", "GE "),
            ("AND", "AND "),
            ("OR", "OR "),
            ("NOT", "NOT "),
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
            display_hint: "nil".to_string(),
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
                display_hint: "number".to_string(),
                semantics: ProtocolValueSemantics {
                    semantic_kind: "number",
                    shape: "scalar",
                    capabilities: vec!["numeric", "exactNumeric", "stackItem", "displayable", "aiExplainable"],
                    origin: "literal",
                },
            }
        }
        Value::Tensor { shape, data, display_hint } => {
            let payload_data: Vec<Fraction> = data
                .iter()
                .map(|cf| {
                    let (p, q) = cf.to_ratio().expect("non-Nil CF must yield a ratio");
                    Fraction {
                        numerator: p.to_string(),
                        denominator: q.to_string(),
                    }
                })
                .collect();
            let is_string = display_hint.as_deref() == Some("string");
            let hint = display_hint.clone().unwrap_or_else(|| "tensor".to_string());
            let semantic_kind = if is_string { "string" } else { "tensor" };
            ProtocolValue {
                kind: "tensor",
                value: ProtocolValuePayload::Tensor(TensorPayload {
                    shape: shape.clone(),
                    data: payload_data,
                    display_hint: display_hint.clone(),
                }),
                continued_fraction: v.nested_display(),
                display_hint: hint,
                semantics: ProtocolValueSemantics {
                    semantic_kind,
                    shape: if is_string { "string" } else { "tensor" },
                    capabilities: if is_string {
                        vec!["string", "stackItem", "displayable", "aiExplainable"]
                    } else {
                        vec!["tensor", "stackItem", "displayable", "aiExplainable"]
                    },
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
