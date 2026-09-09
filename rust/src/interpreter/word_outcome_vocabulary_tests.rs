use super::word_outcome_vocabulary::{builtin_outcomes_for, conservative_outcomes};

#[test]
fn builtin_outcomes_include_value_and_declared_errors() {
    let outcomes = builtin_outcomes_for("ADD");
    assert!(outcomes.contains("value"));
    assert!(outcomes.contains("error:nonNumeric"));
    assert!(outcomes.contains("error:shapeMismatch"));
}

#[test]
fn builtin_outcomes_include_declared_nil_projections() {
    let outcomes = builtin_outcomes_for("MOD");
    assert!(outcomes.contains("nil:divisionByZero"));
    assert!(outcomes.contains("nil:undecidable"));
}

#[test]
fn conservative_outcomes_cover_the_whole_registry() {
    let outcomes = conservative_outcomes();
    assert!(outcomes.contains("value"));
    assert!(outcomes.contains("error:stackUnderflow"));
    assert!(outcomes.contains("nil:spaceExhausted"));
    assert!(outcomes.len() > 40);
}
