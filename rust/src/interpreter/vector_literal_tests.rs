//! Tests for compile-time literal-vector lowering (`CompiledOp::PushVectorLiteral`).
//!
//! A fully-literal vector is prebuilt once at compile time with the same
//! promoted value and element hint `collect_vector` produces, so the line runs
//! compiled instead of falling back to the interpreter. These tests pin that the
//! lowered path is byte-for-byte identical to the interpreted one across element
//! kinds, that non-literal vectors still fall back, and that errors are kept.

use crate::interpreter::Interpreter;

fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    use std::task::{Context, Poll};
    let mut fut = Box::pin(fut);
    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(waker);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// Run `src` twice — lowering on and off — and assert the resulting stacks are
/// identical (value and the rendered form, which depends on the element hint).
fn assert_on_equals_off(src: &str) -> String {
    let mut on = Interpreter::new();
    on.set_vector_literal_enabled(true);
    block_on(on.execute(src)).unwrap();

    let mut off = Interpreter::new();
    off.set_vector_literal_enabled(false);
    block_on(off.execute(src)).unwrap();

    assert_eq!(
        format!("{:?}", on.get_stack()),
        format!("{:?}", off.get_stack()),
        "lowering ON vs OFF diverged for: {src}"
    );
    let render_on = render(&on);
    assert_eq!(render_on, render(&off), "rendered form diverged for: {src}");
    render_on
}

fn render(interp: &Interpreter) -> String {
    interp
        .get_stack()
        .last()
        .map(|v| format!("{v}"))
        .unwrap_or_default()
}

#[test]
fn literal_vector_shapes_match_interpreter() {
    // Numeric (tensor-promoted), boolean (TruthValue hint), string (Text),
    // NIL-bearing, nested, and arithmetic-over-literals all agree.
    let cases = [
        "{ [ 1 2 3 ] [ 4 5 6 ] + } 'W' DEF W",
        "{ [ TRUE FALSE TRUE ] } 'W' DEF W",
        "{ [ 'a' 'b' 'c' ] } 'W' DEF W",
        "{ [ 1 NIL 3 ] } 'W' DEF W",
        "{ [ [ 1 2 ] [ 3 4 ] ] } 'W' DEF W",
        "{ [ 1 2 3 4 ] [ 2 2 2 2 ] * [ 1 1 1 1 ] - } 'W' DEF W",
    ];
    for src in cases {
        assert_on_equals_off(src);
    }
}

#[test]
fn boolean_vector_keeps_truth_value_rendering() {
    // The element hint is what makes a boolean vector render as TRUE/FALSE; the
    // lowered op must carry it so the display is unchanged.
    let rendered = assert_on_equals_off("{ [ TRUE FALSE ] } 'W' DEF W");
    assert!(
        rendered.contains("TRUE") && rendered.contains("FALSE"),
        "boolean vector should render as TRUE/FALSE, got: {rendered}"
    );
}

#[test]
fn symbol_in_vector_is_data_not_executed() {
    // Data-ization (SPEC §4.3): a bare symbol inside a vector literal is the
    // symbol text as data, even when it names a defined user word. `[ TEN 2 3 ]`
    // is therefore a fully literal vector — TEN is the string "TEN", never the
    // word's result — and lowers identically on the compiled and interpreted
    // paths. This is the regression guard for the retired word-execution behavior.
    let src = "{ [ 10 ] } 'TEN' DEF\n{ [ TEN 2 3 ] } 'W' DEF\nW";
    let rendered = assert_on_equals_off(src);
    assert!(
        rendered.contains("TEN"),
        "the symbol must appear as data, got: {rendered}"
    );
    assert!(
        !rendered.contains("10"),
        "the user word must NOT be executed inside the vector, got: {rendered}"
    );
}

#[test]
fn vector_literal_is_independent_of_dictionary_state() {
    // The core of data-ization (SPEC §4.3): the *same* source vector produces
    // the *same* value whether or not the symbol names a defined word. Before,
    // `[ FOO 1 ]` executed FOO when defined and was data otherwise — a
    // dictionary-state-dependent meaning. Now both are the data `[ "FOO" 1 ]`.
    let mut with_word = Interpreter::new();
    block_on(with_word.execute("{ [ 99 ] } 'FOO' DEF\n[ FOO 1 ]")).unwrap();

    let mut without_word = Interpreter::new();
    block_on(without_word.execute("[ FOO 1 ]")).unwrap();

    assert_eq!(
        format!("{}", with_word.get_stack().last().unwrap()),
        format!("{}", without_word.get_stack().last().unwrap()),
        "a vector literal must not depend on whether the symbol is a defined word"
    );
    assert!(
        !format!("{}", with_word.get_stack().last().unwrap()).contains("99"),
        "the defined word must not be executed inside the vector"
    );
}

#[test]
fn empty_vector_still_errors_both_paths() {
    // `[ ]` is rejected by the interpreter; the lowering must not paper over it
    // by silently building a NIL — it stays a fallback so the same error is
    // raised whether lowering is on or off.
    for enabled in [true, false] {
        let mut interp = Interpreter::new();
        interp.set_vector_literal_enabled(enabled);
        let err = block_on(interp.execute("{ [ ] } 'W' DEF\nW"))
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(
            err.to_lowercase().contains("empty"),
            "empty vector should error (enabled={enabled}), got: {err:?}"
        );
    }
}

#[test]
fn matches_readme_vector_example() {
    let rendered = assert_on_equals_off("{ [ 1 2 3 ] [ 4 5 6 ] + } 'W' DEF W");
    assert!(
        rendered.contains("5/1") && rendered.contains("7/1") && rendered.contains("9/1"),
        "expected [ 5/1 7/1 9/1 ], got: {rendered}"
    );
}
