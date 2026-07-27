//! `VENT` and `^`: the selective release of the flow.
//!
//! The property that matters most here is laziness. A blocked unit is not
//! evaluated, so it cannot divide by zero, cannot name a word that does not
//! exist, and cannot reach the dictionary. These tests would all pass
//! trivially if `VENT` were eager and merely discarded a result, so each one
//! blocks a unit that *would* fail loudly if it ran.

mod support;
use support::{failure, line};

use ajisai_core::{Error, Interpreter};

#[test]
fn a_true_gate_releases_the_unit_and_a_false_gate_blocks_it() {
    assert_eq!(line("TRUE VENT 42"), "42");
    assert_eq!(line("FALSE VENT 42"), "");
    assert_eq!(line("TRUE VENT { 1 2 ADD }"), "3");
    assert_eq!(line("FALSE VENT { 1 2 ADD }"), "");
}

/// A unit that is a single quote is entered rather than pushed: the flow goes
/// through the quoted channel. This is what makes `VENT` the branching word,
/// and why Ajisai needs no `IF`.
#[test]
fn a_quote_unit_is_entered_not_pushed() {
    assert_eq!(line("TRUE VENT { 1 2 ADD }"), "3");
    assert_eq!(line("{ 1 2 ADD }"), "{ 1 2 ADD }"); // unvented, it is a value
}

/// The whole point. Each blocked unit contains something that fails hard.
#[test]
fn a_blocked_unit_is_never_evaluated() {
    assert_eq!(line("FALSE VENT { 1 0 DIV } 7"), "7");
    assert_eq!(line("FALSE VENT NOSUCHWORD 7"), "7");
    assert_eq!(line("FALSE VENT { NOSUCHWORD 1 0 DIV } 7"), "7");
    assert_eq!(line("FALSE VENT { [ 1 ] NOT } 7"), "7");
    // ...and each of them does fail when the gate opens.
    assert!(matches!(
        failure("TRUE VENT { 1 0 DIV }"),
        Error::DivisionByZero
    ));
    assert!(matches!(
        failure("TRUE VENT NOSUCHWORD"),
        Error::UnknownWord(_)
    ));
}

/// A blocked unit has no effect on the dictionary either.
#[test]
fn a_blocked_definition_does_not_reach_the_dictionary() {
    let mut interpreter = Interpreter::new();
    interpreter
        .execute("FALSE VENT { { 1 } \"GHOST\" DEF }")
        .expect("the vent blocks the definition");
    assert!(interpreter.execute("GHOST").is_err());
    interpreter
        .execute("TRUE VENT { { 1 } \"REAL\" DEF }")
        .expect("the vent releases the definition");
    interpreter.execute("REAL").expect("REAL is defined");
}

/// An undetermined gate blocks the unit *and* leaves a mark, so that a vent
/// that could not decide is observably different from one that decided not to
/// open. Blocking silently would be the operational form of reading `UNKNOWN`
/// as `FALSE`.
#[test]
fn an_undetermined_gate_blocks_and_says_so() {
    assert_eq!(line("UNKNOWN VENT { 1 0 DIV }"), "UNKNOWN");
    assert_eq!(line("NIL 1 LT VENT { 99 }"), "UNKNOWN");
    assert_ne!(line("UNKNOWN VENT { 1 }"), line("FALSE VENT { 1 }"));
}

/// Anything that is not a truth value is an error, `NIL` included, and the
/// failing word leaves the flow as it found it.
#[test]
fn a_gate_must_be_a_truth_value() {
    for source in ["NIL VENT 1", "5 VENT 1", "[ 1 ] VENT 1", "{ 1 } VENT 1"] {
        assert!(
            matches!(failure(source), Error::NotATruthValue { .. }),
            "`{source}` should be NotATruthValue"
        );
    }
    let mut interpreter = Interpreter::new();
    let _ = interpreter.execute("9 NIL VENT 1");
    assert_eq!(ajisai_core::render_stack(&interpreter), vec!["9", "NIL"]);
}

#[test]
fn a_vent_with_nothing_to_release_is_an_error() {
    assert!(matches!(failure("TRUE VENT"), Error::VentMissingUnit));
    assert!(matches!(failure("TRUE VENT KEEP"), Error::VentMissingUnit));
}

/// A unit is one node — with two rules that stop it splitting something that
/// must not be split.
#[test]
fn a_unit_carries_its_modes_and_its_nested_vents() {
    // A mode word attaches to the word it governs, so a blocked vent never
    // leaves a mode armed with nothing to consume it.
    assert_eq!(line("1 2 3 FALSE VENT STAK ADD"), "1 2 3");
    assert_eq!(line("1 2 3 TRUE VENT STAK ADD"), "6");
    // A nested vent carries its own unit, so the outer gate governs the inner
    // vent *and* everything the inner vent would have released.
    assert_eq!(line("FALSE VENT VENT { 1 0 DIV } 7"), "7");
    assert_eq!(line("TRUE TRUE VENT VENT { 5 }"), "5");
    // Blocked at the outer gate: the inner vent never runs, so the value it
    // would have read as its own gate is still standing.
    assert_eq!(line("TRUE FALSE VENT VENT { 1 0 DIV } 7"), "TRUE 7");
    // Released at the outer gate, blocked at the inner one.
    assert_eq!(line("FALSE TRUE VENT VENT { 1 0 DIV } 7"), "7");
}

/// `KEEP VENT` draws the gate off the flow, runs the unit against the flow
/// beneath it, and returns the gate to the surface. That ordering is what
/// makes the two-branch idiom work out of two orthogonal features.
#[test]
fn keep_vent_gives_two_branches_without_an_if() {
    assert_eq!(
        line("5 0 GT KEEP VENT { \"positive\" } NOT VENT { \"not positive\" }"),
        "\"positive\""
    );
    assert_eq!(
        line("0 5 GT KEEP VENT { \"positive\" } NOT VENT { \"not positive\" }"),
        "\"not positive\""
    );
    // And when the gate cannot be settled, neither branch runs.
    assert_eq!(
        line("NIL 5 GT KEEP VENT { \"positive\" } NOT VENT { \"not positive\" }"),
        "UNKNOWN UNKNOWN"
    );
}

#[test]
fn stak_has_no_reading_for_a_gate() {
    assert!(matches!(
        failure("TRUE STAK VENT 1"),
        Error::ModeUnsupported { .. }
    ));
}

/// `VENT` and `^` are the same word.
#[test]
fn the_symbol_form_is_the_same_word() {
    let pairs = [
        ("TRUE VENT 42", "TRUE ^ 42"),
        ("FALSE VENT { 1 0 DIV } 7", "FALSE ^ { 1 0 DIV } 7"),
        ("UNKNOWN VENT { 1 }", "UNKNOWN ^ { 1 }"),
        (
            "5 0 GT KEEP VENT { 1 } NOT VENT { 2 }",
            "5 0 GT & ^ { 1 } NOT ^ { 2 }",
        ),
    ];
    for (canonical, symbolic) in pairs {
        assert_eq!(line(canonical), line(symbolic));
    }
    assert_eq!(
        format!("{:?}", failure("NIL VENT 1")),
        format!("{:?}", failure("NIL ^ 1"))
    );
}
