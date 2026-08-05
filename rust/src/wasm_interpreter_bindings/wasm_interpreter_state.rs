use super::wasm_value_conversion::{value_to_js, UserWordData};
use super::{set_js_prop, AjisaiInterpreter};
use crate::builtins;
use crate::interpreter;
use crate::interpreter::debug_diagnosis::DebugDiagnosis;
use crate::tokenizer;
use crate::types::arena::{arena_to_value, json_to_arena_node, ValueArena};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

fn diagnosis_to_js(diagnosis: &DebugDiagnosis) -> JsValue {
    let obj = js_sys::Object::new();

    set_js_prop(&obj, "when", &(diagnosis.when.as_protocol_str().into()));
    set_js_prop(&obj, "why", &(diagnosis.why.as_protocol_str().into()));
    set_js_prop(&obj, "summary", &(diagnosis.summary.clone().into()));

    let where_obj = js_sys::Object::new();
    set_js_prop(
        &where_obj,
        "kind",
        &(diagnosis.where_.kind.as_protocol_str().into()),
    );
    if let Some(word) = &diagnosis.where_.word {
        set_js_prop(&where_obj, "word", &(word.clone().into()));
    }
    if let Some(dictionary) = &diagnosis.where_.dictionary {
        set_js_prop(&where_obj, "dictionary", &(dictionary.clone().into()));
    }
    set_js_prop(&obj, "where", &where_obj.into());

    let evidence_arr = js_sys::Array::new();
    for item in &diagnosis.evidence {
        evidence_arr.push(&JsValue::from_str(item));
    }
    set_js_prop(&obj, "evidence", &evidence_arr.into());

    let checks_arr = js_sys::Array::new();
    for c in &diagnosis.next_checks {
        let check_obj = js_sys::Object::new();
        set_js_prop(&check_obj, "label", &(c.label.clone().into()));
        set_js_prop(&check_obj, "detail", &(c.detail.clone().into()));
        checks_arr.push(&check_obj);
    }
    set_js_prop(&obj, "nextChecks", &checks_arr.into());

    obj.into()
}

#[wasm_bindgen]
impl AjisaiInterpreter {
    #[wasm_bindgen]
    pub fn collect_stack(&self) -> JsValue {
        let js_array = js_sys::Array::new();
        // Keep the WASM boundary on the Phase 4 `(value, role)` façade rather
        // than independently indexing the legacy value and role vectors.
        // The `Stack` owns each value with its role in lockstep, so iterating
        // its slots yields aligned `(value, role)` observations by construction
        // — no snapshot type and no alignment assertion are needed.
        for (value, role) in self.interpreter.get_stack().iter_slots() {
            js_array.push(&value_to_js(value, Some(role)));
        }
        js_array.into()
    }

    #[wasm_bindgen]
    pub fn collect_user_words_info(&self) -> JsValue {
        let js_array = js_sys::Array::new();

        let mut names: Vec<&String> = self.interpreter.user_words.keys().collect();
        names.sort();
        for name in names {
            let is_protected = self
                .interpreter
                .dependents
                .get(name)
                .is_some_and(|deps| !deps.is_empty());

            let item = js_sys::Array::new();
            // The dictionary slot stays in the shape for the host, which reads
            // a fixed triple; there is one User tier, so it is constant.
            item.push(&"USER".into());
            item.push(&name.clone().into());
            item.push(&is_protected.into());

            js_array.push(&item);
        }

        js_array.into()
    }

    /// Content identity (Section 8.6) of each user word, as `[fqName, id]`
    /// pairs. The host uses these to deduplicate identical definitions on
    /// import and to key shared word groups by content rather than by name.
    #[wasm_bindgen]
    pub fn collect_word_identities(&self) -> JsValue {
        let js_array = js_sys::Array::new();
        let mut names: Vec<&String> = self.interpreter.user_words.keys().collect();
        names.sort();
        for name in names {
            if let Some(id) = self.interpreter.word_identity(name) {
                let item = js_sys::Array::new();
                item.push(&name.clone().into());
                item.push(&id.clone().into());
                js_array.push(&item);
            }
        }
        js_array.into()
    }

    pub(crate) fn collect_user_words_for_state(&self) -> JsValue {
        let mut names: Vec<String> = self.interpreter.user_words.keys().cloned().collect();
        names.sort();
        let words_info: Vec<UserWordData> = names
            .into_iter()
            .map(|name| UserWordData {
                // Kept in the serialized shape for older snapshots to decode
                // against; there is one User tier, so it no longer selects.
                dictionary: None,
                definition: self.interpreter.lookup_word_definition_tokens(&name),
                name,
            })
            .collect();
        to_value(&words_info).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn collect_core_words_info(&self) -> JsValue {
        to_value(&builtins::collect_core_builtin_definitions()).unwrap_or(JsValue::NULL)
    }

    /// Returns the canonical Core-listed words.
    ///
    /// Tuple shape: `(name, description, syntax)` — same as
    /// `collect_core_words_info` so the GUI can render either list with the
    /// same code path.
    #[wasm_bindgen]
    pub fn collect_core_listed_words_info(&self) -> JsValue {
        let entries: Vec<(String, String, String)> = builtins::collect_core_builtin_definitions()
            .into_iter()
            .map(|(n, d, s)| (n.to_string(), d.to_string(), s.to_string()))
            .collect();

        to_value(&entries).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn collect_builtin_word_registry(&self) -> JsValue {
        to_value(&crate::coreword_registry::get_builtin_word_registry()).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn is_safe_preview_word(&self, name: &str) -> bool {
        crate::coreword_registry::is_safe_preview_word(name)
    }

    #[wasm_bindgen]
    pub fn collect_core_word_aliases_info(&self) -> JsValue {
        to_value(&crate::core_word_aliases::collect_core_word_aliases()).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn collect_input_helper_words_info(&self) -> JsValue {
        to_value(&crate::core_word_aliases::collect_input_helper_words()).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn lookup_word_definition(&self, name: &str) -> JsValue {
        let upper_name = name.to_uppercase();
        self.interpreter
            .lookup_word_definition_tokens(&upper_name)
            .map(|def| JsValue::from_str(&def))
            .unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn remove_word(&mut self, name: &str) {
        let upper_name = name.to_uppercase();
        if self.interpreter.user_words.remove(&upper_name).is_some() {
            let _ = self.interpreter.rebuild_dependencies();
        }
    }

    /// Discard every value on the stack, leaving the dictionary, the output
    /// and every other piece of session state untouched.
    ///
    /// A REPL keeps its stack between runs, which is right, and until now the
    /// only way to get rid of a leftover intermediate was the full reset — and
    /// that takes the User dictionary with it. Clearing values is not a
    /// language operation (no Word does it, and none should: a program's own
    /// values are its own business), so it belongs here, on the host, where the
    /// person at the keyboard is the one asking.
    #[wasm_bindgen]
    pub fn clear_stack(&mut self) {
        self.interpreter
            .update_stack_with_hints(Vec::new(), Vec::new());
    }

    /// The one stack format persistence accepts (SPEC §2.3). Unlike
    /// `collect_stack`, which serializes the *observation* wire format (a
    /// CodeBlock shows as `nil`, an ExactScalar as a marked rational
    /// approximation), this captures the exact value so `restore_stack_snapshot`
    /// returns identical values. The two surfaces are deliberately distinct:
    /// observation is lossy-but-honest, persistence is lossless. Restoring the
    /// observation format is not offered — it would silently downgrade exact
    /// values. The payload is an opaque JSON string produced by
    /// `crate::types::value_persist`.
    #[wasm_bindgen]
    pub fn snapshot_stack(&self) -> Result<String, String> {
        crate::types::value_persist::encode_stack(self.interpreter.get_stack().iter_slots())
    }

    /// Restore a stack from a `snapshot_stack` payload, reinstating exact
    /// values (CodeBlock, ExactScalar, …) and their stack-position roles.
    #[wasm_bindgen]
    pub fn restore_stack_snapshot(&mut self, snapshot_json: &str) -> Result<(), String> {
        let slots = crate::types::value_persist::decode_stack(snapshot_json)?;
        let (stack, hints): (Vec<_>, Vec<_>) = slots.into_iter().unzip();
        self.interpreter.update_stack_with_hints(stack, hints);
        Ok(())
    }

    #[wasm_bindgen]
    pub fn update_input_buffer(&mut self, _text: String) {}

    /// Inject the host-received bytes for a serial port (Section 9.4). Replaces
    /// any buffer previously set for this port id and clears the port's
    /// disconnected flag. `SERIAL@READ` drains this buffer.
    #[wasm_bindgen]
    pub fn update_serial_inbox(&mut self, _port_id: String, _bytes: Vec<u8>) {}

    /// Mark a serial port as disconnected by the host. Once its inbox is empty,
    /// `SERIAL@READ` projects `NilReason::PortDisconnected`.
    #[wasm_bindgen]
    pub fn mark_serial_disconnected(&mut self, _port_id: String) {}

    /// Clear all injected serial receive buffers and disconnected flags.
    #[wasm_bindgen]
    pub fn clear_serial_inboxes(&mut self) {}

    #[wasm_bindgen]
    pub fn extract_io_output_buffer(&self) -> String {
        String::new()
    }

    #[wasm_bindgen]
    pub fn clear_io_output_buffer(&mut self) {}

    /// Override the execution step budget (water level, SPEC §5.3) for
    /// subsequent executions. A runtime safety control, not a language
    /// semantic: the host may raise or lower it; never calling this keeps
    /// the default (100,000). A zero or non-positive value is ignored so a
    /// malformed host call cannot disable the safety budget entirely.
    #[wasm_bindgen]
    pub fn set_max_execution_steps(&mut self, steps: usize) {
        if steps > 0 {
            self.interpreter.set_max_execution_steps(steps);
        }
    }

    #[wasm_bindgen]
    pub fn collect_error_flow_trace(&mut self) -> JsValue {
        let arr = js_sys::Array::new();
        for event in self.interpreter.drain_error_flow_trace() {
            let obj = js_sys::Object::new();
            set_js_prop(&obj, "kind", &(event.kind.as_protocol_str().into()));
            if let Some(word) = event.word {
                set_js_prop(&obj, "word", &(word.into()));
            }
            if let Some(absence) = event.absence {
                let absence_obj = js_sys::Object::new();
                if let Some(reason) = &absence.reason {
                    set_js_prop(&absence_obj, "reason", &(reason.as_protocol_str().into()));
                }
                set_js_prop(
                    &absence_obj,
                    "origin",
                    &(absence.origin.as_protocol_str().into()),
                );
                set_js_prop(
                    &absence_obj,
                    "recoverability",
                    &(absence.recoverability.as_protocol_str().into()),
                );
                if let Some(diagnosis) = &absence.diagnosis {
                    set_js_prop(&absence_obj, "diagnosis", &diagnosis_to_js(diagnosis));
                }
                set_js_prop(&obj, "absence", &absence_obj.into());
            }
            set_js_prop(
                &obj,
                "stackLenBefore",
                &((event.stack_len_before as u32).into()),
            );
            set_js_prop(
                &obj,
                "stackLenAfter",
                &((event.stack_len_after as u32).into()),
            );
            set_js_prop(&obj, "message", &(event.message.into()));
            if let Some(diagnosis) = event.diagnosis {
                set_js_prop(&obj, "diagnosis", &diagnosis_to_js(&diagnosis));
            }
            arr.push(&obj);
        }
        arr.into()
    }

    #[wasm_bindgen]
    pub fn push_json_string(&mut self, json_string: &str) -> Result<JsValue, JsValue> {
        let obj = js_sys::Object::new();

        match serde_json::from_str::<serde_json::Value>(json_string) {
            Ok(json_val) => {
                let mut arena = ValueArena::new();
                match json_to_arena_node(&mut arena, json_val) {
                    Ok(root) => {
                        self.interpreter.stack.push(arena_to_value(&arena, root));
                        set_js_prop(&obj, "status", &("OK".into()));
                    }
                    Err(e) => {
                        set_js_prop(&obj, "status", &("ERROR".into()));
                        set_js_prop(&obj, "message", &(e.to_string().into()));
                    }
                }
            }
            Err(e) => {
                set_js_prop(&obj, "status", &("ERROR".into()));
                set_js_prop(
                    &obj,
                    "message",
                    &(format!("JSON parse error: {}", e).into()),
                );
            }
        }
        Ok(obj.into())
    }

    #[wasm_bindgen]
    pub fn restore_user_words(&mut self, words_js: JsValue) -> Result<(), String> {
        let words: Vec<UserWordData> = serde_wasm_bindgen::from_value(words_js)
            .map_err(|e| format!("Failed to deserialize words: {}", e))?;

        // Defer per-word identity recomputation during the bulk restore and
        // recompute once below via rebuild_dependencies. This turns O(N^2)
        // identity hashing on import into O(N). The flag is always cleared,
        // even on error, so later interactive definitions recompute normally.
        self.interpreter.defer_identity_recompute = true;
        let restore_result = self.define_restored_words(words);
        self.interpreter.defer_identity_recompute = false;
        restore_result?;

        self.interpreter
            .rebuild_dependencies()
            .map_err(|e| e.to_string())?;

        let _ = self.interpreter.collect_output();

        Ok(())
    }

    fn define_restored_words(&mut self, words: Vec<UserWordData>) -> Result<(), String> {
        for word in words {
            // A restored word's saved `dictionary` label is legacy state: the
            // dictionary has two tiers and User is one of them, so every
            // restored definition lands in the same place.
            let definition = match &word.definition {
                Some(def) if !def.is_empty() => def.clone(),
                _ => continue,
            };

            let tokens = tokenizer::tokenize(&definition)
                .map_err(|e| format!("Failed to tokenize definition for {}: {}", word.name, e))?;

            interpreter::execute_def::op_def_inner(&mut self.interpreter, &word.name, &tokens)
                .map_err(|e| format!("Failed to restore word {}: {}", word.name, e))?;
        }
        Ok(())
    }
}
