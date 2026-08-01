//! The Coreword registry: what the runtime knows about each Core Word.
//!
//! The contract half of that — stack arity, NIL policy, purity, determinism —
//! is **not** written here. It is read from `kernel::generated`, projected from
//! `spec/words.json`, and this module joins it with the runtime-local
//! classifications that the specification does not declare (`category`,
//! `partiality`, `safety_level`, `safe_preview`).
//!
//! Two of the vocabularies that used to be declared in this file were narrower
//! than the canonical ones and mislabelled Words as a result: the hand-written
//! `NilPolicy` had 5 of the specification's 7 values, and `WordPurity` had 3 of
//! its 4 with `conditional` inexpressible, so every higher-order Word was
//! recorded as `pure`. Determinism was a `bool` where the specification
//! distinguishes `deterministic` / `stateRelative` / `hostRelative`. All three
//! now come from the generated enums, where a value the canon admits is a
//! variant by construction.

use crate::kernel::generated::{GeneratedWord, GENERATED_WORDS};
mod contract;

use contract::mass_from_arity;
pub(crate) use contract::{
    execution_form_from_contract, partiality_from_contract, safe_preview_from_contract,
    safety_from_contract, stability_from_contract,
};
pub use contract::{mass_contract, ExecutionForm, MassContract};
use serde::Serialize;
#[cfg(test)]
use std::collections::HashSet;

pub use crate::kernel::generated::{Determinism, NilPolicy, Partiality, Purity};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SafetyLevel {
    A,
    B,
    C,
    D,
    Quarantined,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WordProfile {
    /// Host-independent, portable Ajisai semantics.
    Core,
    /// Requires an explicit host capability before execution.
    Hosted,
    /// Reserved for words whose behavior is intentionally platform-specific.
    PlatformSpecific,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorewordMetadata {
    pub name: String,
    pub category: String,
    /// Declared in `spec/words.json`.
    pub purity: Purity,
    /// Declared in `spec/words.json`, in the specification's own spelling.
    pub effects: Vec<String>,
    /// Declared in `spec/words.json`. Was a `bool`, which could not express
    /// the specification's distinction between a Word that reads runtime state
    /// (`stateRelative`) and one that reads the host (`hostRelative`).
    pub determinism: Determinism,
    pub safe_preview: bool,
    pub partiality: Partiality,
    /// Declared in `spec/words.json`.
    pub nil_policy: NilPolicy,
    pub safety_level: SafetyLevel,
    /// Static flow-mass contract (SPEC §13.1): arity / production, with
    /// bifurcation governed by the `KEEP` modifier (§13.2). Derived from the
    /// declared stack arity.
    pub mass: MassContract,
    /// Portability profile used by conformance tooling to keep the Core
    /// profile free of host-boundary words.
    pub profile: WordProfile,
}

impl CorewordMetadata {
    /// Whether the Word's result depends on nothing but its operands.
    ///
    /// The coarse question the old `deterministic: bool` answered, kept as a
    /// derived accessor so callers that only need the bit do not have to
    /// enumerate the three canonical classes.
    pub fn is_deterministic(&self) -> bool {
        self.determinism == Determinism::Deterministic
    }
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

pub fn get_words_by_category(category: &str) -> Vec<CorewordMetadata> {
    let needle = category.to_lowercase();
    get_builtin_word_registry()
        .iter()
        .filter(|word| word.category == needle)
        .cloned()
        .collect()
}

pub fn get_words_by_purity(purity: Purity) -> Vec<CorewordMetadata> {
    get_builtin_word_registry()
        .iter()
        .filter(|word| word.purity == purity)
        .cloned()
        .collect()
}

pub fn get_words_by_profile(profile: WordProfile) -> Vec<CorewordMetadata> {
    get_builtin_word_registry()
        .iter()
        .filter(|word| word.profile == profile)
        .cloned()
        .collect()
}

pub fn get_core_profile_words() -> Vec<CorewordMetadata> {
    get_words_by_profile(WordProfile::Core)
}

pub fn get_hosted_profile_words() -> Vec<CorewordMetadata> {
    get_words_by_profile(WordProfile::Hosted)
}

pub fn is_safe_preview_word(name: &str) -> bool {
    get_coreword_metadata(name)
        .map(|word| word.safe_preview)
        .unwrap_or(false)
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

fn builtin_profile(name: &str) -> WordProfile {
    // Output is the only hosted effect, so PRINT is the only non-Core-profile
    // Word. Dictionary mutation is an effect too, but not a hosted one.
    if name == "PRINT" {
        WordProfile::Hosted
    } else {
        WordProfile::Core
    }
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
        category: spec.category.to_lowercase(),
        purity: word.purity,
        effects: word.effects.iter().map(|e| e.to_string()).collect(),
        determinism: word.determinism,
        safe_preview: safe_preview_from_contract(word),
        partiality: partiality_from_contract(word),
        nil_policy: word.nil_policy,
        safety_level: safety_from_contract(word),
        mass: mass_from_arity(word),
        profile: builtin_profile(word.name),
    }
}

#[cfg(test)]
mod contract_tests;
#[cfg(test)]
mod safety_tests;
