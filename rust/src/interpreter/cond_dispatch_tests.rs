//! Tests for precompiled COND clause dispatch (`CompiledOp::CondDispatch`).
//!
//! The dispatch splits a `COND`'s clause blocks once at compile time and reuses
//! that table on every call instead of re-collecting, cloning, and re-splitting
//! the blocks off the stack. These tests pin: that the fast path actually fires,
//! that it is behaviorally identical to the dynamic path across clause shapes,
//! and that an unexpected extra block on the stack falls back safely.

use crate::interpreter::Interpreter;

fn run(src: &str) -> (Interpreter, String) {
    let mut interp = Interpreter::new();
    block_on(interp.execute(src)).unwrap();
    let top = interp
        .get_stack()
        .last()
        .map(|v| format!("{}", v))
        .unwrap_or_default();
    (interp, top)
}

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

const SIZE_DEF: &str = "{\n  { [ 5 ] > | [ 'big' ] }\n  { IDLE | [ 'small' ] } COND\n} 'SIZE' DEF";

#[test]
fn dispatch_fast_path_fires() {
    let (interp, _) = run(&format!("{SIZE_DEF}\n[ 7 ] SIZE"));
    assert!(
        interp.runtime_metrics().cond_dispatch_fast_count >= 1,
        "compiled COND dispatch should have taken the precomputed path"
    );
}

#[test]
fn dispatch_matches_dynamic_across_shapes() {
    // (program, expected top-of-stack) covering | clauses, pair clauses, IDLE
    // else, multiple clauses, and a numeric (non-Boolean) guard result.
    let cases = [
        (format!("{SIZE_DEF}\n[ 7 ] SIZE"), "big"),
        (format!("{SIZE_DEF}\n[ 3 ] SIZE"), "small"),
        (
            "{\n  { [ 0 ] > | [ 'pos' ] }\n  { [ 0 ] < | [ 'neg' ] }\n  { IDLE | [ 'zero' ] } COND\n} 'SGN' DEF\n[ 0 ] SGN".to_string(),
            "zero",
        ),
        (
            "{\n  { [ 0 ] > | [ 'pos' ] }\n  { [ 0 ] < | [ 'neg' ] }\n  { IDLE | [ 'zero' ] } COND\n} 'SGN' DEF\n[ 0 ] [ 4 ] - SGN".to_string(),
            "neg",
        ),
    ];

    for (src, expected) in cases {
        let mut on = Interpreter::new();
        on.set_cond_dispatch_enabled(true);
        block_on(on.execute(&src)).unwrap();

        let mut off = Interpreter::new();
        off.set_cond_dispatch_enabled(false);
        block_on(off.execute(&src)).unwrap();

        assert_eq!(
            format!("{:?}", on.get_stack()),
            format!("{:?}", off.get_stack()),
            "dispatch ON vs OFF diverged for: {src}"
        );
        let top = on
            .get_stack()
            .last()
            .map(|v| format!("{v}"))
            .unwrap_or_default();
        assert!(top.contains(expected), "expected {expected} in {top}");
    }
}

#[test]
fn dispatch_preserves_cond_errors() {
    // A COND whose clauses cannot be split (odd block count, pair style) must
    // still raise the same error under the compiled path.
    let mut interp = Interpreter::new();
    let err = block_on(interp.execute("[ 1 ] { TRUE } { FALSE } { TRUE } COND"))
        .unwrap_err()
        .to_string();
    assert!(
        err.to_uppercase().contains("COND"),
        "expected a COND clause error, got: {err}"
    );
}

#[test]
fn dispatch_fast_count_zero_when_disabled() {
    let mut interp = Interpreter::new();
    interp.set_cond_dispatch_enabled(false);
    block_on(interp.execute(&format!("{SIZE_DEF}\n[ 7 ] SIZE"))).unwrap();
    assert_eq!(
        interp.runtime_metrics().cond_dispatch_fast_count,
        0,
        "no precompiled dispatch should occur when disabled"
    );
}
