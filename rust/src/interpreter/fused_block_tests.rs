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
        // Underflow stays off the fused route; a trailing literal is the
        // block's result like any other value.
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
        // Comparisons, logic, FLOOR/ROUND, SELECT.
        "1 20 RANGE [ 10 LT ] MAP",
        "1 20 RANGE [ 10 GT NOT ] FILTER",
        "1 20 RANGE [ 3 EQ ] FILTER",
        "[ 1/2 3/2 -5/2 ] [ FLOOR ] MAP",
        "[ 1/2 3/2 -5/2 ] [ ROUND ] MAP",
        "[ TRUE FALSE TRUE ] [ NOT ] MAP",
        "[ TRUE FALSE TRUE ] [ TRUE EQ ] FILTER",
        "[ TRUE FALSE ] TRUE [ AND ] FOLD",
        "[ TRUE FALSE ] [ 1 EQ ] MAP",
        "1 10 RANGE [ 'X' BIND X 2 MUL X 1 ADD X 5 LT SELECT ] MAP",
        // The remainder idiom: integer floor division, and its zero divisor.
        "1 30 RANGE [ 'N' BIND N N 7 DIV FLOOR 7 MUL SUB ] MAP",
        "-10 10 RANGE [ 'N' BIND N N -3 DIV FLOOR -3 MUL SUB ] MAP",
        "[ 3 0 4 ] [ 'D' BIND 10 10 D DIV FLOOR D MUL SUB ] MAP",
        "[ -9223372036854775808 ] [ -1 DIV FLOOR ] MAP",
        // A name bound outside the block, and one bound nowhere.
        "5 'K' BIND 1 10 RANGE [ K MUL ] MAP",
        "1 10 RANGE [ K MUL ] MAP",
        "1/2 'K' BIND 1 10 RANGE 0 [ ADD K MUL ] FOLD",
        // A name BIND refuses, and a destructuring BIND.
        "1 10 RANGE [ 'ADD' BIND 1 ] MAP",
        "1 10 RANGE [ [ 'A' ] BIND 1 ] MAP",
        // Domain errors inside the block.
        "1 10 RANGE [ NOT ] MAP",
        "1 10 RANGE [ TRUE ADD ] MAP",
        "1 10 RANGE [ 2 MUL ] FILTER",
        "[ TRUE ] [ 1 LT ] MAP",
        "1 10 RANGE [ 1 2 3 SELECT ] MAP",
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
    assert_eq!(fused_runs("1 1000 RANGE [ 3 GT ] FILTER"), 1);
    assert_eq!(
        fused_runs("1 1000 RANGE [ 'N' BIND N N 7 DIV FLOOR 7 MUL SUB ] MAP"),
        1
    );
    assert_eq!(fused_runs("5 'K' BIND 1 1000 RANGE [ K MUL ] MAP"), 1);
    assert_eq!(fused_runs("1 10 RANGE [ K MUL ] MAP"), 0);
    assert_eq!(fused_runs("1 10 RANGE [ 2 MUL ] FILTER"), 0);
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
    prop_oneof![
        4 => prop_oneof![Just("ADD"), Just("SUB"), Just("MUL"), Just("DIV")],
        2 => prop_oneof![Just("LT"), Just("GT"), Just("EQ")],
        2 => prop_oneof![Just("FLOOR"), Just("ROUND"), Just("NOT"), Just("AND"), Just("SELECT")],
        1 => prop_oneof![Just("TRUE"), Just("FALSE")],
        // Names: bound in the block, bound outside it (`K`), or both.
        2 => prop_oneof![Just("'X' BIND"), Just("X"), Just("'K' BIND"), Just("K")],
    ]
}

fn block() -> impl Strategy<Value = String> {
    prop::collection::vec(prop_oneof![literal(), word().prop_map(String::from)], 1..8)
        .prop_map(|tokens| format!("[ {} ]", tokens.join(" ")))
}

fn vector() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            9 => literal(),
            1 => Just("NIL".to_string()),
            1 => prop_oneof![Just("TRUE".to_string()), Just("FALSE".to_string())],
        ],
        0..12,
    )
    .prop_map(|items| format!("[ {} ]", items.join(" ")))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn random_walks_agree(
        target in vector(),
        seed in literal(),
        code in block(),
        walk in 0usize..4,
        outer in prop::option::of(literal()),
        steps in prop::option::of(0usize..80),
        work in prop::option::of(0u64..200),
        bits in prop::option::of(1u64..130),
    ) {
        let walk = match walk {
            0 => format!("{target} {code} MAP"),
            1 => format!("{target} {code} FILTER"),
            2 => format!("{target} {seed} {code} FOLD"),
            _ => format!("{target} {seed} {code} SCAN"),
        };
        let source = match outer {
            Some(value) => format!("{value} 'K' BIND {walk}"),
            None => walk,
        };
        let limits = Limits { steps, work, bits };
        let fused = observe(&source, true, limits);
        let interpreted = observe(&source, false, limits);
        prop_assert_eq!(fused, interpreted, "`{}` under {:?}", source, limits);
    }
}

/// Well-typed expressions, so the walks below mostly take the fused route
/// rather than refusing at the first ill-typed op: a numeric or a Boolean
/// expression tree over the element (`X`), an outer binding (`K`) and
/// literals, written in postfix.
fn typed_expr() -> impl Strategy<Value = (String, String)> {
    let num_leaf = prop_oneof![
        3 => Just("X".to_string()),
        1 => Just("K".to_string()),
        3 => (-9i64..10).prop_map(|n| n.to_string()),
        1 => (-9i64..9, 1i64..5).prop_map(|(n, d)| format!("{n}/{d}")),
        1 => Just("4611686018427387904".to_string()),
    ];
    let bool_leaf = prop_oneof![Just("TRUE".to_string()), Just("FALSE".to_string())];
    (num_leaf, bool_leaf).prop_recursive(4, 24, 3, |inner| {
        let num = inner.clone().prop_map(|(n, _)| n);
        let boolean = inner.prop_map(|(_, b)| b);
        let num_op = prop_oneof![
            Just("ADD"),
            Just("SUB"),
            Just("MUL"),
            Just("DIV"),
            Just("DIV FLOOR")
        ];
        let cmp = prop_oneof![Just("LT"), Just("GT"), Just("EQ")];
        let numeric = prop_oneof![
            (num.clone(), num.clone(), num_op).prop_map(|(a, b, op)| format!("{a} {b} {op}")),
            (num.clone(), prop_oneof![Just("FLOOR"), Just("ROUND")])
                .prop_map(|(a, op)| format!("{a} {op}")),
            (num.clone(), num.clone(), boolean.clone())
                .prop_map(|(a, b, m)| format!("{a} {b} {m} SELECT")),
        ];
        let truth = prop_oneof![
            (num.clone(), num, cmp).prop_map(|(a, b, op)| format!("{a} {b} {op}")),
            boolean.clone().prop_map(|b| format!("{b} NOT")),
            (boolean.clone(), boolean).prop_map(|(a, b)| format!("{a} {b} AND")),
        ];
        (numeric, truth)
    })
}

fn integer_vector() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            8 => (-50i64..50).prop_map(|n| n.to_string()),
            1 => Just("9223372036854775807".to_string()),
            1 => (-9i64..9, 1i64..5).prop_map(|(n, d)| format!("{n}/{d}")),
        ],
        1..16,
    )
    .prop_map(|items| format!("[ {} ]", items.join(" ")))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn typed_walks_agree(
        target in integer_vector(),
        (num, truth) in typed_expr(),
        walk in 0usize..4,
        outer in (-9i64..10),
        steps in prop::option::of(0usize..400),
        work in prop::option::of(0u64..400),
    ) {
        let code = match walk {
            0 => format!("[ 'X' BIND {num} ] MAP"),
            1 => format!("[ 'X' BIND {truth} ] FILTER"),
            2 => format!("0 [ 'X' BIND {num} ADD ] FOLD"),
            _ => format!("0 [ 'X' BIND {num} ADD ] SCAN"),
        };
        let source = format!("{outer} 'K' BIND {target} {code}");
        let limits = Limits { steps, work, bits: None };
        let fused = observe(&source, true, limits);
        let interpreted = observe(&source, false, limits);
        prop_assert_eq!(fused, interpreted, "`{}` under {:?}", source, limits);
    }
}
