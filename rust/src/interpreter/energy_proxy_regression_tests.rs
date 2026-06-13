//! Cross-cutting test suite: energy-proxy regression guards.
//!
//! Sibling of `perf_regression_tests.rs`, but gating on the deterministic
//! [`energy_proxy_score`](super::energy_proxy::energy_proxy_score) instead
//! of wall time: for a fixed catalog of programs, the observable result
//! must stay identical AND the structural-cost proxy must not grow past
//! its recorded baseline. A change that keeps outputs equal but moves more
//! data (extra flatten/rebuild round-trips, lost fast paths, larger
//! allocations) fails here instead of slipping through.
//!
//! Baseline discipline:
//! - The catalog must contain tensor-bearing programs with a *non-zero*
//!   baseline (see [`tensor_pipeline_is_observed`]); a globally zeroed
//!   counter pipeline would otherwise silently disable the guard.
//! - If an intentional engine change lowers a score, tighten the baseline
//!   in the same PR. If it raises a score, either justify and raise the
//!   baseline explicitly (saying why in the PR) or fix the regression —
//!   never bump casually.
//! - Scores are comparable only within one ENERGY_PROXY_VERSION; bumping
//!   the version (docs/quality/energy-proxy-score.md) re-baselines this
//!   whole table.

use super::energy_proxy::energy_proxy_score;
use crate::interpreter::Interpreter;
use crate::types::Interpretation;

struct EnergyCase {
    label: &'static str,
    code: &'static str,
    /// Expected final stack, bottom to top, as display strings — pins that
    /// the observable meaning of the program is unchanged.
    expected_stack: &'static [&'static str],
    /// Recorded energyProxyScore for ENERGY_PROXY_VERSION = 1. The guard
    /// asserts the live score never exceeds this.
    baseline_score: u64,
}

const ENERGY_CASES: &[EnergyCase] = &[
    EnergyCase {
        label: "scalar-broadcast",
        code: "[ 5 ] [ 0 7 ] RANGE *",
        expected_stack: &["[ 0/1 5/1 10/1 15/1 20/1 25/1 30/1 35/1 ]"],
        baseline_score: 88,
    },
    EnergyCase {
        label: "matrix-add",
        code: "[ [ 1 2 3 ] [ 4 5 6 ] ] [ [ 10 20 30 ] [ 40 50 60 ] ] +",
        expected_stack: &["[ [ 11/1 22/1 33/1 ] [ 44/1 55/1 66/1 ] ]"],
        baseline_score: 100,
    },
    EnergyCase {
        label: "row-broadcast",
        code: "[ [ 1 2 3 ] [ 4 5 6 ] ] [ 10 20 30 ] +",
        expected_stack: &["[ [ 11/1 22/1 33/1 ] [ 14/1 25/1 36/1 ] ]"],
        baseline_score: 84,
    },
    EnergyCase {
        label: "col-broadcast",
        code: "[ [ 1 2 3 ] [ 4 5 6 ] ] [ [ 100 ] [ 200 ] ] +",
        expected_stack: &["[ [ 101/1 102/1 103/1 ] [ 204/1 205/1 206/1 ] ]"],
        baseline_score: 80,
    },
    EnergyCase {
        // NIL bubbles through tensor arithmetic (x/0 in one lane); the
        // result is a single NIL but the broadcast still moved data.
        label: "nil-mixed-tensor",
        code: "[ 1 2 3 ] [ 1 0 1 ] /",
        expected_stack: &["NIL"],
        baseline_score: 70,
    },
    EnergyCase {
        label: "tensor-3d-add",
        code: "[ [ [ 1 2 ] [ 3 4 ] ] [ [ 5 6 ] [ 7 8 ] ] ] \
                [ [ [ 1 1 ] [ 1 1 ] ] [ [ 1 1 ] [ 1 1 ] ] ] +",
        expected_stack: &["[ [ [ 2/1 3/1 ] [ 4/1 5/1 ] ] [ [ 6/1 7/1 ] [ 8/1 9/1 ] ] ]"],
        baseline_score: 120,
    },
    EnergyCase {
        label: "scalar-times-matrix",
        code: "[ 3 ] [ [ 1 2 3 ] [ 4 5 6 ] ] *",
        expected_stack: &["[ [ 3/1 6/1 9/1 ] [ 12/1 15/1 18/1 ] ]"],
        baseline_score: 76,
    },
    EnergyCase {
        // Two chained tensor adds: the intermediate result crosses the
        // flat/nested boundary, so this is the round-trip-sensitive case.
        label: "chained-tensor-add",
        code: "[ 1 2 3 4 ] [ 2 2 ] RESHAPE [ [ 10 20 ] [ 30 40 ] ] + [ [ 1 1 ] [ 1 1 ] ] +",
        expected_stack: &["[ [ 12/1 23/1 ] [ 34/1 45/1 ] ]"],
        baseline_score: 160,
    },
    EnergyCase {
        label: "reshape-mul",
        code: "[ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE [ 2 ] *",
        expected_stack: &["[ [ 2/1 4/1 6/1 ] [ 8/1 10/1 12/1 ] ]"],
        baseline_score: 76,
    },
    EnergyCase {
        // Same-shape 1-D arithmetic takes the SIMD fast path, which records
        // no boundary movement: its honest proxy cost is 0. Kept in the
        // catalog as a guard that this path stays free — if it silently
        // started flattening, the score would rise above 0 and fail.
        label: "same-shape-simd",
        code: "[ 1 2 3 4 5 6 7 8 ] [ 8 7 6 5 4 3 2 1 ] +",
        expected_stack: &["[ 9/1 9/1 9/1 9/1 9/1 9/1 9/1 9/1 ]"],
        baseline_score: 0,
    },
];

/// Candidate programs spanning the structural patterns called out in the
/// work order (§4.2). Run `cargo test energy_proxy_discovery -- --nocapture`
/// to print discovered scores/stacks when seeding or re-baselining.
const DISCOVERY_PROGRAMS: &[(&str, &str)] = &[
    ("scalar-broadcast", "[ 5 ] [ 0 7 ] RANGE *"),
    (
        "matrix-add",
        "[ [ 1 2 3 ] [ 4 5 6 ] ] [ [ 10 20 30 ] [ 40 50 60 ] ] +",
    ),
    ("row-broadcast", "[ [ 1 2 3 ] [ 4 5 6 ] ] [ 10 20 30 ] +"),
    (
        "col-broadcast",
        "[ [ 1 2 3 ] [ 4 5 6 ] ] [ [ 100 ] [ 200 ] ] +",
    ),
    ("nil-mixed-tensor", "[ 1 2 3 ] [ 1 0 1 ] /"),
    (
        "tensor-3d-add",
        "[ [ [ 1 2 ] [ 3 4 ] ] [ [ 5 6 ] [ 7 8 ] ] ] [ [ [ 1 1 ] [ 1 1 ] ] [ [ 1 1 ] [ 1 1 ] ] ] +",
    ),
    ("scalar-times-matrix", "[ 3 ] [ [ 1 2 3 ] [ 4 5 6 ] ] *"),
    (
        "chained-tensor-add",
        "[ 1 2 3 4 ] [ 2 2 ] RESHAPE [ [ 10 20 ] [ 30 40 ] ] + [ [ 1 1 ] [ 1 1 ] ] +",
    ),
    ("reshape-mul", "[ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE [ 2 ] *"),
    (
        "same-shape-simd",
        "[ 1 2 3 4 5 6 7 8 ] [ 8 7 6 5 4 3 2 1 ] +",
    ),
];

fn run_collect(code: &str) -> (Vec<String>, u64) {
    let mut interp = Interpreter::new();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        interp.execute(code).await.expect("program should execute");
    });
    let hints = interp.collect_stack_hints().to_vec();
    let stack: Vec<String> = interp
        .get_stack()
        .iter()
        .enumerate()
        .map(|(i, value)| {
            let hint = hints.get(i).copied().unwrap_or(Interpretation::Unassigned);
            crate::types::display::format_with_hint(value, hint)
        })
        .collect();
    let score = energy_proxy_score(&interp.runtime_metrics());
    (stack, score)
}

#[test]
fn output_unchanged_and_score_within_baseline() {
    for case in ENERGY_CASES {
        let (stack, score) = run_collect(case.code);
        assert_eq!(
            stack, case.expected_stack,
            "[{}] output changed — energy regression guards are only valid when meaning is preserved",
            case.label
        );
        assert!(
            score <= case.baseline_score,
            "[{}] energyProxyScore {} exceeds baseline {} — same output, more structural work. \
             Investigate the regression; only raise the baseline with explicit justification.",
            case.label,
            score,
            case.baseline_score
        );
    }
}

#[test]
fn score_is_deterministic_across_runs() {
    for case in ENERGY_CASES {
        let (_, first) = run_collect(case.code);
        let (_, second) = run_collect(case.code);
        assert_eq!(
            first, second,
            "[{}] energyProxyScore must be deterministic across runs",
            case.label
        );
    }
}

#[test]
fn tensor_pipeline_is_observed() {
    // At least one tensor-bearing program must score non-zero, otherwise a
    // broken (globally zeroed) counter pipeline would pass every `<=`
    // assertion above while measuring nothing.
    let max = ENERGY_CASES
        .iter()
        .map(|case| run_collect(case.code).1)
        .max()
        .unwrap_or(0);
    assert!(
        max > 0,
        "no catalog program scored above zero — the VTU counter pipeline appears disabled"
    );
}

#[test]
fn energy_proxy_discovery() {
    // Seeding/re-baselining aid; always passes. Run with --nocapture.
    for (label, code) in DISCOVERY_PROGRAMS {
        let (stack, score) = run_collect(code);
        println!("{label} => score={score} stack={stack:?}");
    }
}
