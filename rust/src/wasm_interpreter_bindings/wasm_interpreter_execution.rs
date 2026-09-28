use super::wasm_interpreter_state::{error_flow_trace_to_js, json_to_js};
use super::{set_js_prop, AjisaiInterpreter};
use crate::agent::report::{ai_payload_json, diagnosis_json};
use crate::agent::run_render::failed_run_diagnosis;
use crate::error::ErrorCategory;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl AjisaiInterpreter {
    #[wasm_bindgen]
    pub async fn execute(&mut self, code: &str) -> Result<JsValue, JsValue> {
        let obj = js_sys::Object::new();

        match self.interpreter.execute(code).await {
            Ok(()) => {
                set_js_prop(&obj, "status", &("OK".into()));
                let output = self.interpreter.collect_output();
                set_js_prop(&obj, "output", &(output.clone().into()));
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
