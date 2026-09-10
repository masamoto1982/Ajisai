use super::Interpreter;
use crate::tokenizer::tokenize;

/// Prediction at the interpreter level answers only *which* outcomes are
/// possible. Whether that set is exact is a property of the final set, which
/// `agent::outcome_report` owns (it may still add `error:unknownWord`) — see
/// `OutcomePrediction`'s doc.
fn predict(source: &str) -> Vec<String> {
    let mut interp = Interpreter::new();
    let tokens = tokenize(source).expect("test source must tokenize");
    interp.predict_program_outcomes(&tokens).outcomes
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
    assert!(predict("[ [ 'a' ADD ] ] 'G' DEF 1 G EXEC").contains(&"error:nonNumeric".to_string()));
    assert!(predict("[ [ 1 0 DIV ] ] 'G' DEF G EXEC").contains(&"nil:divisionByZero".to_string()));
}

#[test]
fn a_code_operand_of_a_higher_order_word_contributes_its_vocabulary() {
    assert!(predict("[ 1 2 3 ] [ 1 ADD ] MAP").contains(&"error:nonNumeric".to_string()));
}

/// A structural category that only one class of Word can raise is dropped
/// when nothing the program reaches is that class. `1 2 ADD` contains no
/// `COND`, no `DEF`, no `DEL` and no User Word, so none of the five is
/// possible — and before this narrowing every one of them was predicted.
#[test]
fn a_structural_category_no_reachable_word_can_raise_is_dropped() {
    let outcomes = predict("1 2 ADD");
    for id in [
        "error:condExhausted",
        "error:nameConflict",
        "error:selfReferentialDefinition",
        "error:builtinProtection",
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
        ("NIL [ [ TRUE ] [ 1 ] ] COND", "error:condExhausted"),
        ("[ 1 ADD ] 'INC' DEF", "error:nameConflict"),
        ("[ 1 ADD ] 'INC' DEF", "error:selfReferentialDefinition"),
        ("[ 1 ADD ] 'INC' DEF", "error:builtinProtection"),
        ("'INC' DEL", "error:builtinProtection"),
    ] {
        assert!(
            predict(source).contains(&id.to_string()),
            "{source} reaches the Word that raises {id}, so it must stay"
        );
    }
}

/// `recursionLimitExceeded` is `execute_builtin`'s call-depth guard, so it
/// needs a User-Word activation rather than a named Word.
#[test]
fn the_call_depth_guard_needs_a_user_word_to_be_possible() {
    let calls_user_word = predict("[ 1 ADD ] 'INC' DEF 5 INC");
    assert!(calls_user_word.contains(&"error:recursionLimitExceeded".to_string()));
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
        "error:condExhausted",
        "error:nameConflict",
        "error:selfReferentialDefinition",
        "error:builtinProtection",
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
    let outcomes = predict("[ [ TRUE ] [ 1 ] ] 'BRANCH' DEF NIL BRANCH COND");
    assert!(outcomes.contains(&"error:condExhausted".to_string()));
}

/// A String can name a Word — the higher-order Words take `'NAME'` as their
/// code operand — so a Word can run with no `Token::Symbol` for it anywhere
/// in the source. Reasoning about reachability from Symbols alone omits it.
///
/// `[ 'NOPE' ] 'DEL' MAP` really answers `error:wordNotFound`, a *declared*
/// condition of `DEL`, and the predictor missed it before this change: the
/// unconditional structural ceiling never covered a declared condition. That
/// is a pre-existing under-approximation, not one the reachability narrowing
/// introduced — the narrowing is what made it visible, by threatening to drop
/// `builtinProtection` (which the ceiling *did* cover) for the same reason.
#[test]
fn a_word_named_by_a_string_is_reachable() {
    assert!(predict("[ 'ADD' ] 'DEL' MAP").contains(&"error:builtinProtection".to_string()));
    assert!(predict("[ 'NOPE' ] 'DEL' MAP").contains(&"error:wordNotFound".to_string()));
}

/// A String that names nothing is just data and pulls in no vocabulary.
#[test]
fn a_string_that_names_no_word_stays_a_literal() {
    let outcomes = predict("5 'x' BIND");
    assert!(!outcomes.contains(&"error:condExhausted".to_string()));
    assert!(!outcomes.contains(&"error:recursionLimitExceeded".to_string()));
}
