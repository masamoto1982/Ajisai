mod builtin_word_definitions;
mod builtin_word_details;
#[cfg(test)]
mod builtin_word_details_lookup_tests;
#[cfg(test)]
mod builtin_word_details_tests;
mod builtin_word_lookup_docs;
mod generated_core_word_docs;

pub use builtin_word_definitions::lookup_builtin_spec;
// The whole-table walk is a consistency-checking affordance: every runtime
// consumer now reaches a Word through the generated registry and looks its
// prose up by name, so the only readers of the table as a whole are the tests
// asserting that the two halves still describe the same 57 Words.
#[cfg_attr(not(test), allow(unused_imports))]
pub use builtin_word_definitions::builtin_specs;
// Re-exported for the wasm bindings (feature = "wasm") only; the re-export is
// unused in a default build, so the lint is allowed there only.
#[cfg_attr(not(feature = "wasm"), allow(unused_imports))]
pub use builtin_word_definitions::collect_core_builtin_definitions;
pub use builtin_word_details::lookup_builtin_detail;

use crate::kernel::generated::{WordId, GENERATED_WORDS};
use crate::types::{Capabilities, Stability, Tier, WordDefinition};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Register every Core Word the specification declares.
///
/// The inventory is the generated registry, not the hand-written spec table:
/// `spec/words.json` decides which Words exist, and a Word that appears there
/// is registered whether or not anyone remembered to add prose for it (the
/// missing-documentation panic below is what says so out loud).
pub fn register_builtins(dictionary: &mut HashMap<String, Arc<WordDefinition>>) {
    for word in GENERATED_WORDS {
        let name = word.name;
        let description = generated_core_word_docs::GENERATED_CORE_WORD_DOCS
            .iter()
            .find(|doc| doc.name == name)
            .expect("every registered Core Word must have generated documentation")
            .hover_summary;
        let capabilities = core_builtin_capabilities(word.id);
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
                text_references: HashSet::new(),
                original_source: None,
                namespace: None,
                registration_order: 0,
                execution_plans: None,
            }),
        );
    }
}

/// The host capabilities a Core Word needs, keyed by its canonical identity.
///
/// Keyed on `WordId` rather than on the spelling of the name, so renaming a
/// Word in `spec/words.json` moves its capability with it instead of silently
/// dropping it to `PURE`.
fn core_builtin_capabilities(id: WordId) -> Capabilities {
    match id {
        WordId::Def | WordId::Del => Capabilities::MUTATES_DICT,
        WordId::Print => Capabilities::IO,
        _ => Capabilities::PURE,
    }
}
