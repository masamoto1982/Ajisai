//! `ajisai coverage` — mechanical aggregation of the contract coverage ratio
//! defined in `docs/dev/capability-transition-measurement-design.md` §4:
//! the fraction of word occurrences in a program that resolve to a definition
//! carrying complete §7.14 contract metadata.
//!
//! Observational only: this module executes nothing and defines no language
//! semantics (canonical source: `SPECIFICATION.html`). Classification mirrors
//! the static best-effort resolution of `ajisai check` (`resolve_words`), so
//! the two commands agree on what a bare token binds to.

use crate::coreword_registry::get_coreword_metadata;
use crate::interpreter::Interpreter;
use crate::types::Token;
use std::collections::HashSet;

/// Version tag from the design memo §6 (`TRANSITION_METRICS_VERSION`).
/// Any change to the counting rules in this module — what enters the
/// denominator, what counts as covered — must increment this constant and
/// update the memo in the same change; ratios produced under different
/// versions are not comparable.
pub(crate) const TRANSITION_METRICS_VERSION: u64 = 1;

/// Modifier words are excluded from the denominator (memo §4): they select
/// *how* a word touches the stream, and the "contract of a word" concept does
/// not apply to them. The `^` sugar never reaches this module — it tokenizes
/// as `Token::NilCoalesce`, not as a `Symbol` (spelled-out `VENT` counts).
const MODIFIER_WORDS: [&str; 4] = ["TOP", "STAK", "EAT", "KEEP"];

/// How a counted word occurrence resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OccurrenceKind {
    /// Canonical Core word with complete §7.14 metadata.
    Core,
    /// Module word (qualified, canonical-module bare form, or imported short
    /// name) with complete §7.14 metadata.
    Module,
    /// Word the file itself defines via `DEF` — no contract declaration
    /// mechanism exists for user words yet, so these count as uncovered
    /// (memo §4, deliberately conservative).
    UserDefined,
    /// Qualified `DICT@WORD` reference into a user dictionary (runtime
    /// state); statically accepted but carries no contract metadata.
    UserDictionary,
    /// Known to the interpreter's core vocabulary but absent from the §7.14
    /// registry — a registry gap, reported honestly as uncovered.
    Unregistered,
    /// Does not resolve at all.
    Unknown,
}

impl OccurrenceKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            OccurrenceKind::Core => "core",
            OccurrenceKind::Module => "module",
            OccurrenceKind::UserDefined => "userDefined",
            OccurrenceKind::UserDictionary => "userDictionary",
            OccurrenceKind::Unregistered => "unregistered",
            OccurrenceKind::Unknown => "unknown",
        }
    }

    pub(crate) fn is_covered(self) -> bool {
        matches!(self, OccurrenceKind::Core | OccurrenceKind::Module)
    }

    const ALL: [OccurrenceKind; 6] = [
        OccurrenceKind::Core,
        OccurrenceKind::Module,
        OccurrenceKind::UserDefined,
        OccurrenceKind::UserDictionary,
        OccurrenceKind::Unregistered,
        OccurrenceKind::Unknown,
    ];
}

/// An uncovered word with its occurrence count, in first-appearance order.
pub(crate) struct UncoveredWord {
    pub word: String,
    pub kind: OccurrenceKind,
    pub count: u64,
}

/// The aggregated coverage of one program.
pub(crate) struct Coverage {
    /// Denominator: counted word occurrences (literals, modifiers, and
    /// structural tokens excluded).
    pub total: u64,
    /// Numerator: occurrences resolving to complete §7.14 metadata.
    pub covered: u64,
    /// Modifier occurrences excluded from the denominator (informational).
    pub excluded_modifiers: u64,
    /// Occurrence count per resolution kind.
    pub by_kind: [(OccurrenceKind, u64); 6],
    pub uncovered: Vec<UncoveredWord>,
}

impl Coverage {
    pub(crate) fn to_json(&self) -> serde_json::Value {
        let mut breakdown = serde_json::Map::new();
        for (kind, count) in &self.by_kind {
            breakdown.insert(kind.as_str().to_string(), (*count).into());
        }
        serde_json::json!({
            "transitionMetricsVersion": TRANSITION_METRICS_VERSION,
            "covered": self.covered,
            "total": self.total,
            "ratioDisplay": format!("{}/{}", self.covered, self.total),
            "excludedModifierCount": self.excluded_modifiers,
            "breakdown": breakdown,
            "uncovered": self.uncovered.iter().map(|u| serde_json::json!({
                "word": u.word,
                "kind": u.kind.as_str(),
                "count": u.count,
            })).collect::<Vec<_>>(),
        })
    }
}

/// Count and classify every word occurrence in the token stream.
///
/// Denominator rules (memo §4): only `Token::Symbol` occurrences enter, and
/// modifier words are excluded. Number/string literals, vector and block
/// brackets, `^` (`NilCoalesce`), pipeline/clause separators, line breaks,
/// and comments (dropped by the tokenizer) never reach the count.
pub(crate) fn analyze(interp: &Interpreter, tokens: &[Token]) -> Coverage {
    let (defined, imported_shorts) = local_context(tokens);
    let modules: HashSet<String> = crate::interpreter::modules::available_module_names()
        .into_iter()
        .map(|name| name.to_uppercase())
        .collect();

    let mut total: u64 = 0;
    let mut covered: u64 = 0;
    let mut excluded_modifiers: u64 = 0;
    let mut by_kind = OccurrenceKind::ALL.map(|kind| (kind, 0u64));
    let mut uncovered: Vec<UncoveredWord> = Vec::new();

    for token in tokens {
        let Token::Symbol(symbol) = token else {
            continue;
        };
        let normalized = super::normalize_word(symbol);
        let canonical = crate::core_word_aliases::canonicalize_core_word_name(&normalized);
        if MODIFIER_WORDS.contains(&canonical.as_ref()) {
            excluded_modifiers += 1;
            continue;
        }
        let kind = classify(interp, &canonical, &modules, &defined, &imported_shorts);
        total += 1;
        if kind.is_covered() {
            covered += 1;
        } else if let Some(entry) = uncovered
            .iter_mut()
            .find(|u| u.word == canonical.as_ref() && u.kind == kind)
        {
            entry.count += 1;
        } else {
            uncovered.push(UncoveredWord {
                word: canonical.to_string(),
                kind,
                count: 1,
            });
        }
        for slot in by_kind.iter_mut() {
            if slot.0 == kind {
                slot.1 += 1;
            }
        }
    }

    Coverage {
        total,
        covered,
        excluded_modifiers,
        by_kind,
        uncovered,
    }
}

/// Resolution mirror of `resolve_words` (cli/mod.rs), refined into coverage
/// kinds. Bare names prefer the registry (which itself prefers a Canonical
/// Core entry, matching runtime resolution order) before file-local context.
fn classify(
    interp: &Interpreter,
    canonical: &str,
    modules: &HashSet<String>,
    defined: &HashSet<String>,
    imported_shorts: &HashSet<String>,
) -> OccurrenceKind {
    if let Some((module, _)) = canonical.split_once('@') {
        return if modules.contains(module) {
            if get_coreword_metadata(canonical).is_some() {
                OccurrenceKind::Module
            } else {
                OccurrenceKind::Unknown
            }
        } else {
            OccurrenceKind::UserDictionary
        };
    }
    if let Some(metadata) = get_coreword_metadata(canonical) {
        return if metadata.is_canonical_core() {
            OccurrenceKind::Core
        } else {
            OccurrenceKind::Module
        };
    }
    if imported_shorts.contains(canonical) {
        return OccurrenceKind::Module;
    }
    if defined.contains(canonical) {
        return OccurrenceKind::UserDefined;
    }
    if interp.core_vocabulary.contains_key(canonical) {
        return OccurrenceKind::Unregistered;
    }
    OccurrenceKind::Unknown
}

/// File-local context pre-pass, identical in shape to the one in
/// `resolve_words`: `'NAME' DEF` definitions and short names made available
/// by `'MODULE' IMPORT[-ONLY]`, anywhere in the file.
fn local_context(tokens: &[Token]) -> (HashSet<String>, HashSet<String>) {
    let mut defined: HashSet<String> = HashSet::new();
    let mut imported_shorts: HashSet<String> = HashSet::new();
    for (i, token) in tokens.iter().enumerate() {
        let Token::String(text) = token else {
            continue;
        };
        let next_words: Vec<String> = tokens[i + 1..]
            .iter()
            .filter(|t| !matches!(t, Token::LineBreak))
            .take(2)
            .filter_map(|t| match t {
                Token::Symbol(s) => Some(super::normalize_word(s)),
                _ => None,
            })
            .collect();
        if next_words.iter().any(|w| w == "DEF") {
            defined.insert(text.to_uppercase());
        }
        if next_words
            .iter()
            .any(|w| w == "IMPORT" || w == "IMPORT-ONLY")
        {
            let module = text.to_uppercase();
            if let Some(catalog) = crate::interpreter::modules::module_catalog_words(&module) {
                for word in catalog {
                    imported_shorts.insert(word.short_name.to_uppercase());
                }
            }
        }
    }
    (defined, imported_shorts)
}
