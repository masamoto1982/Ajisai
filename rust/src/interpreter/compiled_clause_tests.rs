//! Tests for compiled COND clause guard/body sub-plans.
//!
//! When a `COND` is lowered to `CondDispatch`, each clause's guard and body are
//! compiled into sub-plans and run via `execute_compiled_plan` instead of being
//! re-interpreted every iteration. These tests pin that the compiled path fires,
//! is byte-for-byte equivalent to the interpreted one, still trampolines a
//! guarded tail self-call inside a compiled body, and keeps the boundary on
//! unguarded recursion.

use crate::interpreter::Interpreter;

const COUNTDOWN: &str =
    "{\n  { [ 0 ] > | [ 1 ] - DOWN }\n  { IDLE | [ 'done' ] } COND\n} 'DOWN' DEF";

fn fresh() -> Interpreter {
    Interpreter::new()
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
