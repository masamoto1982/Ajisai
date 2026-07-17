mod builtin_word_definitions;
mod builtin_word_details;
mod builtin_word_lookup_docs;

pub use builtin_word_definitions::{
    builtin_specs, collect_core_builtin_definitions, lookup_builtin_spec, BuiltinExecutorKey,
    BuiltinSpec, WordShape,
};
pub use builtin_word_details::lookup_builtin_detail;
pub use builtin_word_details::render_four_section;

use crate::types::{Capabilities, Stability, Tier, WordDefinition};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub fn register_builtins(dictionary: &mut HashMap<String, Arc<WordDefinition>>) {
    for spec in builtin_specs() {
        let name = spec.name;
        let description = spec.hover_summary;
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
        (Some(BuiltinExecutorKey::Import), _) => Capabilities::MUTATES_DICT,
        (Some(BuiltinExecutorKey::ImportOnly), _) => Capabilities::MUTATES_DICT,
        (Some(BuiltinExecutorKey::Unimport), _) => Capabilities::MUTATES_DICT,
        (Some(BuiltinExecutorKey::UnimportOnly), _) => Capabilities::MUTATES_DICT,
        (Some(BuiltinExecutorKey::Force), _) => Capabilities::MUTATES_DICT,
        (Some(BuiltinExecutorKey::Eval), _) => Capabilities::EVAL,
        (Some(BuiltinExecutorKey::Spawn), _) => Capabilities::SPAWN,
        (Some(BuiltinExecutorKey::Await), _) => Capabilities::SPAWN,
        (Some(BuiltinExecutorKey::Status), _) => Capabilities::SPAWN,
        (Some(BuiltinExecutorKey::Kill), _) => Capabilities::SPAWN,
        (Some(BuiltinExecutorKey::Monitor), _) => Capabilities::SPAWN,
        (Some(BuiltinExecutorKey::Supervise), _) => Capabilities::SPAWN,
        (Some(BuiltinExecutorKey::Print), _) => Capabilities::IO,
        (Some(BuiltinExecutorKey::Precompute), _) => Capabilities::MUTATES_DICT,
        _ => Capabilities::PURE,
    }
}
