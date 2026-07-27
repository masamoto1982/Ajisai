//! Strong Kleene logic, end to end, and the three-way split between `NIL`,
//! `UNKNOWN`, and an error.

mod support;
use support::{failure, line};

use ajisai_core::Error;

#[test]
fn negation_table() {
    assert_eq!(line("TRUE NOT"), "FALSE");
    assert_eq!(line("FALSE NOT"), "TRUE");
    assert_eq!(line("UNKNOWN NOT"), "UNKNOWN");
}

#[test]
fn conjunction_table() {
    assert_eq!(line("TRUE TRUE AND"), "TRUE");
    assert_eq!(line("TRUE FALSE AND"), "FALSE");
    assert_eq!(line("TRUE UNKNOWN AND"), "UNKNOWN");
    assert_eq!(line("FALSE TRUE AND"), "FALSE");
    assert_eq!(line("FALSE FALSE AND"), "FALSE");
    assert_eq!(line("FALSE UNKNOWN AND"), "FALSE");
    assert_eq!(line("UNKNOWN TRUE AND"), "UNKNOWN");
    assert_eq!(line("UNKNOWN FALSE AND"), "FALSE");
    assert_eq!(line("UNKNOWN UNKNOWN AND"), "UNKNOWN");
}

#[test]
fn disjunction_table() {
    assert_eq!(line("TRUE TRUE OR"), "TRUE");
    assert_eq!(line("TRUE FALSE OR"), "TRUE");
    assert_eq!(line("TRUE UNKNOWN OR"), "TRUE");
    assert_eq!(line("FALSE TRUE OR"), "TRUE");
    assert_eq!(line("FALSE FALSE OR"), "FALSE");
    assert_eq!(line("FALSE UNKNOWN OR"), "UNKNOWN");
    assert_eq!(line("UNKNOWN TRUE OR"), "TRUE");
    assert_eq!(line("UNKNOWN FALSE OR"), "UNKNOWN");
    assert_eq!(line("UNKNOWN UNKNOWN OR"), "UNKNOWN");
}

/// The canonical way `UNKNOWN` enters a program without being written: a
/// comparison that observation cannot settle. Every one of these is reachable
/// from source, so K3 is not a logic without a way to reach its third value.
#[test]
fn comparison_is_a_canonical_source_of_unknown() {
    assert_eq!(line("NIL 1 LT"), "UNKNOWN");
    assert_eq!(line("1 NIL GT"), "UNKNOWN");
    assert_eq!(line("NIL NIL EQ"), "UNKNOWN");
    assert_eq!(line("NIL 1 EQ"), "UNKNOWN");
    assert_eq!(line("UNKNOWN 1 LE"), "UNKNOWN");
    assert_eq!(line("[ 1 ] NIL NE"), "UNKNOWN");
    // And the literal word, for completeness.
    assert_eq!(line("UNKNOWN"), "UNKNOWN");
}

#[test]
fn unknown_is_not_nil() {
    assert_eq!(line("UNKNOWN NIL? "), "FALSE");
    assert_eq!(line("NIL UNKNOWN?"), "FALSE");
    assert_eq!(line("UNKNOWN UNKNOWN?"), "TRUE");
    assert_eq!(line("NIL NIL?"), "TRUE");
    // They render differently and they are different values.
    assert_ne!(line("NIL"), line("UNKNOWN"));
}

/// Arithmetic and comparison carry absence and indeterminacy by *different*
/// rules, and where they meet, absence wins.
#[test]
fn nil_and_unknown_propagate_separately() {
    assert_eq!(line("NIL 1 ADD"), "NIL");
    assert_eq!(line("UNKNOWN 1 ADD"), "UNKNOWN");
    assert_eq!(line("NIL UNKNOWN ADD"), "NIL");
    assert_eq!(line("UNKNOWN NIL MUL"), "NIL");
    // Comparison sends both to UNKNOWN, so there is no conflict to resolve.
    assert_eq!(line("NIL UNKNOWN EQ"), "UNKNOWN");
}

/// `NIL` in a logical position is an error, not a third reading of falsity.
/// This is the guard that keeps K3 from collapsing back into two values.
#[test]
fn nil_is_not_a_truth_value() {
    for source in ["NIL NOT", "NIL TRUE AND", "TRUE NIL OR", "NIL VENT { 1 }"] {
        match failure(source) {
            Error::NotATruthValue { found, .. } => assert_eq!(found, "NIL"),
            other => panic!("`{source}` gave {other:?}, expected NotATruthValue"),
        }
    }
}

#[test]
fn numbers_are_not_truth_values_either() {
    assert!(matches!(failure("1 NOT"), Error::NotATruthValue { .. }));
    assert!(matches!(
        failure("[ 1 ] TRUE AND"),
        Error::NotATruthValue { .. }
    ));
}

/// An error is the third negative outcome and it is not a value: nothing
/// converts it into `NIL` or `UNKNOWN`, and there is no word that catches it.
#[test]
fn errors_are_not_values() {
    assert!(matches!(failure("1 0 DIV"), Error::DivisionByZero));
    assert!(matches!(
        failure("NOSUCHWORD"),
        Error::UnknownWord(name) if name == "NOSUCHWORD"
    ));
}

/// UNKNOWN never silently becomes a Boolean.
#[test]
fn unknown_does_not_decay_into_a_boolean() {
    assert_eq!(line("UNKNOWN BOOLEAN?"), "FALSE");
    // `p OR NOT p` is not a tautology when p is unknown.
    assert_eq!(line("UNKNOWN KEEP NOT OR"), "UNKNOWN");
}

/// A filter must decide, and K3 says it cannot when the verdict is `UNKNOWN`.
/// Keeping would read UNKNOWN as TRUE, dropping would read it as FALSE, so the
/// word refuses instead of picking one silently.
#[test]
fn filter_refuses_an_undecided_predicate() {
    assert_eq!(line("[ 1 2 3 ] { 1 GT } FILTER"), "[ 2 3 ]");
    assert!(matches!(
        failure("[ 1 NIL 3 ] { 1 GT } FILTER"),
        Error::UndecidedPredicate { .. }
    ));
}
