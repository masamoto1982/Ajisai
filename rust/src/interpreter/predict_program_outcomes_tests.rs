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
