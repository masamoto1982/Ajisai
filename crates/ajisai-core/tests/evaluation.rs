//! The evaluation model: exactness, vectors, basins, quotes, and the
//! dictionary.

mod support;
use support::{failure, line};

use ajisai_core::{Error, Interpreter};

/// No result depends on how far a computation was carried, because nothing is
/// ever rounded.
#[test]
fn arithmetic_is_exact() {
    assert_eq!(line("1 3 DIV 3 MUL"), "1");
    assert_eq!(line("0.1 0.2 ADD"), "3/10");
    assert_eq!(line("0.1 0.2 ADD 0.3 EQ"), "TRUE");
    assert_eq!(line("2 3 DIV"), "2/3");
    assert_eq!(line("1 3 DIV 1 6 DIV ADD"), "1/2");
    // Arbitrary precision, not 64 bits.
    assert_eq!(
        line("99999999999999999999 99999999999999999999 MUL"),
        "9999999999999999999800000000000000000001"
    );
}

#[test]
fn division_by_zero_is_an_error_not_an_infinity() {
    assert!(matches!(failure("1 0 DIV"), Error::DivisionByZero));
    assert!(matches!(failure("0 0 DIV"), Error::DivisionByZero));
}

/// A basin runs its body on a fresh flow and collects what stands there. A
/// vector literal is not a special case of the parser; it is this.
#[test]
fn a_basin_collects_the_flow_it_encloses() {
    assert_eq!(line("[ 1 2 3 ]"), "[ 1 2 3 ]");
    assert_eq!(line("[ 1 2 ADD ]"), "[ 3 ]");
    assert_eq!(line("[ 1 2 3 STAK ADD ]"), "[ 6 ]");
    assert_eq!(line("[ ]"), "[ ]");
    assert_eq!(line("[ [ 1 2 ] [ 3 4 ] ]"), "[ [ 1 2 ] [ 3 4 ] ]");
    // The enclosing flow is untouched by what happens inside.
    assert_eq!(line("9 [ 1 2 ADD ]"), "9 [ 3 ]");
}

/// Vectors nest, so a matrix needs no separate type and no shape metadata.
#[test]
fn vectors_nest() {
    assert_eq!(line("[ [ 1 2 ] [ 3 4 ] ] LENGTH"), "2");
    assert_eq!(line("[ [ 1 2 ] [ 3 4 ] ] 1 NTH 0 NTH"), "3");
    assert_eq!(
        line("[ [ 1 2 ] [ 3 4 ] ] { { 2 MUL } MAP } MAP"),
        "[ [ 2 4 ] [ 6 8 ] ]"
    );
}

#[test]
fn vector_words() {
    assert_eq!(line("[ 1 2 3 ] LENGTH"), "3");
    assert_eq!(line("[ 1 2 3 ] 1 NTH"), "2");
    assert_eq!(line("[ 1 2 3 ] FIRST"), "1");
    assert_eq!(line("[ 1 2 3 ] REST"), "[ 2 3 ]");
    assert_eq!(line("[ 1 2 ] 3 APPEND"), "[ 1 2 3 ]");
    assert_eq!(line("[ 1 ] [ 2 ] CONCAT"), "[ 1 2 ]");
    assert_eq!(line("[ 1 2 3 ] REVERSE"), "[ 3 2 1 ]");
    assert_eq!(line("0 5 RANGE"), "[ 0 1 2 3 4 ]");
    assert_eq!(line("3 3 RANGE"), "[ ]");
}

/// An empty vector has no first element. That is an absence, not a broken
/// rule, so it is `NIL` rather than an error.
#[test]
fn an_empty_vector_yields_absence_not_an_error() {
    assert_eq!(line("[ ] FIRST"), "NIL");
    assert_eq!(line("[ ] REST"), "[ ]");
    assert_eq!(line("[ ] LENGTH"), "0");
    // An index outside the vector is a different thing: a broken rule.
    assert!(matches!(
        failure("[ 1 2 ] 9 NTH"),
        Error::IndexOutOfRange { .. }
    ));
}

#[test]
fn words_over_quotes() {
    assert_eq!(line("[ 1 2 3 ] { 2 MUL } MAP"), "[ 2 4 6 ]");
    assert_eq!(line("[ 1 2 3 4 ] { 2 GT } FILTER"), "[ 3 4 ]");
    assert_eq!(line("[ 1 2 3 ] 0 { ADD } FOLD"), "6");
    assert_eq!(line("[ 1 2 3 ] 1 { MUL } FOLD"), "6");
    assert_eq!(line("1 2 { ADD } EXEC"), "3");
}

/// A quote handed to `MAP` runs in a basin seeded with its element, so it
/// cannot reach past its own operands into the surrounding flow.
#[test]
fn a_quote_cannot_reach_past_its_operands() {
    assert_eq!(line("99 [ 1 2 ] { 2 MUL } MAP"), "99 [ 2 4 ]");
    assert!(matches!(
        failure("99 [ 1 2 ] { ADD } MAP"),
        Error::StackUnderflow { .. }
    ));
    // ...whereas EXEC deliberately does run against the current flow.
    assert_eq!(line("99 1 { ADD } EXEC"), "100");
}

/// A quote that leaves anything other than one value is an error, not a
/// silently reshaped result.
#[test]
fn a_mapping_quote_must_leave_exactly_one_value() {
    assert!(matches!(
        failure("[ 1 2 ] { DUP } MAP"),
        Error::TypeMismatch { .. }
    ));
    assert!(matches!(
        failure("[ 1 2 ] { DROP } MAP"),
        Error::TypeMismatch { .. }
    ));
}

/// A definition is two ordinary values and one ordinary word. There is no
/// defining syntax and no parser special case.
#[test]
fn definitions_need_no_syntax() {
    assert_eq!(line("{ 2 MUL } \"DOUBLE\" DEF 21 DOUBLE"), "42");
    assert_eq!(
        line("{ 2 MUL } \"DOUBLE\" DEF { DOUBLE DOUBLE } \"QUAD\" DEF 5 QUAD"),
        "20"
    );
    // Names are case-insensitive and canonically uppercase: both spellings
    // reach the one definition.
    assert_eq!(line("{ 1 ADD } \"bump\" DEF 1 BUMP 1 bump"), "2 2");
}

#[test]
fn definitions_can_be_removed() {
    let mut interpreter = Interpreter::new();
    interpreter.execute("{ 1 } \"ONE\" DEF ONE").unwrap();
    interpreter.execute("\"ONE\" DEL").unwrap();
    assert!(matches!(
        interpreter.execute("ONE"),
        Err(Error::UnknownWord(_))
    ));
}

#[test]
fn built_in_and_directive_names_are_reserved() {
    for name in ["ADD", "+", "VENT", "^", "KEEP", "STAK", "TRUE", "NIL"] {
        let source = format!("{{ 1 }} \"{name}\" DEF");
        assert!(
            matches!(failure(&source), Error::ReservedWord(_)),
            "`{name}` should be reserved"
        );
    }
}

/// Recursion is bounded by a budget, and reaching it is an error rather than
/// a crash.
#[test]
fn runaway_recursion_is_bounded() {
    assert!(matches!(
        failure("{ LOOP } \"LOOP\" DEF LOOP"),
        Error::DepthLimitExceeded { .. }
    ));
}

/// A failing word leaves the flow as it found it: operands are validated
/// before anything is committed.
#[test]
fn a_failing_word_does_not_half_consume_the_flow() {
    let mut interpreter = Interpreter::new();
    let _ = interpreter.execute("1 [ 2 ] ADD");
    assert_eq!(ajisai_core::render_stack(&interpreter), vec!["1", "[ 2 ]"]);

    let mut interpreter = Interpreter::new();
    let _ = interpreter.execute("7 1 0 DIV");
    assert_eq!(ajisai_core::render_stack(&interpreter), vec!["7", "1", "0"]);
}

#[test]
fn comments_and_whitespace_are_not_program() {
    assert_eq!(line("1 2 ADD # this is ignored\n 10 ADD"), "13");
    assert_eq!(line("\n\t 1   2\nADD "), "3");
}

#[test]
fn malformed_source_is_rejected() {
    assert!(matches!(failure("[ 1 2"), Error::Unbalanced { .. }));
    assert!(matches!(failure("1 2 ]"), Error::Unbalanced { .. }));
    assert!(matches!(failure("{ 1"), Error::Unbalanced { .. }));
    assert!(matches!(failure("\"unclosed"), Error::Unbalanced { .. }));
    assert!(matches!(failure("1.2.3"), Error::MalformedToken(_)));
}

/// The flow persists between fragments, which is what makes a session real.
#[test]
fn the_flow_persists_across_fragments() {
    let mut interpreter = Interpreter::new();
    interpreter.execute("1 2").unwrap();
    interpreter.execute("ADD").unwrap();
    interpreter.execute("10 MUL").unwrap();
    assert_eq!(ajisai_core::render_stack(&interpreter), vec!["30"]);
}

/// `SPECIFICATION.md` §5.7 promises the flow is untouched by a word that
/// fails. Every one of these words draws its own operands and can fail after
/// doing so, which is exactly where the promise used to break.
#[test]
fn every_failing_word_leaves_the_flow_untouched() {
    let cases = [
        // A higher-order word that took its quote before checking its vector.
        ("1 { } MAP", vec!["1", "{ }"]),
        ("1 { } FILTER", vec!["1", "{ }"]),
        ("1 2 { } FOLD", vec!["1", "2", "{ }"]),
        // A quote that fails partway through, having already pushed.
        ("9 { 1 0 DIV } EXEC", vec!["9", "{ 1 0 DIV }"]),
        ("9 { 1 2 NOSUCHWORD } EXEC", vec!["9", "{ 1 2 NOSUCHWORD }"]),
        // The dictionary words, which took both operands before checking.
        ("{ 1 } \"ADD\" DEF", vec!["{ 1 }", "\"ADD\""]),
        ("5 \"NOPE\" DEL", vec!["5", "\"NOPE\""]),
        ("5 [ 88 ] DEF", vec!["5", "[ 88 ]"]),
        // A released vent whose unit fails — the gate comes back too.
        ("TRUE VENT { 1 0 DIV }", vec!["TRUE"]),
        ("7 TRUE VENT { NOSUCHWORD }", vec!["7", "TRUE"]),
        // A user definition that fails partway through its body.
        ("{ 1 0 DIV } \"BOOM\" DEF 9 BOOM", vec!["9"]),
    ];
    for (source, expected) in cases {
        let mut interpreter = Interpreter::new();
        assert!(
            interpreter.execute(source).is_err(),
            "`{source}` should fail"
        );
        assert_eq!(
            ajisai_core::render_stack(&interpreter),
            expected,
            "`{source}` disturbed the flow"
        );
    }
}

/// The promise is about the flow. A quote that defines a word and then fails
/// leaves the definition behind, and the specification says so rather than
/// pretending otherwise.
#[test]
fn the_dictionary_is_not_rolled_back() {
    let mut interpreter = Interpreter::new();
    assert!(interpreter
        .execute("{ { 1 } \"GHOST\" DEF 1 0 DIV } EXEC")
        .is_err());
    interpreter.execute("GHOST").expect("GHOST survived");
}

/// A name must be read as text. `[ 68 79 85 ]` is a vector of three numbers
/// that happens to spell `DOU`, and treating it as a name would mean the
/// reading a program asserts about its own data counts for nothing.
#[test]
fn a_name_must_carry_the_text_role() {
    assert!(matches!(
        failure("{ 2 MUL } [ 68 79 85 66 76 69 ] DEF"),
        Error::TypeMismatch { .. }
    ));
    assert!(matches!(
        failure("{ 1 } \"X\" DEF [ 88 ] DEL"),
        Error::TypeMismatch { .. }
    ));
    // Saying so explicitly is enough — it is the same vector either way.
    assert_eq!(line("{ 2 MUL } [ 68 79 ] >TEXT DEF 21 DO"), "42");
    assert_eq!(line("{ 2 MUL } \"DOUBLE\" DEF 21 DOUBLE"), "42");
}

/// Case folding is ASCII-only and nothing else is normalized, so word identity
/// does not depend on which Unicode table an implementation was built with.
#[test]
fn word_names_fold_case_in_ascii_only() {
    assert_eq!(line("{ 1 ADD } \"bump\" DEF 1 BUMP"), "2");
    assert_eq!(line("{ 1 ADD } \"Bump\" DEF 1 bUmP"), "2");
    // A non-ASCII name is itself, and is usable.
    assert_eq!(line("{ 2 MUL } \"倍\" DEF 21 倍"), "42");
}
