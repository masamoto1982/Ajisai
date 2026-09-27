use super::wasm_value_conversion::{value_to_js, UserWordData};
use super::AjisaiInterpreter;
use crate::builtins;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl AjisaiInterpreter {
    #[wasm_bindgen]
    pub fn collect_stack(&self) -> JsValue {
        let js_array = js_sys::Array::new();
        for value in self.interpreter.get_stack().iter() {
            js_array.push(&value_to_js(value));
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
                description: self.interpreter.lookup_word_description(&name),
                name,
            })
            .collect();
        to_value(&words_info).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn collect_core_words_info(&self) -> JsValue {
        to_value(&builtins::collect_core_builtin_definitions()).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn lookup_word_definition(&self, name: &str) -> JsValue {
        let upper_name = name.to_uppercase();
        self.interpreter
            .lookup_word_definition_tokens(&upper_name)
            .map(|def| JsValue::from_str(&def))
            .unwrap_or(JsValue::NULL)
    }

    /// A User Word's `#:contract`-derived description, for a host affordance
    /// like the Dictionary panel's hover — never checked against the Word's
    /// actual behavior (that stays a CLI-only, opt-in `check --contract`).
    #[wasm_bindgen]
    pub fn lookup_word_description(&self, name: &str) -> JsValue {
        let upper_name = name.to_uppercase();
        self.interpreter
            .lookup_word_description(&upper_name)
            .map(|desc| JsValue::from_str(&desc))
            .unwrap_or(JsValue::NULL)
    }

    /// Answer the host's lookup of `name` against the current dictionary.
    ///
    /// This is a *query*, not a run. Looking a Word up used to be the Word
    /// `LOOKUP`, which meant asking what `ADD` does went through `execute` and
    /// came back on a side channel that no evaluation rule read. The host asks
    /// here instead, so nothing about a lookup touches the stack, the
    /// dictionary, or the output buffer.
    ///
    /// Returns `{ kind: "documentation" | "definition", text }`, or `NULL` for a
    /// name the dictionary does not hold — the caller reports the unknown name
    /// itself, since it is the one that read it off the input.
    #[wasm_bindgen]
    pub fn resolve_host_lookup(&self, name: &str) -> JsValue {
        use crate::interpreter::host_lookup::{resolve_host_lookup, HostLookup};

        let (kind, text) = match resolve_host_lookup(&self.interpreter, name) {
            Ok(HostLookup::Documentation(text)) => ("documentation", text),
            Ok(HostLookup::Definition(text)) => ("definition", text),
            Err(_) => return JsValue::NULL,
        };

        let obj = js_sys::Object::new();
        super::set_js_prop(&obj, "kind", &JsValue::from_str(kind));
        super::set_js_prop(&obj, "text", &JsValue::from_str(&text));
        obj.into()
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
        self.interpreter.update_stack(crate::types::Stack::new());
    }

    /// The one stack format persistence accepts (LANG.OBSERVATION.FIREWALL). Unlike
    /// `collect_stack`, which serializes the *observation* wire format (a
    /// CodeBlock shows as `nil`, an ExactScalar as a marked rational
    /// approximation), this captures the exact value so `restore_stack_snapshot`
    /// returns identical values. The two surfaces are deliberately distinct:
    /// observation is lossy-but-honest, persistence is lossless. Restoring the
    /// observation format is not offered — it would silently downgrade exact
    /// values. The payload is an opaque JSON string produced by
    /// `crate::types::value_persist`.
    #[wasm_bindgen]
    pub fn snapshot_stack(&self) -> String {
        crate::types::value_persist::encode_stack(self.interpreter.get_stack().iter())
    }

    /// Restore a stack from a `snapshot_stack` payload, reinstating exact
    /// values (CodeBlock, ExactScalar, …).
    #[wasm_bindgen]
    pub fn restore_stack_snapshot(&mut self, snapshot_json: &str) -> Result<(), String> {
        let stack = crate::types::value_persist::decode_stack(snapshot_json)?;
        self.interpreter.update_stack(stack);
        Ok(())
    }

    /// Override the execution step budget (water level, LANG.MACHINE.LIMITS) for
    /// subsequent executions. A runtime safety control, not a language
    /// semantic: the host may raise or lower it; never calling this keeps the
    /// interpreter's own `DEFAULT_MAX_EXECUTION_STEPS`. A zero or non-positive
    /// value is ignored so a malformed host call cannot disable the safety
    /// budget entirely.
    ///
    /// The default's *value* is deliberately not restated here. It is derived
    /// from the host time budget and has already moved once (it was 100,000);
    /// every doc comment that spelled the number out went on claiming the old
    /// one, on both sides of the wasm boundary. The constant is that number's
    /// single representation, so this names it instead.
    #[wasm_bindgen]
    pub fn set_max_execution_steps(&mut self, steps: usize) {
        if steps > 0 {
            self.interpreter.set_max_execution_steps(steps);
        }
    }

    #[wasm_bindgen]
    pub fn restore_user_words(&mut self, words_js: JsValue) -> Result<(), String> {
        let words: Vec<UserWordData> = serde_wasm_bindgen::from_value(words_js)
            .map_err(|e| format!("Failed to deserialize words: {}", e))?;

        // A restored word's saved `dictionary` label is legacy state: the
        // dictionary has two tiers and User is one of them, so every restored
        // definition lands in the same place.
        let entries = words.into_iter().map(|word| {
            (
                word.name,
                word.definition.unwrap_or_default(),
                word.description,
            )
        });

        // Skipping an unreadable entry rather than raising is what keeps the
        // rest of a dictionary: see `restore_user_word_definitions`. The
        // skipped names are not raised here either — throwing would abort the
        // host's own post-restore reconciliation and leave the session holding
        // a half-restored dictionary, which is the outcome this avoids. The
        // host reports them by comparing what it asked for against
        // `collect_user_words_info`.
        let _skipped = self
            .interpreter
            .restore_user_word_definitions(entries)
            .map_err(|e| e.to_string())?;

        let _ = self.interpreter.collect_output();

        Ok(())
    }
}

/// Not part of the exported surface: every run folds the trace into its own
/// result envelope (`errorFlowTrace`), so no host calls this directly.
impl AjisaiInterpreter {
    pub(crate) fn collect_error_flow_trace(&mut self) -> JsValue {
        // Rendered by the CLI's own serializer and converted, so the trace a
        // GUI reads is the one an agent reads. A hand-built copy here was the
        // third spelling of a diagnosis (beside the CLI's and the value
        // node's), and it had already dropped a resource limit's `progress`.
        use serde::Serialize as _;
        let events: Vec<serde_json::Value> = self
            .interpreter
            .drain_error_flow_trace()
            .iter()
            .map(crate::agent::report::error_flow_event_json)
            .collect();
        serde_json::Value::Array(events)
            .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
            .expect("a serde_json value always converts to a JS value")
    }
}
