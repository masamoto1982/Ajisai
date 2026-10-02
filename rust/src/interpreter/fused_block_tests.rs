//! The fused walk (`fused_block`) against the interpreted walk it replaces.
//!
//! LANG.AUTHORITY.FREEDOM makes the route unobservable, so the two runs of each
//! program here must agree on everything a host can read: the stack, the
//! error, the resource usage, the runtime metrics, the epochs and the error
//! flow trace. Hand-picked cases cover each bail-out (zero divisor, NIL lane,
//! ceilings); the property test drives random blocks, vectors and limits.

use crate::interpreter::Interpreter;
use crate::types::display::render_stack;
use proptest::prelude::*;

#[derive(Debug, PartialEq)]
struct Observation {
    outcome: std::result::Result<(), String>,
    stack: Vec<String>,
    /// The values themselves, not only their rendering: a dense Tensor and
    /// a boxed Vector of the same numbers render alike.
    values: String,
    usage: crate::interpreter::ResourceUsage,
    collection_work: u64,
    metrics: String,
    epochs: crate::interpreter::EpochSnapshot,
    trace: String,
}

#[derive(Debug, Clone, Copy, Default)]
struct Limits {
    steps: Option<usize>,
    work: Option<u64>,
    bits: Option<u64>,
}

fn observe(source: &str, fused: bool, limits: Limits) -> Observation {
    let mut interp = Interpreter::new();
    interp.set_fused_block_enabled(fused);
    interp.set_scalar_fastpath_enabled(true);
    if let Some(steps) = limits.steps {
        interp.set_max_execution_steps(steps);
    }
    let mut runtime = *interp.runtime_limits();
    if let Some(work) = limits.work {
        runtime.max_numeric_work = work;
    }
    if let Some(bits) = limits.bits {
        runtime.max_bigint_bits = bits;
    }
    interp.set_runtime_limits(runtime);
    let outcome = crate::agent::block_on(interp.execute(source)).map_err(|e| format!("{e:?}"));
    Observation {
        outcome: outcome.map(|_| ()),
        stack: render_stack(interp.get_stack()),
        values: format!("{:?}", interp.get_stack()),
        usage: interp.resource_usage(),
        collection_work: interp.collection_work_used(),
        metrics: format!("{:?}", interp.runtime_metrics()),
        epochs: interp.current_epoch_snapshot(),
        trace: format!("{:?}", interp.error_flow_trace_log),
    }
}

fn assert_same(source: &str, limits: Limits) -> Observation {
    let fused = observe(source, true, limits);
    let interpreted = observe(source, false, limits);
    assert_eq!(
        fused, interpreted,
        "routes disagree on `{source}` under {limits:?}"
    );
    fused
}

fn fused_runs(source: &str) -> u64 {
    let before = crate::interpreter::fused_block::fused_runs_on_this_thread();
    let mut interp = Interpreter::new();
    let _ = crate::agent::block_on(interp.execute(source));
    crate::interpreter::fused_block::fused_runs_on_this_thread() - before
}

#[test]
fn hand_picked_programs_agree() {
    for source in [
        "1 100 RANGE [ 2 MUL 1 ADD ] MAP",
        "1 100 RANGE 0 [ ADD ] FOLD",
        "1 100 RANGE 0 [ ADD ] SCAN",
        "1 1 200 RANGE DIV 0 [ ADD ] FOLD",
        "1 30 RANGE 1 [ MUL ] FOLD",
        "1 30 RANGE 1 [ MUL ] SCAN",
        "[ 1/2 2/3 -3/4 5 ] [ 1/3 SUB 7 DIV ] MAP",
        "[ 1 2 3 ] [ 1 2 ADD ADD ] MAP",
        "[ 1 2 3 ] 10 [ SUB ] FOLD",
        "[ 1 2 3 ] 10 [ DIV ] FOLD",
        // A zero divisor projects a NIL: the ordinary walk's to report.
        "[ 1 0 2 ] [ 1 SWAP DIV ] MAP",
        "[ 1 0 2 ] [ 5 DIV ] MAP",
        "[ 1 0 2 ] 1 [ DIV ] FOLD",
        // A NIL lane, an irrational, a Boolean.
        "[ 1 NIL 2 ] [ 1 ADD ] MAP",
        "[ 1 2 ] [ 2 SQRT ADD ] MAP",
        "[ TRUE FALSE ] [ 1 ADD ] MAP",
        "[ 1 2 ] NIL [ ADD ] FOLD",
        // Underflow and a trailing literal stay off the fused route.
        "[ 1 2 ] [ ADD ] MAP",
        "[ 1 2 ] [ 1 ADD 5 ] MAP",
        "[ ] [ 1 ADD ] MAP",
        "[ ] 0 [ ADD ] FOLD",
        // An integer overflow leaves the integer tier for the rational one.
        "[ 9223372036854775806 1 ] [ 1 ADD ] MAP",
        "[ 9223372036854775807 1 ] [ 1 ADD ] MAP",
        "[ -9223372036854775808 ] [ 1 SUB ] MAP",
        "[ 1 2 3 ] 4611686018427387904 [ ADD 2 MUL ] SCAN",
        // A deep block runs on the rational tier.
        "[ 1 2 ] [ 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 ADD ADD ADD ADD ADD ADD ADD ADD ADD ADD ADD ADD ADD ADD ADD ADD ] MAP",
        // Big operands.
        "[ 9223372036854775807 2 ] [ 9223372036854775807 MUL ] MAP",
        "1 70 RANGE 1 [ MUL ] SCAN",
    ] {
        assert_same(source, Limits::default());
    }
}

#[test]
fn ceilings_agree_at_their_boundaries() {
    let source = "1 50 RANGE 0 [ 3 MUL ADD ] FOLD";
    for steps in [1, 50, 99, 100, 101, 102, 150] {
        assert_same(
            source,
            Limits {
                steps: Some(steps),
                ..Limits::default()
            },
        );
    }
    for work in [0, 1, 50, 99, 100, 101, 1000] {
        assert_same(
            source,
            Limits {
                work: Some(work),
                ..Limits::default()
            },
        );
    }
    for bits in [1, 8, 16, 64] {
        assert_same(
            "1 40 RANGE 1 [ MUL ] FOLD",
            Limits {
                bits: Some(bits),
                ..Limits::default()
            },
        );
    }
}

/// The equality tests above would pass if nothing ever took the fused route;
/// this pins that the walks it is for do, and the ones it is not for do not.
#[test]
fn the_fused_route_is_taken_where_it_applies() {
    assert_eq!(fused_runs("1 1000 RANGE [ 2 MUL 1 ADD ] MAP"), 1);
    assert_eq!(fused_runs("1 1000 RANGE 0 [ ADD ] FOLD"), 1);
    assert_eq!(fused_runs("1 1000 RANGE 0 [ ADD ] SCAN"), 1);
    assert_eq!(fused_runs("[ 1 0 2 ] [ 5 SWAP DIV ] MAP"), 0);
    assert_eq!(fused_runs("[ 1 0 2 ] [ 5 DIV ] MAP"), 1);
    assert_eq!(fused_runs("[ 0 1 2 ] [ 5 SWAP DIV ] MAP"), 0);
    assert_eq!(fused_runs("[ 1 NIL 2 ] [ 1 ADD ] MAP"), 0);
    assert_eq!(fused_runs("[ 9223372036854775807 ] [ 1 ADD ] MAP"), 1);
}

fn literal() -> impl Strategy<Value = String> {
    prop_oneof![
        (-20i64..20).prop_map(|n| n.to_string()),
        (-9i64..9, 1i64..9).prop_map(|(n, d)| format!("{n}/{d}")),
        Just("9223372036854775807".to_string()),
        Just("-9223372036854775808".to_string()),
        Just("4611686018427387904".to_string()),
        Just("0".to_string()),
    ]
}

fn word() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("ADD"), Just("SUB"), Just("MUL"), Just("DIV")]
}

fn block() -> impl Strategy<Value = String> {
    prop::collection::vec(prop_oneof![literal(), word().prop_map(String::from)], 1..6)
        .prop_map(|tokens| format!("[ {} ]", tokens.join(" ")))
}

fn vector() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![9 => literal(), 1 => Just("NIL".to_string())],
        0..12,
    )
    .prop_map(|items| format!("[ {} ]", items.join(" ")))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn random_walks_agree(
        target in vector(),
        seed in literal(),
        code in block(),
        walk in 0usize..3,
        steps in prop::option::of(0usize..80),
        work in prop::option::of(0u64..200),
        bits in prop::option::of(1u64..130),
    ) {
        let source = match walk {
            0 => format!("{target} {code} MAP"),
            1 => format!("{target} {seed} {code} FOLD"),
            _ => format!("{target} {seed} {code} SCAN"),
        };
        let limits = Limits { steps, work, bits };
        let fused = observe(&source, true, limits);
        let interpreted = observe(&source, false, limits);
        prop_assert_eq!(fused, interpreted, "`{}` under {:?}", source, limits);
    }
}
