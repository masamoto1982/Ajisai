//! A value's display is source that rebuilds it.
//!
//! Ajisai renders every stack value as text a reader can copy back into the
//! editor and run. For most domains that was always so — `[ 1/1 2/1 ]`, `'ab'`,
//! `TRUE`, `NIL`, `1/3` — and the Record was the one hole: it rendered as
//! `{ 'x': 1/1 }`, which is not a literal because a Record has none, `RECORD`
//! being the only way one comes to exist (LANG.RECORDS.STRUCTURE). It now
//! renders as that call.
//!
//! The law is stated by executing it: render a value, run what was rendered,
//! and require the result to be the same value. A law about a display that is
//! only ever compared against another string would pin the spelling, which is
//! exactly what may change; this pins the property that makes the spelling
//! worth having.
//!
//! Two domains are deliberately out of scope, because the display does not
//! claim to round-trip them and `display.rs` says so:
//!
//! - A Symbol renders as its bare name, which calls a Word rather than
//!   pushing the name.
//! - A role-dependent rendering (datetime, interval, continued fraction) comes
//!   from `format_with_hint`, not from the structural renderer under test.

use ajisai_core::interpreter::Interpreter;

/// Run `source` and render whatever it left on the stack.
async fn run(source: &str) -> Vec<String> {
    let mut interpreter = Interpreter::new();
    interpreter
        .execute(source)
        .await
        .unwrap_or_else(|e| panic!("`{source}` should run, got: {e}"));
    interpreter
        .get_stack()
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// The law: running a value's display leaves exactly that value.
///
/// Re-rendering rather than comparing values directly is what makes this
/// checkable through the public surface, and it is not weaker: the renderer is
/// total and deterministic, so two values that render alike are
/// indistinguishable to every observation surface there is
/// (LANG.OBSERVATION.PROTOCOL).
async fn assert_round_trips(program: &str) {
    let rendered = run(program).await;
    assert_eq!(
        rendered.len(),
        1,
        "`{program}` should leave exactly one value to round-trip, left {rendered:?}"
    );
    let display = &rendered[0];

    let again = run(display).await;
    assert_eq!(
        again.len(),
        1,
        "the display `{display}` should leave exactly one value, left {again:?} — \
         every fragment the renderer writes must net one stack value, or fragments \
         stop composing when they nest"
    );
    assert_eq!(
        &again[0], display,
        "`{program}` rendered as `{display}`, which rebuilt a different value"
    );
}

#[tokio::test]
async fn a_record_round_trips() {
    for program in [
        "[ 'x' 'y' ] [ 1 2 ] RECORD",
        "[ ] [ ] RECORD",
        "[ 'only' ] [ 42 ] RECORD",
        "[ 'a' ] [ NIL ] RECORD",
        "[ 'a' 'b' ] [ TRUE FALSE ] RECORD",
        "[ 'r' ] [ 1/3 ] RECORD",
        "[ 'v' ] [ [ 1 2 3 ] ] RECORD",
    ] {
        assert_round_trips(program).await;
    }
}

#[tokio::test]
async fn a_record_a_core_word_built_round_trips() {
    // The shapes a program actually meets, rather than ones written by hand.
    for program in [
        "[ 'b' 'a' 'b' ] TALLY",
        "[ 1 2 3 ] [ 'a' 'b' 'a' ] GROUP",
        "[ 'x' ] [ 1 ] RECORD 'y' 2 WITH",
        "[ 'x' 'y' ] [ 1 2 ] RECORD 'x' WITHOUT",
        "[ 'x' 'y' ] [ 1 2 ] RECORD [ 'y' 'z' ] [ 9 3 ] RECORD MERGE",
        "'{\"a\": 1, \"b\": [true, null]}' JSON-DECODE",
    ] {
        assert_round_trips(program).await;
    }
}

/// A Record inside a Vector is the case that forced the `COLLECT` phrase.
///
/// A bracket literal does not evaluate what is written inside it, so
/// `[ [ 'a' ] [ 1 ] RECORD ]` is a three-element Vector — two Vectors and the
/// name `RECORD` — rather than the one-element Vector it looks like. Rendering
/// it that way would have been the one failure mode worse than not
/// round-tripping at all: a display that reads back as a *different* value,
/// silently.
#[tokio::test]
async fn a_record_nested_in_a_vector_round_trips() {
    for program in [
        "[ 'a' ] [ 1 ] RECORD 1 COLLECT",
        "[ 'a' ] [ 1 ] RECORD [ 'b' ] [ 2 ] RECORD 2 COLLECT",
        "1 [ 'a' ] [ 2 ] RECORD 2 COLLECT",
        "[ 'a' ] [ 1 ] RECORD 1 COLLECT 1 COLLECT",
        "[ 'outer' ] [ 'a' ] [ 1 ] RECORD 1 COLLECT RECORD",
    ] {
        assert_round_trips(program).await;
    }
}

/// The bracket literal is still what an ordinary Vector renders as. The
/// `COLLECT` phrase is reached only by a Vector that holds a Record, so the
/// overwhelmingly common display is untouched by the Record change.
#[tokio::test]
async fn a_vector_without_a_record_still_renders_as_a_literal() {
    for (program, expected) in [
        ("[ 1 2 3 ]", "[ 1/1 2/1 3/1 ]"),
        ("[ ]", "[ ]"),
        ("[ 'a' 'b' ]", "[ 'a' 'b' ]"),
        ("[ [ 1 ] [ 2 ] ]", "[ [ 1/1 ] [ 2/1 ] ]"),
    ] {
        let rendered = run(program).await;
        assert_eq!(
            rendered,
            [expected],
            "`{program}` should render as a literal"
        );
        assert_round_trips(program).await;
    }
}

/// The domains that always round-tripped must keep doing so: the Record change
/// reaches them through the shared renderer, and a regression there would be
/// far louder than the one it was made for.
#[tokio::test]
async fn the_other_domains_still_round_trip() {
    for program in [
        "42",
        "-7",
        "1/3",
        "0.25",
        "'hello'",
        "TRUE",
        "FALSE",
        "NIL",
        "[ 1 2 ] [ 3 4 ] ADD",
    ] {
        assert_round_trips(program).await;
    }
}
