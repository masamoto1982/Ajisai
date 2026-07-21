use super::wasm_value_conversion::{
    extract_display_hint_from_js, js_value_to_value, value_to_js, UserWordData,
};
use super::{set_js_prop, AjisaiInterpreter};
use crate::builtins;
use crate::elastic::ElasticMode;
use crate::interpreter;
use crate::interpreter::debug_diagnosis::DebugDiagnosis;
use crate::tokenizer;
use crate::types::arena::{arena_to_value, json_to_arena_node, ValueArena};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

fn js_string_array(value: &JsValue) -> Vec<String> {
    let arr = js_sys::Array::from(value);
    let mut out = Vec::with_capacity(arr.length() as usize);
    for i in 0..arr.length() {
        if let Some(s) = arr.get(i).as_string() {
            out.push(s);
        }
    }
    out
}

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
    if let Some(module) = &diagnosis.where_.module {
        set_js_prop(&where_obj, "module", &(module.clone().into()));
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

        for dict_name in self.interpreter.user_dictionary_names() {
            for (name, _def) in self.interpreter.user_dictionary_words(&dict_name) {
                let fq_name = format!("{}@{}", dict_name, name);
                let is_protected = self
                    .interpreter
                    .dependents
                    .get(&fq_name)
                    .map_or(false, |deps| !deps.is_empty());

                let item = js_sys::Array::new();
                item.push(&dict_name.clone().into());
                item.push(&name.clone().into());
                item.push(&is_protected.into());

                js_array.push(&item);
            }
        }

        js_array.into()
    }

    /// Content identity (Section 8.6) of each user word, as `[fqName, id]`
    /// pairs. The host uses these to deduplicate identical definitions on
    /// import and to key shared word groups by content rather than by name.
    #[wasm_bindgen]
    pub fn collect_word_identities(&self) -> JsValue {
        let js_array = js_sys::Array::new();
        for dict_name in self.interpreter.user_dictionary_names() {
            for (name, _def) in self.interpreter.user_dictionary_words(&dict_name) {
                let fq_name = format!("{}@{}", dict_name, name);
                if let Some(id) = self.interpreter.word_identity(&fq_name) {
                    let item = js_sys::Array::new();
                    item.push(&fq_name.clone().into());
                    item.push(&id.clone().into());
                    js_array.push(&item);
                }
            }
        }
        js_array.into()
    }

    pub(crate) fn collect_imported_modules_array(&self) -> JsValue {
        let arr = js_sys::Array::new();
        for name in self.interpreter.import_table.modules.keys() {
            arr.push(&JsValue::from_str(name));
        }
        arr.into()
    }

    pub(crate) fn collect_user_words_for_state(&self) -> JsValue {
        let words_info: Vec<UserWordData> = self
            .interpreter
            .user_dictionary_names()
            .into_iter()
            .flat_map(|dict_name| {
                self.interpreter
                    .user_dictionary_words(&dict_name)
                    .into_iter()
                    .map(move |(name, _def)| UserWordData {
                        dictionary: Some(dict_name.clone()),
                        name: name.clone(),
                        definition: self
                            .interpreter
                            .lookup_word_definition_tokens(&format!("{}@{}", dict_name, name)),
                    })
            })
            .collect();
        to_value(&words_info).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn collect_core_words_info(&self) -> JsValue {
        to_value(&builtins::collect_core_builtin_definitions()).unwrap_or(JsValue::NULL)
    }

    /// Returns Core-listed words (canonical core + Canonical Module words
    /// that are core-listed, e.g. SORT). This is the listing-based Core
    /// view defined by the redesigned vocabulary system; bare module words
    /// are surfaced for visibility only — invoking SORT bare still requires
    /// `'ALGO' IMPORT` per current execution semantics.
    ///
    /// Tuple shape: `(name, description, syntax)` — same as
    /// `collect_core_words_info` so the GUI can render either list with the
    /// same code path.
    #[wasm_bindgen]
    pub fn collect_core_listed_words_info(&self) -> JsValue {
        let mut entries: Vec<(String, String, String)> =
            builtins::collect_core_builtin_definitions()
                .into_iter()
                .map(|(n, d, s)| (n.to_string(), d.to_string(), s.to_string()))
                .collect();

        for word in crate::coreword_registry::get_core_listed_words() {
            if word.is_canonical_core() {
                continue;
            }
            // Boundary word whose canonical home is a module. Pull the
            // user-facing description from the owning module spec; module
            // canonical metadata does not carry syntax, so leave it blank.
            let module_name = match word.canonical_module() {
                Some(m) => m.to_string(),
                None => continue,
            };
            let description =
                interpreter::modules::module_word_description(&module_name, &word.name)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| word.category.clone());
            entries.push((word.name.clone(), description, String::new()));
        }

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
    pub fn collect_imported_modules(&self) -> JsValue {
        let arr = js_sys::Array::new();
        for name in self.interpreter.import_table.modules.keys() {
            arr.push(&JsValue::from_str(name));
        }
        arr.into()
    }

    /// All importable module names, in specification order. Drives the GUI's
    /// module selector, which pre-lists every module (active or not) so an
    /// inactive module can be surfaced greyed-out and toggled with IMPORT.
    #[wasm_bindgen]
    pub fn collect_available_modules(&self) -> JsValue {
        let arr = js_sys::Array::new();
        for name in interpreter::modules::available_module_names() {
            arr.push(&JsValue::from_str(name));
        }
        arr.into()
    }

    /// Full word catalog for a module, regardless of import state.
    /// Tuple shape: `(shortName, description, imported: bool)`.
    /// `imported` reflects the live import table so the GUI can render active
    /// words normally and inactive words greyed-out within the same sheet.
    #[wasm_bindgen]
    pub fn collect_module_catalog_words_info(&self, module_name: &str) -> JsValue {
        let upper = module_name.to_uppercase();
        let arr = js_sys::Array::new();
        let Some(catalog) = interpreter::modules::module_catalog_words(&upper) else {
            return arr.into();
        };
        let imported = self.interpreter.import_table.modules.get(&upper);
        for word in catalog {
            let short_upper = word.short_name.to_uppercase();
            let is_imported = imported.map_or(false, |entry| {
                if entry.import_all_public {
                    return true;
                }
                entry.imported_words.contains(&short_upper)
            });
            let item = js_sys::Array::new();
            item.push(&JsValue::from_str(word.short_name));
            item.push(&JsValue::from_str(word.description));
            item.push(&is_imported.into());
            arr.push(&item);
        }
        arr.into()
    }

    /// Detailed import state for persistence. Tuple shape:
    /// `(module, importAllPublic: bool, words: string[], samples: string[])`.
    /// Captures partial imports (IMPORT-ONLY / UNIMPORT-ONLY results) that
    /// `collect_imported_modules` (module names only) cannot represent.
    #[wasm_bindgen]
    pub fn collect_import_state(&self) -> JsValue {
        let arr = js_sys::Array::new();
        for (name, entry) in &self.interpreter.import_table.modules {
            let item = js_sys::Array::new();
            item.push(&JsValue::from_str(name));
            item.push(&entry.import_all_public.into());
            let words = js_sys::Array::new();
            for w in &entry.imported_words {
                words.push(&JsValue::from_str(w));
            }
            item.push(&words.into());
            item.push(&js_sys::Array::new().into());
            arr.push(&item);
        }
        arr.into()
    }

    /// Restore a detailed import state previously captured by
    /// `collect_import_state`. Reinstates partial imports exactly, unlike
    /// `restore_imported_modules` which forces a full IMPORT per module.
    #[wasm_bindgen]
    pub fn restore_import_state(&mut self, state_js: JsValue) {
        let arr = js_sys::Array::from(&state_js);
        for i in 0..arr.length() {
            let entry = js_sys::Array::from(&arr.get(i));
            let Some(module) = entry.get(0).as_string() else {
                continue;
            };
            let import_all_public = entry.get(1).as_bool().unwrap_or(false);
            let words = js_string_array(&entry.get(2));
            let samples = js_string_array(&entry.get(3));
            interpreter::modules::restore_import_entry(
                &mut self.interpreter,
                &module,
                import_all_public,
                words,
                samples,
            );
        }
    }

    /// Tuple shape: `(name, description)`.
    #[wasm_bindgen]
    pub fn collect_module_words_info(&self, module_name: &str) -> JsValue {
        let upper = module_name.to_uppercase();
        let arr = js_sys::Array::new();
        let Some(imported) = self.interpreter.import_table.modules.get(&upper) else {
            return arr.into();
        };
        if let Some(module_dict) = self.interpreter.module_vocabulary.get(&upper) {
            for (name, def) in &module_dict.words {
                let short_name = name
                    .split_once('@')
                    .map(|(_, short)| short)
                    .unwrap_or(name.as_str());
                if !imported.import_all_public && !imported.imported_words.contains(short_name) {
                    continue;
                }
                let item = js_sys::Array::new();
                item.push(&JsValue::from_str(name));
                item.push(
                    &def.description
                        .clone()
                        .map(JsValue::from)
                        .unwrap_or(JsValue::NULL),
                );
                arr.push(&item);
            }
        }
        arr.into()
    }

    #[wasm_bindgen]
    pub fn collect_dictionary_dependencies(&self) -> JsValue {
        let arr = js_sys::Array::new();
        for (dict_name, dep) in &self.interpreter.dictionary_dependencies {
            let item = js_sys::Array::new();
            item.push(&JsValue::from_str(dict_name));

            let depends_on = js_sys::Array::new();
            for name in &dep.depends_on {
                depends_on.push(&JsValue::from_str(name));
            }
            item.push(&depends_on.into());

            let depended_by = js_sys::Array::new();
            for name in &dep.depended_by {
                depended_by.push(&JsValue::from_str(name));
            }
            item.push(&depended_by.into());
            arr.push(&item);
        }
        arr.into()
    }

    #[wasm_bindgen]
    pub fn restore_imported_modules(&mut self, modules_js: JsValue) {
        let arr = js_sys::Array::from(&modules_js);
        for i in 0..arr.length() {
            if let Some(name) = arr.get(i).as_string() {
                interpreter::modules::restore_module(&mut self.interpreter, &name);
            }
        }
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
        if let Some((dict_name, short_name)) = self.interpreter.split_qualified_name(&upper_name) {
            if let Some(dict) = self.interpreter.user_dictionaries.get_mut(&dict_name) {
                dict.words.remove(&short_name);
            }
            let _ = self.interpreter.rebuild_dependencies();
            return;
        }

        for dict in self.interpreter.user_dictionaries.values_mut() {
            if dict.words.remove(&upper_name).is_some() {
                let _ = self.interpreter.rebuild_dependencies();
                return;
            }
        }
    }

    #[wasm_bindgen]
    pub fn restore_stack(&mut self, stack_js: JsValue) -> Result<(), String> {
        let js_array = js_sys::Array::from(&stack_js);
        let mut stack = Vec::new();
        let mut hints: Vec<crate::types::Interpretation> = Vec::new();
        for i in 0..js_array.length() {
            let item = js_array.get(i);
            stack.push(js_value_to_value(item.clone())?);
            let hint = extract_display_hint_from_js(&item);
            hints.push(hint);
        }
        self.interpreter.update_stack_with_hints(stack, hints);
        Ok(())
    }

    #[wasm_bindgen]
    pub fn update_input_buffer(&mut self, text: String) {
        self.interpreter.input_buffer = text;
    }

    /// Inject the host-received bytes for a serial port (Section 9.4). Replaces
    /// any buffer previously set for this port id and clears the port's
    /// disconnected flag. `SERIAL@READ` drains this buffer.
    #[wasm_bindgen]
    pub fn update_serial_inbox(&mut self, port_id: String, bytes: Vec<u8>) {
        self.interpreter.serial_disconnected.remove(&port_id);
        self.interpreter.serial_inbox.insert(port_id, bytes);
    }

    /// Mark a serial port as disconnected by the host. Once its inbox is empty,
    /// `SERIAL@READ` projects `NilReason::PortDisconnected`.
    #[wasm_bindgen]
    pub fn mark_serial_disconnected(&mut self, port_id: String) {
        self.interpreter.serial_disconnected.insert(port_id);
    }

    /// Clear all injected serial receive buffers and disconnected flags.
    #[wasm_bindgen]
    pub fn clear_serial_inboxes(&mut self) {
        self.interpreter.serial_inbox.clear();
        self.interpreter.serial_disconnected.clear();
    }

    #[wasm_bindgen]
    pub fn extract_io_output_buffer(&self) -> String {
        self.interpreter.io_output_buffer.clone()
    }

    #[wasm_bindgen]
    pub fn clear_io_output_buffer(&mut self) {
        self.interpreter.io_output_buffer.clear();
    }

    #[wasm_bindgen]
    pub fn set_execution_mode(&mut self, mode: &str) {
        self.interpreter
            .set_elastic_mode(ElasticMode::from_str(mode));
    }

    #[wasm_bindgen]
    pub fn get_execution_mode(&self) -> String {
        self.interpreter.elastic_mode().as_str().to_string()
    }

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

    /// Only exported when the `elastic-engine` feature is compiled in; the
    /// GUI already tolerates the `hedgedTrace` payload field being absent.
    #[cfg(feature = "elastic-engine")]
    #[wasm_bindgen]
    pub fn collect_hedged_trace(&mut self) -> JsValue {
        let arr = js_sys::Array::new();
        for item in self.interpreter.drain_hedged_trace() {
            arr.push(&JsValue::from_str(&item));
        }
        arr.into()
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
                        set_js_prop(&obj, "message", &(format!("{}", e).into()));
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
            self.interpreter.active_user_dictionary = word
                .dictionary
                .clone()
                .unwrap_or_else(|| "EXAMPLE".to_string())
                .to_uppercase();
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
