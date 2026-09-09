use super::Interpreter;
use crate::tokenizer::tokenize;

fn predict(source: &str) -> (Vec<String>, bool) {
    let mut interp = Interpreter::new();
    let tokens = tokenize(source).expect("test source must tokenize");
    let prediction = interp.predict_program_outcomes(&tokens);
    (prediction.outcomes, prediction.exact)
}

#[test]
fn calling_any_word_at_all_is_never_exact() {
    // `TRUE` alone has no declared error/NIL vocabulary, but calling it still
    // spends at least one execution step — so under a strict enough profile
    // (which this predictor does not itself narrow by, see the module doc)
    // even this could hit a resource ceiling. Only a program that calls
    // nothing (pure literals) can be exact.
    let (outcomes, exact) = predict("TRUE");
    assert!(outcomes.contains(&"value".to_string()));
    assert!(outcomes.contains(&"error:executionLimitExceeded".to_string()));
    assert!(!exact, "outcomes: {outcomes:?}");
}

#[test]
fn an_arithmetic_call_over_approximates_with_its_declared_vocabulary() {
    let (outcomes, exact) = predict("1 2 ADD");
    assert!(outcomes.contains(&"value".to_string()));
    assert!(outcomes.contains(&"error:nonNumeric".to_string()));
    assert!(outcomes.contains(&"error:shapeMismatch".to_string()));
    assert!(outcomes.contains(&"error:executionLimitExceeded".to_string()));
    assert!(!exact, "a multi-outcome prediction cannot be exact");
}

#[test]
fn an_empty_program_never_underflows_or_touches_resource_ceilings() {
    let (outcomes, exact) = predict("");
    assert_eq!(outcomes, vec!["value".to_string()]);
    assert!(exact, "outcomes: {outcomes:?}");
}

#[test]
fn a_bare_word_with_no_operands_predicts_stack_underflow() {
    let (outcomes, _exact) = predict("ADD");
    assert!(outcomes.contains(&"error:stackUnderflow".to_string()));
}

#[test]
fn a_fully_supplied_call_does_not_predict_stack_underflow() {
    let (outcomes, _exact) = predict("1 2 ADD");
    assert!(!outcomes.contains(&"error:stackUnderflow".to_string()));
}

#[test]
fn a_called_user_word_contributes_its_bodys_vocabulary() {
    let (outcomes, _exact) = predict("[ 1 ADD ] 'INC' DEF 5 INC");
    assert!(outcomes.contains(&"error:nonNumeric".to_string()));
}

#[test]
fn an_uncalled_definitions_body_vocabulary_does_not_leak_to_the_top_level() {
    let (with_call, _) = predict("[ 1 ADD ] 'INC' DEF 5 INC");
    let (without_call, _) = predict("[ 1 ADD ] 'INC' DEF");
    assert!(with_call.contains(&"error:nonNumeric".to_string()));
    assert!(!without_call.contains(&"error:nonNumeric".to_string()));
}

#[test]
fn a_code_operand_of_a_higher_order_word_contributes_its_vocabulary() {
    let (outcomes, _exact) = predict("[ 1 2 3 ] [ 1 ADD ] MAP");
    assert!(outcomes.contains(&"error:nonNumeric".to_string()));
}
