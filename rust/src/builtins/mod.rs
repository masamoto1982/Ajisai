// rust/src/builtins/mod.rs
//
// Built-in words module
//
// This module is organized into:
// - definitions.rs: Word definitions (name, short description, category)
// - details.rs: Detailed documentation for each word (used by ? command)

mod definitions;
mod details;

pub use definitions::get_builtin_definitions;
pub use definitions::get_extension_definitions;
pub use details::get_builtin_detail;

use std::collections::{HashMap, HashSet};
use crate::types::WordDefinition;

/// Registers all built-in words into the dictionary.
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

/// Registers extension words (Music DSL) into the dictionary.
///
/// Extension words have native Rust implementations but are NOT built-in protected:
/// - Users can override them with DEF (with ! force flag if dependents exist)
/// - Users can delete them with DEL
/// - The `?` command shows documentation via `original_source`
pub fn register_extensions(dictionary: &mut HashMap<String, WordDefinition>) {
    for (name, description, _, _) in get_extension_definitions() {
        dictionary.insert(name.to_string(), WordDefinition {
            lines: vec![],
            is_builtin: false,
            description: Some(description.to_string()),
            dependencies: HashSet::new(),
            original_source: Some(get_builtin_detail(name)),
        });
    }
}
