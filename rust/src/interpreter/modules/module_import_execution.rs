use std::collections::HashSet;

use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{is_string_value, value_as_string};
use crate::interpreter::{ConsumptionMode, ImportedModule, Interpreter};
use crate::types::Value;

use super::module_registry::{ensure_module_dictionary, extract_module_name_from_value};

fn parse_only_items(value: &Value) -> Result<Vec<String>> {
    if is_string_value(value) {
        let s = value_as_string(value)
            .ok_or_else(|| AjisaiError::from("IMPORT-ONLY expects string selectors"))?;
        return Ok(vec![s]);
    }

    let Some(items) = value.as_vector() else {
        return Err(AjisaiError::from(
            "IMPORT-ONLY expects a vector of word names",
        ));
    };

    let mut result = Vec::new();
    for item in items {
        if !is_string_value(item) {
            return Err(AjisaiError::from("IMPORT-ONLY selectors must be strings"));
        }
        let Some(name) = value_as_string(item) else {
            continue;
        };
        result.push(name);
    }
    Ok(result)
}

fn emit_import_conflict_warnings(interp: &mut Interpreter, module_name: &str) {
    let Some(module_dict) = interp.module_vocabulary.get(module_name) else {
        return;
    };

    for short_name in module_dict.sample_words.keys() {
        let mut collisions: Vec<String> = Vec::new();
        for (dict_name, dict) in &interp.user_dictionaries {
            if dict.words.contains_key(short_name) {
                collisions.push(format!("{}@{}", dict_name, short_name));
            }
        }
        if collisions.is_empty() {
            continue;
        }

        let mut all_paths = vec![format!("{}@{}", module_name, short_name)];
        all_paths.extend(collisions);
        interp.output_buffer.push_str(&format!(
            "Warning: '{}' now exists in both {}. Use a qualified path when calling this word.\n",
            short_name,
            all_paths.join(" and ")
        ));
    }
}

pub(super) fn import_all_public(interp: &mut Interpreter, module_name: &str) -> Result<()> {
    ensure_module_dictionary(interp, module_name)?;
    let module_dict = interp
        .module_vocabulary
        .get(module_name)
        .ok_or_else(|| AjisaiError::UnknownModule(module_name.to_string()))?;

    let mut imported_words = HashSet::new();
    let mut imported_samples = HashSet::new();

    for qualified in module_dict.words.keys() {
        if let Some((_, short)) = qualified.split_once('@') {
            imported_words.insert(short.to_string());
        }
    }
    for short in module_dict.sample_words.keys() {
        imported_samples.insert(short.clone());
    }

    let entry = interp
        .import_table
        .modules
        .entry(module_name.to_string())
        .or_insert_with(|| ImportedModule {
            import_all_public: false,
            imported_words: HashSet::new(),
            imported_samples: HashSet::new(),
        });

    entry.import_all_public = true;
    entry.imported_words = imported_words;
    entry.imported_samples = imported_samples;
    emit_import_conflict_warnings(interp, module_name);
    Ok(())
}

pub(super) fn op_import(interp: &mut Interpreter) -> Result<()> {
    // TODO: Module dictionaries are being migrated toward Coreword Registry.
    // IMPORT should eventually become a compatibility no-op or category-view operation.
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let value = if is_keep_mode {
        interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    let module_name = extract_module_name_from_value(&value)
        .ok_or_else(|| AjisaiError::UnknownModule(value.to_string()))?
        .to_uppercase();

    import_all_public(interp, &module_name)?;
    interp.bump_module_epoch();
    interp.rebuild_dependencies()?;
    Ok(())
}

pub(super) fn op_import_only(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let selectors = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let module_value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let module_name = extract_module_name_from_value(&module_value)
        .ok_or_else(|| AjisaiError::UnknownModule(module_value.to_string()))?
        .to_uppercase();

    ensure_module_dictionary(interp, &module_name)?;
    let selected = parse_only_items(&selectors)?;

    let module_dict = interp
        .module_vocabulary
        .get(&module_name)
        .ok_or_else(|| AjisaiError::UnknownModule(module_name.clone()))?;
    let mut validated: Vec<(String, bool)> = Vec::new();
    for item in selected {
        let short = item.to_uppercase();
        let qualified = format!("{}@{}", module_name, short);
        if module_dict.words.contains_key(&qualified) {
            validated.push((short, true));
        } else if module_dict.sample_words.contains_key(&short) {
            validated.push((short, false));
        } else {
            return Err(AjisaiError::from(format!(
                "Unknown module word '{}' in {}",
                short, module_name
            )));
        }
    }

    let entry = interp
        .import_table
        .modules
        .entry(module_name.clone())
        .or_insert_with(|| ImportedModule {
            import_all_public: false,
            imported_words: HashSet::new(),
            imported_samples: HashSet::new(),
        });

    for (short, is_word) in validated {
        if is_word {
            entry.imported_words.insert(short.clone());
        } else {
            entry.imported_samples.insert(short.clone());
        }
    }

    emit_import_conflict_warnings(interp, &module_name);
    interp.bump_module_epoch();
    interp.rebuild_dependencies()?;
    Ok(())
}

pub(super) fn restore_module(interp: &mut Interpreter, module_name: &str) -> bool {
    let upper = module_name.to_uppercase();
    import_all_public(interp, &upper).is_ok()
}
