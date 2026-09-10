use std::collections::BTreeSet;

use super::word_outcome_vocabulary::{
    builtin_outcomes_for, close_over_nil_reason_loss, conservative_outcomes,
};

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
