//! Shared helpers for the crate's in-tree behavioral probes.
//!
//! Every `*_tests` module used to carry its own copy of "run this source on a
//! fresh interpreter and read the answer" — one decision spelled a dozen
//! ways. It is written once here. A helper with a genuinely different
//! contract (a rendered rather than displayed stack, a different work
//! counter, a `Result` the probe inspects itself) stays with the tests that
//! need it.

use crate::error::{ErrorCategory, NilReason};
use crate::interpreter::debug_diagnosis::DebugDiagnosis;
use crate::interpreter::{Interpreter, RuntimeLimits};
use crate::types::{Value, ValueData};

/// A fresh interpreter with `limits` in force.
pub(crate) fn with_limits(limits: RuntimeLimits) -> Interpreter {
    let mut interp = Interpreter::new();
    interp.set_runtime_limits(limits);
    interp
}

/// Run `code` on a fresh interpreter; a failure to run is the probe's own bug.
pub(crate) async fn run(code: &str) -> Interpreter {
    let mut interp = Interpreter::new();
    interp
        .execute(code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` must not error: {e}"));
    interp
}

/// [`run`], with the materialization ceiling raised to ten million elements
/// so a probe over a wide collection is not refused by the host safety
/// control before it reaches the Word under test.
pub(crate) async fn run_with_wide_materialization(code: &str) -> Interpreter {
    let mut interp = Interpreter::new();
    let mut limits = *interp.runtime_limits();
    limits.max_materialized_elements = 10_000_000;
    interp.set_runtime_limits(limits);
    interp
        .execute(code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` must compute, got: {e:?}"));
    interp
}

/// The stack `code` leaves, bottom to top.
pub(crate) async fn run_ok(code: &str) -> Vec<Value> {
    run(code).await.get_stack().to_vec()
}

/// The value `code` leaves on top.
pub(crate) async fn top_of(code: &str) -> Value {
    run(code)
        .await
        .stack
        .last()
        .cloned()
        .expect("an answer was pushed")
}

/// The whole stack `code` leaves, each value displayed, space-joined bottom
/// to top — the form most Word probes compare against.
pub(crate) async fn top(code: &str) -> String {
    run(code)
        .await
        .get_stack()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The value `code` leaves on top, displayed; `<empty>` for an empty stack.
pub(crate) async fn answer(code: &str) -> String {
    run(code)
        .await
        .get_stack()
        .last()
        .map(|v| format!("{v}"))
        .unwrap_or_else(|| "<empty>".to_string())
}

/// The protocol id of the error category `code` raises.
pub(crate) async fn error_of(code: &str) -> String {
    let mut interp = Interpreter::new();
    let err = interp.execute(code).await.expect_err("must raise an ERROR");
    ErrorCategory::from_error(&err)
        .expect("a program ERROR has a category")
        .as_protocol_str()
        .to_string()
}

/// The protocol id of the reason on the NIL `code` projects, or `None` when
/// the NIL carries no reason. Anything but a NIL is the probe's own bug.
pub(crate) async fn reason(code: &str) -> Option<String> {
    let answer = top_of(code).await;
    assert!(answer.is_nil(), "`{code}` must project NIL, got {answer:?}");
    answer
        .nil_reason()
        .map(|reason| reason.as_protocol_str().to_string())
}

/// The reason on the NIL at the top of `interp`'s stack, if the top is a
/// reasoned NIL.
pub(crate) fn top_nil_reason(interp: &Interpreter) -> Option<NilReason> {
    interp
        .get_stack()
        .last()
        .and_then(|v| v.nil_reason().cloned())
}

/// Numeric work charged by running `code`.
pub(crate) async fn charged_by(code: &str) -> u64 {
    run(code).await.numeric_work_used()
}

/// The truth value at the bottom of `interp`'s stack, read off its display.
pub(crate) fn bool_of(interp: &Interpreter) -> bool {
    let v = &interp.get_stack()[0];
    let s = format!("{}", v);
    match s.as_str() {
        "1" | "1/1" | "TRUE" => true,
        "0" | "0/1" | "FALSE" => false,
        other => panic!("expected boolean (0 or 1), got {}", other),
    }
}

/// Whether the top value `code` leaves is a dense `Tensor`.
pub(crate) async fn top_is_dense(code: &str) -> bool {
    let interp = run_with_wide_materialization(code).await;
    matches!(
        interp.get_stack().as_slice().last().map(|v| &v.data),
        Some(ValueData::Tensor { .. })
    )
}

/// What `left right EQ` answers, or `None` when it answers a non-truth value.
pub(crate) async fn equals(left: &str, right: &str) -> Option<bool> {
    let interp = run_with_wide_materialization(&format!("{left} {right} EQ")).await;
    interp.get_stack().as_slice().last()?.as_truth()
}

/// The diagnosis of the failure `source` must end in.
pub(crate) async fn diagnose(source: &str) -> DebugDiagnosis {
    let mut interp = Interpreter::new();
    assert!(
        interp.execute(source).await.is_err(),
        "`{source}` must fail"
    );
    interp
        .drain_error_flow_trace()
        .iter()
        .rev()
        .find_map(|event| event.diagnosis.clone())
        .expect("a failed run records a diagnosis")
}
