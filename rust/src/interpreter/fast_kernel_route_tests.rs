//! Differential coverage for the specialized HOF kernels
//! (`higher_order/fast_kernels.rs`), on the shared route-equivalence
//! harness (`route_equivalence.rs`). The kernels are routing state only:
//! with them enabled or disabled, the execution outcome — Ok stack or error,
//! rendered forms, hints, and NIL reasons — must be identical for every
//! program. The division/modulo-by-zero cases pin the Bubble Rule: a kernel
//! must decline such input so the generic route produces the same NIL
//! bubbles it would produce alone, never a route-specific error.

use super::route_equivalence::{assert_configs_equal, observe, Observation};
use crate::error::NilReason;
use crate::interpreter::Interpreter;

fn observe_kernels(lines: &[&str], enabled: bool) -> (Interpreter, Observation) {
    observe(|i| i.set_fast_kernel_enabled(enabled), lines)
}

/// Run `lines` with the fast kernels enabled and disabled and assert the two
/// observations are identical. Returns the kernel-enabled interpreter for
/// follow-up metric assertions.
fn assert_kernel_on_equals_off(lines: &[&str]) -> Interpreter {
    let (on, _off) = assert_configs_equal(
        "fast kernel",
        |i| i.set_fast_kernel_enabled(true),
        |i| i.set_fast_kernel_enabled(false),
        lines,
    );
    on
}

// ── Bubble Rule: zero divisors/moduli must bubble, never error ────────────

#[test]
fn map_division_by_zero_constant_bubbles_on_both_routes() {
    let interp = assert_kernel_on_equals_off(&["[ 1 2 3 ] { [ 0 ] / } MAP"]);
    let stack = interp.get_stack();
    assert_eq!(stack.len(), 1);
    let result = stack.last().unwrap();
    assert_eq!(result.len(), 3, "MAP must keep the element count");
    for i in 0..3 {
        let elem = result.child(i).expect("child index in 0..3 must be valid");
        assert!(elem.is_nil(), "element {i} must be a NIL bubble");
        assert_eq!(
            elem.nil_reason(),
            Some(&NilReason::DivisionByZero),
            "element {i} must carry the generic route's reason"
        );
    }
}

#[test]
fn map_modulo_by_zero_constant_matches_generic_route() {
    // The generic executor currently raises "Modulo by zero" for a zero
    // modulus (tensor_cmds::op_mod); whatever the canonical outcome, the
    // kernel route must produce exactly the same one.
    assert_kernel_on_equals_off(&["[ 1 2 3 ] { [ 0 ] % } MAP"]);
}

#[test]
fn fold_division_by_zero_element_matches_generic_route() {
    // The zero sits in the data, not the block, so the bulk kernel must
    // decline up front and the per-element kernel must decline at the zero.
    assert_kernel_on_equals_off(&["[ 2 0 4 ] [ 10 ] { / } FOLD"]);
}

#[test]
fn fold_modulo_by_zero_element_matches_generic_route() {
    assert_kernel_on_equals_off(&["[ 3 0 ] [ 10 ] { % } FOLD"]);
}

#[test]
fn scalar_division_by_zero_baseline_is_a_bubble() {
    // The reference outcome the kernels must reproduce: DIV by zero is a
    // NIL bubble with reason divisionByZero (SPEC Bubble Rule), not an error.
    let (interp, obs) = observe_kernels(&["[ 1 ] [ 0 ] /"], true);
    assert_eq!(obs.outcome, Ok(()));
    assert_eq!(
        interp.get_stack().last().and_then(|v| v.nil_reason()),
        Some(&NilReason::DivisionByZero)
    );
}

// ── Route equivalence on kernel-eligible happy paths ──────────────────────

#[test]
fn bulk_map_kernel_engages_and_matches_generic_route() {
    let interp = assert_kernel_on_equals_off(&["[ 1 2 3 ] { [ 2 ] * } MAP"]);
    assert!(
        interp.runtime_metrics().vtu_bulk_kernel_use_count >= 1,
        "the bulk kernel must engage for a 1-D dense multiply-by-constant MAP"
    );
    assert_eq!(
        interp.get_stack().last().map(|v| format!("{v}")),
        Some("[ 2/1 4/1 6/1 ]".to_string())
    );
}

#[test]
fn bulk_map_division_by_nonzero_constant_still_uses_the_kernel() {
    let interp = assert_kernel_on_equals_off(&["[ 2 4 6 ] { [ 2 ] / } MAP"]);
    assert!(
        interp.runtime_metrics().vtu_bulk_kernel_use_count >= 1,
        "declining zero divisors must not disable nonzero divisions"
    );
    assert_eq!(
        interp.get_stack().last().map(|v| format!("{v}")),
        Some("[ 1/1 2/1 3/1 ]".to_string())
    );
}

#[test]
fn bulk_fold_kernel_engages_and_matches_generic_route() {
    let interp = assert_kernel_on_equals_off(&["[ 1 2 3 4 ] [ 0 ] { + } FOLD"]);
    assert!(
        interp.runtime_metrics().vtu_bulk_kernel_use_count >= 1,
        "the bulk kernel must engage for a 1-D dense + FOLD"
    );
}

#[test]
fn bulk_fold_division_without_zeros_still_uses_the_kernel() {
    let interp = assert_kernel_on_equals_off(&["[ 2 5 ] [ 100 ] { / } FOLD"]);
    assert!(
        interp.runtime_metrics().vtu_bulk_kernel_use_count >= 1,
        "the zero pre-scan must not decline zero-free divisions"
    );
}

#[test]
fn predicate_family_matches_generic_route() {
    assert_kernel_on_equals_off(&["[ 0 1 2 ] { [ 1 ] < } FILTER"]);
    assert_kernel_on_equals_off(&["[ 0 1 2 ] { [ 1 ] = } ANY"]);
    assert_kernel_on_equals_off(&["[ 1 1 1 ] { [ 1 ] = } ALL"]);
    assert_kernel_on_equals_off(&["[ 0 1 0 ] { NOT } COUNT"]);
}

#[test]
fn unary_map_not_kernel_matches_generic_route() {
    assert_kernel_on_equals_off(&["[ 0 1 2 ] { NOT } MAP"]);
}

#[test]
fn kernels_never_give_meaning_to_unresolved_words() {
    // ABS/NEG are MATH module words. Without an IMPORT the generic route
    // raises UnknownWord; the kernel route must not quietly compute them
    // from the token text (regression: textual kernel detection used to).
    let (_on, obs_on) = observe_kernels(&["[ -1 2 -3 ] { ABS } MAP"], true);
    let (_off, obs_off) = observe_kernels(&["[ -1 2 -3 ] { ABS } MAP"], false);
    assert_eq!(obs_on, obs_off, "ABS without IMPORT diverged by route");
    assert!(
        matches!(&obs_on.outcome, Err((_, category)) if category == "unknownWord"),
        "un-imported ABS must be an unknown word, got {:?}",
        obs_on.outcome
    );

    assert_kernel_on_equals_off(&["[ 1 2 3 ] { NEG } MAP"]);
}

// ── The error surface must never name the route ───────────────────────────

#[test]
fn no_outcome_on_either_route_mentions_internal_mechanisms() {
    let programs: [&[&str]; 6] = [
        &["[ 1 2 3 ] { [ 0 ] / } MAP"],
        &["[ 1 2 3 ] { [ 0 ] % } MAP"],
        &["[ 2 0 4 ] [ 10 ] { / } FOLD"],
        &["[ 3 0 ] [ 10 ] { % } FOLD"],
        &["[ 1 2 3 ] { [ 2 ] / } MAP"],
        &["[ 0 1 2 ] { [ 1 ] < } FILTER"],
    ];
    for lines in programs {
        for enabled in [true, false] {
            let (_interp, obs) = observe_kernels(lines, enabled);
            if let Err((message, _category)) = &obs.outcome {
                for needle in ["fast kernel", "quantized", "fastpath"] {
                    assert!(
                        !message.to_lowercase().contains(needle),
                        "error for {lines:?} (kernels={enabled}) leaks '{needle}': {message}"
                    );
                }
            }
        }
    }
}
