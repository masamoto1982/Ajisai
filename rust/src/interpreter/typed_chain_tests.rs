//! The typed chain (`typed_chain`, LANG.CONTRACT.TYPES) against the five ops
//! it stands for: with every route on and every route off, and against the
//! program with its signatures erased, the same stack, outcome, charges and
//! trace (LANG.AUTHORITY.FREEDOM) — including where the chain must decline
//! part way (a value past 64 bits, a step or work ceiling).

use crate::interpreter::route_observation::{observe, Limits, Observation};
use crate::interpreter::typed_chain::chain_runs_on_this_thread;
use proptest::prelude::*;

fn run(source: &str, routes: bool, limits: Limits) -> Observation {
    observe(
        source,
        |interp| {
            interp.set_fused_block_enabled(routes);
            interp.set_quickening_enabled(routes);
            interp.set_dense_kernels_enabled(routes);
        },
        limits,
    )
}

fn erased(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.starts_with("#:contract"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `source` typed, routes on, against routes off; and when it answers, the
/// signature-erased program with routes on (the gradual guarantee).
fn holds(source: &str, limits: Limits) -> Observation {
    let typed = run(source, true, limits);
    assert_eq!(typed, run(source, false, limits), "{source}");
    let untyped = run(&erased(source), true, limits);
    if format!("{typed:?}").contains("outcome: Ok") {
        assert_eq!(typed, untyped, "{source}");
    }
    typed
}

const SUMSQ: &str = "#:contract SUMSQ [int] -> int\n\
                     [ [ 'X' BIND X X MUL ] MAP 0 [ ADD ] FOLD ] 'SUMSQ' DEF\n";

#[test]
fn a_typed_map_into_fold_runs_as_one_walk_and_answers_as_two() {
    let before = chain_runs_on_this_thread();
    let observed = holds(&format!("{SUMSQ}1 1000 RANGE SUMSQ"), Limits::default());
    assert!(
        format!("{observed:?}").contains("333833500"),
        "{observed:?}"
    );
    assert!(chain_runs_on_this_thread() > before);
}

#[test]
fn an_untyped_body_keeps_its_five_ops() {
    let before = chain_runs_on_this_thread();
    run(
        &format!("{}1 1000 RANGE SUMSQ", erased(SUMSQ)),
        true,
        Limits::default(),
    );
    assert_eq!(chain_runs_on_this_thread(), before);
}

#[test]
fn a_value_past_64_bits_part_way_declines_to_the_written_ops() {
    // Each square fits; their sum does not, so the chain declines and the
    // ops run as written, ending in the output's typeViolation.
    let source = format!("{SUMSQ}[ 3037000499 3037000499 ] SUMSQ");
    let observed = holds(&source, Limits::default());
    assert!(
        format!("{observed:?}").contains("TypeViolation"),
        "{observed:?}"
    );
    let rat = "#:contract SUMS [rat] -> [rat]\n\
               [ [ 2 DIV ] MAP 0 [ ADD ] SCAN ] 'SUMS' DEF\n\
               [ 9223372036854775807 9223372036854775807 1 ] SUMS";
    holds(rat, Limits::default());
}

#[test]
fn a_ceiling_inside_the_chain_stops_where_the_ops_stop() {
    let source = format!("{SUMSQ}1 50 RANGE SUMSQ");
    for steps in [1, 2, 3, 40, 60, 99, 100, 101, 102, 103, 150, 160] {
        holds(
            &source,
            Limits {
                steps: Some(steps),
                ..Limits::default()
            },
        );
    }
    for work in [1, 10, 49, 50, 51, 99, 100, 101, 1000] {
        holds(
            &source,
            Limits {
                work: Some(work),
                ..Limits::default()
            },
        );
    }
    holds(
        &source,
        Limits {
            bits: Some(32),
            ..Limits::default()
        },
    );
}

#[test]
fn a_typed_word_inside_a_walked_block_still_checks_its_boundary() {
    let source = "#:contract SQ int -> int\n[ 'X' BIND X X MUL ] 'SQ' DEF\n\
                  [ 1 4294967296 ] [ SQ ] MAP";
    let observed = holds(source, Limits::default());
    assert!(
        format!("{observed:?}").contains("TypeViolation"),
        "{observed:?}"
    );
}

/// One element: mostly a small rational, sometimes a point or zero.
fn element() -> impl Strategy<Value = String> {
    prop_oneof![
        5 => (-9i64..10).prop_map(|n| n.to_string()),
        3 => (-9i64..10, 1i64..7).prop_map(|(n, d)| format!("{n}/{d}")),
        1 => Just("1/0".to_string()),
        1 => Just("0/0".to_string()),
        1 => Just("0".to_string()),
    ]
}

const MAPS: [&str; 5] = [
    "[ 'X' BIND X X MUL ]",
    "[ 3 ADD ]",
    "[ 2 DIV ]",
    "[ 1 SUB 2 MUL ]",
    "[ 1/3 MUL ]",
];
const FOLDS: [&str; 4] = ["[ ADD ]", "[ MUL ]", "[ SUB ]", "[ ADD 1/2 MUL ]"];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]
    #[test]
    fn every_chain_shape_answers_as_its_ops(
        items in prop::collection::vec(element(), 1..40),
        map in 0..MAPS.len(),
        fold in 0..FOLDS.len(),
        seed in element(),
        scan in any::<bool>(),
    ) {
        let (walk, out) = if scan { ("SCAN", "[rat]") } else { ("FOLD", "rat") };
        let source = format!(
            "#:contract W [rat] -> {out}\n[ {} {seed} {} {walk} ] 'W' DEF\n[ {} ] W",
            MAPS[map],
            FOLDS[fold],
            items.join(" "),
        );
        holds(&source, Limits::default());
    }
}

#[test]
fn the_points_over_zero_ride_the_chain() {
    let before = chain_runs_on_this_thread();
    holds("#:contract W [rat] -> [rat]\n[ [ 2 DIV ] MAP 1/2 [ ADD ] SCAN ] 'W' DEF\n[ 1/2 1/0 3 0/0 ] W", Limits::default());
    assert_eq!(chain_runs_on_this_thread(), before + 1);
}
