use crate::builtins::builtin_specs;
use serde::Serialize;
#[cfg(test)]
use std::collections::HashSet;

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

/// Static mass contract (SPEC §13.1): a word's flow-mass relationship under the
/// default `TOP`/`EAT` mode. `consumes` operands are read and `produces` results
/// are pushed; under `KEEP` the `consumes` operands are additionally retained
/// (bifurcation, §13.2). This is the machine-readable form of the §13.1 "arity /
/// consumption / production / bifurcation" declaration; the NIL-projection part
/// of §13.1 is carried by `nil_policy`.
///
/// `Dynamic` marks a data-dependent arity (e.g. the `STAK` count-driven fold or
/// runtime-shaped vector ops) that is not statically pinned; the static
/// mass-conservation validator abstains on `Dynamic` words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum MassContract {
    Fixed { consumes: u8, produces: u8 },
    Dynamic,
}

impl MassContract {
    /// `(consumes, produces)` when the contract is statically fixed.
    pub fn fixed(self) -> Option<(u8, u8)> {
        match self {
            MassContract::Fixed { consumes, produces } => Some((consumes, produces)),
            MassContract::Dynamic => None,
        }
    }
}

/// How a Coreword takes effect, as a machine-readable signal independent of the
/// human-facing `stack_effect` prose.
///
/// Most words are ordinary `RuntimeWord`s dispatched by name and consuming/
/// producing stack values. The lazy control directives of SPEC §6.4 are not:
/// the tokenizer emits them as dedicated tokens (`^`/`VENT` -> `NilCoalesce`,
/// `~`/`FLOW` -> `Pipeline`) and the execution loop interprets the *following
/// source unit* positionally rather than popping operands. This enum lets
/// generators and consistency tests assert that classification instead of
/// parsing the prose.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ExecutionForm {
    /// Ordinary word: dispatched by name, operates on stack operands.
    RuntimeWord,
    /// No-op control directive: a positional marker with no runtime effect
    /// (e.g. `FLOW` / `~`).
    NoOpControlDirective,
    /// Lazy NIL-coalescing control directive: inspects the stack top and, if it
    /// is non-NIL, keeps it and skips the following source unit *unevaluated*;
    /// if it is NIL, discards it and evaluates the following source unit as the
    /// fallback. The fallback is source that follows the directive, never a
    /// value already on the stack (e.g. `VENT` / `^`).
    LazyNextUnitFallback,
}

/// The canonical mass contract for a Coreword, keyed by its canonical name.
/// Builtin mass is authored on `BuiltinSpec`; this adapter exists for older
/// analyzers until Phase 3 finishes moving all metadata consumers to the shared
/// typed spec directly. Unknown or non-core names conservatively return
/// `Dynamic`.
pub fn mass_contract(name: &str) -> MassContract {
    let canonical = crate::core_word_aliases::canonicalize_core_word_name(name);
    crate::builtins::lookup_builtin_spec(&canonical)
        .map(|spec| spec.mass)
        .unwrap_or(MassContract::Dynamic)
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
    pub purity: WordPurity,
    pub effects: Vec<String>,
    pub deterministic: bool,
    pub safe_preview: bool,
    pub partiality: Partiality,
    pub nil_policy: NilPolicy,
    pub safety_level: SafetyLevel,
    /// Static flow-mass contract (SPEC §13.1): arity / production, with
    /// bifurcation governed by the `KEEP` modifier (§13.2).
    pub mass: MassContract,
    /// Portability profile used by conformance tooling to keep the Core
    /// profile free of host-boundary words.
    pub profile: WordProfile,
}

fn build_builtin_word_registry() -> Vec<CorewordMetadata> {
    builtin_specs()
        .iter()
        .map(core_word_metadata_from_spec)
        .collect()
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

pub fn get_words_by_purity(purity: WordPurity) -> Vec<CorewordMetadata> {
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
    // Output is the only effect, so PRINT is the only non-Core-profile Word.
    if name == "PRINT" {
        WordProfile::Hosted
    } else {
        WordProfile::Core
    }
}

fn core_word_metadata_from_spec(spec: &crate::builtins::BuiltinSpec) -> CorewordMetadata {
    let profile = builtin_profile(spec.name);
    CorewordMetadata {
        name: spec.name.to_string(),
        category: spec.category.to_lowercase(),
        purity: spec.purity,
        effects: spec.effects.iter().map(|e| e.to_string()).collect(),
        deterministic: spec.deterministic,
        safe_preview: spec.safe_preview,
        partiality: spec.partiality,
        nil_policy: spec.nil_policy,
        safety_level: spec.safety_level,
        mass: spec.mass,
        profile,
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
        collect_duplicate_entries, get_builtin_word_registry, get_coreword_metadata,
        get_hosted_profile_words, is_safe_preview_word, NilPolicy, Partiality, SafetyLevel,
        WordProfile, WordPurity,
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
    fn aq_ver_contract_b_arithmetic_division_creates_nil_under_bubble_rule() {
        let div = get_coreword_metadata("DIV").expect("DIV must be in registry");
        assert_eq!(div.partiality, Partiality::Projecting);
        assert_eq!(div.nil_policy, NilPolicy::CreatesNil);
        assert_eq!(div.safety_level, SafetyLevel::B);

        let add = get_coreword_metadata("ADD").expect("ADD must be in registry");
        assert_eq!(add.partiality, Partiality::Total);
        assert_eq!(add.nil_policy, NilPolicy::Passthrough);
        assert_eq!(add.safety_level, SafetyLevel::A);
    }

    #[test]
    fn aq_ver_contract_f_comparison_words_project_undecidable_to_unknown() {
        // SPEC §7.14 (revised): all six comparison primitives are
        // Projecting/Passthrough/B. They are Projecting because every
        // well-shaped input yields a `TruthValue` result — the
        // comparison-budget exhaustion path (§7.4.1) projects onto the
        // logical `Unknown` (U), a TruthValue result, not a reasoned NIL.
        // They are Passthrough because they still pass NIL operands through
        // (§7.12); they are no longer CreatesNil for the undecidable case.
        for name in &["EQ", "NEQ", "LT", "LTE", "GT", "GTE"] {
            let meta = get_coreword_metadata(name)
                .unwrap_or_else(|| panic!("{} must be in registry", name));
            assert_eq!(
                meta.partiality,
                Partiality::Projecting,
                "{} must be Projecting (SPEC §7.14)",
                name
            );
            assert_eq!(
                meta.nil_policy,
                NilPolicy::Passthrough,
                "{} must be Passthrough (SPEC §7.14, revised)",
                name
            );
            assert_eq!(
                meta.safety_level,
                SafetyLevel::B,
                "{} must be SafetyLevel B (SPEC §9.4)",
                name
            );
        }
    }

    #[test]
    fn aq_ver_contract_g_rounding_modulo_create_nil_under_undecidable() {
        // MOD/FLOOR/CEIL/ROUND operate on ExactScalar (CF) operands whose
        // partial-quotient budget can exhaust, yielding an Undecidable NIL
        // (SPEC §7.4.1). They are therefore Projecting/CreatesNil/B, matching
        // DIV and the comparison words. ADD/SUB/MUL stay Total because their
        // CF arithmetic always yields a value (never a budget miss).
        for name in &["MOD", "FLOOR", "CEIL", "ROUND"] {
            let meta = get_coreword_metadata(name)
                .unwrap_or_else(|| panic!("{} must be in registry", name));
            assert_eq!(
                meta.partiality,
                Partiality::Projecting,
                "{} must be Projecting (SPEC §7.4.1)",
                name
            );
            assert_eq!(
                meta.nil_policy,
                NilPolicy::CreatesNil,
                "{} must be CreatesNil (SPEC §7.4.1)",
                name
            );
            assert_eq!(
                meta.safety_level,
                SafetyLevel::B,
                "{} must be SafetyLevel B",
                name
            );
        }

        // ADD/SUB/MUL must remain Total: their ExactReal arithmetic always
        // produces a value, so they never create a budget-exhaustion NIL.
        for name in &["ADD", "SUB", "MUL"] {
            let meta = get_coreword_metadata(name)
                .unwrap_or_else(|| panic!("{} must be in registry", name));
            assert_eq!(
                meta.nil_policy,
                NilPolicy::Passthrough,
                "{} must stay Passthrough (CF arithmetic is total)",
                name
            );
        }
    }

    #[test]
    fn aq_ver_contract_i_nil_diagnostic_accessors_consume_nil() {
        // SPEC §4.5.0 / §7.15: the five diagnostic absence accessors inspect a
        // NIL rather than propagate it, so their nil_policy is ConsumesNil (the
        // VENT-family "inspect or branch on NIL" classification). They are pure,
        // total, safety-A observations that retain their inspection target, so
        // their mass contract is Dynamic (net +1, like the LENGTH/GET
        // inspection words of §7.1.1 — a Fixed contract would mis-model the
        // retained operand for the static depth analyzer).
        for name in &["NIL?", "NIL-REASON"] {
            let meta = get_coreword_metadata(name)
                .unwrap_or_else(|| panic!("{} must be in registry", name));
            assert_eq!(
                meta.nil_policy,
                NilPolicy::ConsumesNil,
                "{} must be ConsumesNil (SPEC §4.5.0)",
                name
            );
            assert_eq!(
                meta.purity,
                WordPurity::Pure,
                "{} must be Pure (SPEC §7.15)",
                name
            );
            assert_eq!(
                meta.partiality,
                Partiality::Total,
                "{} must be Total — a well-formed observation never raises (SPEC §4.5.0)",
                name
            );
            assert_eq!(
                meta.safety_level,
                SafetyLevel::A,
                "{} must be SafetyLevel A (pure, total, deterministic)",
                name
            );
            assert!(
                matches!(meta.mass, super::MassContract::Dynamic),
                "{} retains its inspection target, so its mass contract is Dynamic",
                name
            );
        }
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
    fn aq_ver_contract_e_builtin_spec_stability_matches_safety_level() {
        // Three-layer documentation model §5.3: stability label must agree
        // with the §7.14 contract metadata declared on each `BuiltinSpec`.
        // The mapping is:
        //   safety_level A or B          -> "stable"
        //   safety_level C, D, or
        //   Quarantined                  -> "experimental"
        // This test catches drift between BuiltinSpec.stability and the
        // registry contract.
        for spec in crate::builtins::builtin_specs() {
            let meta = get_coreword_metadata(spec.name)
                .unwrap_or_else(|| panic!("{} must be in registry", spec.name));
            let expected = match meta.safety_level {
                SafetyLevel::A | SafetyLevel::B => "stable",
                SafetyLevel::C | SafetyLevel::D | SafetyLevel::Quarantined => "experimental",
            };
            assert_eq!(
                spec.stability, expected,
                "{}: BuiltinSpec.stability = {:?} but safety_level = {:?} maps to {:?}",
                spec.name, spec.stability, meta.safety_level, expected
            );
        }
    }

    #[test]
    fn aq_ver_contract_f_mass_contract_is_derived_from_builtin_spec() {
        for spec in crate::builtins::builtin_specs() {
            assert_eq!(
                super::mass_contract(spec.name),
                spec.mass,
                "{}: mass_contract adapter must read BuiltinSpec.mass",
                spec.name
            );
        }
    }

    #[test]
    fn aq_ver_listing_a_no_two_entries_share_a_name() {
        let registry = get_builtin_word_registry();
        let dupes = collect_duplicate_entries(registry);
        assert!(
            dupes.is_empty(),
            "built-in word names must be unique (duplicates: {:?})",
            dupes
        );
    }

    #[test]
    fn aq_ver_profile_a_print_is_the_only_hosted_word() {
        // Output is the only effect (LANG.EFFECTS.OUTPUT), so PRINT is the only
        // Word outside the Core profile.
        let hosted = get_hosted_profile_words();
        assert_eq!(
            hosted.iter().map(|w| w.name.as_str()).collect::<Vec<_>>(),
            vec!["PRINT"],
            "PRINT must be the only Hosted-profile Word"
        );
    }

    #[test]
    fn aq_ver_profile_b_core_profile_excludes_print() {
        for word in get_builtin_word_registry()
            .iter()
            .filter(|word| word.profile == WordProfile::Core)
        {
            assert_ne!(
                word.name, "PRINT",
                "PRINT is the effectful Word and must not be Core-profile"
            );
        }
    }
}
