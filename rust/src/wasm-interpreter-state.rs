use super::wasm_value_conversion::{
    arena_node_to_js, extract_display_hint_from_js, js_value_to_value, UserWordData,
};
use super::{set_js_prop, AjisaiInterpreter};
use crate::builtins;
use crate::elastic::ElasticMode;
use crate::interpreter;
use crate::tokenizer;
use crate::types::arena::{arena_to_value, json_to_arena_node, value_to_arena, ValueArena};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl AjisaiInterpreter {
    #[wasm_bindgen]
    pub fn collect_stack(&self) -> JsValue {
        let js_array = js_sys::Array::new();
        let hints = self.interpreter.collect_stack_hints();
        for (i, value) in self.interpreter.get_stack().iter().enumerate() {
            let hint = hints
                .get(i)
                .copied()
                .unwrap_or(crate::types::DisplayHint::Auto);
            let (arena, root) = value_to_arena(value);
            js_array.push(&arena_node_to_js(&arena, root, Some(hint)));
        }
        js_array.into()
    }

    #[wasm_bindgen]
    pub fn collect_user_words_info(&self) -> JsValue {
        let js_array = js_sys::Array::new();

        for dict_name in self.interpreter.user_dictionary_names() {
            for (name, def) in self.interpreter.user_dictionary_words(&dict_name) {
                let fq_name = format!("{}@{}", dict_name, name);
                let is_protected = self
                    .interpreter
                    .dependents
                    .get(&fq_name)
                    .map_or(false, |deps| !deps.is_empty());

                let item = js_sys::Array::new();
                item.push(&dict_name.clone().into());
                item.push(&name.clone().into());
                item.push(
                    &def.description
                        .clone()
                        .map(JsValue::from)
                        .unwrap_or(JsValue::NULL),
                );
                item.push(&is_protected.into());

                js_array.push(&item);
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
                    .map(move |(name, def)| UserWordData {
                        dictionary: Some(dict_name.clone()),
                        name: name.clone(),
                        definition: self
                            .interpreter
                            .lookup_word_definition_tokens(&format!("{}@{}", dict_name, name)),
                        description: def.description.clone(),
                    })
            })
            .collect();
        to_value(&words_info).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn collect_core_words_info(&self) -> JsValue {
        to_value(&builtins::collect_core_builtin_definitions()).unwrap_or(JsValue::NULL)
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

    #[wasm_bindgen]
    pub fn collect_module_sample_words_info(&self, module_name: &str) -> JsValue {
        let upper = module_name.to_uppercase();
        let arr = js_sys::Array::new();
        if let Some(module_dict) = self.interpreter.module_vocabulary.get(&upper) {
            for (name, def) in &module_dict.sample_words {
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
    pub fn collect_module_words_info(&self, module_name: &str) -> JsValue {
        let upper = module_name.to_uppercase();
        let arr = js_sys::Array::new();
        if let Some(module_dict) = self.interpreter.module_vocabulary.get(&upper) {
            for (name, def) in &module_dict.words {
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
            if let Some(dict) = self.interpreter.module_vocabulary.get_mut(&dict_name) {
                dict.sample_words.remove(&short_name);
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
        for dict in self.interpreter.module_vocabulary.values_mut() {
            if dict.sample_words.remove(&upper_name).is_some() {
                let _ = self.interpreter.rebuild_dependencies();
                return;
            }
        }
    }

    #[wasm_bindgen]
    pub fn restore_stack(&mut self, stack_js: JsValue) -> Result<(), String> {
        let js_array = js_sys::Array::from(&stack_js);
        let mut stack = Vec::new();
        let mut hints: Vec<crate::types::DisplayHint> = Vec::new();
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

    #[wasm_bindgen]
    pub fn collect_hedged_trace(&mut self) -> JsValue {
        let arr = js_sys::Array::new();
        for item in self.interpreter.drain_hedged_trace() {
            arr.push(&JsValue::from_str(&item));
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

        for word in words {
            self.interpreter.active_user_dictionary = word
                .dictionary
                .clone()
                .unwrap_or_else(|| "DEMO".to_string())
                .to_uppercase();
            let definition = match &word.definition {
                Some(def) if !def.is_empty() => def.clone(),
                _ => continue,
            };

            let tokens = tokenizer::tokenize(&definition)
                .map_err(|e| format!("Failed to tokenize definition for {}: {}", word.name, e))?;

            interpreter::execute_def::op_def_inner(
                &mut self.interpreter,
                &word.name,
                &tokens,
                word.description.clone(),
            )
            .map_err(|e| format!("Failed to restore word {}: {}", word.name, e))?;
        }

        self.interpreter
            .rebuild_dependencies()
            .map_err(|e| e.to_string())?;

        let _ = self.interpreter.collect_output();

        Ok(())
    }
}
