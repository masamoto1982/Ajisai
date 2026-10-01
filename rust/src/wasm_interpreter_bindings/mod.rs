// `wasm-bindgen` expands `#[wasm_bindgen]` items into generated glue that
// contains `unsafe`, so this module re-permits `unsafe_code` over the crate-root
// `#![deny(unsafe_code)]` (structural-memory-safety roadmap Phase 4). No
// hand-written `unsafe` lives here; the allow only covers macro-generated code.
#![allow(unsafe_code)]

//! The browser playground's stateful session (`AjisaiInterpreter`), the value
//! conversion into the `JsValue` shapes it returns, and the one-shot agent
//! entry points. Dictionary and stack-state methods live in
//! `wasm_interpreter_state`.
//!
//! `js_sys::Reflect::set(...).unwrap()` in `set_js_prop`: every target is a
//! plain JS object this module just created with `js_sys::Object::new()`, so
//! the usual failure causes (a Proxy handler, a frozen object) cannot occur.
//! Should one ever fail anyway, console_error_panic_hook puts the stack trace
//! in the browser console.

use crate::agent::api;
use crate::agent::report::{ai_payload_json, diagnosis_json, failed_run_diagnosis};
use crate::error::ErrorCategory;
use crate::interpreter::Interpreter;
use crate::types::value_protocol::{value_to_protocol, ProtocolNode, ProtocolValue};
use crate::types::Value;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

mod wasm_interpreter_state;

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
}

fn set_js_prop(obj: &js_sys::Object, key: &str, value: &JsValue) {
    js_sys::Reflect::set(obj, &JsValue::from_str(key), value).unwrap();
}

/// A `serde_json` rendering as the plain JS value it describes. Every
/// structured payload this boundary returns (diagnoses, the error-flow trace,
/// a value's `semantics`) is rendered by the CLI's own serializer and
/// converted here, so the GUI reads what an agent reads.
fn json_to_js(value: serde_json::Value) -> JsValue {
    use serde::Serialize as _;
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .expect("a serde_json value always converts to a JS value")
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
        }
    }

    /// The resource ceilings this interpreter is actually running under, as
    /// JSON, under the same names every other Ajisai host publishes them by —
    /// literally the same, since `interpreter::limit_profile` is the one place
    /// the ceiling set is enumerated and the receipt reads it too. This
    /// sentence used to be a claim with nothing checking it.
    ///
    /// LANG.MACHINE.LIMITS makes limits a host safety control rather than value
    /// semantics, so two conforming hosts legitimately disagree about them —
    /// and they do: the playground runs the interpreter defaults while the MCP
    /// agent profile is an order of magnitude tighter. That is only a trap for
    /// someone who prototypes here and runs there while neither host says what
    /// it applies. Read from the live interpreter rather than from a constant,
    /// so what is displayed is what is enforced.
    #[wasm_bindgen]
    pub fn host_profile(&self) -> String {
        let limits = self.interpreter.runtime_limits();
        let step_limit = self.interpreter.max_execution_steps();
        serde_json::json!({
            "profile": "browser-playground",
            "limits": crate::interpreter::limit_profile::to_json(limits, step_limit),
        })
        .to_string()
    }
    #[wasm_bindgen]
    pub async fn execute(&mut self, code: &str) -> Result<JsValue, JsValue> {
        let obj = js_sys::Object::new();

        match self.interpreter.execute(code).await {
            Ok(()) => {
                set_js_prop(&obj, "status", &("OK".into()));
                let output = self.interpreter.collect_output();
                // This host reads the text buffer; the structured effect log
                // carries the same emissions and is drained with it, so a
                // long session does not keep every payload it ever printed.
                self.interpreter.take_host_effects();
                set_js_prop(&obj, "output", &(output.into()));
                set_js_prop(&obj, "stack", &(self.collect_stack()));
                set_js_prop(&obj, "userWords", &(self.collect_user_words_for_state()));
                set_js_prop(&obj, "errorFlowTrace", &(self.collect_error_flow_trace()));
            }
            Err(e) => {
                let error_msg = e.to_string();
                set_js_prop(&obj, "status", &("ERROR".into()));
                set_js_prop(&obj, "message", &(error_msg.into()));
                set_js_prop(&obj, "error", &(true.into()));
                // Whatever the program printed before it failed is part of the
                // report, not something the failure erases: `PRINT` is the
                // language's trace tool and the run that ends in an error is
                // exactly the run whose trace is wanted. The native CLI has
                // always emitted it (`render_completed_run` carries `output`
                // down the Err arm too); only this host dropped it, so a
                // multi-line program that failed late showed the error and
                // nothing else. Draining the buffer here also stops the
                // orphaned output from surfacing at the head of the next run.
                set_js_prop(&obj, "output", &(self.interpreter.collect_output().into()));
                self.interpreter.take_host_effects();
                // The failure's category travels where the CLI puts it,
                // `aiDiagnostic.category`, built by the same functions — never
                // parsed back out of `message`, which is display text. So does
                // its diagnosis: the top-level `diagnosis`, the one copy, which
                // the trace's error event no longer repeats.
                let trace = self.interpreter.drain_error_flow_trace();
                let diagnosis = failed_run_diagnosis(&self.interpreter, &e, &trace);
                let category = ErrorCategory::from_error(&e);
                let ai = ai_payload_json(&diagnosis.ai_payload(category.as_ref()));
                set_js_prop(&obj, "diagnosis", &json_to_js(diagnosis_json(&diagnosis)));
                set_js_prop(&obj, "aiDiagnostic", &json_to_js(ai));
                set_js_prop(&obj, "errorFlowTrace", &error_flow_trace_to_js(&trace));
                // An ERROR result carries no `userWords`, which is the
                // protocol's way of saying the run committed nothing to the
                // dictionary. The run said otherwise while it was going: every
                // `DEF` it reached printed `Defined word:`, and those lines are
                // in the `output` above, still claiming a Word the host is
                // about to discard. Naming what was discarded is what turns the
                // report back into a true one.
                let changes = self.interpreter.dictionary_changes_this_run();
                if !changes.is_empty() {
                    let names = js_sys::Array::new();
                    for name in changes {
                        names.push(&JsValue::from_str(name));
                    }
                    set_js_prop(&obj, "discardedDictionaryChanges", &names);
                }
            }
        }
        Ok(obj.into())
    }

    #[wasm_bindgen]
    pub fn reset(&mut self) -> JsValue {
        self.reset_runtime()
    }

    fn reset_runtime(&mut self) -> JsValue {
        let obj = js_sys::Object::new();

        let outcome = self.interpreter.execute_reset();

        match outcome {
            Ok(()) => {
                set_js_prop(&obj, "status", &("OK".into()));
                set_js_prop(&obj, "output", &("System reinitialized.".into()));
                set_js_prop(&obj, "stack", &(self.collect_stack()));
                set_js_prop(&obj, "userWords", &(self.collect_user_words_for_state()));
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

/// Not part of the exported surface: every run folds the trace into its own
/// result envelope (`errorFlowTrace`), so no host calls this directly.
impl AjisaiInterpreter {
    pub(crate) fn collect_error_flow_trace(&mut self) -> JsValue {
        let events = self.interpreter.drain_error_flow_trace();
        error_flow_trace_to_js(&events)
    }
}

// Rendered by the CLI's own serializer and converted, so the trace a GUI
// reads is the one an agent reads. A hand-built copy here was the third
// spelling of a diagnosis (beside the CLI's and the value node's), and it had
// already dropped a resource limit's `progress`.
fn error_flow_trace_to_js(
    events: &[crate::interpreter::error_flow_trace::ErrorFlowEvent],
) -> JsValue {
    json_to_js(serde_json::Value::Array(
        events
            .iter()
            .map(crate::agent::report::error_flow_event_json)
            .collect(),
    ))
}

#[derive(Serialize, Deserialize)]
pub(crate) struct UserWordData {
    pub(crate) name: String,
    pub(crate) definition: Option<String>,
    /// The `#:contract`-derived hover text (see `execute_def::set_word_
    /// description`), round-tripped through save/export so it survives a
    /// restore rather than existing only for the session that typed it.
    #[serde(default)]
    pub(crate) description: Option<String>,
}

// The pure Value -> protocol mapping (`ProtocolNode`,
// `value_to_protocol`) lives in `crate::types::value_protocol` so the native
// CLI shares the exact same wire format. Extracting it out of the `JsValue`
// glue also lets the entire decision be unit / MC/DC / property tested
// natively (AQ-REQ-003, `types/value_protocol_tests.rs`), with
// `protocol_to_js` reduced to a mechanical shim.

/// Mechanical shim: render a `ProtocolNode` into the `JsValue` the GUI
/// receives. Carries no decision logic — every behavioral choice lives in
/// `value_to_protocol`, which is verified natively.
///
/// The `semantics` bag is the one rendering both hosts share
/// (`agent::report::semantics_json`), converted rather than rebuilt: this
/// boundary used to build it by hand, field by field, and the two copies had
/// drifted — the WASM absence dropped `origin` and `recoverability`, and its
/// diagnosis omitted `progress` from a resource limit — two spellings of one
/// protocol, where LANG.OBSERVATION.PROTOCOL promises one.
fn protocol_to_js(node: &ProtocolNode) -> JsValue {
    let obj = js_sys::Object::new();
    set_js_prop(
        &obj,
        "semantics",
        &json_to_js(crate::agent::report::semantics_json(&node.semantics)),
    );
    set_js_prop(&obj, "type", &node.type_str.into());
    match &node.value {
        ProtocolValue::Null => set_js_prop(&obj, "value", &JsValue::NULL),
        ProtocolValue::Bool(b) => set_js_prop(&obj, "value", &(*b).into()),
        ProtocolValue::Text(s) => set_js_prop(&obj, "value", &s.clone().into()),
        ProtocolValue::Number {
            numerator,
            denominator,
        } => {
            let num_obj = js_sys::Object::new();
            set_js_prop(&num_obj, "numerator", &numerator.clone().into());
            set_js_prop(&num_obj, "denominator", &denominator.clone().into());
            set_js_prop(&obj, "value", &num_obj.into());
        }
        ProtocolValue::Children(kids) => {
            let arr = js_sys::Array::new();
            for kid in kids {
                arr.push(&protocol_to_js(kid));
            }
            set_js_prop(&obj, "value", &arr.into());
        }
        ProtocolValue::Record { keys, values } => {
            let record_obj = js_sys::Object::new();
            let key_arr = js_sys::Array::new();
            for key in keys {
                key_arr.push(&protocol_to_js(key));
            }
            let value_arr = js_sys::Array::new();
            for value in values {
                value_arr.push(&protocol_to_js(value));
            }
            set_js_prop(&record_obj, "keys", &key_arr.into());
            set_js_prop(&record_obj, "values", &value_arr.into());
            set_js_prop(&obj, "value", &record_obj.into());
        }
    }
    obj.into()
}

fn value_to_js(value: &Value) -> JsValue {
    protocol_to_js(&value_to_protocol(value))
}

// ── One-shot agent entry points ──────────────────────────────────────────
//
// The host-neutral agent boundary (`crate::agent`), returning the same JSON
// envelope the native `ajisai agent <operation>` CLI emits
// (`docs/dev/agent-cli-output-contract.md`), serialized as a JSON string so a
// Node host parses it identically to the native CLI's stdout — no bespoke
// per-host JS object shape, no separate normalizer.
//
// Deliberately separate from `AjisaiInterpreter`: that struct is the browser
// playground's stateful, step-mode session API and keeps its own JS-object
// result shape. These create one fresh interpreter per call, mirroring the
// native CLI's one-process-per-call model.

/// The agent-profile ceilings every one-shot call runs under. `step_limit`
/// overrides the default execution step budget when positive; `0` or omitted
/// keeps the interpreter default.
fn agent_options(step_limit: Option<u32>) -> api::ComputeOptions {
    api::ComputeOptions::agent(step_limit.filter(|&n| n > 0).map(|n| n as usize))
}

/// Execute one Ajisai source document under the same tightened
/// agent-profile runtime limits the native `ajisai agent compute` CLI
/// applies. `step_limit` overrides the default execution step budget when
/// positive; `0` or omitted keeps the interpreter default.
#[wasm_bindgen]
pub async fn agent_compute(source: &str, step_limit: Option<u32>) -> String {
    api::compute(source, agent_options(step_limit))
        .await
        .to_json()
        .to_string()
}

/// Parse and resolve `source` without executing it; also verifies declared
/// `#:contract` declarations conservatively, matching `ajisai agent check`.
#[wasm_bindgen]
pub fn agent_check(source: &str) -> String {
    api::check(source, true).to_json().to_string()
}

/// Infer machine-readable contracts for user-defined Words without
/// executing their bodies, matching `ajisai agent infer-contracts`.
#[wasm_bindgen]
pub fn agent_infer_contracts(source: &str) -> String {
    api::infer_contracts(source).to_json().to_string()
}

/// Predict the finite set of outcome ids `source` could produce without
/// executing it, under the same agent-profile ceilings `agent_compute`
/// applies, matching `ajisai agent outcomes`. `step_limit` as for
/// `agent_compute`.
#[wasm_bindgen]
pub fn agent_predict_outcomes(source: &str, step_limit: Option<u32>) -> String {
    api::predict_outcomes(source, agent_options(step_limit))
        .to_json()
        .to_string()
}
