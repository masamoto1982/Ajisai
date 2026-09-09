//! Tests for compiled COND clause guard/body sub-plans.
//!
//! When a `COND` is lowered to `CondDispatch`, each clause's guard and body are
//! compiled into sub-plans and run via `execute_compiled_plan` instead of being
//! re-interpreted every iteration. These tests pin that the compiled path fires
//! and is byte-for-byte equivalent to the interpreted one.
//!
//! Two tests formerly here — one pinning that a guarded tail self-call inside
//! a compiled clause body still trampolined past `MAX_USER_WORD_DEPTH`, one
//! pinning that bare (unguarded) recursion still hit the native
//! recursion-depth error — were removed along with the feature and the
//! boundary they distinguished: SPEC §8.7's DEF-time acyclicity check now
//! refuses any self-referential definition outright, so neither a
//! trampolined nor an unguarded recursive word can be defined any more.

use crate::interpreter::Interpreter;

// `STEP` has one COND with a guard (`0 >`) and a body (`[ 1 ] -`) that both
// compile, and no self-call — `MAP` supplies the repetition instead of
// recursion, so applying it to 20 elements runs 20 compiled clause
// dispatches.
const STEP: &str = "[ [ [ 0 > | [ 1 ] - ] [ IDLE | [ 0 ] ] ] COND ] 'STEP' DEF";
const RUN_STEP_OVER_20: &str = "[ 1 20 ] RANGE [ STEP ] MAP";

fn fresh() -> Interpreter {
    Interpreter::new()
}

#[tokio::test]
async fn compiled_clause_path_fires() {
    let mut interp = fresh();
    interp.execute(STEP).await.unwrap();
    interp.execute(RUN_STEP_OVER_20).await.unwrap();
    assert!(
        interp.runtime_metrics().cond_clause_compiled_count >= 20,
        "expected compiled clause executions, got {}",
        interp.runtime_metrics().cond_clause_compiled_count
    );
}

/// Compiled-path witness (docs/dev/auditable-kernel-work-order-2026-09.md
/// Phase 3, §3.5 / §3.4 pitfall A): the same scalar-guard refusal
/// `control_cond_tests::test_cond_scalar_guard_is_not_truthy` pins on the
/// interpreted path, exercised through a compiled clause's guard sub-plan —
/// `evaluate_guard_isolated` is shared by both entry points, but the pitfall
/// this file's own header warns against is certifying a fix from one path
/// alone.
#[tokio::test]
async fn compiled_guard_scalar_result_is_not_truthy() {
    let mut interp = fresh();
    interp
        .execute("[ [ [ 1 | 'x' ] [ IDLE | 'y' ] ] COND ] 'BADCOND' DEF")
        .await
        .unwrap();
    let result = interp.execute("5 BADCOND").await;
    assert!(
        result.is_err(),
        "a compiled guard reducing to a bare scalar should be refused"
    );
    let message = result.err().unwrap().to_string();
    assert!(
        message.contains("COND: guard must return TRUE or FALSE"),
        "unexpected error: {message}"
    );
    assert!(
        interp.runtime_metrics().cond_clause_compiled_count >= 1,
        "the guard must actually have run through the compiled sub-plan, not just the interpreted fallback"
    );
}

#[tokio::test]
async fn compiled_clause_disabled_count_is_zero() {
    let mut interp = fresh();
    interp.set_compiled_clause_enabled(false);
    interp.execute(STEP).await.unwrap();
    interp.execute(RUN_STEP_OVER_20).await.unwrap();
    assert_eq!(
        interp.runtime_metrics().cond_clause_compiled_count,
        0,
        "no compiled clause executions expected when disabled"
    );
}
