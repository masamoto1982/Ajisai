//! Per-word outcome vocabulary for static outcome prediction. Deliberately
//! independent of `word_contract.rs`'s `WordContract`/`AccumulatedContract`
//! (which do not carry outcome information and are already at the file's own
//! 500-line budget): a fresh, self-contained recursive walk over a word's body,
//! parallel to (not sharing state with) contract inference.
//!
//! A word's own declared vocabulary (`spec/words.json`'s `errorWhen` +
//! `projection.reason`, read through the generated registry's `error_when`
//! and `projection_reasons` — the `reason` ids, not the `when` condition
//! names its `projection` field carries) covers what that Word is
//! observed to do: `scripts/check-word-outcome-containment.mjs` holds every
//! executed cell of `docs/semantics-table.json` to the raising Word's own
//! repertoire, widened only by the machine-attributable error categories and
//! the non-projectable NIL reasons that reach a Word by passthrough.
//!
//! This doc used to credit `scripts/check-outcome-bijection.mjs` with that
//! property. It does not have it: that gate asks whether an observed outcome
//! resolves to a *registered id*, and whether every registered id is observed
//! *somewhere* — both registry-level, neither per-word. The difference was
//! load-bearing rather than pedantic. `MIN` and `MAX` raised
//! a length-mismatch category their contracts did not name, and
//! this module's soundness argument rested on a property nothing checked and
//! that the vocabulary itself violated; only the structural ceiling below
//! kept the prediction sound in practice. Composing a
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
use std::sync::{Arc, OnceLock};

use crate::error::{ErrorCategory, NilReason};
use crate::kernel::generated::{generated_word, GENERATED_WORDS};
use crate::types::{Token, WordDefinition};

use super::Interpreter;

/// Every `spec/outcomes.json` error category that is `kind: "structural"`
/// (not any specific Word's own declared `errorWhen`) — the fixed,
/// non-`Declared` `ErrorCategory` variants, read through the real
/// `as_protocol_str()` so this can never drift from the wire spelling.
/// `DivisionByZero` is excluded: it is not a registered outcome category at
/// all (`scripts/check-outcome-registry.mjs`'s documented exclusion —
/// diagnostic-trace-only).
fn structural_error_categories() -> [ErrorCategory; 7] {
    [
        ErrorCategory::StackUnderflow,
        ErrorCategory::UnknownWord,
        ErrorCategory::MalformedSource,
        ErrorCategory::ExecutionLimitExceeded,
        ErrorCategory::ResourceLimitExceeded,
        ErrorCategory::RecursionLimitExceeded,
        ErrorCategory::ContractViolation,
    ]
}

/// Every outcome id a builtin could ever produce, per `spec/words.json`'s own
/// declaration: `value`, one `error:<category>` per declared `errorWhen`
/// condition, and one `nil:<reason>` per declared projection reason — plus
/// `NIL`'s own `nil:literal`, which no declaration can carry (see
/// [`NIL_LITERAL`]).
pub(crate) fn builtin_outcomes_for(name: &str) -> BTreeSet<String> {
    let mut outcomes = BTreeSet::new();
    outcomes.insert("value".to_string());
    let canonical = name.to_uppercase();
    if canonical == NIL_WORD {
        outcomes.insert(NIL_LITERAL.to_string());
    }
    if let Some(word) = generated_word(&canonical) {
        for condition in word.error_when {
            outcomes.insert(format!("error:{condition}"));
        }
        for reason in word.projection_reasons {
            outcomes.insert(format!("nil:{reason}"));
        }
    }
    outcomes
}

/// The outcome id of a NIL that carries no reason.
///
/// It is the one id a Word's `spec/words.json` declaration can never name.
/// The declaration derives outcomes from `errorWhen` (what the Word raises)
/// and `projection.reason` (what the Word *computes* an absence for), and
/// `spec/outcomes.json` defines `literal` as the complement of both: "a NIL
/// the program wrote rather than computed ... not produced by any Word's
/// projection". So a vocabulary union over declarations alone omits it by
/// construction — the under-approximation this pair of constants exists to
/// close, measured on `NIL 1 ADD` (really `nil:literal`, predicted without
/// any `nil:` id at all) and on 676 of the exhaustive table's 6,593 cells.
pub(crate) const NIL_LITERAL: &str = "nil:literal";

/// The one Word that answers with a reasonless NIL by definition: `NIL`
/// pushes the literal, and `spec/words.json` records its projection as
/// `never` — correctly, since pushing an absence is not projecting one.
/// `[ NIL 1 ]` needs no separate case: a NIL inside a vector literal is the
/// same `Token::Symbol("NIL")` this name matches, and both walks resolve
/// every Symbol wherever it is written.
const NIL_WORD: &str = "NIL";

/// Widen `outcomes` to admit `nil:literal` whenever the program can produce
/// *any* NIL — the closure that keeps prediction sound against reason loss.
///
/// A reason is metadata on a whole `Value`. A dense tensor lane used to hold
/// presence but not a reason, so a computed, reasoned NIL that crossed one
/// came back reasonless, and a reasonless NIL reads back as `literal`:
///
/// ```text
/// [ 1 2 ] [ 1 0 ] DIV 1 GET NIL-REASON            -> 'divisionByZero'
/// [ 1 2 ] [ 1 0 ] DIV [ 1 1 ] DIV 1 GET NIL-REASON -> 'literal'   (then)
/// ```
///
/// The second program contains no `NIL` token, so "a NIL literal is written
/// somewhere" is *not* a sound trigger, however exactly it matches the
/// exhaustive table (where every `nil:literal` cell does take `nilLiteral`
/// as an input). Predicting from what the program can produce instead of
/// from what it writes stays sound whatever the value representation does
/// with reasons — including now that a dense lane keeps its reason in the
/// tensor's absence map (`DenseTensor::absences`) and that program answers
/// `'divisionByZero'`, since dropping an outcome the program cannot reach is
/// the allowed direction (pitfall A) and adding one it can is not.
pub(crate) fn close_over_nil_reason_loss(outcomes: &mut BTreeSet<String>) {
    if outcomes.iter().any(|id| id.starts_with("nil:")) {
        outcomes.insert(NIL_LITERAL.to_string());
    }
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

/// What the walk learned about *which* Words a program can reach, carried
/// alongside the outcome ids it collects.
///
/// The walk already visits every Word a program could execute — every
/// `Token::Symbol` wherever written, recursing through `DEF`'d bodies — so it
/// knows this; it simply threw the names away. Keeping them lets
/// `structural_ceiling_ids` answer a question it could not before: whether
/// anything the program can reach is even able to raise a given structural
/// category.
#[derive(Default)]
pub(crate) struct Reachability {
    /// Canonical names of every Word the walk resolved, including Words
    /// reached only through a `DEF`'d body.
    names: BTreeSet<String>,
    /// A resolved Word was user-defined, so a User-Word activation happens.
    calls_user_word: bool,
    /// A name did not resolve, so the walk fell back to the conservative
    /// universe and `names` is no longer a complete account of what runs.
    /// Every gated category stays in while this holds.
    unresolved: bool,
    /// Each User Word's vocabulary, once walked. The dictionary is fixed for
    /// one prediction, and everything a walk adds to the fields above stays
    /// added, so a second call site needs only the set: without this, a
    /// Word calling its dependency twice walked it twice, and a chain of
    /// such Words took time exponential in its length.
    walked: HashMap<String, BTreeSet<String>>,
}

impl Reachability {
    fn saw(&mut self, name: &str, is_builtin: bool) {
        self.names.insert(name.to_uppercase());
        self.calls_user_word |= !is_builtin;
    }
}

/// Every structural error category (see `structural_error_categories`),
/// except `stackUnderflow` (given a precise, flow-sensitive answer by
/// `predict_program_outcomes`'s own `FlowSim` run) and `malformedSource`
/// (impossible past the point prediction's caller already tokenized the
/// source successfully). "Structural" means *not* any specific Word's own
/// declared `errorWhen` — a cross-cutting engine condition (a name reused
/// across scopes, a definition shadowing a builtin, a numeric literal too
/// long for the profile) that a per-word
/// vocabulary union can never include on its own, and so would otherwise
/// under-approximate for any non-trivial program. Included whenever the
/// program is non-empty (`predict_program_outcomes` decides that), not
/// narrowed further in V1 — see that module's doc for why.
pub(crate) fn structural_ceiling_ids(reach: &Reachability) -> BTreeSet<String> {
    structural_error_categories()
        .into_iter()
        .map(|category| category.as_protocol_str())
        // `contractViolation` is decided before execution from the
        // `#:contract` directives alone, so `agent::outcome_report` adds it
        // exactly when the source carries one, rather than this walk adding
        // it to every program.
        .filter(|id| {
            *id != "stackUnderflow" && *id != "malformedSource" && *id != "contractViolation"
        })
        .map(|id| format!("error:{id}"))
        .filter(|id| {
            // `recursionLimitExceeded` is `execute_builtin`'s call-depth guard,
            // so it needs a User-Word activation or a Word that evaluates a
            // block, and nothing else does.
            if id == "error:recursionLimitExceeded" {
                return reach.unresolved
                    || reach.calls_user_word
                    || ["EXEC", "MAP", "FILTER", "FOLD", "SCAN"]
                        .iter()
                        .any(|word| reach.names.contains(*word));
            }
            true
        })
        .collect()
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
    reach: &mut Reachability,
) -> BTreeSet<String> {
    reach.saw(name, def.is_builtin);
    if def.is_builtin {
        return builtin_outcomes_for(name);
    }
    if let Some(outcomes) = reach.walked.get(name) {
        return outcomes.clone();
    }
    if !visiting.insert(name.to_string()) {
        return conservative_outcomes();
    }
    let mut outcomes = BTreeSet::new();
    for token in def.body.iter() {
        match token {
            Token::Symbol(symbol) => {
                outcomes.extend(resolve_and_collect(interp, symbol, visiting, reach));
            }
            // A String still names a Word for `DEF`, `DEL` and `BIND`, and
            // this walk does not tell an operand position from a code one,
            // so it keeps the resolved Word's vocabulary. That
            // over-approximates — a data string spelling a Word name pulls
            // its vocabulary in for nothing — which is the allowed
            // direction.
            //
            // What a String can no longer do is make a Word *run*. The
            // higher-order Words used to take `'NAME'` as their code
            // operand (`[ 1 2 3 ] 'DBL' MAP`), which is why this branch
            // first existed; that spelling is gone (see
            // `higher_order::common::extract_executable_code`), because a
            // name computed at run time defeated the DEF-time acyclicity
            // check LANG.DICTIONARY.ACYCLIC's termination argument rests
            // on. Every reachable Word is now named by a `Token::Symbol`
            // somewhere, so the arm above carries the whole call graph.
            Token::String(text)
                if interp
                    .resolve_word_entry(&crate::word_name::canonical_word_name(text))
                    .is_some() =>
            {
                outcomes.extend(resolve_and_collect(interp, text, visiting, reach));
            }
            _ => {}
        }
    }
    visiting.remove(name);
    outcomes.insert("value".to_string());
    reach.walked.insert(name.to_string(), outcomes.clone());
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
    reach: &mut Reachability,
) -> BTreeSet<String> {
    let canonical = crate::word_name::canonical_word_name(symbol);
    match interp.resolve_word_entry(&canonical) {
        Some((dep_name, dep_def)) => {
            outcome_vocabulary_for_word(interp, &dep_name, &dep_def, visiting, reach)
        }
        None => {
            // Reachability is no longer known: an unresolved name could be
            // anything once it exists, so every gated category stays in.
            reach.unresolved = true;
            let mut fallback = conservative_outcomes();
            fallback.insert("error:unknownWord".to_string());
            fallback
        }
    }
}

// Where an error category is repaired, read from the outcome registry
// (`spec/outcomes.json`) rather than restated here.
//
// The diagnosis used to answer this with its own seven-value
// `recoverability` scale (`fixInput`, `fixProgram`, `fixHost`, …), computed
// from the cause class beside a registry that already declares the answer as
// `repair: "program"` (absent: the operand is what is wrong). Two
// classifications of one fact drift, and the one an agent could check
// against `word_contract` was not the one it was sent. The registry is the
// answer; this module only reads it.
const OUTCOMES_JSON: &str = include_str!("../../../spec/outcomes.json");

fn program_repaired() -> &'static HashSet<String> {
    static IDS: OnceLock<HashSet<String>> = OnceLock::new();
    IDS.get_or_init(|| {
        let parsed: serde_json::Value =
            serde_json::from_str(OUTCOMES_JSON).expect("spec/outcomes.json must parse");
        parsed["errorCategories"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|entry| entry["repair"].as_str() == Some("program"))
            .filter_map(|entry| entry["id"].as_str().map(str::to_string))
            .collect()
    })
}

/// `Some("program")` exactly when spec/outcomes.json marks `category`
/// `repair: program`; `None` otherwise, as the registry leaves the field
/// absent — which it defines as "the operand is what is wrong".
pub(crate) fn repair_for_category(category: &str) -> Option<&'static str> {
    program_repaired().contains(category).then_some("program")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{builtin_outcomes_for, close_over_nil_reason_loss, conservative_outcomes};

    #[test]
    fn builtin_outcomes_include_value_and_declared_errors() {
        let outcomes = builtin_outcomes_for("ADD");
        assert!(outcomes.contains("value"));
        assert!(outcomes.contains("error:nonNumeric"));
        assert!(outcomes.contains("error:shapeMismatch"));
    }

    #[test]
    fn builtin_outcomes_include_declared_nil_projections() {
        let outcomes = builtin_outcomes_for("DIV");
        assert!(outcomes.contains("nil:divisionByZero"));
        let outcomes = builtin_outcomes_for("POW");
        assert!(outcomes.contains("nil:domainMiss"));
    }

    #[test]
    fn conservative_outcomes_cover_the_whole_registry() {
        let outcomes = conservative_outcomes();
        assert!(outcomes.contains("value"));
        assert!(outcomes.contains("error:stackUnderflow"));
        assert!(outcomes.contains("nil:spaceExhausted"));
        assert!(outcomes.len() > 35, "{}", outcomes.len());
    }

    /// `NIL` answers with a reasonless NIL, which is `nil:literal` — the one
    /// outcome id no `spec/words.json` declaration can carry, since the
    /// declaration only names what a Word *raises* or *projects* and
    /// `spec/outcomes.json` defines `literal` as the complement of both.
    #[test]
    fn the_nil_word_carries_its_own_literal_outcome() {
        let outcomes = builtin_outcomes_for("NIL");
        assert!(outcomes.contains("nil:literal"), "outcomes: {outcomes:?}");
        // Not handed to every Word: `ADD` declares no projection at all.
        assert!(!builtin_outcomes_for("ADD").contains("nil:literal"));
    }

    /// A reason is metadata on a whole `Value` and a dense tensor lane cannot
    /// hold one, so a computed NIL that crosses a lane comes back reasonless and
    /// reads as `literal`. Predicting from what a program can *produce* keeps
    /// that sound; predicting from the `NIL` tokens it *writes* would not.
    #[test]
    fn any_reachable_nil_admits_a_reasonless_one() {
        let mut projecting: BTreeSet<String> = BTreeSet::new();
        projecting.insert("value".to_string());
        projecting.insert("nil:divisionByZero".to_string());
        close_over_nil_reason_loss(&mut projecting);
        assert!(projecting.contains("nil:literal"));

        // A program that can produce no NIL at all is left alone: the widening
        // is a closure over reason loss, not a blanket.
        let mut total: BTreeSet<String> = BTreeSet::new();
        total.insert("value".to_string());
        total.insert("error:nonNumeric".to_string());
        close_over_nil_reason_loss(&mut total);
        assert!(!total.contains("nil:literal"));
    }
}
