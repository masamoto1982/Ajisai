//! CS2: `VENT`/`FLOW` canonical-name ↔ sugar unification.
//!
//! SPEC §6.4: `VENT` (sugar `^`) is a *lazy* NIL-coalescing control directive —
//! if the stack top is non-NIL it is kept and the following source unit is
//! skipped unevaluated; if it is NIL the top is discarded and the following unit
//! is evaluated as the fallback. `FLOW` (sugar `~`) is a no-op pipeline marker.
//!
//! The canonical spelled-out name and its sugar must produce the *same* token
//! and therefore the *same* execution result. Before this change the sugar
//! tokenized to a dedicated control token while the spelled-out name fell
//! through to a builtin with no executor and raised `UnknownWord`. These tests
//! lock the two spellings together across the non-NIL keep/skip, the lazy
//! (unevaluated) fallback, the NIL fallback, balanced vector/block group skips,
//! nesting, stack underflow, and the `FLOW` no-op.

use crate::builtins::lookup_builtin_spec;
use crate::coreword_registry::ExecutionForm;
use crate::interpreter::Interpreter;
use crate::types::Value;

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

fn display(stack: &[Value]) -> String {
    stack
        .iter()
        .map(|v| format!("{v}"))
        .collect::<Vec<_>>()
        .join(" ")
}

// --- non-NIL top: keep it, skip the following unit ------------------------

#[tokio::test]
async fn vent_canonical_matches_sugar_non_nil_keep() {
    // Mirrors conformance `core-vent-nonnil-keeps-top`: 5 ^ 99 -> 5/1.
    assert_eq!(display(&run_ok("5 ^ 99").await), "5/1");
    assert_eq!(display(&run_ok("5 VENT 99").await), "5/1");
    assert_eq!(run_ok("5 ^ 99").await, run_ok("5 VENT 99").await);
    // Case folding: the canonical name is not case-sensitive.
    assert_eq!(run_ok("5 vent 99").await, run_ok("5 VENT 99").await);
}

#[tokio::test]
async fn vent_canonical_matches_sugar_fallback_unevaluated() {
    // Mirrors `core-vent-nonnil-fallback-unevaluated`: the fallback is skipped
    // *unevaluated*, so an undefined word there must not raise. This is the
    // proof that the spelled-out name takes the lazy path, not a strict
    // stack-consuming word (which would try to resolve UNDEFINED-FALLBACK).
    assert_eq!(display(&run_ok("5 ^ UNDEFINED-FALLBACK").await), "5/1");
    assert_eq!(display(&run_ok("5 VENT UNDEFINED-FALLBACK").await), "5/1");
}

#[tokio::test]
async fn vent_canonical_matches_sugar_one_token_skip() {
    // Mirrors `core-vent-one-token-skip-trap`: 1 ^ 2 3 ADD -> 4/1 (only the
    // single token `2` is skipped, then `3 ADD` runs on the kept `1`).
    assert_eq!(display(&run_ok("1 ^ 2 3 ADD").await), "4/1");
    assert_eq!(display(&run_ok("1 VENT 2 3 ADD").await), "4/1");
}

// --- NIL top: discard it, evaluate the following unit as the fallback -----

#[tokio::test]
async fn vent_canonical_matches_sugar_nil_fallback() {
    assert_eq!(display(&run_ok("NIL ^ 99").await), "99/1");
    assert_eq!(display(&run_ok("NIL VENT 99").await), "99/1");
    assert_eq!(run_ok("NIL ^ 99").await, run_ok("NIL VENT 99").await);
}

// --- balanced group skip (vector, block, nested) --------------------------

#[tokio::test]
async fn vent_canonical_skips_balanced_vector_group() {
    // Mirrors `core-vent-group-skip-atomic`: the whole `[ ... ]` is one unit.
    assert_eq!(display(&run_ok("1 ^ [ 2 3 ADD ]").await), "1/1");
    assert_eq!(display(&run_ok("1 VENT [ 2 3 ADD ]").await), "1/1");
    // A trailing unit after the skipped group still runs.
    assert_eq!(display(&run_ok("1 ^ [ 2 3 ] 4").await), "1/1 4/1");
    assert_eq!(display(&run_ok("1 VENT [ 2 3 ] 4").await), "1/1 4/1");
}

#[tokio::test]
async fn vent_canonical_skips_balanced_block_group() {
    assert_eq!(display(&run_ok("1 ^ { 2 3 ADD }").await), "1/1");
    assert_eq!(display(&run_ok("1 VENT { 2 3 ADD }").await), "1/1");
}

#[tokio::test]
async fn vent_canonical_skips_nested_group_atomically() {
    assert_eq!(display(&run_ok("1 ^ [ [ 2 ] 3 ]").await), "1/1");
    assert_eq!(display(&run_ok("1 VENT [ [ 2 ] 3 ]").await), "1/1");
}

// --- stack underflow: both spellings error identically --------------------

#[tokio::test]
async fn vent_canonical_matches_sugar_stack_underflow() {
    assert!(run("^ 99").await.is_err());
    assert!(run("VENT 99").await.is_err());
}

// --- machine-readable contract (§7.14 metadata) ---------------------------

#[test]
fn vent_contract_is_lazy_not_eager_binary() {
    let vent = lookup_builtin_spec("VENT").expect("VENT spec");
    // The typed classification, not just the prose, marks VENT as lazy.
    assert_eq!(vent.execution_form, ExecutionForm::LazyNextUnitFallback);
    // VENT has no executor: it is realised as a control token, not dispatched.
    assert!(vent.executor_key.is_none());
    // The stack-effect prose must not describe the old eager `[a] [b]` binary.
    assert!(
        !vent.stack_effect.contains("[a] [b]"),
        "VENT stack_effect must not describe an eager two-operand pop: {:?}",
        vent.stack_effect
    );
    // Mass is data-dependent, never a fixed two-in/one-out contract.
    assert!(vent.mass.fixed().is_none());
}

#[test]
fn flow_contract_is_noop_control_directive() {
    let flow = lookup_builtin_spec("FLOW").expect("FLOW spec");
    assert_eq!(flow.execution_form, ExecutionForm::NoOpControlDirective);
    assert!(flow.executor_key.is_none());
}

// --- FLOW (~) is a no-op pipeline marker in both spellings -----------------

#[tokio::test]
async fn flow_canonical_matches_sugar_noop() {
    // Mirrors conformance `1 2 ~ ADD` -> 3/1.
    assert_eq!(display(&run_ok("1 2 ~ ADD").await), "3/1");
    assert_eq!(display(&run_ok("1 2 FLOW ADD").await), "3/1");
    assert_eq!(run_ok("1 2 ~ ADD").await, run_ok("1 2 FLOW ADD").await);
    assert_eq!(run_ok("1 2 flow ADD").await, run_ok("1 2 FLOW ADD").await);
}
