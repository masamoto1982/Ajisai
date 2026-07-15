//! Tests for internal tail-call elimination (the "internal GOTO" trampoline).
//!
//! A guarded tail self-call — a recursive call in the tail position of a `COND`
//! clause body — is run as an internal backward jump rather than a native
//! recursive call. These tests pin the observable consequences:
//!   * such loops run in O(1) native stack, far past `MAX_USER_WORD_DEPTH`;
//!   * value results are identical to the legacy recursion path;
//!   * non-tail and unguarded recursion keep their existing depth-limit error;
//!   * an unbounded guarded loop still terminates via the step budget;
//!   * the optimization can be switched off for an A/B comparison.

use crate::interpreter::Interpreter;

const COUNTDOWN_DEF: &str =
    "{\n  { [ 0 ] > | [ 1 ] - DOWN }\n  { IDLE | [ 'done' ] } COND\n} 'DOWN' DEF";

fn fresh() -> Interpreter {
    Interpreter::new()
}

fn top_string(interp: &Interpreter) -> String {
    interp
        .get_stack()
        .last()
        .map(|v| format!("{}", v))
        .unwrap_or_default()
}

#[tokio::test]
async fn guarded_tail_recursion_exceeds_native_depth_limit() {
    // 2000 is far beyond MAX_USER_WORD_DEPTH (256); without the trampoline this
    // is a "recursion limit exceeded" error.
    let mut interp = fresh();
    interp.execute(COUNTDOWN_DEF).await.unwrap();
    let result = interp.execute("[ 2000 ] DOWN").await;
    assert!(
        result.is_ok(),
        "guarded tail recursion should trampoline past the depth guard: {result:?}"
    );
    let top = top_string(&interp);
    assert!(top.contains("done"), "unexpected result: {top}");
    // The native call stack never grew: one entry, unwound back to zero.
    assert_eq!(interp.call_depth, 0, "call_depth must unwind to 0");
}

#[tokio::test]
async fn trampoline_matches_value_of_legacy_recursion() {
    // At a depth both paths can run, ON and OFF must produce identical stacks.
    // OFF is kept shallow here: native COND recursion is heavy per frame and a
    // unit-test thread has a small stack. The deep contrast is exercised on a
    // large-stack thread in `deep_ab_off_hits_limit_on_succeeds`.
    for depth in [1u32, 2, 7, 40] {
        let line = format!("[ {depth} ] DOWN");

        let mut on = fresh();
        on.execute(COUNTDOWN_DEF).await.unwrap();
        on.execute(&line).await.unwrap();

        let mut off = fresh();
        off.set_tail_call_enabled(false);
        off.execute(COUNTDOWN_DEF).await.unwrap();
        off.execute(&line).await.unwrap();

        assert_eq!(
            format!("{:?}", on.get_stack()),
            format!("{:?}", off.get_stack()),
            "ON and OFF diverged at depth {depth}"
        );
    }
}

// Minimal future driver so this test can run on a thread with a large stack
// (native COND recursion at depth 2000 would otherwise blow the small default
// test-thread stack before the depth guard catches it).
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

#[test]
fn deep_ab_off_hits_limit_on_succeeds() {
    // 64 MiB stack so the OFF path reaches the recursion-depth guard and returns
    // a recoverable error rather than aborting the process.
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            let deep = "[ 2000 ] DOWN";

            let mut on = fresh();
            block_on(on.execute(COUNTDOWN_DEF)).unwrap();
            assert!(
                block_on(on.execute(deep)).is_ok(),
                "ON should complete depth 2000 in O(1) native stack"
            );

            let mut off = fresh();
            off.set_tail_call_enabled(false);
            block_on(off.execute(COUNTDOWN_DEF)).unwrap();
            let err = block_on(off.execute(deep)).unwrap_err().to_string();
            assert!(
                err.contains("recursion limit exceeded"),
                "OFF should hit the native depth guard: {err}"
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

#[tokio::test]
async fn trampoline_records_backward_jumps() {
    let mut interp = fresh();
    interp.execute(COUNTDOWN_DEF).await.unwrap();
    interp.execute("[ 50 ] DOWN").await.unwrap();
    assert!(
        interp.runtime_metrics().tail_call_jump_count >= 50,
        "expected >= 50 backward jumps, got {}",
        interp.runtime_metrics().tail_call_jump_count
    );
}

#[tokio::test]
async fn unguarded_self_recursion_still_hits_depth_limit() {
    // `{ REC }` has no base case and no COND guard: it is deliberately NOT
    // trampolined, so it must surface the native recursion-depth error rather
    // than spin forever or trap. This pins the boundary of the optimization.
    let mut interp = fresh();
    interp.execute("{ REC } 'REC' DEF").await.unwrap();
    let err = interp.execute("REC").await.unwrap_err();
    // SPEC §11.1: the depth guard has its own user-level category — it must
    // not surface as a stringly-typed Custom error.
    assert_eq!(
        crate::error::ErrorCategory::from_error(&err),
        crate::error::ErrorCategory::RecursionLimitExceeded,
        "depth guard must carry the RecursionLimitExceeded category: {err}"
    );
    let err = err.to_string();
    assert!(
        err.contains("recursion limit exceeded"),
        "bare self-recursion must stay on the depth-limited path: {err}"
    );
    assert_eq!(interp.call_depth, 0, "call_depth must unwind to 0");
}

#[tokio::test]
async fn unbounded_guarded_loop_terminates_via_step_budget() {
    // A guarded tail loop with no reachable base case trampolines forever in
    // O(1) stack, so termination must come from the execution step budget
    // (water level), not a stack overflow.
    let mut interp = fresh();
    interp.set_max_execution_steps(5_000);
    // Guard `[ 0 ] >=` on a value that never reaches the base: count upward.
    interp
        .execute(
            "{\n  { [ 0 ] >= | [ 1 ] + LOOPUP }\n  { IDLE | [ 'never' ] } COND\n} 'LOOPUP' DEF",
        )
        .await
        .unwrap();
    let err = interp
        .execute("[ 1 ] LOOPUP")
        .await
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("step limit") || err.contains("Execution step limit"),
        "unbounded guarded loop should hit the step budget: {err}"
    );
    assert_eq!(interp.call_depth, 0, "call_depth must unwind to 0");
}
