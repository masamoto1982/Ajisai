use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{is_string_value, value_as_string};
use crate::interpreter::{Interpreter, ModuleDictionary};
use crate::types::{Tier, Value, ValueData, WordDefinition};

use super::module_builtins::MODULE_SPECS;
use super::module_samples::build_sample_words;

pub(super) fn ensure_module_dictionary(interp: &mut Interpreter, module_name: &str) -> Result<()> {
    if interp.module_vocabulary.contains_key(module_name) {
        return Ok(());
    }
    let module = MODULE_SPECS
        .iter()
        .find(|module| module.name == module_name)
        .ok_or_else(|| AjisaiError::UnknownModule(module_name.to_string()))?;

    let mut words = HashMap::new();
    for word in module.words {
        let qualified = format!("{}@{}", module.name, word.short_name);
        words.insert(
            qualified,
            Arc::new(WordDefinition {
                lines: Arc::from([]),
                is_builtin: true,
                tier: Tier::Standard,
                stability: word.stability,
                capabilities: word.capabilities,
                description: Some(word.description.to_string()),
                dependencies: HashSet::new(),
                original_source: None,
                namespace: Some(module.name.to_string()),
                registration_order: 0,
                execution_plans: None,
            }),
        );
    }

    let sample_words = build_sample_words(module.name, module.sample_words)?;
    interp.module_vocabulary.insert(
        module_name.to_string(),
        ModuleDictionary {
            words,
            sample_words,
        },
    );
    interp.bump_module_epoch();
    Ok(())
}

pub(super) fn extract_module_name_from_value(value: &Value) -> Option<String> {
    if is_string_value(value) {
        return value_as_string(value);
    }

    match &value.data {
        ValueData::Vector(children)
        | ValueData::Record {
            pairs: children, ..
        } => {
            if children.len() != 1 {
                return None;
            }
            if !is_string_value(&children[0]) {
                return None;
            }
            value_as_string(&children[0])
        }
        _ => None,
    }
}

pub(super) fn execute_module_word(interp: &mut Interpreter, name: &str) -> Option<Result<()>> {
    let upper = name.to_uppercase();
    let (module_name, word_name) = upper.split_once('@')?;
    let module = MODULE_SPECS.iter().find(|m| m.name == module_name)?;
    let word = module.words.iter().find(|w| w.short_name == word_name)?;
    Some((word.executor)(interp))
}

pub(super) fn is_mode_preserving_word(name: &str) -> bool {
    let upper = name.to_uppercase();
    let Some((module_name, word_name)) = upper.split_once('@') else {
        return false;
    };

    MODULE_SPECS
        .iter()
        .find(|m| m.name == module_name)
        .and_then(|m| m.words.iter().find(|w| w.short_name == word_name))
        .map(|w| w.preserves_modes)
        .unwrap_or(false)
}
