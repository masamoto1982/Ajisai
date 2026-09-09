//! Per-word outcome vocabulary for static outcome prediction (Phase 5,
//! `docs/dev/auditable-kernel-work-order-2026-09.md`). Deliberately
//! independent of `word_contract.rs`'s `WordContract`/`AccumulatedContract`
//! (which do not carry outcome information and are already at the file's
//! own 500-line budget): a fresh, self-contained recursive walk over a
//! word's body, parallel to (not sharing state with) contract inference.
//!
//! A word's own declared vocabulary (`spec/words.json`'s `errorWhen` +
//! `projection.reason`, read from the spec itself — the generated registry's
//! own `projection` field carries the `when` condition names, not the
//! `reason` ids `errorWhen`'s sibling would suggest, so this reads the JSON
//! directly rather than through that indirection) is always sound by
//! construction: the outcome-bijection gate
//! (`scripts/check-outcome-bijection.mjs`) already proves every observed
//! outcome for a Word resolves into its declared repertoire. Composing a
//! program's prediction as the *union* of every reachable word's own
//! vocabulary can therefore never under-approximate — the one failure mode
//! Phase 5's pitfall A forbids — at the cost of precision: it does not try
//! to prove that a *specific* declared condition on a *specific* call is
//! actually unreachable given the operands that reach it. That is a known,
//! deliberate V1 limitation, not an oversight.
//!
//! # Why every Symbol counts, including one inside a data literal
//!
//! This walk deliberately does *not* consult
//! `word_contract_widen::classify_vector_positions`, though an earlier
//! version did. That classifier answers a different question — "does this
//! `[ ... ]` run *here*, at the point it is written" — which is exactly
//! right for arity/space/cost, where an unexecuted literal is one opaque
//! push. It is the wrong question for reachability: a block can be pushed
//! by one Word and executed by another, arbitrarily far away.
//! `[ [ 'a' ADD ] ] 'G' DEF 1 G EXEC` runs that `ADD` and answers
//! `nonNumeric`, but the classifier calls the inner block `Data` at every
//! point this walk sees it, so skipping `Data` dropped `nonNumeric` from
//! the prediction — an under-approximation, measured, not hypothetical.
//!
//! Counting every Symbol over-approximates instead: a Word name written in
//! a genuinely inert vector contributes a vocabulary nothing will ever
//! reach. That is the allowed direction (pitfall A), and `exact` reports
//! it.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use crate::error::{ErrorCategory, NilReason};
use crate::kernel::generated::GENERATED_WORDS;
use crate::types::{Token, WordDefinition};

use super::Interpreter;

const WORDS_JSON: &str = include_str!("../../../spec/words.json");

/// Every `spec/outcomes.json` error category that is `kind: "structural"`
/// (not any specific Word's own declared `errorWhen`) — the fixed,
/// non-`Declared` `ErrorCategory` variants, read through the real
/// `as_protocol_str()` so this can never drift from the wire spelling.
/// `DivisionByZero` is excluded: it is not a registered outcome category at
/// all (`scripts/check-outcome-registry.mjs`'s documented exclusion —
/// diagnostic-trace-only, Phase 2 of this work order).
fn structural_error_categories() -> [ErrorCategory; 14] {
    [
        ErrorCategory::StackUnderflow,
        ErrorCategory::StructureError,
        ErrorCategory::UnknownWord,
        ErrorCategory::IndexOutOfBounds,
        ErrorCategory::VectorLengthMismatch,
        ErrorCategory::ShapeMismatch,
        ErrorCategory::MalformedSource,
        ErrorCategory::NameConflict,
        ErrorCategory::ExecutionLimitExceeded,
        ErrorCategory::ResourceLimitExceeded,
        ErrorCategory::RecursionLimitExceeded,
        ErrorCategory::BuiltinProtection,
        ErrorCategory::CondExhausted,
        ErrorCategory::SelfReferentialDefinition,
    ]
}

/// `strings(value)` reads a schema field that is one string, an array of
/// strings, or absent/null (`errorWhen` is always an array; `projection.
/// when`/`projection.reason` are the string-or-array-or-null shape
/// `spec/words.schema.json` documents) into a uniform `Vec<&str>`.
fn strings(value: &serde_json::Value) -> Vec<&str> {
    match value {
        serde_json::Value::String(s) => vec![s.as_str()],
        serde_json::Value::Array(items) => items.iter().filter_map(|v| v.as_str()).collect(),
        _ => Vec::new(),
    }
}

/// One builtin's declared `errorWhen` conditions and projection reasons.
type WordVocabulary = (Vec<String>, Vec<String>);

/// Every builtin's own `(errorWhen, projection.reason)` from `spec/words.json`,
/// keyed by its canonical uppercase name — built once and cached, mirroring
/// `execution_receipt::registry_digest`'s reuse of the same embedded file for
/// a different purpose.
fn word_outcome_table() -> &'static HashMap<String, WordVocabulary> {
    static TABLE: std::sync::OnceLock<HashMap<String, WordVocabulary>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let parsed: serde_json::Value =
            serde_json::from_str(WORDS_JSON).expect("spec/words.json must parse");
        let mut table = HashMap::new();
        for entry in parsed["entries"].as_array().into_iter().flatten() {
            let Some(name) = entry["name"].as_str() else {
                continue;
            };
            let error_when = strings(&entry["errorWhen"])
                .into_iter()
                .map(str::to_string)
                .collect();
            let reasons = strings(&entry["projection"]["reason"])
                .into_iter()
                .map(str::to_string)
                .collect();
            table.insert(name.to_string(), (error_when, reasons));
        }
        table
    })
}

/// Every outcome id a builtin could ever produce, per `spec/words.json`'s own
/// declaration: `value`, one `error:<category>` per declared `errorWhen`
/// condition, and one `nil:<reason>` per declared projection reason.
pub(crate) fn builtin_outcomes_for(name: &str) -> BTreeSet<String> {
    let mut outcomes = BTreeSet::new();
    outcomes.insert("value".to_string());
    if let Some((error_when, reasons)) = word_outcome_table().get(&name.to_uppercase()) {
        for condition in error_when {
            outcomes.insert(format!("error:{condition}"));
        }
        for reason in reasons {
            outcomes.insert(format!("nil:{reason}"));
        }
    }
    outcomes
}

/// The full outcome universe: every NIL reason and error category
/// `spec/outcomes.json` registers, plus `value`. Used as the sound (but
/// maximally coarse) fallback whenever prediction cannot resolve a
/// dependency at all — the same "when in doubt, widen to everything" rule
/// `WordContract::conservative` already applies to the other contract axes.
/// Built from the Rust registry rather than parsed from `spec/outcomes.json`
/// text: `NilReason::ALL` plus the structural categories give every
/// non-`Declared` id, and unioning every generated Word's own `errorWhen`
/// gives every `Declared` one — exactly `spec/outcomes.json`'s two kinds,
/// per the bidirectional correspondence `scripts/check-outcome-registry.mjs`
/// already enforces between them.
pub(crate) fn conservative_outcomes() -> BTreeSet<String> {
    static UNIVERSE: std::sync::OnceLock<BTreeSet<String>> = std::sync::OnceLock::new();
    UNIVERSE
        .get_or_init(|| {
            let mut outcomes = BTreeSet::new();
            outcomes.insert("value".to_string());
            for reason in NilReason::ALL {
                outcomes.insert(format!("nil:{}", reason.as_protocol_str()));
            }
            for category in structural_error_categories() {
                outcomes.insert(format!("error:{}", category.as_protocol_str()));
            }
            for word in GENERATED_WORDS {
                for condition in word.error_when {
                    outcomes.insert(format!("error:{condition}"));
                }
            }
            outcomes
        })
        .clone()
}

/// Every structural error category (see `structural_error_categories`),
/// except `stackUnderflow` (given a precise, flow-sensitive answer by
/// `predict_program_outcomes`'s own `FlowSim` run) and `malformedSource`
/// (impossible past the point prediction's caller already tokenized the
/// source successfully). "Structural" means *not* any specific Word's own
/// declared `errorWhen` — a cross-cutting engine condition (a name reused
/// across scopes, a definition shadowing a builtin, a `COND` that runs out
/// of clauses, a numeric literal too long for the profile) that a per-word
/// vocabulary union can never include on its own, and so would otherwise
/// under-approximate for any non-trivial program. Included whenever the
/// program is non-empty (`predict_program_outcomes` decides that), not
/// narrowed further in V1 — see that module's doc for why, and for the one
/// declared exception (`OR-NIL`'s `missingFollowingSourceUnit`) that needs
/// its own handling instead, since it is tied to a specific Word's own
/// vocabulary but that Word tokenizes to `Token::NilCoalesce`, never a
/// `Token::Symbol("OR-NIL")` a normal body walk would see.
pub(crate) fn structural_ceiling_ids() -> &'static BTreeSet<String> {
    static CEILING: std::sync::OnceLock<BTreeSet<String>> = std::sync::OnceLock::new();
    CEILING.get_or_init(|| {
        structural_error_categories()
            .into_iter()
            .map(|category| category.as_protocol_str())
            .filter(|id| *id != "stackUnderflow" && *id != "malformedSource")
            .map(|id| format!("error:{id}"))
            .collect()
    })
}

/// The set of outcome ids reachable through `name`'s body: its own declared
/// vocabulary (if a builtin) or the union of every dependency it calls
/// (if user-defined), recursing through `DEF`'d words and `[ ... ]` code
/// operands alike. `visiting` guards a cycle that should not exist (DEF-time
/// acyclicity already rejects every reference cycle) but is kept as a
/// defensive fallback to the sound conservative universe rather than an
/// infinite recursion.
pub(crate) fn outcome_vocabulary_for_word(
    interp: &mut Interpreter,
    name: &str,
    def: &Arc<WordDefinition>,
    visiting: &mut HashSet<String>,
) -> BTreeSet<String> {
    if def.is_builtin {
        return builtin_outcomes_for(name);
    }
    if !visiting.insert(name.to_string()) {
        return conservative_outcomes();
    }
    let mut outcomes = BTreeSet::new();
    for line in def.lines.iter() {
        for token in line.body_tokens.iter() {
            match token {
                Token::Symbol(symbol) => {
                    outcomes.extend(resolve_and_collect(interp, symbol, visiting));
                }
                // `OR-NIL` desugars to this token rather than a Symbol, but
                // is a real Word with its own declared vocabulary — see
                // `structural_ceiling_ids`'s doc for why it needs this
                // separate case.
                Token::NilCoalesce => outcomes.extend(builtin_outcomes_for("OR-NIL")),
                _ => {}
            }
        }
    }
    visiting.remove(name);
    outcomes.insert("value".to_string());
    outcomes
}

/// Resolve one Symbol (a body-level call or a `Code`-classified operand
/// element) and collect its outcome vocabulary, falling back to the
/// conservative universe for anything that fails to resolve — an unresolved
/// name is exactly `error:unknownWord` at runtime, so the fallback (which
/// includes it) stays sound, just coarser than naming only that one id.
pub(crate) fn resolve_and_collect(
    interp: &mut Interpreter,
    symbol: &str,
    visiting: &mut HashSet<String>,
) -> BTreeSet<String> {
    let canonical = crate::core_word_aliases::canonicalize_core_word_name(symbol);
    match interp.resolve_word_entry(&canonical) {
        Some((dep_name, dep_def)) => {
            outcome_vocabulary_for_word(interp, &dep_name, &dep_def, visiting)
        }
        None => {
            let mut fallback = conservative_outcomes();
            fallback.insert("error:unknownWord".to_string());
            fallback
        }
    }
}
