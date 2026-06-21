//! Firewall between the logical Unknown (U) and operational NIL in the
//! generic passthrough helpers (SPEC §7.5 / §2.3).
//!
//! `nil_passthrough_unary/value/binary` now key off `is_operational_nil()`
//! rather than `is_nil()` (which is a storage predicate that also matches
//! U). This stops U from being silently absorbed as an operational NIL when
//! it reaches an arithmetic/generic Coreword that uses those helpers, while
//! leaving genuine operational NIL passthrough intact.
//!
//! Observed behavior of `U 1 +` (recorded here per the task's request to
//! observe and report what U does when it reaches an arithmetic broadcast):
//!   - BEFORE the fix: `nil_passthrough_binary` matched U via `is_nil()` and
//!     absorbed it into an operational NIL node (`Ok(Nil)`, hint demoted
//!     `TruthValue` -> `Nil`). U was silently collapsed to an operational
//!     absence even though it is a logical truth value.
//!   - AFTER the fix: passthrough no longer matches U, so U reaches
//!     `op_add`'s broadcast guard (`apply_binary_broadcast_with_metrics`,
//!     left unchanged per task Step 3), which raises an explicit error
//!     "Cannot broadcast NIL values". U is NOT collapsed into a reasonless
//!     operational NIL.
//! Either an explicit error or preserved-U is acceptable per the acceptance
//! criteria; the firewall guarantee is that `NilReason::LogicallyUnknown`
//! is never silently turned into a reasonless operational NIL.

use crate::error::NilReason;
use crate::interpreter::Interpreter;
use crate::types::Value;

/// Source that yields the logical Unknown (U): ((√2+1) − (√2+1)) == 0 is
/// undecidable within the comparison budget, so EQ emits U with an
/// agreedPrefix. (Plain √2 − √2 now collapses to an exact 0 and decides.)
const PRODUCE_U: &str = "'math' IMPORT 2 SQRT 1 ADD 2 SQRT 1 ADD SUB 0 EQ";

async fn run(code: &str) -> Result<Vec<Value>, String> {
    let mut interp = Interpreter::new();
    interp.execute(code).await.map_err(|e| e.to_string())?;
    Ok(interp.get_stack().to_vec())
}

async fn run_ok(code: &str) -> Vec<Value> {
    run(code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` unexpectedly errored: {e}"))
}

/// Passing U to an arithmetic word must not silently collapse it into a
/// reasonless operational NIL. The firewall holds if EITHER an explicit
/// error is raised OR `is_unknown()` is preserved; what must never happen is
/// a successful result that is an operational NIL with the
/// `LogicallyUnknown` reason erased.
#[tokio::test]
async fn u_into_arithmetic_does_not_silently_collapse_to_reasonless_nil() {
    let res = run(&format!("{PRODUCE_U} 1 +")).await;
    match res {
        Err(_) => {
            // Acceptable: U reaches the broadcast guard and errors explicitly
            // (the guard is intentionally left unchanged in this task).
        }
        Ok(stack) => {
            assert_eq!(stack.len(), 1);
            let top = &stack[0];
            assert!(
                top.is_unknown(),
                "U passed to + must preserve is_unknown() if it does not \
                 error; got {top:?}"
            );
            assert!(
                !(top.is_nil() && !top.is_unknown()),
                "U must never collapse to a reasonless operational NIL; got {top:?}"
            );
        }
    }
}

/// A genuine operational NIL with a reason still passes through arithmetic
/// unchanged (no regression): the reason is preserved and the result is an
/// operational NIL (not U).
#[tokio::test]
async fn operational_nil_passthrough_preserves_reason() {
    // `1 0 DIV` is a recoverable DivisionByZero Bubble (operational NIL).
    let nil_src = "1 0 DIV";
    let stack = run_ok(&format!("{nil_src} 1 +")).await;
    assert_eq!(stack.len(), 1);
    let top = &stack[0];
    assert!(
        top.is_operational_nil(),
        "NIL 1 + must stay an operational NIL, got {top:?}"
    );
    assert_eq!(
        top.nil_reason().cloned(),
        Some(NilReason::DivisionByZero),
        "operational NIL must keep its reason through arithmetic passthrough, got {top:?}"
    );
}

/// The K3 logic path (`logic_kleene`) is unaffected: `U U AND` is U.
/// (Ajisai has no `DUP`; two independently-produced U operands stand in for
/// the task's `U DUP AND`, exercising the same `Unknown AND Unknown` cell.)
#[tokio::test]
async fn u_and_u_is_unknown() {
    let stack = run_ok(&format!("{PRODUCE_U} {PRODUCE_U} AND")).await;
    assert_eq!(stack.len(), 1);
    assert!(
        stack[0].is_unknown(),
        "U U AND must remain Unknown, got {:?}",
        stack[0]
    );
}
