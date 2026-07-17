//! Shared route-equivalence harness (A3 in
//! docs/dev/user-surface-information-hiding.md). Every routing/optimization
//! toggle must be invisible: the same program produces the same
//! *observation*, where an observation is everything a user or user program
//! can see — the outcome (Ok, or the error's display text and protocol
//! category), the stack's debug form, rendered forms, hints, and NIL
//! protocol reasons. Unlike the older per-optimization helpers, the error
//! and NIL surfaces are part of the comparison: an optimization that turns
//! a Bubble-Rule NIL into a route-specific error, or reworded an error, is
//! caught here.
//!
//! A new optimization adds its switch to `ROUTE_TOGGLES`; when it introduces
//! a new class of observable outcome, it also adds a program that exercises
//! that class to `CANONICAL_CORPUS`.

use crate::error::ErrorCategory;
use crate::interpreter::Interpreter;

pub(super) fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    use std::task::{Context, Poll};
    let mut fut = Box::pin(fut);
    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(waker);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// Everything a user program can observe from one execution.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Observation {
    pub outcome: Result<(), (String, String)>,
    pub stack_debug: String,
    pub rendered: Vec<String>,
    pub hints: Vec<String>,
    pub nil_reasons: Vec<Option<String>>,
}

/// Run `lines` on a fresh interpreter after applying `configure`, capturing
/// the observation. Execution stops at the first error, exactly as a user
/// session would.
pub(super) fn observe(
    configure: impl FnOnce(&mut Interpreter),
    lines: &[&str],
) -> (Interpreter, Observation) {
    let mut interp = Interpreter::new();
    configure(&mut interp);
    let mut outcome = Ok(());
    for line in lines {
        if let Err(e) = block_on(interp.execute(line)) {
            let category = ErrorCategory::from_error(&e).as_protocol_str().to_string();
            outcome = Err((e.to_string(), category));
            break;
        }
    }
    let stack = interp.get_stack();
    let observation = Observation {
        outcome,
        stack_debug: format!("{stack:?}"),
        rendered: stack.iter().map(|v| format!("{v}")).collect(),
        hints: stack.iter().map(|v| format!("{:?}", v.hint)).collect(),
        nil_reasons: stack
            .iter()
            .map(|v| v.nil_reason().map(|r| r.as_protocol_str().to_string()))
            .collect(),
    };
    (interp, observation)
}

/// Assert that two interpreter configurations observe `lines` identically.
/// Returns both interpreters for follow-up metric assertions.
pub(super) fn assert_configs_equal(
    label: &str,
    config_a: impl FnOnce(&mut Interpreter),
    config_b: impl FnOnce(&mut Interpreter),
    lines: &[&str],
) -> (Interpreter, Interpreter) {
    let (interp_a, obs_a) = observe(config_a, lines);
    let (interp_b, obs_b) = observe(config_b, lines);
    assert_eq!(obs_a, obs_b, "{label}: routes diverged for {lines:?}");
    (interp_a, interp_b)
}

/// A named routing switch. `set` must only change which internal route
/// executes, never what a program observes — that is what the matrix test
/// below enforces.
pub(super) struct RouteToggle {
    pub name: &'static str,
    pub set: fn(&mut Interpreter, bool),
}

pub(super) const ROUTE_TOGGLES: &[RouteToggle] = &[
    RouteToggle {
        name: "scalar_fastpath",
        set: |i, b| i.set_scalar_fastpath_enabled(b),
    },
    RouteToggle {
        name: "shape_ic",
        set: |i, b| i.set_shape_ic_enabled(b),
    },
    RouteToggle {
        name: "hof_memo",
        set: |i, b| i.set_hof_memo_enabled(b),
    },
    RouteToggle {
        name: "fast_kernel",
        set: |i, b| i.set_fast_kernel_enabled(b),
    },
    RouteToggle {
        name: "tail_call",
        set: |i, b| i.tail_call_enabled = b,
    },
    RouteToggle {
        name: "cond_dispatch",
        set: |i, b| i.cond_dispatch_enabled = b,
    },
    RouteToggle {
        name: "vector_literal",
        set: |i, b| i.vector_literal_enabled = b,
    },
    RouteToggle {
        name: "compiled_clause",
        set: |i, b| i.compiled_clause_enabled = b,
    },
];

/// Programs covering the observable-outcome classes: plain values, NIL
/// bubbles with reasons, hard errors (typed and custom), NIL passthrough,
/// KEEP mode, compiled word plans, the HOF family, and the tail-recursive
/// countdown idiom the compiled/IC routes target.
pub(super) const CANONICAL_CORPUS: &[&[&str]] = &[
    &["1 2 +"],
    &["[ 1 2 [ 3 4 ] ]"],
    &["[ 1 ] [ 0 ] /"],
    &["[ 10 20 ] [ 99 ] GET"],
    &["__NO_SUCH_WORD__"],
    &["[ 1 2 3 ] { [ 0 ] / } MAP"],
    &["[ 1 2 3 ] { [ 0 ] % } MAP"],
    &["[ 2 0 4 ] [ 10 ] { / } FOLD"],
    &["[ 1 2 3 ] { [ 2 ] * } MAP"],
    &["[ 0 1 2 ] { [ 1 ] < } FILTER"],
    &["[ 1 2 3 4 ] [ 0 ] { + } FOLD"],
    &["'AB' 'AB' ="],
    &["{ [ 3 ] [ 4 ] + } 'MONO' DEF", "MONO MONO"],
    &[
        "{ + } 'PLUS' DEF",
        "[ 1 ] [ 2 ] PLUS",
        "NIL [ 2 ] PLUS",
        "[ 5 ] [ 6 ] PLUS",
    ],
    &["{ KEEP + } 'KADD' DEF", "[ 2 ] [ 3 ] KADD"],
    &[
        "{\n  { [ 0 ] > | [ 1 ] - DOWN }\n  { IDLE | [ 0 ] } COND\n} 'DOWN' DEF",
        "[ 50 ] DOWN",
    ],
];

#[test]
fn every_route_toggle_is_invisible_over_the_canonical_corpus() {
    for toggle in ROUTE_TOGGLES {
        for lines in CANONICAL_CORPUS {
            assert_configs_equal(
                &format!("toggle `{}` off", toggle.name),
                |i| (toggle.set)(i, true),
                |i| (toggle.set)(i, false),
                lines,
            );
        }
    }
}

#[test]
fn all_routes_off_matches_all_routes_on() {
    // The fully generic interpreter (every routing switch off) is the
    // semantic baseline; the fully optimized default must be
    // indistinguishable from it.
    for lines in CANONICAL_CORPUS {
        assert_configs_equal(
            "all toggles off",
            |_| {},
            |i| {
                for toggle in ROUTE_TOGGLES {
                    (toggle.set)(i, false);
                }
            },
            lines,
        );
    }
}
