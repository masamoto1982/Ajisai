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

use std::collections::{BTreeSet, HashSet};

use crate::types::Token;

use super::word_contract::ContractFlow;
use super::word_contract_flow::FlowSim;
use super::word_contract_widen::{classify_vector_positions, LiteralContext};
use super::word_outcome_vocabulary::{
    builtin_outcomes_for, resolve_and_collect, structural_ceiling_ids,
};
use super::Interpreter;

/// The outcome of a static prediction: every outcome id the program could
/// produce, and whether that set could be narrowed all the way to a single
/// outcome. `exact` is derived, not tracked incrementally: predicting more
/// than one outcome id is itself proof the prediction over-approximates a
/// program that (being deterministic and total) produces exactly one
/// outcome when actually run.
pub struct OutcomePrediction {
    pub outcomes: Vec<String>,
    pub exact: bool,
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

        let contexts = classify_vector_positions(tokens);
        for (idx, token) in tokens.iter().enumerate() {
            match token {
                Token::Number(_) | Token::String(_) => flow.feed_literal(),
                Token::Symbol(symbol) => {
                    if contexts[idx].in_vector_literal() && contexts[idx] != LiteralContext::Code {
                        // Inert data (e.g. the body literal of
                        // `[ ... ] 'NAME' DEF`): never runs here. `NAME`'s
                        // own body is still walked, via
                        // `word_outcome_vocabulary`, if and when a later
                        // call actually resolves to it.
                        flow.feed_literal();
                        continue;
                    }
                    if contexts[idx].in_vector_literal() {
                        // A `Code` operand still contributes its own
                        // vocabulary (it will really run), but not to the
                        // top level's own arity: the enclosing `[ ... ]`
                        // already counted as one opaque push.
                        flow.feed_literal();
                        outcomes.extend(resolve_and_collect(self, symbol, &mut visiting));
                        continue;
                    }
                    let canonical = crate::core_word_aliases::canonicalize_core_word_name(symbol);
                    match self.infer_word_contract(&canonical) {
                        Some(contract) => flow.feed_word(&canonical, &contract.flow),
                        None => flow.go_dynamic(),
                    }
                    outcomes.extend(resolve_and_collect(self, symbol, &mut visiting));
                }
                // `OR-NIL` desugars to this token rather than a Symbol; see
                // `word_outcome_vocabulary::structural_ceiling_ids`'s doc.
                Token::NilCoalesce if !contexts[idx].in_vector_literal() => {
                    flow.feed_structural(token);
                    outcomes.extend(builtin_outcomes_for("OR-NIL"));
                }
                Token::VectorStart
                | Token::VectorEnd
                | Token::NilCoalesce
                | Token::CondClauseSep
                | Token::LineBreak => flow.feed_structural(token),
            }
        }

        let (top_level_flow, flow_unmodelled) = flow.finish();
        let provably_no_underflow =
            !flow_unmodelled && matches!(top_level_flow, ContractFlow::Fixed { consumes: 0, .. });
        if !provably_no_underflow {
            outcomes.insert("error:stackUnderflow".to_string());
        }
        if !tokens.is_empty() {
            outcomes.extend(structural_ceiling_ids().iter().cloned());
        }
        outcomes.insert("value".to_string());

        let exact = outcomes.len() == 1;
        OutcomePrediction {
            outcomes: outcomes.into_iter().collect(),
            exact,
        }
    }
}
