//! Static outcome prediction for a whole program: the finite set of outcome
//! ids a program could produce, computed without executing it.
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
//! to, and `definitionConflict` fires on
//! programs whose every individual Word call (`DEF`, `DEF` again)
//! has a perfectly ordinary declared vocabulary that simply
//! does not list them. Modeling exactly
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
    close_over_nil_reason_loss, resolve_and_collect, structural_ceiling_ids, Reachability,
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
    /// every definition is reachable. `unsettled` names the Words that
    /// registration could not bind to one body (defined twice, or by a `DEF`
    /// it does not read): a call to one is an unknown arity, since the body
    /// registered is only the last one read. Their vocabulary needs no such
    /// care — every body is a literal in `tokens`, walked below.
    pub(crate) fn predict_program_outcomes(
        &mut self,
        tokens: &[Token],
        unsettled: &dyn Fn(&str) -> bool,
    ) -> OutcomePrediction {
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
                    let canonical = crate::word_name::canonical_word_name(text);
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
                        let canonical = crate::word_name::canonical_word_name(symbol);
                        if unsettled(&canonical) {
                            flow.go_dynamic();
                        } else {
                            match self.infer_word_contract(&canonical) {
                                Some(contract) => flow.feed_word(&contract.flow),
                                None => flow.go_dynamic(),
                            }
                        }
                    }
                    // Its *outcomes* count either way: a block written here
                    // may be executed anywhere later. See
                    // `word_outcome_vocabulary`'s module doc.
                    outcomes.extend(resolve_and_collect(self, symbol, &mut visiting, &mut reach));
                }
                Token::VectorStart | Token::VectorEnd => flow.feed_structural(token),
                Token::Value(_) => flow.feed_literal(),
            }
        }

        let (top_level_flow, flow_unmodelled) = flow.finish();
        let provably_no_underflow = !flow_unmodelled
            && matches!(top_level_flow, ContractFlow::Fixed { consumes: 0, .. })
            && !self.a_block_may_underflow(tokens, unsettled);
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

    /// Whether a block a higher-order Word runs can underflow. `MAP`/`FILTER`
    /// run their block on a scratch stack holding one element, `FOLD`/`SCAN`
    /// on one holding the accumulator and an element, so the top-level flow
    /// says nothing about it: `[ 1 2 ] [ ADD ] MAP` balances at the top level
    /// and underflows inside. Every such call in `tokens` counts, wherever it
    /// is written — a `DEF`'d body is a literal in `tokens` too, and a block
    /// pushed as data may run later. A block that is not the literal written
    /// just before the Word is code this walk never read.
    fn a_block_may_underflow(
        &mut self,
        tokens: &[Token],
        unsettled: &dyn Fn(&str) -> bool,
    ) -> bool {
        for (idx, token) in tokens.iter().enumerate() {
            let Token::Symbol(symbol) = token else {
                continue;
            };
            let entry: u16 = match &*crate::word_name::canonical_word_name(symbol) {
                "MAP" | "FILTER" => 1,
                "FOLD" | "SCAN" => 2,
                _ => continue,
            };
            let Some(open) = idx
                .checked_sub(1)
                .and_then(|close| literal_open(tokens, close))
            else {
                return true;
            };
            let mut block = FlowSim::new();
            for inner in &tokens[open + 1..idx - 1] {
                match inner {
                    Token::Number(_) | Token::String(_) | Token::Value(_) => block.feed_literal(),
                    Token::VectorStart | Token::VectorEnd => block.feed_structural(inner),
                    Token::Symbol(name) => {
                        let canonical = crate::word_name::canonical_word_name(name);
                        match (!unsettled(&canonical))
                            .then(|| self.infer_word_contract(&canonical))
                            .flatten()
                        {
                            Some(contract) => block.feed_word(&contract.flow),
                            None => block.go_dynamic(),
                        }
                    }
                }
            }
            match block.finish() {
                (ContractFlow::Fixed { consumes, .. }, false) if consumes <= entry => {}
                _ => return true,
            }
        }
        false
    }
}

/// The index of the `[` that the `]` at `close` closes, or `None` when the
/// token at `close` is not a `]` or nothing opens it.
fn literal_open(tokens: &[Token], close: usize) -> Option<usize> {
    if tokens.get(close) != Some(&Token::VectorEnd) {
        return None;
    }
    let mut depth = 0usize;
    for at in (0..=close).rev() {
        match tokens[at] {
            Token::VectorEnd => depth += 1,
            Token::VectorStart => {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::tokenizer::tokenize;

    /// Prediction at the interpreter level answers only *which* outcomes are
    /// possible. Whether that set is exact is a property of the final set, which
    /// `agent::outcome_report` owns (it may still add `error:unknownWord`) — see
    /// `OutcomePrediction`'s doc.
    fn predict(source: &str) -> Vec<String> {
        let mut interp = Interpreter::new();
        let tokens = tokenize(source).expect("test source must tokenize");
        interp
            .predict_program_outcomes(&tokens, &|_| false)
            .outcomes
    }

    #[test]
    fn calling_any_word_at_all_brings_the_structural_ceiling_with_it() {
        // `TRUE` alone has no declared error/NIL vocabulary, but calling it still
        // spends at least one execution step — so under a strict enough profile
        // (which this predictor does not itself narrow by, see the module doc)
        // even this could hit a resource ceiling.
        let outcomes = predict("TRUE");
        assert!(outcomes.contains(&"value".to_string()));
        assert!(outcomes.contains(&"error:executionLimitExceeded".to_string()));
        assert!(outcomes.len() > 1, "outcomes: {outcomes:?}");
    }

    #[test]
    fn an_arithmetic_call_over_approximates_with_its_declared_vocabulary() {
        let outcomes = predict("1 2 ADD");
        assert!(outcomes.contains(&"value".to_string()));
        assert!(outcomes.contains(&"error:nonNumeric".to_string()));
        assert!(outcomes.contains(&"error:shapeMismatch".to_string()));
        assert!(outcomes.contains(&"error:executionLimitExceeded".to_string()));
    }

    #[test]
    fn an_empty_program_never_underflows_or_touches_resource_ceilings() {
        assert_eq!(predict(""), vec!["value".to_string()]);
    }

    #[test]
    fn a_bare_word_with_no_operands_predicts_stack_underflow() {
        assert!(predict("ADD").contains(&"error:stackUnderflow".to_string()));
    }

    #[test]
    fn a_fully_supplied_call_does_not_predict_stack_underflow() {
        assert!(!predict("1 2 ADD").contains(&"error:stackUnderflow".to_string()));
    }

    #[test]
    fn a_called_user_word_contributes_its_bodys_vocabulary() {
        assert!(predict("[ 1 ADD ] 'INC' DEF 5 INC").contains(&"error:nonNumeric".to_string()));
    }

    /// A Word name written inside a `[ ... ]` counts even where that literal is
    /// inert at the point it is written, because a block can be pushed by one
    /// Word and executed by another arbitrarily far away.
    /// `[ [ 'a' ADD ] ] 'G' DEF 1 G EXEC` really does run that `ADD` and answer
    /// `nonNumeric`; an earlier version consulted `classify_vector_positions`
    /// and skipped anything it called `Data`, which dropped exactly that
    /// outcome from the prediction. Over-approximating here (an uncalled
    /// definition's vocabulary joins the set too) is the allowed direction;
    /// omitting a reachable outcome is not. See `word_outcome_vocabulary`'s
    /// module doc.
    #[test]
    fn a_word_named_inside_a_literal_still_contributes_its_vocabulary() {
        assert!(predict("[ 1 ADD ] 'INC' DEF").contains(&"error:nonNumeric".to_string()));
        assert!(
            predict("[ [ 'a' ADD ] ] 'G' DEF 1 G EXEC").contains(&"error:nonNumeric".to_string())
        );
        assert!(predict("[ [ -1 SQRT ] ] 'G' DEF G EXEC").contains(&"nil:domainMiss".to_string()));
    }

    /// A higher-order block runs on its own stack, so a top level that
    /// balances proves nothing about it. Each of these really answers
    /// `stackUnderflow`.
    #[test]
    fn a_block_that_underflows_its_own_stack_predicts_stack_underflow() {
        for source in [
            "[ 1 2 ] [ ADD ] MAP",
            "[ 1 2 ] 0 [ ADD ADD ] FOLD",
            "[ 1 2 ] [ POW ] FILTER",
            "[ [ 1 ] [ 2 ] ] [ HAS? ] MAP",
            "[ [ ADD ] MAP ] 'W' DEF [ 1 2 ] W",
            "[ ADD ] 'B' BIND [ 1 2 ] B MAP",
        ] {
            assert!(
                predict(source).contains(&"error:stackUnderflow".to_string()),
                "{source}"
            );
        }
        // A block its entry stack feeds stays precise.
        for source in ["[ 1 2 ] [ 1 ADD ] MAP", "[ 1 2 ] 0 [ ADD ] FOLD"] {
            assert!(
                !predict(source).contains(&"error:stackUnderflow".to_string()),
                "{source}"
            );
        }
    }

    #[test]
    fn a_code_operand_of_a_higher_order_word_contributes_its_vocabulary() {
        assert!(predict("[ 1 2 3 ] [ 1 ADD ] MAP").contains(&"error:nonNumeric".to_string()));
    }

    /// A structural category that only one class of Word can raise is dropped
    /// when nothing the program reaches is that class. `1 2 ADD` contains no
    /// `DEF`, no `DEL` and no User Word, so none of the four is possible — and
    /// before this narrowing every one of them was predicted.
    #[test]
    fn a_structural_category_no_reachable_word_can_raise_is_dropped() {
        let outcomes = predict("1 2 ADD");
        for id in [
            "error:nameConflict",
            "error:selfReferentialDefinition",
            "error:protectedWord",
            "error:recursionLimitExceeded",
        ] {
            assert!(
                !outcomes.contains(&id.to_string()),
                "{id} is unreachable for `1 2 ADD` but was predicted: {outcomes:?}"
            );
        }
        // The ungated ones stay: they are spread across the arithmetic and
        // collection modules and this change deliberately does not model them.
        assert!(outcomes.contains(&"error:executionLimitExceeded".to_string()));
        assert!(outcomes.contains(&"error:resourceLimitExceeded".to_string()));
    }

    #[test]
    fn each_gated_category_returns_when_its_own_trigger_is_reachable() {
        for (source, id) in [
            ("[ 1 ADD ] 'INC' DEF", "error:nameConflict"),
            ("[ 1 ADD ] 'INC' DEF", "error:selfReferentialDefinition"),
            ("[ 1 ADD ] 'INC' DEF", "error:protectedWord"),
            ("'INC' DEL", "error:protectedWord"),
        ] {
            assert!(
                predict(source).contains(&id.to_string()),
                "{source} reaches the Word that raises {id}, so it must stay"
            );
        }
    }

    /// `recursionLimitExceeded` is `execute_builtin`'s call-depth guard, so it
    /// needs a User-Word activation or a Word that evaluates a block — a
    /// chain of blocks bound to one another nests as deep as a call chain.
    #[test]
    fn the_call_depth_guard_needs_a_user_word_or_a_block_to_be_possible() {
        let calls_user_word = predict("[ 1 ADD ] 'INC' DEF 5 INC");
        assert!(calls_user_word.contains(&"error:recursionLimitExceeded".to_string()));
        let runs_a_block = predict("[ 1 ] 'B' BIND B EXEC");
        assert!(runs_a_block.contains(&"error:recursionLimitExceeded".to_string()));
        assert!(!predict("1 2 ADD").contains(&"error:recursionLimitExceeded".to_string()));
    }

    /// The guard: an unresolved name means the walk no longer knows what runs, so
    /// every gated category comes back rather than being narrowed away on an
    /// incomplete picture. Dropping one here would be the under-approximation
    /// pitfall A forbids.
    #[test]
    fn an_unresolved_name_restores_every_gated_category() {
        let outcomes = predict("FROBNICATE");
        for id in [
            "error:selfReferentialDefinition",
            "error:recursionLimitExceeded",
        ] {
            assert!(
                outcomes.contains(&id.to_string()),
                "{id} must survive an unresolved name: {outcomes:?}"
            );
        }
    }

    /// A Word reached only through a `DEF`'d body counts: the walk recurses, so
    /// the reachability set it builds is not just the top-level symbols.
    #[test]
    fn a_gated_trigger_inside_a_definition_body_still_counts() {
        let outcomes = predict("[ 'INC' DEL ] 'DROP-INC' DEF DROP-INC");
        assert!(outcomes.contains(&"error:protectedWord".to_string()));
    }

    /// A String is not a code operand, so neither of these runs `DEL` at all: both
    /// answer `notExecutable`, which is what `MAP`'s own contract declares, and the
    /// prediction has to contain it.
    ///
    /// These two programs used to be the witnesses that a Word could run with no
    /// `Token::Symbol` for it anywhere in the source — the higher-order Words took
    /// `'NAME'` as their code operand, and `[ 'NOPE' ] 'DEL' MAP` really answered
    /// `wordNotFound`. That spelling is gone, because a name the program *computed*
    /// appeared in no token for the DEF-time acyclicity check to read, which left
    /// LANG.DICTIONARY.ACYCLIC's termination argument resting on a runtime ceiling.
    /// They are kept here as the regression: a call must never again be reachable
    /// without a Symbol naming it.
    #[test]
    fn a_string_is_not_a_code_operand() {
        assert!(predict("[ 'ADD' ] 'DEL' MAP").contains(&"error:notExecutable".to_string()));
        assert!(predict("[ 'NOPE' ] 'DEL' MAP").contains(&"error:notExecutable".to_string()));
    }

    /// A String that names nothing is just data and pulls in no vocabulary.
    #[test]
    fn a_string_that_names_no_word_stays_a_literal() {
        let outcomes = predict("5 'x' BIND");
        assert!(!outcomes.contains(&"error:wordNotFound".to_string()));
        assert!(!outcomes.contains(&"error:recursionLimitExceeded".to_string()));
    }

    /// A reasonless NIL is `nil:literal`, and no Word's `spec/words.json`
    /// declaration names it — so a vocabulary union alone omits it. It was the
    /// second most frequent outcome in the exhaustive table (676 of 6,593 cells)
    /// and the predictor produced it for no program at all.
    #[test]
    fn a_written_nil_predicts_its_own_literal_outcome() {
        for source in ["NIL", "NIL 1 ADD", "TRUE NIL AND", "[ NIL 1 ] 0 GET"] {
            assert!(
                predict(source).contains(&"nil:literal".to_string()),
                "{source} really answers nil:literal, so it must be predicted"
            );
        }
    }

    /// The trigger is what the program can *produce*, not what it writes:
    /// `[ 4 -1 ] SQRT [ 1 1 ] DIV 1 GET` answers `nil:literal` with no
    /// `NIL` token anywhere, because the negative radicand's reason does not
    /// survive a second lane-wise pass.
    #[test]
    fn a_computed_nil_admits_the_reasonless_one_too() {
        let outcomes = predict("[ 4 -1 ] SQRT [ 1 1 ] DIV 1 GET");
        assert!(outcomes.contains(&"nil:domainMiss".to_string()));
        assert!(
            outcomes.contains(&"nil:literal".to_string()),
            "{outcomes:?}"
        );
    }

    /// A program that can produce no NIL keeps a NIL-free prediction — the
    /// widening is a closure over reason loss, not a blanket.
    #[test]
    fn a_program_that_cannot_produce_a_nil_predicts_none() {
        let outcomes = predict("1 2 ADD");
        assert!(
            !outcomes.iter().any(|id| id.starts_with("nil:")),
            "{outcomes:?}"
        );
    }
}
