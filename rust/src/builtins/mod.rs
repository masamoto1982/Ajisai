mod builtin_word_definitions;
mod builtin_word_details;
#[cfg(test)]
mod builtin_word_details_tests;
mod builtin_word_lookup_docs;
mod builtin_word_types;
mod generated_core_word_docs;

pub use builtin_word_definitions::{builtin_specs, lookup_builtin_spec, BuiltinSpec};
// Re-exported for the wasm bindings (feature = "wasm") only; the re-export is
// unused in a default build, so the lint is allowed there only.
#[cfg_attr(not(feature = "wasm"), allow(unused_imports))]
pub use builtin_word_definitions::collect_core_builtin_definitions;
pub use builtin_word_details::lookup_builtin_detail;
pub use builtin_word_types::BuiltinExecutorKey;

use crate::types::{Capabilities, Stability, Tier, WordDefinition};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub fn register_builtins(dictionary: &mut HashMap<String, Arc<WordDefinition>>) {
    for spec in builtin_specs() {
        let name = spec.name;
        let description = generated_core_word_docs::GENERATED_CORE_WORD_DOCS
            .iter()
            .find(|doc| doc.name == name)
            .expect("every registered Core Word must have generated documentation")
            .hover_summary;
        let capabilities = core_builtin_capabilities(spec.executor_key, name);
        dictionary.insert(
            name.to_string(),
            Arc::new(WordDefinition {
                lines: std::sync::Arc::from([]),
                is_builtin: true,
                tier: Tier::Core,
                stability: Stability::Stable,
                capabilities,
                description: Some(description.to_string()),
                dependencies: HashSet::new(),
                original_source: None,
                namespace: None,
                registration_order: 0,
                execution_plans: None,
            }),
        );
    }
}

fn core_builtin_capabilities(key: Option<BuiltinExecutorKey>, name: &str) -> Capabilities {
    match (key, name) {
        (Some(BuiltinExecutorKey::Def), _) => Capabilities::MUTATES_DICT,
        (Some(BuiltinExecutorKey::Del), _) => Capabilities::MUTATES_DICT,
        (Some(BuiltinExecutorKey::Force), _) => Capabilities::MUTATES_DICT,
        (Some(BuiltinExecutorKey::Print), _) => Capabilities::IO,
        _ => Capabilities::PURE,
    }
}
