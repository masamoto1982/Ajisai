//! The dense column kernels (`dense_kernels`) against the general routes they
//! stand in for.
//!
//! LANG.AUTHORITY.FREEDOM makes the route unobservable, so each program here
//! runs with the kernels on and off and the two runs must agree on the
//! stack's values (`Debug`, so a dense Tensor and a boxed Vector differ), the
//! error, the resource usage, the metrics and the error flow trace.

use crate::interpreter::Interpreter;
use proptest::prelude::*;

#[derive(Debug, PartialEq)]
struct Observation {
    outcome: std::result::Result<(), String>,
    values: String,
    usage: crate::interpreter::ResourceUsage,
    metrics: String,
    trace: String,
}

fn observe(source: &str, dense: bool, bits: Option<u64>) -> Observation {
    let mut interp = Interpreter::new();
    interp.set_dense_kernels_enabled(dense);
    if let Some(bits) = bits {
        let mut limits = *interp.runtime_limits();
        limits.max_bigint_bits = bits;
        interp.set_runtime_limits(limits);
    }
    let outcome = crate::agent::block_on(interp.execute(source)).map_err(|e| format!("{e:?}"));
    Observation {
        outcome: outcome.map(|_| ()),
        // The values, not the `Stack`: its `fresh_from` mark only schedules
        // the next nesting check, and a refused general route pops and
        // re-pushes operands the kernel never touched.
        values: format!("{:?}", interp.get_stack().as_slice()),
        usage: interp.resource_usage(),
        metrics: format!("{:?}", interp.runtime_metrics()),
        trace: format!("{:?}", interp.error_flow_trace_log),
    }
}

fn assert_same(source: &str, bits: Option<u64>) {
    assert_eq!(
        observe(source, true, bits),
        observe(source, false, bits),
        "routes disagree on `{source}` under bits {bits:?}"
    );
}

#[test]
fn hand_picked_programs_agree() {
    for source in [
        // Integer lanes, on both sides of a scalar and against a Tensor,
        // below and above the old SIMD threshold of eight lanes.
        "1 1000 RANGE 2 MUL",
        "3 1 1000 RANGE SUB",
        "1 5 RANGE 1 5 RANGE ADD",
        "1 1000 RANGE 1 1000 RANGE MUL",
        "[ 1 2 3 ] 10 ADD",
        // Rational lanes and division.
        "1 1000 RANGE 2 DIV",
        "1 1000 RANGE 7 DIV 1/3 ADD",
        "1 1 100 RANGE DIV 1 1 100 RANGE DIV SUB",
        "-3 3 RANGE 2 DIV",
        "1/2 1 10 RANGE DIV",
        // A zero divisor lane, a NIL lane, a mismatch, a one-lane Tensor.
        "1 -3 3 RANGE DIV",
        "1 [ 1 0 2 ] DIV",
        "[ 1 NIL 3 ] 2 MUL",
        "[ 1 2 3 ] [ 1 2 ] ADD",
        "[ 1 2 3 ] [ 2 ] ADD",
        "[ 2 ] [ 1 2 3 ] LT",
        // Overflow out of a machine word.
        "[ 9223372036854775807 1 ] 1 ADD",
        "[ 4611686018427387904 2 ] 2 MUL",
        "[ -9223372036854775808 ] 1 SUB",
        // A quotient whose denominator is i64::MIN cannot be negated in a
        // machine word.
        "[ 7 -7 2 ] -9223372036854775808 DIV",
        "[ -9223372036854775808 4 ] -1 DIV",
        "[ 9223372036854775807 3 ] 2 DIV 3 DIV",
        "[ 9223372036854775807/2 ] 9223372036854775807/3 ADD",
        // Comparisons.
        "1 1000 RANGE 500 GT",
        "1 1000 RANGE 500 LT",
        "500 1 1000 RANGE LT",
        "1 1 100 RANGE DIV 1/7 GT",
        "1 100 RANGE 100 1 RANGE LT",
        "[ 1/2 -1/3 9223372036854775807 ] [ 1/3 -1/2 -9223372036854775808 ] GT",
        // FLOOR / ROUND.
        "1 1000 RANGE 3 DIV FLOOR",
        "-20 20 RANGE 3 DIV FLOOR",
        "-20 20 RANGE 2 DIV ROUND",
        "-20 20 RANGE 7 DIV ROUND",
        "1 1000 RANGE FLOOR",
        "[ 1/2 NIL ] FLOOR",
        "[ [ 1/2 3/2 ] [ 5/2 7/2 ] ] FLOOR",
    ] {
        assert_same(source, None);
    }
}

fn kernel_hits(source: &str) -> u64 {
    let before = crate::interpreter::dense_kernels::kernel_hits_on_this_thread();
    let mut interp = Interpreter::new();
    let _ = crate::agent::block_on(interp.execute(source));
    crate::interpreter::dense_kernels::kernel_hits_on_this_thread() - before
}

/// The equality tests would pass if no kernel ever answered; this pins that
/// the shapes they are for do, and the ones they are not for do not.
#[test]
fn the_kernels_answer_where_they_apply() {
    assert_eq!(kernel_hits("1 1000 RANGE 2 MUL"), 1);
    assert_eq!(kernel_hits("1 1000 RANGE 2 DIV"), 1);
    assert_eq!(kernel_hits("1 1000 RANGE 2 DIV FLOOR"), 2);
    assert_eq!(kernel_hits("1 1000 RANGE 500 GT"), 1);
    assert_eq!(kernel_hits("1 1000 RANGE 1 1000 RANGE ADD"), 1);
    assert_eq!(kernel_hits("1 [ 1 0 2 ] DIV"), 0);
    assert_eq!(kernel_hits("[ 1 NIL 3 ] 2 MUL"), 0);
    assert_eq!(kernel_hits("[ 9223372036854775807 1 ] 1 ADD"), 0);
    assert_eq!(kernel_hits("[ 1 2 3 ] [ 2 ] ADD"), 0);
    assert_eq!(kernel_hits("2 3 ADD"), 0);
}

#[test]
fn the_size_ceiling_agrees() {
    for bits in [1, 4, 8, 63, 64] {
        assert_same("1 300 RANGE 1000 MUL", Some(bits));
        assert_same("1 300 RANGE 7 DIV", Some(bits));
    }
}

fn lane() -> impl Strategy<Value = String> {
    prop_oneof![
        6 => (-30i64..30).prop_map(|n| n.to_string()),
        2 => (-9i64..9, 1i64..9).prop_map(|(n, d)| format!("{n}/{d}")),
        1 => Just("0".to_string()),
        1 => Just("9223372036854775807".to_string()),
        1 => Just("-9223372036854775808".to_string()),
        1 => Just("NIL".to_string()),
    ]
}

fn operand() -> impl Strategy<Value = String> {
    prop_oneof![
        3 => prop::collection::vec(lane(), 1..12).prop_map(|l| format!("[ {} ]", l.join(" "))),
        2 => lane(),
        1 => (-5i64..5, 1i64..40).prop_map(|(a, n)| format!("{a} {} RANGE", a + n)),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn random_operations_agree(
        a in operand(),
        b in operand(),
        word in prop_oneof![
            Just("ADD"), Just("SUB"), Just("MUL"), Just("DIV"), Just("LT"), Just("GT"),
        ],
        unary in prop_oneof![Just(""), Just("FLOOR"), Just("ROUND")],
        bits in prop::option::of(1u64..130),
    ) {
        let source = format!("{a} {b} {word} {unary}");
        prop_assert_eq!(
            observe(&source, true, bits),
            observe(&source, false, bits),
            "`{}` under bits {:?}", source, bits
        );
    }
}

/// Shapes the kernels are for — one length, no absent lane — so most cases
/// reach them rather than declining at the shape check; the lanes still
/// reach zero and the edges of a machine word.
fn kernel_lane() -> impl Strategy<Value = String> {
    prop_oneof![
        6 => (-30i64..30).prop_map(|n| n.to_string()),
        3 => (-9i64..9, 1i64..9).prop_map(|(n, d)| format!("{n}/{d}")),
        1 => Just("4611686018427387904".to_string()),
        1 => Just("-9223372036854775808".to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn kernel_shaped_operations_agree(
        (a, b) in (1usize..24).prop_flat_map(|n| (
            prop::collection::vec(kernel_lane(), n),
            prop::collection::vec(kernel_lane(), n),
        )),
        scalar in kernel_lane(),
        shape in 0usize..3,
        word in prop_oneof![
            Just("ADD"), Just("SUB"), Just("MUL"), Just("DIV"), Just("LT"), Just("GT"),
        ],
        unary in prop_oneof![Just(""), Just("FLOOR"), Just("ROUND")],
    ) {
        let (a, b) = (format!("[ {} ]", a.join(" ")), format!("[ {} ]", b.join(" ")));
        let source = match shape {
            0 => format!("{a} {b} {word} {unary}"),
            1 => format!("{a} {scalar} {word} {unary}"),
            _ => format!("{scalar} {b} {word} {unary}"),
        };
        prop_assert_eq!(
            observe(&source, true, None),
            observe(&source, false, None),
            "`{}`", source
        );
    }
}
