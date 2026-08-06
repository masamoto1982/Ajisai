use super::{set_js_prop, AjisaiInterpreter};
use crate::tokenizer;
use crate::types::ExecutionLine;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl AjisaiInterpreter {
    #[wasm_bindgen]
    pub async fn execute(&mut self, code: &str) -> Result<JsValue, JsValue> {
        self.interpreter.definition_to_load = None;
        self.interpreter.documentation_to_show = None;
        let obj = js_sys::Object::new();

        match self.interpreter.execute(code).await {
            Ok(()) => {
                set_js_prop(&obj, "status", &("OK".into()));
                let output = self.interpreter.collect_output();
                set_js_prop(&obj, "output", &(output.clone().into()));
                set_js_prop(&obj, "stack", &(self.collect_stack()));
                set_js_prop(&obj, "userWords", &(self.collect_user_words_for_state()));

                if let Some(def_str) = self.interpreter.definition_to_load.take() {
                    set_js_prop(&obj, "definition_to_load", &(def_str.into()));
                }
                if let Some(doc) = self.interpreter.documentation_to_show.take() {
                    set_js_prop(&obj, "documentation", &(doc.into()));
                }
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
                set_js_prop(&obj, "errorFlowTrace", &(self.collect_error_flow_trace()));
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
                set_js_prop(&obj, "errorFlowTrace", &(self.collect_error_flow_trace()));
            }
            Err(e) => {
                self.step_mode = false;
                set_js_prop(&obj, "status", &("ERROR".into()));
                set_js_prop(&obj, "message", &(e.to_string().into()));
                set_js_prop(&obj, "error", &(true.into()));
                set_js_prop(&obj, "hasMore", &(false.into()));
                // Same as the whole-program path: a failing step keeps what it
                // printed, and draining the buffer keeps it out of the next run.
                set_js_prop(&obj, "output", &(self.interpreter.collect_output().into()));
                set_js_prop(&obj, "errorFlowTrace", &(self.collect_error_flow_trace()));
            }
        }

        obj.into()
    }

    #[wasm_bindgen]
    pub fn reset(&mut self) -> JsValue {
        self.reset_runtime()
    }

    /// Compatibility alias for [`Self::reset`].
    #[wasm_bindgen]
    pub fn reset_session(&mut self) -> JsValue {
        self.reset_runtime()
    }

    fn reset_runtime(&mut self) -> JsValue {
        let obj = js_sys::Object::new();

        self.step_mode = false;
        self.step_tokens.clear();
        self.step_position = 0;
        self.current_step_code.clear();

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
