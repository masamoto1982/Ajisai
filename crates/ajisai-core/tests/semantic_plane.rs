//! The Semantic Plane: one canonical home, stated propagation, stated limits.

mod support;
use support::{failure, line};

use ajisai_core::{role, Error, Interpreter, Role, Value};

/// A value's reading lives on the value. Nothing else stores it, so nothing
/// has to be kept in sync with anything.
#[test]
fn a_role_lives_on_the_value_and_travels_with_it() {
    let mut interpreter = Interpreter::new();
    interpreter.execute("[ 1 3 ] >INTERVAL").unwrap();
    assert_eq!(interpreter.stack()[0].role(), Role::Interval);

    // Into a vector and back out again, through a basin, through a quote
    // boundary, and through the dictionary — the reading is still there.
    assert_eq!(line("[ [ 1 3 ] >INTERVAL ] 0 NTH"), "1..3");
    assert_eq!(line("[ [ 1 3 ] >INTERVAL ] { } MAP 0 NTH"), "1..3");
    assert_eq!(line("{ [ 1 3 ] >INTERVAL } \"SPAN\" DEF SPAN"), "1..3");
}

/// Roles change how a value reads, never what it is.
#[test]
fn the_semantic_plane_does_not_change_the_data_plane() {
    // Same data, different reading, same arithmetic.
    assert_eq!(line("[ 1 3 ] 0 NTH"), "1");
    assert_eq!(line("[ 1 3 ] >INTERVAL 0 NTH"), "1");
    assert_eq!(line("\"A\" 0 NTH"), "65");
    assert_eq!(line("[ 1 3 ] LENGTH [ 1 3 ] >INTERVAL LENGTH EQ"), "TRUE");
}

/// Equality is Data Plane equality. The reading is deliberately excluded, so
/// `EQ` cannot be used to smuggle a reading into a computation.
#[test]
fn equality_ignores_the_reading() {
    assert_eq!(line("\"A\" [ 65 ] EQ"), "TRUE");
    assert_eq!(line("[ 1 3 ] >INTERVAL [ 1 3 ] EQ"), "TRUE");
    assert_eq!(line("\"hi\" \"hi\" >RAW EQ"), "TRUE");
}

/// A reading is only ever set through a checked path. There is no way to put a
/// role on a value whose shape contradicts it.
#[test]
fn asserting_a_reading_is_checked() {
    assert_eq!(line("[ 104 105 ] >TEXT"), "\"hi\"");
    assert_eq!(line("[ 1 3 ] >INTERVAL"), "1..3");
    for source in [
        "[ -1 ] >TEXT",        // not a codepoint
        "[ [ 1 ] ] >TEXT",     // not a number
        "[ 3 1 ] >INTERVAL",   // bounds inverted
        "[ 1 2 3 ] >INTERVAL", // wrong length
        "5 >TEXT",             // not a vector
    ] {
        assert!(
            matches!(failure(source), Error::BadRole { .. }),
            "`{source}` should be BadRole"
        );
    }
}

/// Propagation follows exactly one rule: a result keeps the reading its source
/// had whenever the result's shape still admits that reading, and drops to
/// `RAW` when it does not.
#[test]
fn propagation_follows_the_single_retain_rule() {
    // Still text after REST and REVERSE.
    assert_eq!(line("\"abc\" REST"), "\"bc\"");
    assert_eq!(line("\"abc\" REVERSE"), "\"cba\"");
    assert_eq!(line("\"ab\" \"cd\" CONCAT"), "\"abcd\"");
    // No longer an interval once the shape changes, so the reading drops.
    assert_eq!(line("[ 1 3 ] >INTERVAL REST"), "[ 3 ]");
    assert_eq!(line("[ 1 3 ] >INTERVAL REVERSE"), "[ 3 1 ]");
    // Two containers only agree on a reading if they had the same one.
    assert_eq!(line("\"ab\" [ 1 2 ] CONCAT"), "[ 97 98 1 2 ]");
    // MAP keeps the reading only when the mapped elements still admit it.
    assert_eq!(line("\"ab\" { 1 ADD } MAP"), "\"bc\"");
    assert_eq!(line("\"ab\" { 100000 MUL } MAP"), "[ 9700000 9800000 ]");
}

/// `>RAW` forgets a reading; the data is untouched.
#[test]
fn a_reading_can_be_forgotten() {
    assert_eq!(line("\"hi\" >RAW"), "[ 104 105 ]");
    assert_eq!(line("\"hi\" >RAW ROLE"), "\"RAW\"");
    assert_eq!(line("\"hi\" >RAW >TEXT"), "\"hi\"");
}

/// A reading is observable, which is what makes the plane real rather than
/// decorative.
#[test]
fn a_reading_is_observable() {
    assert_eq!(line("\"hi\" ROLE"), "\"TEXT\"");
    assert_eq!(line("[ 1 3 ] >INTERVAL ROLE"), "\"INTERVAL\"");
    assert_eq!(line("5 ROLE"), "\"RAW\"");
    // Observe without swallowing.
    assert_eq!(line("\"hi\" KEEP ROLE"), "\"hi\" \"TEXT\"");
}

/// Every role has a generator, a consumer, and a propagation rule. A role with
/// no way to reach it would be exactly the kind of unreachable variant this
/// rebuild removed.
#[test]
fn every_role_is_reachable_and_rendered() {
    assert_eq!(Role::ALL.len(), 3);
    for role in Role::ALL {
        let reached = match role {
            Role::Raw => line("5 >RAW ROLE"),
            Role::Text => line("[ 104 105 ] >TEXT ROLE"),
            Role::Interval => line("[ 1 3 ] >INTERVAL ROLE"),
        };
        assert_eq!(reached, format!("\"{}\"", role.name()));
    }
}

/// The well-formedness rule is one function, and the asserting and propagating
/// paths both call it.
#[test]
fn admits_and_retain_agree() {
    let interval = Value::vector(vec![Value::integer(1), Value::integer(3)]);
    let broken = Value::vector(vec![Value::integer(3), Value::integer(1)]);
    assert!(role::admits(Role::Interval, &interval).is_ok());
    assert!(role::admits(Role::Interval, &broken).is_err());
    assert_eq!(role::retain(Role::Interval, &interval), Role::Interval);
    assert_eq!(role::retain(Role::Interval, &broken), Role::Raw);
}
