//! Differential coverage for the call-site shape inline cache (`shape_ic.rs`),
//! on the shared route-equivalence harness (`route_equivalence.rs`). The IC
//! is routing state only: with it enabled or disabled, the execution outcome
//! — Ok stack or error, rendered forms, hints, and NIL protocol reasons —
//! must be identical for every program. These tests drive compiled word
//! plans (the only place the IC lives) through hits, misses, and demotions.

use super::route_equivalence::{assert_configs_equal, observe};
use crate::interpreter::Interpreter;

fn run_lines(lines: &[&str], ic_enabled: bool) -> Interpreter {
    let (interp, obs) = observe(|i| i.set_shape_ic_enabled(ic_enabled), lines);
    assert_eq!(obs.outcome, Ok(()), "unexpected error for: {lines:?}");
    interp
}

fn assert_ic_on_equals_off(lines: &[&str]) -> (Interpreter, Interpreter) {
    assert_configs_equal(
        "shape IC",
        |i| i.set_shape_ic_enabled(true),
        |i| i.set_shape_ic_enabled(false),
        lines,
    )
}

#[test]
fn every_ic_word_matches_baseline_in_a_compiled_body() {
    for op in ["+", "-", "*", "/", "<", "<=", ">", ">=", "=", "!="] {
        let def = format!("{{ [ 7 ] [ 2 ] {op} }} 'ICW' DEF");
        let (on, _off) = assert_ic_on_equals_off(&[&def, "ICW"]);
        assert!(
            on.runtime_metrics().shape_ic_hit_count > 0,
            "expected a shape-IC hit for compiled `{op}`"
        );
    }
}

#[test]
fn monomorphic_site_hits_on_every_call() {
    let lines = ["{ [ 3 ] [ 4 ] + } 'MONO' DEF", "MONO MONO MONO MONO"];
    let (on, _off) = assert_ic_on_equals_off(&lines);
    assert!(
        on.runtime_metrics().shape_ic_hit_count >= 4,
        "each call of a scalar-monomorphic site should hit the IC, got {}",
        on.runtime_metrics().shape_ic_hit_count
    );
    assert_eq!(on.runtime_metrics().shape_ic_miss_count, 0);
}

#[test]
fn nil_operand_demotes_the_site_and_preserves_nil_passthrough() {
    let lines = [
        "{ + } 'PLUS' DEF",
        "[ 1 ] [ 2 ] PLUS",
        "NIL [ 2 ] PLUS",
        "[ 5 ] [ 6 ] PLUS",
    ];
    let (on, _off) = assert_ic_on_equals_off(&lines);
    let metrics = on.runtime_metrics();
    assert!(
        metrics.shape_ic_hit_count >= 1,
        "the first scalar call should hit"
    );
    assert!(
        metrics.shape_ic_miss_count >= 1,
        "the NIL call should demote the site to the generic route"
    );
    // After demotion the generic route's own fast path still fires, so the
    // final scalar call keeps its result identical (asserted above) while the
    // IC stays generic (no further IC hits are required for correctness).
}

#[test]
fn division_by_zero_bubble_is_identical_through_the_ic_route() {
    let lines = ["{ / } 'DIVW' DEF", "[ 1 ] [ 0 ] DIVW"];
    assert_ic_on_equals_off(&lines);
}

#[test]
fn keep_mode_retains_operands_identically() {
    let lines = ["{ KEEP + } 'KADD' DEF", "[ 2 ] [ 3 ] KADD"];
    assert_ic_on_equals_off(&lines);
}

#[test]
fn mixed_shape_operands_fall_back_identically() {
    // A 3-lane vector and a scalar: the scalar fast path rejects the pair,
    // the IC records a miss, and the broadcast result must match baseline.
    let lines = ["{ + } 'BADD' DEF", "[ 1 2 3 ] [ 10 ] BADD"];
    let (on, _off) = assert_ic_on_equals_off(&lines);
    assert!(on.runtime_metrics().shape_ic_miss_count >= 1);
}

#[test]
fn text_operands_fall_back_identically() {
    let lines = ["{ = } 'TEQ' DEF", "'AB' 'AB' TEQ"];
    assert_ic_on_equals_off(&lines);
}

#[test]
fn recursive_countdown_loop_matches_baseline_and_hits() {
    // The hot-loop idiom the IC targets: a guarded tail-recursive countdown
    // whose per-iteration work is scalar arithmetic and comparison.
    let lines = [
        "{\n  { [ 0 ] > | [ 1 ] - DOWN }\n  { IDLE | [ 0 ] } COND\n} 'DOWN' DEF",
        "[ 50 ] DOWN",
    ];
    let (on, _off) = assert_ic_on_equals_off(&lines);
    assert!(
        on.runtime_metrics().shape_ic_hit_count > 50,
        "loop iterations should hit the IC, got {}",
        on.runtime_metrics().shape_ic_hit_count
    );
}

#[test]
fn disabled_ic_records_no_metrics() {
    let off = run_lines(&["{ [ 1 ] [ 2 ] + } 'W' DEF", "W"], false);
    assert_eq!(off.runtime_metrics().shape_ic_hit_count, 0);
    assert_eq!(off.runtime_metrics().shape_ic_miss_count, 0);
}
