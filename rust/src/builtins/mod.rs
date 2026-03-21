#[path = "builtin-word-definitions.rs"]
mod builtin_word_definitions;
#[path = "builtin-word-details.rs"]
mod builtin_word_details;

pub use builtin_word_definitions::collect_builtin_definitions;
pub use builtin_word_details::lookup_builtin_detail;

use crate::types::WordDefinition;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Registers core built-in words into the dictionary.
pub fn register_builtins(dictionary: &mut HashMap<String, Arc<WordDefinition>>) {
    for (name, description, _, _) in collect_builtin_definitions() {
        dictionary.insert(
            name.to_string(),
            Arc::new(WordDefinition {
                lines: std::sync::Arc::from([]),
                is_builtin: true,
                description: Some(description.to_string()),
                dependencies: HashSet::new(),
                original_source: None,
            }),
        );
    }
}
