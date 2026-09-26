//! The Coreword registry: what the runtime knows about each Core Word.
//!
//! The contract half of that — stack arity, NIL policy, purity, determinism —
//! is **not** written here. It is read from `kernel::generated`, projected from
//! `spec/words.json`, and this module joins it with the runtime-local facts
//! derived from it (the flow-mass contract the analyzers read). It adds no
//! classification of its own: a label the registry does not declare is one a
//! reader cannot check against the specification.
//!
//! Two of the vocabularies that used to be declared in this file were narrower
//! than the canonical ones and mislabelled Words as a result: the hand-written
//! `NilPolicy` had 5 of the specification's 7 values, and `WordPurity` had 3 of
//! its 4 with `conditional` inexpressible, so every higher-order Word was
//! recorded as `pure`. Determinism was a `bool` where the specification
//! distinguishes `deterministic` / `stateRelative` / `hostRelative`. All three
//! now come from the generated enums, where a value the canon admits is a
//! variant by construction.

use crate::kernel::generated::GENERATED_WORDS;
mod contract;

use contract::mass_from_arity;
pub use contract::{mass_contract, MassContract};
use serde::Serialize;
#[cfg(test)]
use std::collections::HashSet;

pub use crate::kernel::generated::{Determinism, GeneratedWord, NilPolicy, Partiality, Purity};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorewordMetadata {
    pub name: String,
    pub family: String,
    /// Declared in `spec/words.json`.
    pub purity: Purity,
    /// Declared in `spec/words.json`, in the specification's own spelling.
    pub effects: Vec<String>,
    /// Declared in `spec/words.json`. Was a `bool`, which could not express
    /// the specification's distinction between a Word that reads runtime state
    /// (`stateRelative`) and one that reads the host (`hostRelative`).
    pub determinism: Determinism,
    pub partiality: Partiality,
    /// Declared in `spec/words.json`.
    pub nil_policy: NilPolicy,
    /// Static flow-mass contract: arity / production (LANG.STACK.CONSUMPTION).
    /// Derived from the declared stack arity (LANG.MACHINE.WORD).
    pub mass: MassContract,
}

/// The registry is built by walking the *generated* inventory and joining each
/// Word's prose entry, rather than the reverse. `spec/words.json` decides which
/// Words exist; a prose entry without a declared Word is not a Word.
fn build_builtin_word_registry() -> Vec<CorewordMetadata> {
    GENERATED_WORDS.iter().map(core_word_metadata).collect()
}

/// The complete built-in word registry. Built once on first access and
/// cached for the process lifetime.
pub fn get_builtin_word_registry() -> &'static [CorewordMetadata] {
    static REGISTRY: std::sync::OnceLock<Vec<CorewordMetadata>> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(build_builtin_word_registry)
}

/// Metadata lookup by bare word name.
///
/// Built-in words form a single flat namespace, so lookup is an exact match on
/// the upper-cased name. A qualified `DICTIONARY@WORD` token never names a
/// built-in — it addresses a User dictionary word — and so resolves to `None`.
pub fn get_coreword_metadata(name: &str) -> Option<CorewordMetadata> {
    let upper = name.to_uppercase();
    get_builtin_word_registry()
        .iter()
        .find(|m| m.name == upper)
        .cloned()
}

/// Alias of `get_coreword_metadata`. Use this in new code.
pub fn get_builtin_word_metadata(name: &str) -> Option<CorewordMetadata> {
    get_coreword_metadata(name)
}

/// The declared contract row for a Word, by bare name.
///
/// [`CorewordMetadata`] is the runtime's *joined* view — the declaration plus
/// the runtime-local classifications — and it is serialized to several
/// surfaces. A caller that wants the declaration itself, in the
/// specification's own spelling (the conditions the Word names for projecting
/// and for raising, its declared arity, how one line of it is written), reads
/// the generated row instead of widening that view.
pub fn get_declared_word(name: &str) -> Option<&'static GeneratedWord> {
    let upper = name.to_uppercase();
    GENERATED_WORDS.iter().find(|word| word.name == upper)
}

/// Validates that no two registry entries share a `name`. Built-in words form
/// a single flat namespace, so a repeated name is always a genuine duplicate.
/// Used internally by tests.
#[cfg(test)]
fn collect_duplicate_entries(registry: &[CorewordMetadata]) -> Vec<String> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut dupes: Vec<String> = Vec::new();
    for word in registry {
        if !seen.insert(word.name.as_str()) {
            dupes.push(word.name.clone());
        }
    }
    dupes
}

/// Join a declared Word with its hand-written prose entry.
///
/// Every declared Word must have one: the inventory equivalence is asserted in
/// `kernel::generated`, so a missing entry is a build-time contradiction rather
/// than a Word that quietly loses its documentation.
fn core_word_metadata(word: &GeneratedWord) -> CorewordMetadata {
    let spec = crate::builtins::lookup_builtin_spec(word.name)
        .unwrap_or_else(|| panic!("declared Word {} has no runtime spec entry", word.name));
    CorewordMetadata {
        name: word.name.to_string(),
        family: spec.family.to_string(),
        purity: word.purity,
        effects: word.effects.iter().map(|e| e.to_string()).collect(),
        determinism: word.determinism,
        partiality: word.partiality,
        nil_policy: word.nil_policy,
        mass: mass_from_arity(word),
    }
}

#[cfg(test)]
mod contract_tests;
#[cfg(test)]
mod safety_tests;
