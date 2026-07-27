//! The contract lint reports; it does not prove.
//!
//! Half of these tests are about what the lint *declines* to say. A lint that
//! guesses is worse than no lint, so the ones below fix the boundary where it
//! stops.

use ajisai_core::lint::{lint, Severity};
use ajisai_core::Interpreter;

fn findings(source: &str) -> Vec<String> {
    let interpreter = Interpreter::new();
    lint(&interpreter, source)
        .expect("parses")
        .iter()
        .map(|finding| finding.to_string())
        .collect()
}

fn errors(source: &str) -> usize {
    let interpreter = Interpreter::new();
    lint(&interpreter, source)
        .expect("parses")
        .iter()
        .filter(|finding| finding.severity == Severity::Error)
        .count()
}

#[test]
fn obvious_underflow_is_reported() {
    let reported = findings("1 ADD");
    assert_eq!(reported.len(), 1);
    assert!(reported[0].contains("needs 2"), "{reported:?}");
    assert!(reported[0].contains("( a b -- sum )"), "{reported:?}");
    assert_eq!(errors("ADD"), 1);
    assert_eq!(errors("1 2 ADD"), 0);
}

#[test]
fn obvious_type_mismatch_is_reported() {
    let reported = findings("[ 1 2 ] 3 ADD");
    assert_eq!(reported.len(), 1);
    assert!(reported[0].contains("expected number"), "{reported:?}");
    assert_eq!(errors("1 2 CONCAT"), 2); // both operands are wrong
    assert_eq!(errors("[ 1 ] [ 2 ] CONCAT"), 0);
}

#[test]
fn an_unknown_word_is_reported() {
    assert_eq!(errors("1 NOSUCHWORD"), 1);
    assert!(findings("1 NOSUCHWORD")[0].contains("unknown word"));
}

#[test]
fn a_dangling_mode_is_reported() {
    assert_eq!(errors("1 2 ADD KEEP"), 1);
    assert!(findings("1 2 ADD KEEP")[0].contains("no word consumed it"));
}

#[test]
fn a_vent_with_no_unit_is_reported() {
    assert_eq!(errors("TRUE VENT"), 1);
}

/// An operand that may be absent, flowing into a word that refuses absence,
/// is worth a look but is not a contradiction: the vector may never be empty.
#[test]
fn a_possible_nil_reaching_a_refusing_word_is_an_advisory() {
    let interpreter = Interpreter::new();
    let reported = lint(&interpreter, "[ 1 2 ] FIRST NOT").expect("parses");
    assert_eq!(reported.len(), 1);
    assert_eq!(reported[0].severity, Severity::Advisory);
    assert!(reported[0].message.contains("may be NIL"), "{reported:?}");
}

/// Both terms of the policy are read: the `UNKNOWN` side too.
#[test]
fn a_possible_unknown_reaching_a_refusing_word_is_an_advisory() {
    let interpreter = Interpreter::new();
    // A comparison is a canonical UNKNOWN source, and CONCAT refuses UNKNOWN.
    // The operand is also the wrong type, so both findings fire; the advisory
    // is the one under test.
    let reported = lint(&interpreter, "1 2 LT [ 1 ] CONCAT").expect("parses");
    assert!(
        reported
            .iter()
            .any(|f| f.severity == Severity::Advisory && f.message.contains("may be UNKNOWN")),
        "{reported:?}"
    );
}

/// Where the flow stops being knowable, the lint stops reporting rather than
/// guessing. Each of these programs is genuinely fine at runtime, and a lint
/// that modelled them naively would accuse all of them.
#[test]
fn the_lint_goes_quiet_rather_than_guessing() {
    for source in [
        "1 2 3 STAK ADD",            // a mode reshapes the whole flow
        "1 2 3 STAK ADD ADD",        // ...and the depth after it is not known
        "TRUE VENT { 1 } ADD",       // the unit may or may not have run
        "[ 1 2 ] { 1 ADD } MAP ADD", // a dynamic word
    ] {
        assert_eq!(
            errors(source),
            0,
            "the lint should stay quiet on `{source}`: {:?}",
            findings(source)
        );
    }
}

/// The lint never claims success. It reports what it found, and a clean run
/// says only that nothing obvious turned up.
#[test]
fn the_lint_makes_no_safety_claim() {
    // Each of these passes the lint and fails at runtime. That is not a bug in
    // the lint: it is the documented limit of what a stack-and-type check can
    // see, and the reason the lint's clean message is worded as it is.
    for source in ["1 0 DIV", "[ 1 2 ] 9 NTH", "[ 1 2 ] { DUP } MAP"] {
        assert_eq!(errors(source), 0, "`{source}`");
        let mut interpreter = Interpreter::new();
        assert!(
            interpreter.execute(source).is_err(),
            "`{source}` should still fail at runtime"
        );
    }
}

/// A user definition's effect is not inferred, so a program using one is not
/// accused of anything.
#[test]
fn user_definitions_are_not_second_guessed() {
    let mut interpreter = Interpreter::new();
    interpreter.execute("{ 1 ADD } \"BUMP\" DEF").unwrap();
    let reported = lint(&interpreter, "1 BUMP BUMP ADD").expect("parses");
    assert!(reported.is_empty(), "{reported:?}");
}

/// Quote and basin bodies are linted on their own, so a mistake inside one is
/// still found.
#[test]
fn nested_bodies_are_linted() {
    assert_eq!(errors("[ 1 NOSUCHWORD ]"), 1);
    assert_eq!(errors("{ NOSUCHWORD }"), 1);
    assert_eq!(errors("TRUE VENT { NOSUCHWORD }"), 1);
}

/// The lint never blocks execution.
#[test]
fn findings_do_not_stop_a_program() {
    let mut interpreter = Interpreter::new();
    assert_eq!(errors("1 2 3 ADD ADD ADD"), 1); // the third ADD underflows
    interpreter.execute("1 2 3 ADD ADD").expect("runs fine");
}
