#[path = "builtin-word-definitions.rs"]
mod builtin_word_definitions;
#[path = "builtin-word-details.rs"]
mod builtin_word_details;
#[path = "detail-lookup-arithmetic-logic.rs"]
mod detail_lookup_arithmetic_logic;
#[path = "detail-lookup-cond.rs"]
mod detail_lookup_cond;
#[path = "detail-lookup-control-higher-order.rs"]
mod detail_lookup_control_higher_order;
#[path = "detail-lookup-io-module.rs"]
mod detail_lookup_io_module;
#[path = "detail-lookup-modifier.rs"]
mod detail_lookup_modifier;
#[path = "detail-lookup-string-cast.rs"]
mod detail_lookup_string_cast;
#[path = "detail-lookup-vector-ops.rs"]
mod detail_lookup_vector_ops;

pub use builtin_word_definitions::{
    builtin_specs, collect_builtin_definitions, lookup_builtin_spec, BuiltinExecutorKey,
};
pub use builtin_word_details::lookup_builtin_detail;

use crate::types::WordDefinition;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub fn register_builtins(dictionary: &mut HashMap<String, Arc<WordDefinition>>) {
    for spec in builtin_specs() {
        let name = spec.name;
        let description = spec.short_description;
        dictionary.insert(
            name.to_string(),
            Arc::new(WordDefinition {
                lines: std::sync::Arc::from([]),
                is_builtin: true,
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
