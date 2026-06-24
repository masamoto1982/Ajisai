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
fn non_literal_vector_still_works() {
    // A vector containing a user word is not a literal: it must fall back to the
    // interpreter (which executes the word during collection) and still produce
    // the same result with lowering on or off.
    let src = "{ [ 10 ] } 'TEN' DEF\n{ [ TEN 2 3 ] } 'W' DEF\nW";
    let rendered = assert_on_equals_off(src);
    assert!(
        rendered.contains("10") && rendered.contains("2") && rendered.contains('3'),
        "expected the word-produced element, got: {rendered}"
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
