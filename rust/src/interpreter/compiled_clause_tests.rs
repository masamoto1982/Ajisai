//! Tests for compiled COND clause guard/body sub-plans.
//!
//! When a `COND` is lowered to `CondDispatch`, each clause's guard and body are
//! compiled into sub-plans and run via `execute_compiled_plan` instead of being
//! re-interpreted every iteration. These tests pin that the compiled path fires,
//! is byte-for-byte equivalent to the interpreted one, still trampolines a
//! guarded tail self-call inside a compiled body, and keeps the boundary on
//! unguarded recursion.

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

const COUNTDOWN: &str =
    "{\n  { [ 0 ] > | [ 1 ] - DOWN }\n  { IDLE | [ 'done' ] } COND\n} 'DOWN' DEF";

fn fresh() -> Interpreter {
    Interpreter::new()
}

fn assert_on_equals_off(src: &str) {
    let mut on = fresh();
    on.set_compiled_clause_enabled(true);
    block_on(on.execute(src)).unwrap();

    let mut off = fresh();
    off.set_compiled_clause_enabled(false);
    block_on(off.execute(src)).unwrap();

    assert_eq!(
        format!("{:?}", on.get_stack()),
        format!("{:?}", off.get_stack()),
        "compiled-clause ON vs OFF diverged for: {src}"
    );
}

#[tokio::test]
async fn compiled_clause_path_fires() {
    // Both the guard (`[ 0 ] >`) and body (`[ 1 ] - DOWN`) compile, so each
    // iteration runs at least one compiled sub-plan.
    let mut interp = fresh();
    interp.execute(COUNTDOWN).await.unwrap();
    interp.execute("[ 20 ] DOWN").await.unwrap();
    assert!(
        interp.runtime_metrics().cond_clause_compiled_count >= 20,
        "expected compiled clause executions, got {}",
        interp.runtime_metrics().cond_clause_compiled_count
    );
}

#[tokio::test]
async fn compiled_body_still_trampolines_past_depth_limit() {
    // The tail self-call now sits in a *compiled* clause body; the compiled
    // executor must defer it so the loop still runs past MAX_USER_WORD_DEPTH.
    let mut interp = fresh();
    interp.execute(COUNTDOWN).await.unwrap();
    let result = interp.execute("[ 3000 ] DOWN").await;
    assert!(
        result.is_ok(),
        "compiled clause body should still trampoline: {result:?}"
    );
    assert_eq!(interp.call_depth, 0, "call_depth must unwind to 0");
}

#[tokio::test]
async fn compiled_clause_disabled_count_is_zero() {
    let mut interp = fresh();
    interp.set_compiled_clause_enabled(false);
    interp.execute(COUNTDOWN).await.unwrap();
    interp.execute("[ 20 ] DOWN").await.unwrap();
    assert_eq!(
        interp.runtime_metrics().cond_clause_compiled_count,
        0,
        "no compiled clause executions expected when disabled"
    );
}

#[test]
fn compiled_clause_matches_interpreted_across_shapes() {
    assert_on_equals_off(&format!("{COUNTDOWN}\n[ 7 ] DOWN"));
    assert_on_equals_off(
        "{\n  { [ 5 ] > | [ 'big' ] }\n  { IDLE | [ 'small' ] } COND\n} 'SIZE' DEF\n[ 7 ] SIZE",
    );
    // Multi-clause with numeric guard fall-through and arithmetic bodies.
    assert_on_equals_off(
        "{\n  { [ 0 ] > | [ 10 ] + }\n  { [ 0 ] < | [ 10 ] - }\n  { IDLE | [ 0 ] } COND\n} 'ADJ' DEF\n[ 3 ] ADJ",
    );
    assert_on_equals_off(
        "{\n  { [ 0 ] > | [ 10 ] + }\n  { [ 0 ] < | [ 10 ] - }\n  { IDLE | [ 0 ] } COND\n} 'ADJ' DEF\n[ 0 ] [ 4 ] - ADJ",
    );
}

#[tokio::test]
async fn unguarded_recursion_unaffected_by_compiled_clauses() {
    // `{ REC }` has no COND, so no compiled clause is involved; it must keep the
    // native recursion-depth error rather than trampolining.
    let mut interp = fresh();
    interp.execute("{ REC } 'REC' DEF").await.unwrap();
    let err = interp.execute("REC").await.unwrap_err().to_string();
    assert!(
        err.contains("recursion limit exceeded"),
        "bare recursion must stay depth-limited: {err}"
    );
}
