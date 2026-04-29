use crate::builtins::{builtin_specs, BuiltinExecutorKey};
use crate::interpreter::modules::module_word_metadata_entries;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WordPurity {
    Pure,
    Observable,
    Effectful,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorewordMetadata {
    pub name: String,
    pub category: String,
    pub purity: WordPurity,
    pub effects: Vec<String>,
    pub deterministic: bool,
    pub safe_preview: bool,
    pub formerly_module: Option<String>,
}

pub fn get_builtin_word_registry() -> Vec<CorewordMetadata> {
    let mut registry: Vec<CorewordMetadata> = builtin_specs()
        .iter()
        .map(|spec| core_word_metadata(spec.name, spec.category, spec.executor_key))
        .collect();
    registry.extend(module_word_metadata_entries());
    registry
}

pub fn get_coreword_metadata(name: &str) -> Option<CorewordMetadata> {
    let upper = name.to_uppercase();
    get_builtin_word_registry()
        .into_iter()
        .find(|word| word.name == upper)
}

pub fn get_words_by_category(category: &str) -> Vec<CorewordMetadata> {
    let needle = category.to_lowercase();
    get_builtin_word_registry()
        .into_iter()
        .filter(|word| word.category == needle)
        .collect()
}

pub fn get_words_by_purity(purity: WordPurity) -> Vec<CorewordMetadata> {
    get_builtin_word_registry()
        .into_iter()
        .filter(|word| word.purity == purity)
        .collect()
}

pub fn is_safe_preview_word(name: &str) -> bool {
    get_coreword_metadata(name)
        .map(|word| word.safe_preview)
        .unwrap_or(false)
}

fn core_word_metadata(
    name: &str,
    category: &str,
    executor_key: Option<BuiltinExecutorKey>,
) -> CorewordMetadata {
    match executor_key {
        Some(BuiltinExecutorKey::Print) => effectful(name, category, &["console-write"]),
        Some(BuiltinExecutorKey::Def) => {
            effectful(name, category, &["dictionary-write", "dictionary-register"])
        }
        Some(BuiltinExecutorKey::Del) => effectful(name, category, &["dictionary-delete"]),
        Some(BuiltinExecutorKey::Import) => effectful(name, category, &["dictionary-import"]),
        Some(BuiltinExecutorKey::ImportOnly) => {
            effectful(name, category, &["dictionary-import-only"])
        }
        Some(BuiltinExecutorKey::Force) => effectful(name, category, &["interpreter-mode-write"]),
        Some(BuiltinExecutorKey::Eval) => effectful(name, category, &["code-execution"]),
        Some(BuiltinExecutorKey::Spawn)
        | Some(BuiltinExecutorKey::Await)
        | Some(BuiltinExecutorKey::Status)
        | Some(BuiltinExecutorKey::Kill)
        | Some(BuiltinExecutorKey::Monitor)
        | Some(BuiltinExecutorKey::Supervise) => effectful(name, category, &["runtime-control"]),
        Some(BuiltinExecutorKey::Lookup) => {
            observable(name, category, &["dictionary-read"], Some(true))
        }
        _ => pure(name, category),
    }
}

pub(crate) fn pure(name: &str, category: &str) -> CorewordMetadata {
    CorewordMetadata {
        name: name.to_string(),
        category: category.to_lowercase(),
        purity: WordPurity::Pure,
        effects: vec![],
        deterministic: true,
        safe_preview: true,
        formerly_module: None,
    }
}

pub(crate) fn observable(
    name: &str,
    category: &str,
    effects: &[&str],
    deterministic_override: Option<bool>,
) -> CorewordMetadata {
    CorewordMetadata {
        name: name.to_string(),
        category: category.to_lowercase(),
        purity: WordPurity::Observable,
        effects: effects.iter().map(|x| x.to_string()).collect(),
        deterministic: deterministic_override.unwrap_or(false),
        safe_preview: false,
        formerly_module: None,
    }
}

pub(crate) fn effectful(name: &str, category: &str, effects: &[&str]) -> CorewordMetadata {
    CorewordMetadata {
        name: name.to_string(),
        category: category.to_lowercase(),
        purity: WordPurity::Effectful,
        effects: effects.iter().map(|x| x.to_string()).collect(),
        deterministic: false,
        safe_preview: false,
        formerly_module: None,
    }
}

#[cfg(test)]
mod tests {
    //! AQ-VER-007 — Coreword purity / safe-preview integrity tests.
    //!
    //! These tests are linked from `docs/quality/TRACEABILITY_MATRIX.md`
    //! to AQ-REQ-007 ("Built-in word purity classification and `safe_preview`
    //! gating remain self-consistent"). Test names are prefixed with their
    //! verification ID so that a `cargo test aq_ver_007` invocation runs
    //! the full coreword-registry coverage subset.

    use super::{get_builtin_word_registry, is_safe_preview_word, WordPurity};

    #[test]
    fn aq_ver_007_a_metadata_exists_for_all_builtin_words() {
        let registry = get_builtin_word_registry();
        assert!(!registry.is_empty(), "registry must not be empty");
        for word in registry {
            assert!(!word.name.is_empty(), "name must not be empty");
            assert!(
                !word.category.is_empty(),
                "{} has empty category",
                word.name
            );
            assert!(
                matches!(
                    word.purity,
                    WordPurity::Pure | WordPurity::Observable | WordPurity::Effectful
                ),
                "{} has invalid purity",
                word.name
            );
        }
    }

    #[test]
    fn aq_ver_007_b_pure_words_must_be_safe_and_deterministic_without_effects() {
        let registry = get_builtin_word_registry();
        for word in registry.iter().filter(|w| w.purity == WordPurity::Pure) {
            assert!(
                word.effects.is_empty(),
                "{} pure words must have no effects",
                word.name
            );
            assert!(
                word.deterministic,
                "{} pure words must be deterministic",
                word.name
            );
            assert!(
                word.safe_preview,
                "{} pure words must be safe preview",
                word.name
            );
        }
    }

    #[test]
    fn aq_ver_007_c_effectful_words_must_not_be_safe_preview() {
        let registry = get_builtin_word_registry();
        for word in registry
            .iter()
            .filter(|w| w.purity == WordPurity::Effectful)
        {
            assert!(
                !word.safe_preview,
                "{} effectful words must disable safe preview",
                word.name
            );
            assert!(
                !word.effects.is_empty(),
                "{} effectful words must declare effects",
                word.name
            );
        }
    }

    #[test]
    fn aq_ver_007_d_observable_words_are_nondeterministic_and_not_safe_preview_by_default() {
        let registry = get_builtin_word_registry();
        for word in registry
            .iter()
            .filter(|w| w.purity == WordPurity::Observable)
        {
            assert!(
                !word.effects.is_empty(),
                "{} observable words must declare effects",
                word.name
            );
            // LOOKUP reads interpreter dictionary state and is deterministic
            // for the same interpreter snapshot; tracked as a documented
            // exception under AQ-VER-007-D.
            if word.name != "LOOKUP" {
                assert!(
                    !word.deterministic,
                    "{} observable words are expected to be non-deterministic by default",
                    word.name
                );
            }
            assert!(
                !word.safe_preview,
                "{} observable words must not run in auto preview",
                word.name
            );
        }
    }

    /// AQ-VER-007-E — MC/DC truth table for `is_safe_preview_word`.
    ///
    /// The decision under test is logically:
    ///
    /// ```text
    /// metadata_present(name) && metadata_safe_preview(name)
    /// ```
    ///
    /// implemented in `is_safe_preview_word` via
    /// `get_coreword_metadata(name).map(|w| w.safe_preview).unwrap_or(false)`.
    /// We exercise all three reachable rows (the `metadata_present == false`
    /// row collapses both `safe_preview` cases to the `unwrap_or(false)`
    /// short-circuit, so it is covered by a single unknown-name probe):
    ///
    /// | row | metadata_present | safe_preview | expected | rationale                          |
    /// |-----|------------------|--------------|----------|------------------------------------|
    /// | 1   | true             | true         | true     | known pure word (e.g. `ADD`)       |
    /// | 2   | true             | false        | false    | known effectful word (e.g. `PRINT`)|
    /// | 3   | true             | false        | false    | known observable word (e.g. `NOW`) |
    /// | 4   | false            | n/a          | false    | unknown name → unwrap_or(false)    |
    ///
    /// Rows 1 vs 2 demonstrate independent effect of `safe_preview`;
    /// rows 1 vs 4 demonstrate independent effect of `metadata_present`.
    #[test]
    fn aq_ver_007_e_is_safe_preview_word_decision_truth_table() {
        // Row 1: metadata present, safe_preview=true → true.
        assert!(
            is_safe_preview_word("ADD"),
            "row1: pure builtin ADD must be safe preview"
        );
        // Row 2: metadata present, safe_preview=false (effectful) → false.
        assert!(
            !is_safe_preview_word("PRINT"),
            "row2: effectful builtin PRINT must not be safe preview"
        );
        // Row 3: metadata present, safe_preview=false (observable) → false.
        assert!(
            !is_safe_preview_word("NOW"),
            "row3: observable builtin NOW must not be safe preview"
        );
        // Row 4: metadata absent → unwrap_or(false) short-circuit.
        assert!(
            !is_safe_preview_word("__AJISAI_NO_SUCH_WORD__"),
            "row4: unknown name must default to false"
        );

        // Case-insensitive lookup also reaches the safe_preview=true arm,
        // confirming that the upper-casing inside get_coreword_metadata
        // does not flip the decision.
        assert!(
            is_safe_preview_word("add"),
            "row1 (lowercase): case-insensitive lookup must still be safe preview"
        );
    }
}
