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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Partiality {
    Total,
    Partial,
    Projecting,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NilPolicy {
    Passthrough,
    CreatesNil,
    RejectsNil,
    ConsumesNil,
    PreservesReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SafetyLevel {
    A,
    B,
    C,
    D,
    Quarantined,
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
    pub partiality: Partiality,
    pub nil_policy: NilPolicy,
    pub safety_level: SafetyLevel,
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
    let mut meta = match executor_key {
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
    };
    apply_contract_overrides(&mut meta, executor_key);
    meta
}

fn apply_contract_overrides(meta: &mut CorewordMetadata, executor_key: Option<BuiltinExecutorKey>) {
    use BuiltinExecutorKey::*;
    match executor_key {
        Some(Add) | Some(Sub) | Some(Mul) | Some(Floor) | Some(Ceil) | Some(Round) => {
            meta.partiality = Partiality::Total;
            meta.nil_policy = NilPolicy::Passthrough;
            meta.safety_level = SafetyLevel::A;
        }
        Some(Div) | Some(Mod) => {
            meta.partiality = Partiality::Partial;
            meta.nil_policy = NilPolicy::Passthrough;
            meta.safety_level = SafetyLevel::B;
        }
        Some(Eq) | Some(Lt) | Some(Le) | Some(And) | Some(Or) | Some(Not) => {
            meta.partiality = Partiality::Total;
            meta.nil_policy = NilPolicy::Passthrough;
            meta.safety_level = SafetyLevel::A;
        }
        Some(Get) | Some(Insert) | Some(Replace) | Some(Remove) | Some(Take) | Some(Split) => {
            meta.partiality = Partiality::Partial;
            meta.nil_policy = NilPolicy::RejectsNil;
            meta.safety_level = SafetyLevel::B;
        }
        Some(Length) | Some(Concat) | Some(Reverse) | Some(Range) | Some(Reorder)
        | Some(Collect) | Some(Shape) | Some(Rank) | Some(Reshape) | Some(Transpose)
        | Some(Fill) => {
            meta.partiality = Partiality::Partial;
            meta.nil_policy = NilPolicy::RejectsNil;
            meta.safety_level = SafetyLevel::B;
        }
        Some(True) | Some(False) | Some(Nil) | Some(Idle) => {
            meta.partiality = Partiality::Total;
            meta.nil_policy = NilPolicy::PreservesReason;
            meta.safety_level = SafetyLevel::A;
        }
        Some(Str) | Some(Num) | Some(Bool) | Some(Chr) | Some(Chars) | Some(Join) => {
            meta.partiality = Partiality::Partial;
            meta.nil_policy = NilPolicy::RejectsNil;
            meta.safety_level = SafetyLevel::B;
        }
        Some(Map) | Some(Filter) | Some(Fold) | Some(Unfold) | Some(Any) | Some(All)
        | Some(Count) | Some(Scan) | Some(Cond) | Some(Exec) => {
            meta.partiality = Partiality::Partial;
            meta.nil_policy = NilPolicy::RejectsNil;
            meta.safety_level = SafetyLevel::B;
        }
        Some(Eval) => {
            meta.partiality = Partiality::Partial;
            meta.nil_policy = NilPolicy::RejectsNil;
            meta.safety_level = SafetyLevel::D;
        }
        Some(Print) => {
            meta.partiality = Partiality::Partial;
            meta.nil_policy = NilPolicy::PreservesReason;
            meta.safety_level = SafetyLevel::D;
        }
        Some(Def) | Some(Del) | Some(Import) | Some(ImportOnly) | Some(Force) => {
            meta.partiality = Partiality::Partial;
            meta.nil_policy = NilPolicy::RejectsNil;
            meta.safety_level = SafetyLevel::D;
        }
        Some(Lookup) => {
            meta.partiality = Partiality::Partial;
            meta.nil_policy = NilPolicy::RejectsNil;
            meta.safety_level = SafetyLevel::C;
        }
        Some(Spawn) | Some(Await) | Some(Status) | Some(Kill) | Some(Monitor)
        | Some(Supervise) => {
            meta.partiality = Partiality::Partial;
            meta.nil_policy = NilPolicy::RejectsNil;
            meta.safety_level = SafetyLevel::Quarantined;
        }
        None => {
            meta.partiality = Partiality::Total;
            meta.nil_policy = NilPolicy::PreservesReason;
            meta.safety_level = SafetyLevel::A;
        }
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
        partiality: Partiality::Total,
        nil_policy: NilPolicy::Passthrough,
        safety_level: SafetyLevel::A,
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
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::C,
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
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::D,
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

    use super::{
        get_builtin_word_registry, get_coreword_metadata, is_safe_preview_word, NilPolicy,
        Partiality, SafetyLevel, WordPurity,
    };

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

    #[test]
    fn aq_ver_contract_a_every_word_has_contract_metadata() {
        let registry = get_builtin_word_registry();
        for word in registry {
            assert!(
                matches!(
                    word.partiality,
                    Partiality::Total | Partiality::Partial | Partiality::Projecting
                ),
                "{} must declare partiality",
                word.name
            );
            assert!(
                matches!(
                    word.nil_policy,
                    NilPolicy::Passthrough
                        | NilPolicy::CreatesNil
                        | NilPolicy::RejectsNil
                        | NilPolicy::ConsumesNil
                        | NilPolicy::PreservesReason
                ),
                "{} must declare nil_policy",
                word.name
            );
            assert!(
                matches!(
                    word.safety_level,
                    SafetyLevel::A
                        | SafetyLevel::B
                        | SafetyLevel::C
                        | SafetyLevel::D
                        | SafetyLevel::Quarantined
                ),
                "{} must declare safety_level",
                word.name
            );
        }
    }

    #[test]
    fn aq_ver_contract_b_arithmetic_passthrough_partial_division() {
        let div = get_coreword_metadata("DIV").expect("DIV must be in registry");
        assert_eq!(div.partiality, Partiality::Partial);
        assert_eq!(div.nil_policy, NilPolicy::Passthrough);
        assert_eq!(div.safety_level, SafetyLevel::B);

        let add = get_coreword_metadata("ADD").expect("ADD must be in registry");
        assert_eq!(add.partiality, Partiality::Total);
        assert_eq!(add.nil_policy, NilPolicy::Passthrough);
        assert_eq!(add.safety_level, SafetyLevel::A);
    }

    #[test]
    fn aq_ver_contract_c_effectful_words_have_d_or_quarantined_safety() {
        let registry = get_builtin_word_registry();
        for word in registry
            .iter()
            .filter(|w| w.purity == WordPurity::Effectful)
        {
            assert!(
                matches!(word.safety_level, SafetyLevel::D | SafetyLevel::Quarantined),
                "{} effectful words must have safety_level D or Quarantined, got {:?}",
                word.name,
                word.safety_level
            );
        }
    }

    #[test]
    fn aq_ver_contract_d_runtime_handle_words_are_quarantined() {
        for name in &["SPAWN", "AWAIT", "STATUS", "KILL", "MONITOR", "SUPERVISE"] {
            let meta = get_coreword_metadata(name)
                .unwrap_or_else(|| panic!("{} must be in registry", name));
            assert_eq!(
                meta.safety_level,
                SafetyLevel::Quarantined,
                "{} must be Quarantined",
                name
            );
        }
    }
}
