mod definitions;
mod details;

pub use definitions::get_builtin_definitions;
pub use details::get_builtin_detail;

use std::collections::{HashMap, HashSet};
use crate::types::WordDefinition;

/// Registers all built-in words (including Music DSL) into the dictionary.
pub fn register_builtins(dictionary: &mut HashMap<String, WordDefinition>) {
    for (name, description, _, _) in get_builtin_definitions() {
        dictionary.insert(name.to_string(), WordDefinition {
            lines: vec![],
            is_builtin: true,
            description: Some(description.to_string()),
            dependencies: HashSet::new(),
            original_source: None,
        });
    }
}
