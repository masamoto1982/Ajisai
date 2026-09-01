//! CS2: `OR-NIL` behavior pinning.
//!
//! SPEC §6.4: `OR-NIL` is a *lazy* NIL-coalescing control directive — if the
//! stack top is non-NIL it is kept and the following source unit is skipped
//! unevaluated; if it is NIL the top is discarded and the following unit is
//! evaluated as the fallback. It has no symbol or legacy-name sugar: `^` and
//! `VENT` (the former canonical spelling) are ordinary, unrecognized names.
//!
//! These tests lock the canonical spelling's behavior across the non-NIL
//! keep/skip, the lazy (unevaluated) fallback, the NIL fallback, balanced
//! vector/block group skips, nesting, stack underflow, and case folding.

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
async fn or_nil_keeps_a_non_nil_top() {
    // Mirrors conformance `core-or-nil-nonnil-keeps-top`: 5 OR-NIL 99 -> 5/1.
    assert_eq!(display(&run_ok("5 OR-NIL 99").await), "5/1");
    // Case folding: the canonical name is not case-sensitive.
    assert_eq!(run_ok("5 or-nil 99").await, run_ok("5 OR-NIL 99").await);
}

#[tokio::test]
async fn or_nil_does_not_evaluate_the_fallback_when_the_top_is_non_nil() {
    // Mirrors `core-or-nil-nonnil-fallback-unevaluated`: the fallback is skipped
    // *unevaluated*, so an undefined word there must not raise. This is the
    // proof that OR-NIL takes the lazy path, not a strict stack-consuming
    // word (which would try to resolve UNDEFINED-FALLBACK).
    assert_eq!(display(&run_ok("5 OR-NIL UNDEFINED-FALLBACK").await), "5/1");
}

#[tokio::test]
async fn or_nil_skips_exactly_one_source_token() {
    // Mirrors `core-or-nil-one-token-skip-trap`: 1 OR-NIL 2 3 ADD -> 4/1 (only
    // the single token `2` is skipped, then `3 ADD` runs on the kept `1`).
    assert_eq!(display(&run_ok("1 OR-NIL 2 3 ADD").await), "4/1");
}

// --- NIL top: discard it, evaluate the following unit as the fallback -----

#[tokio::test]
async fn or_nil_evaluates_the_fallback_on_a_nil_top() {
    assert_eq!(display(&run_ok("NIL OR-NIL 99").await), "99/1");
}

// --- balanced group skip (vector, block, nested) --------------------------

#[tokio::test]
async fn or_nil_skips_balanced_vector_group() {
    // Mirrors `core-or-nil-group-skip-atomic`: the whole `[ ... ]` is one unit.
    assert_eq!(display(&run_ok("1 OR-NIL [ 2 3 ADD ]").await), "1/1");
    // A trailing unit after the skipped group still runs.
    assert_eq!(display(&run_ok("1 OR-NIL [ 2 3 ] 4").await), "1/1 4/1");
}

#[tokio::test]
async fn or_nil_skips_nested_group_atomically() {
    assert_eq!(display(&run_ok("1 OR-NIL [ [ 2 ] 3 ]").await), "1/1");
}

// --- stack underflow --------------------------------------------------------

#[tokio::test]
async fn or_nil_errors_on_stack_underflow() {
    assert!(run("OR-NIL 99").await.is_err());
}

// --- ^ and VENT are ordinary, unrecognized names now -----------------------

#[tokio::test]
async fn caret_and_legacy_vent_no_longer_coalesce_nil() {
    // Neither spelling resolves any more: both are unknown-word errors.
    assert!(run("5 ^ 99").await.is_err());
    assert!(run("5 VENT 99").await.is_err());
}

// --- machine-readable contract (§7.14 metadata) ---------------------------

#[test]
fn or_nil_contract_is_lazy_not_eager_binary() {
    let or_nil = lookup_builtin_spec("OR-NIL").expect("OR-NIL spec");
    // The typed classification, not just the prose, marks OR-NIL as lazy.
    assert_eq!(or_nil.execution_form, ExecutionForm::LazyNextUnitFallback);
    // OR-NIL is realised as a control token, not dispatched by name: its
    // declared arity is `control` on both sides, which is what says so.
    let declared = crate::kernel::generated::generated_word("OR-NIL").expect("OR-NIL is declared");
    assert_eq!(
        declared.stack_inputs,
        crate::kernel::generated::Arity::Control
    );
    assert_eq!(
        declared.stack_outputs,
        crate::kernel::generated::Arity::Control
    );
    // The stack-effect prose must not describe the old eager `[a] [b]` binary.
    assert!(
        !or_nil.stack_effect.contains("[a] [b]"),
        "OR-NIL stack_effect must not describe an eager two-operand pop: {:?}",
        or_nil.stack_effect
    );
    // Mass is data-dependent, never a fixed two-in/one-out contract.
    assert!(crate::coreword_registry::mass_contract("OR-NIL")
        .fixed()
        .is_none());
}
