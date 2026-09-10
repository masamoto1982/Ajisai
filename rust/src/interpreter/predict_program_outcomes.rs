//! Static outcome prediction for a whole program (Phase 5,
//! `docs/dev/auditable-kernel-work-order-2026-09.md` §5): the finite set of
//! outcome ids a program could produce, computed without executing it.
//!
//! Built entirely on `word_outcome_vocabulary`'s per-word walk (a program's
//! top-level body is treated exactly like one more word body to walk) plus a
//! fresh `FlowSim` run for one refinement: whether the top level can be
//! *proven* never to underflow its own (empty) starting stack. Everything
//! else stays at the sound word-vocabulary default rather than attempting
//! finer, value-sensitive narrowing — see `word_outcome_vocabulary`'s module
//! doc for why that default can never under-approximate.
//!
//! # Why every structural category (`word_outcome_vocabulary::
//! structural_ceiling_ids`) is added once the program is non-empty
//!
//! A structural category is by definition not tied to any specific Word's
//! own `errorWhen` — a per-word vocabulary union can never include one on
//! its own. Two witnesses in `spec/outcome-witnesses.json` proved this the
//! hard way during development: `resourceLimitExceeded` fires on a numeric
//! literal alone (`999...9`), with no Word call anywhere to attribute it
//! to, and `condExhausted`/`builtinProtection`/`nameConflict`/
//! `structureError` all fire on programs whose every individual Word call
//! (`COND`, `DEF`, `DEF` again, `DEF`+`DEL`) has a perfectly ordinary
//! declared vocabulary that simply does not list them. Modeling exactly
//! when each is reachable (the real profile's numeric-literal-digit ceiling
//! against the literal's actual digit count, for instance) is a sound
//! refinement future work can add; V1 instead adds the whole set
//! unconditionally whenever there is anything to run at all — sound, and
//! honestly coarse, per pitfall A. `scripts/check-outcome-prediction.mjs`
//! is what caught the gap and is what would catch a future one.
//!
//! # Why `nil:literal` is added from the *rest* of the prediction
//!
//! `nil:literal` is the one outcome id no Word's `spec/words.json`
//! declaration can carry, so the vocabulary union omits it by construction —
//! see `word_outcome_vocabulary::NIL_LITERAL`. Two things put it back: the
//! `NIL` Word's own vocabulary (a written literal), and
//! `word_outcome_vocabulary::close_over_nil_reason_loss`, run last here over
//! the assembled set, for the computed NILs whose reason a dense tensor lane
//! cannot carry.

use std::collections::{BTreeSet, HashSet};

use crate::types::Token;

use super::word_contract::ContractFlow;
use super::word_contract_flow::FlowSim;
use super::word_contract_widen::classify_vector_positions;
use super::word_outcome_vocabulary::{
    builtin_outcomes_for, close_over_nil_reason_loss, resolve_and_collect, structural_ceiling_ids,
    Reachability,
};
use super::Interpreter;

/// The outcome of a static prediction: every outcome id the program could
/// produce.
///
/// Deliberately carries no `exact` flag. Exactness is a property of the
/// *final* set — one outcome id means the prediction narrowed to the single
/// outcome a deterministic, total program actually produces — and this walk
/// does not always build the final set: `agent::outcome_report` may still
/// add `error:unknownWord` afterwards. Deriving it in both places is how
/// two fields that describe one fact start disagreeing (the same defect
/// Phase 2 fixed between `nil` and `suggested` in `contract_report`), so
/// the one caller that owns the final set owns the derivation.
pub struct OutcomePrediction {
    pub outcomes: Vec<String>,
}

impl Interpreter {
    /// Predict every outcome id `tokens` (a full, already-tokenized program)
    /// could produce, without executing it. Callers register any top-level
    /// `DEF`s into `self` first (see `agent::contract_decl::
    /// build_definitions_interpreter`) so a later call to a `DEF`'d name
    /// resolves against its real body — a name that is `DEF`'d but never
    /// called contributes nothing, which is more precise than assuming
    /// every definition is reachable.
    pub(crate) fn predict_program_outcomes(&mut self, tokens: &[Token]) -> OutcomePrediction {
        let mut flow = FlowSim::new();
        let mut visiting: HashSet<String> = HashSet::new();
        let mut outcomes: BTreeSet<String> = BTreeSet::new();
        let mut reach = Reachability::default();

        let contexts = classify_vector_positions(tokens);
        for (idx, token) in tokens.iter().enumerate() {
            match token {
                Token::Number(_) => flow.feed_literal(),
                // A String is one operand to `flow`, and may also name a Word
                // the program runs — see `word_outcome_vocabulary`'s
                // `Token::String` arm for why that is not hypothetical.
                Token::String(text) => {
                    flow.feed_literal();
                    let canonical = crate::core_word_aliases::canonicalize_core_word_name(text);
                    if self.resolve_word_entry(&canonical).is_some() {
                        outcomes.extend(resolve_and_collect(self, text, &mut visiting, &mut reach));
                    }
                }
                Token::Symbol(symbol) => {
                    // Arity and vocabulary read this Symbol differently, and
                    // both readings are right. Inside a `[ ... ]` it is one
                    // opaque push and never applies its own arity, whatever
                    // it turns out to mean — so `flow` sees a literal.
                    if contexts[idx].in_vector_literal() {
                        flow.feed_literal();
                    } else {
                        let canonical =
                            crate::core_word_aliases::canonicalize_core_word_name(symbol);
                        match self.infer_word_contract(&canonical) {
                            Some(contract) => flow.feed_word(&canonical, &contract.flow),
                            None => flow.go_dynamic(),
                        }
                    }
                    // Its *outcomes* count either way: a block written here
                    // may be executed anywhere later. See
                    // `word_outcome_vocabulary`'s module doc.
                    outcomes.extend(resolve_and_collect(self, symbol, &mut visiting, &mut reach));
                }
                // `OR-NIL` desugars to this token rather than a Symbol; see
                // `word_outcome_vocabulary::structural_ceiling_ids`'s doc.
                Token::NilCoalesce => {
                    flow.feed_structural(token);
                    reach.saw_word("OR-NIL");
                    outcomes.extend(builtin_outcomes_for("OR-NIL"));
                }
                Token::VectorStart | Token::VectorEnd | Token::CondClauseSep | Token::LineBreak => {
                    flow.feed_structural(token)
                }
            }
        }

        let (top_level_flow, flow_unmodelled) = flow.finish();
        let provably_no_underflow =
            !flow_unmodelled && matches!(top_level_flow, ContractFlow::Fixed { consumes: 0, .. });
        if !provably_no_underflow {
            outcomes.insert("error:stackUnderflow".to_string());
        }
        if !tokens.is_empty() {
            outcomes.extend(structural_ceiling_ids(&reach));
        }
        // Last, so it sees every `nil:` id the walk collected — including the
        // ones a `DEF`'d body or a String-named Word contributed.
        close_over_nil_reason_loss(&mut outcomes);
        outcomes.insert("value".to_string());

        OutcomePrediction {
            outcomes: outcomes.into_iter().collect(),
        }
    }
}
