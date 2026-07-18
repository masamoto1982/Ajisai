use crate::builtins::builtin_specs;
use crate::interpreter::modules::module_word_metadata_entries;
use crate::interpreter::HostCapability;
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

/// Canonical implementation home for a built-in word.
///
/// Every built-in word has exactly one canonical home. `Core` means the word
/// is implemented as a Canonical Core word in `builtins/`, while `Module(m)`
/// means the canonical implementation lives in module `m` and is invoked as
/// `m@WORD` after `IMPORT 'm'`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "module", rename_all = "lowercase")]
pub enum CanonicalHome {
    Core,
    Module(String),
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
    /// Capability required when `profile == Hosted`; absent for Core words.
    pub required_capability: Option<HostCapability>,
    /// Where the canonical implementation lives (Core or a specific module).
    pub canonical_home: CanonicalHome,
    /// Whether the word appears in the Core word listing view.
    pub listed_in_core: bool,
    /// Module names whose dictionary view includes this word. A word may be
    /// listed in modules other than its canonical home (boundary words).
    /// Listing is presentation-only — it does not affect IMPORT or execution.
    pub listed_in_modules: Vec<String>,
    /// Documentation-only category labels (e.g. CAST, TEXT, TENSOR, RUNTIME)
    /// used by GUI/docs to group words. These are not real modules and cannot
    /// be `IMPORT`ed.
    pub listed_in_categories: Vec<String>,
}

impl CorewordMetadata {
    pub fn is_canonical_core(&self) -> bool {
        matches!(self.canonical_home, CanonicalHome::Core)
    }

    pub fn is_canonical_module(&self) -> bool {
        matches!(self.canonical_home, CanonicalHome::Module(_))
    }

    pub fn canonical_module(&self) -> Option<&str> {
        match &self.canonical_home {
            CanonicalHome::Module(m) => Some(m.as_str()),
            CanonicalHome::Core => None,
        }
    }

    pub fn is_core_listed(&self) -> bool {
        self.listed_in_core
    }

    pub fn is_module_listed(&self) -> bool {
        !self.listed_in_modules.is_empty()
    }

    pub fn is_category_listed(&self) -> bool {
        !self.listed_in_categories.is_empty()
    }

    /// A boundary word appears in both the Core listing view and at least one
    /// module-or-category listing view.
    pub fn is_boundary_word(&self) -> bool {
        self.listed_in_core && (self.is_module_listed() || self.is_category_listed())
    }
}

/// Boundary listing table. For Canonical Core words that should also appear
/// in a module listing view (real modules) and/or a documentation category
/// view (presentation-only labels). Listing is presentation-only — it does
/// **not** add the word to that module's IMPORT-able set, and it does not
/// create new module entities.
///
/// Entries: `(WORD, &[real_module_listings], &[category_listings])`.
const CORE_BOUNDARY_LISTINGS: &[(&str, &[&str], &[&str])] = &[
    ("PRINT", &["IO"], &[]),
    ("STR", &[], &["CAST"]),
    ("NUM", &[], &["CAST"]),
    ("BOOL", &[], &["CAST"]),
    ("CHR", &[], &["TEXT"]),
    ("CHARS", &[], &["TEXT"]),
    ("JOIN", &[], &["TEXT"]),
    ("TRIM", &[], &["TEXT"]),
    ("TRIM-LEFT", &[], &["TEXT"]),
    ("TRIM-RIGHT", &[], &["TEXT"]),
    ("TOKENIZE", &[], &["TEXT"]),
    ("SUBSTITUTE", &[], &["TEXT"]),
    ("STARTS-WITH?", &[], &["TEXT"]),
    ("ENDS-WITH?", &[], &["TEXT"]),
    ("MOD", &["MATH"], &[]),
    ("FLOOR", &["MATH"], &[]),
    ("CEIL", &["MATH"], &[]),
    ("ROUND", &["MATH"], &[]),
    ("QUANTIZE", &["MATH"], &[]),
    ("QUANTIZE-HALF-AWAY", &["MATH"], &[]),
    ("QUANTIZE-FLOOR", &["MATH"], &[]),
    ("QUANTIZE-CEIL", &["MATH"], &[]),
    ("QUANTIZE-TRUNC", &["MATH"], &[]),
    ("SHAPE", &[], &["TENSOR"]),
    ("RANK", &[], &["TENSOR"]),
    ("RESHAPE", &[], &["TENSOR"]),
    ("TRANSPOSE", &[], &["TENSOR"]),
    ("FILL", &[], &["TENSOR"]),
    ("SPAWN", &[], &["RUNTIME"]),
    ("AWAIT", &[], &["RUNTIME"]),
    ("STATUS", &[], &["RUNTIME"]),
    ("KILL", &[], &["RUNTIME"]),
    ("MONITOR", &[], &["RUNTIME"]),
    ("SUPERVISE", &[], &["RUNTIME"]),
];

/// Canonical Module words that should additionally appear in the Core listing
/// view (e.g. `SORT` is canonically `ALGO@SORT`, but is also surfaced in the
/// Core dictionary because it's central to vector reasoning).
///
/// Listing is presentation-only — calling bare `SORT` still requires
/// `'ALGO' IMPORT` per current execution semantics. This table only affects
/// `listed_in_core`, never name resolution.
const MODULE_CORE_LISTINGS: &[&str] = &["SORT"];

fn apply_core_boundary_listings(meta: &mut CorewordMetadata) {
    if !meta.is_canonical_core() {
        return;
    }
    for (name, modules, categories) in CORE_BOUNDARY_LISTINGS {
        if *name == meta.name {
            for m in *modules {
                if !meta.listed_in_modules.iter().any(|x| x == m) {
                    meta.listed_in_modules.push((*m).to_string());
                }
            }
            for c in *categories {
                if !meta.listed_in_categories.iter().any(|x| x == c) {
                    meta.listed_in_categories.push((*c).to_string());
                }
            }
            return;
        }
    }
}

fn apply_module_to_core_listings(meta: &mut CorewordMetadata) {
    if !meta.is_canonical_module() {
        return;
    }
    if MODULE_CORE_LISTINGS.iter().any(|n| *n == meta.name) {
        meta.listed_in_core = true;
    }
}

fn build_builtin_word_registry() -> Vec<CorewordMetadata> {
    let mut registry: Vec<CorewordMetadata> = builtin_specs()
        .iter()
        .map(core_word_metadata_from_spec)
        .collect();
    for meta in registry.iter_mut() {
        apply_core_boundary_listings(meta);
    }
    let mut module_entries = module_word_metadata_entries();
    for meta in module_entries.iter_mut() {
        apply_module_to_core_listings(meta);
    }
    registry.extend(module_entries);
    registry
}

/// The complete built-in word registry. Built once on first access and
/// cached for the process lifetime.
pub fn get_builtin_word_registry() -> &'static [CorewordMetadata] {
    static REGISTRY: std::sync::OnceLock<Vec<CorewordMetadata>> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(build_builtin_word_registry)
}

/// Metadata lookup with namespace-aware disambiguation.
///
/// - `MODULE@WORD` form returns the canonical module entry (or `None` if the
///   module does not own that word).
/// - Bare `WORD` form prefers a Canonical Core entry when one exists; only
///   when no core entry matches does it fall back to a canonical module
///   entry. This mirrors the runtime resolution order in
///   `interpreter/resolve-word.rs`, so callers reasoning about the visible
///   binding for a bare token see the same word the interpreter would run.
pub fn get_coreword_metadata(name: &str) -> Option<CorewordMetadata> {
    let upper = name.to_uppercase();
    let registry = get_builtin_word_registry();

    if let Some((module, word)) = upper.split_once('@') {
        return registry
            .iter()
            .find(|m| {
                m.name == word && m.canonical_module().map(|cm| cm == module).unwrap_or(false)
            })
            .cloned();
    }

    if let Some(core) = registry
        .iter()
        .find(|m| m.name == upper && m.is_canonical_core())
    {
        return Some(core.clone());
    }
    registry.iter().find(|m| m.name == upper).cloned()
}

/// Alias of `get_coreword_metadata`. Use this in new code; the registry
/// covers all built-in words regardless of canonical home.
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

/// Words whose Core listing view includes them (canonical core + core-listed
/// boundary words).
pub fn get_core_listed_words() -> Vec<CorewordMetadata> {
    get_builtin_word_registry()
        .iter()
        .filter(|word| word.listed_in_core)
        .cloned()
        .collect()
}

/// Words whose listing includes the given module name. Includes canonical
/// module words for that module plus core boundary words listed there.
pub fn get_module_listed_words(module_name: &str) -> Vec<CorewordMetadata> {
    let needle = module_name.to_uppercase();
    get_builtin_word_registry()
        .iter()
        .filter(|word| {
            word.canonical_module()
                .map(|m| m == needle)
                .unwrap_or(false)
                || word.listed_in_modules.iter().any(|m| *m == needle)
        })
        .cloned()
        .collect()
}

/// Words tagged with the given documentation category (e.g. CAST, TEXT,
/// TENSOR, RUNTIME). Categories are presentation-only — they are not real
/// modules and do not participate in IMPORT.
pub fn get_category_listed_words(category: &str) -> Vec<CorewordMetadata> {
    let needle = category.to_uppercase();
    get_builtin_word_registry()
        .iter()
        .filter(|word| word.listed_in_categories.iter().any(|c| *c == needle))
        .cloned()
        .collect()
}

pub fn get_canonical_core_words() -> Vec<CorewordMetadata> {
    get_builtin_word_registry()
        .iter()
        .filter(|word| word.is_canonical_core())
        .cloned()
        .collect()
}

/// Canonical Module words. When `module_name` is `Some(m)`, restricts to that
/// module's canonical words.
pub fn get_canonical_module_words(module_name: Option<&str>) -> Vec<CorewordMetadata> {
    let needle = module_name.map(|m| m.to_uppercase());
    get_builtin_word_registry()
        .iter()
        .filter(|word| match (&needle, word.canonical_module()) {
            (Some(n), Some(m)) => n == m,
            (None, Some(_)) => true,
            _ => false,
        })
        .cloned()
        .collect()
}

pub fn get_boundary_words() -> Vec<CorewordMetadata> {
    get_builtin_word_registry()
        .iter()
        .filter(|word| word.is_boundary_word())
        .cloned()
        .collect()
}

/// Returns true if `word_name` is core-listed only (canonical core, no
/// canonical module home). Used by IMPORT-ONLY to silently skip selectors
/// that are core-listed in a module view but not actually owned by that
/// module.
pub fn is_listing_only_for_module(word_name: &str, module_name: &str) -> bool {
    let upper = word_name.to_uppercase();
    let module_upper = module_name.to_uppercase();
    let Some(meta) = get_coreword_metadata(&upper) else {
        return false;
    };
    if meta
        .canonical_module()
        .map(|m| m == module_upper)
        .unwrap_or(false)
    {
        return false;
    }
    meta.listed_in_modules.iter().any(|m| *m == module_upper)
        || meta.listed_in_categories.iter().any(|c| *c == module_upper)
}

pub fn is_safe_preview_word(name: &str) -> bool {
    get_coreword_metadata(name)
        .map(|word| word.safe_preview)
        .unwrap_or(false)
}

/// Validates that no two registry entries share both `name` AND
/// `canonical_home`. Two entries with the same bare name but different homes
/// (e.g. core `GET` vs `JSON@GET`) are legitimate — they live in distinct
/// runtime namespaces and are disambiguated by `get_coreword_metadata`.
/// Used internally by tests to guard against accidental true duplicates.
#[cfg(test)]
fn collect_duplicate_entries(registry: &[CorewordMetadata]) -> Vec<(String, CanonicalHome)> {
    let mut seen: HashSet<(String, CanonicalHome)> = HashSet::new();
    let mut dupes: Vec<(String, CanonicalHome)> = Vec::new();
    for word in registry {
        let key = (word.name.clone(), word.canonical_home.clone());
        if !seen.insert(key.clone()) {
            dupes.push(key);
        }
    }
    dupes
}

/// Returns bare names that appear under more than one canonical home. These
/// are not bugs but require namespace-aware lookup.
#[cfg(test)]
fn collect_namespace_overlapping_names(registry: &[CorewordMetadata]) -> Vec<String> {
    use std::collections::BTreeMap;
    let mut by_name: BTreeMap<&str, Vec<&CanonicalHome>> = BTreeMap::new();
    for word in registry {
        by_name
            .entry(&word.name)
            .or_default()
            .push(&word.canonical_home);
    }
    by_name
        .into_iter()
        .filter(|(_, homes)| homes.len() > 1)
        .map(|(name, _)| name.to_string())
        .collect()
}

#[cfg(test)]
impl std::hash::Hash for CanonicalHome {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            CanonicalHome::Core => 0u8.hash(state),
            CanonicalHome::Module(m) => {
                1u8.hash(state);
                m.hash(state);
            }
        }
    }
}

fn builtin_profile(name: &str) -> (WordProfile, Option<HostCapability>) {
    match name {
        "PRINT" => (WordProfile::Hosted, Some(HostCapability::Effect)),
        _ => (WordProfile::Core, None),
    }
}

fn core_word_metadata_from_spec(spec: &crate::builtins::BuiltinSpec) -> CorewordMetadata {
    let (profile, required_capability) = builtin_profile(spec.name);
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
        required_capability,
        canonical_home: CanonicalHome::Core,
        listed_in_core: true,
        listed_in_modules: Vec::new(),
        listed_in_categories: Vec::new(),
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
        partiality: Partiality::Total,
        nil_policy: NilPolicy::Passthrough,
        safety_level: SafetyLevel::A,
        mass: mass_contract(name),
        profile: WordProfile::Core,
        required_capability: None,
        canonical_home: CanonicalHome::Core,
        listed_in_core: true,
        listed_in_modules: Vec::new(),
        listed_in_categories: Vec::new(),
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
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::C,
        mass: mass_contract(name),
        profile: WordProfile::Core,
        required_capability: None,
        canonical_home: CanonicalHome::Core,
        listed_in_core: true,
        listed_in_modules: Vec::new(),
        listed_in_categories: Vec::new(),
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
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::D,
        mass: mass_contract(name),
        profile: WordProfile::Core,
        required_capability: None,
        canonical_home: CanonicalHome::Core,
        listed_in_core: true,
        listed_in_modules: Vec::new(),
        listed_in_categories: Vec::new(),
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
        collect_duplicate_entries, collect_namespace_overlapping_names, get_boundary_words,
        get_builtin_word_metadata, get_builtin_word_registry, get_canonical_core_words,
        get_canonical_module_words, get_core_listed_words, get_coreword_metadata,
        get_hosted_profile_words, get_module_listed_words, is_listing_only_for_module,
        is_safe_preview_word, CanonicalHome, NilPolicy, Partiality, SafetyLevel, WordProfile,
        WordPurity,
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
    fn aq_ver_contract_h_min_max_sort_project_undecidable_comparison() {
        // SPEC §7.4.3 / §7.14: MIN, MAX, and SORT are total *by projection* —
        // an undecidable governing comparison is projected onto the logical
        // Unknown (U) rather than raising. They must therefore be Projecting,
        // not the pure-class default Total, with Passthrough nil_policy (NIL
        // takes priority over a U-producing comparison, §4.5.2) and safety A
        // (Projecting words are total-by-projection, §7.14). This guards the
        // contract_override in module_builtins from regressing to Total.
        for name in &["MATH@MIN", "MATH@MAX", "ALGO@SORT"] {
            let meta = get_coreword_metadata(name)
                .unwrap_or_else(|| panic!("{} must be in registry", name));
            assert_eq!(
                meta.partiality,
                Partiality::Projecting,
                "{} must be Projecting (SPEC §7.4.3, §7.14)",
                name
            );
            assert_eq!(
                meta.nil_policy,
                NilPolicy::Passthrough,
                "{} must be Passthrough (SPEC §7.4.3)",
                name
            );
            assert_eq!(
                meta.safety_level,
                SafetyLevel::A,
                "{} stays SafetyLevel A (Projecting is total-by-projection, §7.14)",
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
        for name in &[
            "NIL?",
            "NIL-REASON",
            "NIL-ORIGIN",
            "NIL-RECOVERABLE?",
            "NIL-DIAGNOSIS",
        ] {
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

    // ---------------------------------------------------------------------
    // AQ-VER-LISTING — Canonical home / listing tests for the redesigned
    // built-in word vocabulary.
    // ---------------------------------------------------------------------

    #[test]
    fn aq_ver_listing_a_no_two_entries_share_name_and_home() {
        let registry = get_builtin_word_registry();
        let dupes = collect_duplicate_entries(&registry);
        assert!(
            dupes.is_empty(),
            "(name, canonical_home) pair must be unique (duplicates: {:?})",
            dupes
        );
    }

    /// Bare names like `GET` legitimately appear under multiple canonical
    /// homes (core list `GET` and `JSON@GET`). The registry intentionally
    /// keeps both entries — they live in distinct runtime namespaces — but
    /// `get_coreword_metadata("GET")` must always disambiguate to the
    /// canonical core entry, matching the runtime resolution order.
    #[test]
    fn aq_ver_listing_b_namespace_overlap_disambiguates_to_core() {
        let registry = get_builtin_word_registry();
        let overlapping = collect_namespace_overlapping_names(&registry);
        for name in overlapping {
            let resolved = get_coreword_metadata(&name)
                .unwrap_or_else(|| panic!("{} must resolve via bare lookup", name));
            assert!(
                resolved.is_canonical_core(),
                "{} bare lookup must prefer the core canonical entry, got {:?}",
                name,
                resolved.canonical_home
            );
            // The qualified form must reach the module entry instead.
            if let Some(module_entry) = registry
                .iter()
                .find(|m| m.name == name && matches!(m.canonical_home, CanonicalHome::Module(_)))
            {
                let module_name = module_entry
                    .canonical_module()
                    .expect("module entry must have canonical_module");
                let qualified = format!("{}@{}", module_name, name);
                let qualified_resolved = get_coreword_metadata(&qualified)
                    .unwrap_or_else(|| panic!("{} must resolve via qualified lookup", qualified));
                assert_eq!(
                    qualified_resolved.canonical_home,
                    CanonicalHome::Module(module_name.to_string()),
                    "{} qualified lookup must reach the module entry",
                    qualified
                );
            }
        }
    }

    #[test]
    fn aq_ver_listing_c_every_word_has_at_least_one_listing() {
        let registry = get_builtin_word_registry();
        for word in registry {
            let listed = word.listed_in_core
                || !word.listed_in_modules.is_empty()
                || !word.listed_in_categories.is_empty();
            assert!(
                listed,
                "{} must be listed in at least one dictionary view",
                word.name
            );
        }
    }

    #[test]
    fn aq_ver_listing_d_canonical_module_implies_module_listing() {
        for word in get_canonical_module_words(None) {
            let canonical = word
                .canonical_module()
                .expect("canonical module word must report canonical_module")
                .to_string();
            assert!(
                word.listed_in_modules.iter().any(|m| *m == canonical),
                "{} canonical module {} must appear in listed_in_modules ({:?})",
                word.name,
                canonical,
                word.listed_in_modules
            );
        }
    }

    #[test]
    fn aq_ver_listing_f_print_is_canonical_core_listed_in_io() {
        let print = get_builtin_word_metadata("PRINT").expect("PRINT must be in registry");
        assert_eq!(print.canonical_home, CanonicalHome::Core);
        assert!(print.is_canonical_core());
        assert!(print.listed_in_core);
        assert!(print.listed_in_modules.iter().any(|m| m == "IO"));
        assert!(print.is_boundary_word());
    }

    #[test]
    fn aq_ver_listing_g_sort_is_canonical_algo_and_core_listed() {
        let sort = get_builtin_word_metadata("SORT").expect("SORT must be in registry");
        assert_eq!(
            sort.canonical_home,
            CanonicalHome::Module("ALGO".to_string())
        );
        assert!(sort.is_canonical_module());
        assert!(sort.listed_in_core, "SORT must be core-listed");
        assert!(sort.listed_in_modules.iter().any(|m| m == "ALGO"));
        assert!(sort.is_boundary_word());
    }

    #[test]
    fn aq_ver_listing_h_csprng_is_module_only() {
        let csprng = get_builtin_word_metadata("CSPRNG").expect("CSPRNG must be in registry");
        assert_eq!(
            csprng.canonical_home,
            CanonicalHome::Module("CRYPTO".to_string())
        );
        assert!(!csprng.listed_in_core, "CSPRNG must not be core-listed");
        assert_eq!(csprng.listed_in_modules, vec!["CRYPTO".to_string()]);
        assert!(csprng.listed_in_categories.is_empty());
        assert!(!csprng.is_boundary_word());
    }

    #[test]
    fn aq_ver_listing_i_import_is_core_only() {
        for name in &["IMPORT", "IMPORT-ONLY"] {
            let meta = get_builtin_word_metadata(name)
                .unwrap_or_else(|| panic!("{} must be in registry", name));
            assert_eq!(
                meta.canonical_home,
                CanonicalHome::Core,
                "{} must be canonical core",
                name
            );
            assert!(meta.listed_in_core, "{} must be core-listed", name);
            assert!(
                meta.listed_in_modules.is_empty(),
                "{} must not be module-listed",
                name
            );
            assert!(
                meta.listed_in_categories.is_empty(),
                "{} must not be category-listed",
                name
            );
        }
    }

    #[test]
    fn aq_ver_listing_j_known_boundary_words_classified() {
        let expected = [
            "PRINT",
            "STR",
            "NUM",
            "BOOL",
            "CHR",
            "CHARS",
            "JOIN",
            "MOD",
            "FLOOR",
            "CEIL",
            "ROUND",
            "QUANTIZE",
            "QUANTIZE-HALF-AWAY",
            "QUANTIZE-FLOOR",
            "QUANTIZE-CEIL",
            "QUANTIZE-TRUNC",
            "SHAPE",
            "RANK",
            "RESHAPE",
            "TRANSPOSE",
            "FILL",
            "SPAWN",
            "AWAIT",
            "STATUS",
            "KILL",
            "MONITOR",
            "SUPERVISE",
            "SORT",
        ];
        let boundary_names: Vec<String> =
            get_boundary_words().into_iter().map(|w| w.name).collect();
        for name in expected {
            assert!(
                boundary_names.iter().any(|n| n == name),
                "{} must be classified as a boundary word (got: {:?})",
                name,
                boundary_names
            );
        }
    }

    #[test]
    fn aq_ver_listing_k_core_view_includes_core_listed_only() {
        for word in get_core_listed_words() {
            assert!(
                word.listed_in_core,
                "{} returned by get_core_listed_words must have listed_in_core=true",
                word.name
            );
        }
    }

    #[test]
    fn aq_ver_listing_l_module_view_includes_canonical_and_boundary() {
        let io_view: Vec<String> = get_module_listed_words("IO")
            .into_iter()
            .map(|w| w.name)
            .collect();
        assert!(
            io_view.iter().any(|n| n == "PRINT"),
            "IO view must include the boundary word PRINT (got: {:?})",
            io_view
        );
        let algo_view: Vec<String> = get_module_listed_words("ALGO")
            .into_iter()
            .map(|w| w.name)
            .collect();
        assert!(
            algo_view.iter().any(|n| n == "SORT"),
            "ALGO view must include canonical SORT (got: {:?})",
            algo_view
        );
    }

    #[test]
    fn aq_ver_listing_m_listing_only_predicate_distinguishes_canonical_from_boundary() {
        // PRINT is a Core word listed in IO → listing-only relative to IO.
        assert!(is_listing_only_for_module("PRINT", "IO"));
        // SORT is canonical to ALGO → NOT listing-only for ALGO.
        assert!(!is_listing_only_for_module("SORT", "ALGO"));
        // CSPRNG is canonical to CRYPTO → NOT listing-only for CRYPTO.
        assert!(!is_listing_only_for_module("CSPRNG", "CRYPTO"));
        // Unknown word → false.
        assert!(!is_listing_only_for_module("__NOSUCH__", "IO"));
    }

    #[test]
    fn aq_ver_profile_a_hosted_words_declare_capabilities() {
        let hosted = get_hosted_profile_words();
        assert!(
            hosted.iter().any(|w| w.name == "NOW"),
            "TIME@NOW must be classified as Hosted"
        );
        assert!(
            hosted.iter().any(|w| w.name == "CSPRNG"),
            "CRYPTO@CSPRNG must be classified as Hosted"
        );
        for word in hosted {
            assert_eq!(word.profile, WordProfile::Hosted);
            assert!(
                word.required_capability.is_some(),
                "{} Hosted words must declare a required capability",
                word.name
            );
        }
    }

    #[test]
    fn aq_ver_profile_b_core_profile_excludes_host_capabilities() {
        for word in get_builtin_word_registry()
            .iter()
            .filter(|word| word.profile == WordProfile::Core)
        {
            assert!(
                word.required_capability.is_none(),
                "{} Core-profile words must not require host capability metadata",
                word.name
            );
        }
    }

    #[test]
    fn aq_ver_listing_n_canonical_core_helper_excludes_module_words() {
        for word in get_canonical_core_words() {
            assert!(
                word.is_canonical_core(),
                "{} returned by get_canonical_core_words must be canonical core",
                word.name
            );
        }
    }
}
