//! A value's display is source that rebuilds it.
//!
//! Ajisai renders a stack value as text a reader can copy back into the
//! editor and run wherever the value's domain has a literal: `[ 1/1 2/1 ]`,
//! `'ab'`, `TRUE`, `NIL`, `1/3`.
//!
//! The law is stated by executing it: render a value, run what was rendered,
//! and require the result to be the same value. A law about a display that is
//! only ever compared against another string would pin the spelling, which is
//! exactly what may change; this pins the property that makes the spelling
//! worth having.
//!
//! Three kinds of value are deliberately out of scope, because the display
//! does not claim to round-trip them and `display_source.rs` says so:
//!
//! - A Record renders as `{ key value … }`, which is a display, not source:
//!   only `[ ]` delimits, and `RECORD` is how a program builds one. A Vector
//!   holding a Record inherits this.
//! - A Symbol renders as its bare name, which calls a Word rather than
//!   pushing the name.
//! - An irrational scalar renders its normal form as one token,
//!   `1/1+sqrt(2)`: no literal denotes an irrational, and the source that
//!   builds one (`1 2 SQRT ADD`) would read as three elements inside a
//!   Vector literal.

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

/// A Vector renders as its own literal, and the spelling is the one a reader
/// writes: this is the half of the law a round trip alone cannot fix, since a
/// wrong-but-consistent spelling would round-trip too.
#[tokio::test]
async fn a_vector_renders_as_its_own_literal() {
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

/// Every other domain with a literal round-trips.
#[tokio::test]
async fn the_other_domains_round_trip() {
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

/// An irrational displays its exact normal form as one token, so a Vector of
/// them still reads element by element; the display is not source, and
/// reading it back is a call of an unknown Word.
#[tokio::test]
async fn an_irrational_displays_but_is_not_source() {
    for (program, expected) in [
        ("2 SQRT", "sqrt(2)"),
        ("1 2 SQRT ADD", "1/1+sqrt(2)"),
        ("2 SQRT 3 SQRT SUB", "sqrt(2)-sqrt(3)"),
        ("2 SQRT 3 2 COLLECT", "[ sqrt(2) 3/1 ]"),
    ] {
        assert_eq!(run(program).await, [expected]);
    }
    let mut interpreter = Interpreter::new();
    assert!(interpreter.execute("sqrt(2)").await.is_err());
}

/// A Record displays key beside value, and the display is not source: `{` is
/// an ordinary name, so reading it back is a call of an unknown Word.
#[tokio::test]
async fn a_record_displays_but_is_not_source() {
    assert_eq!(
        run("[ 'x' 'y' ] [ 1 2 ] RECORD").await,
        ["{ 'x' 1/1 'y' 2/1 }"]
    );
    assert_eq!(run("[ ] [ ] RECORD").await, ["{ }"]);
    assert_eq!(
        run("[ 'a' ] [ 1 ] RECORD 1 COLLECT").await,
        ["[ { 'a' 1/1 } ]"]
    );
    let mut interpreter = Interpreter::new();
    assert!(interpreter.execute("{ 'x' 1/1 }").await.is_err());
}
