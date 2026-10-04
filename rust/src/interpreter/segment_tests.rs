//! Typed segments (`segment`) against the dispatched route.
//!
//! LANG.AUTHORITY.FREEDOM makes the route unobservable, so a program must
//! leave the same stack, outcome, resource usage, metrics, epochs, trace and
//! source position whether its scalar runs are segments or dispatched Word
//! by Word. The programs put runs in User Word bodies and in blocks the
//! fused walk declines, with names, `BIND`, inlined calls and the plans they
//! build, and call those Words from the top level too.

use crate::interpreter::route_observation::{self, Limits, Observation};
use crate::interpreter::segment::segment_runs_on_this_thread;
use crate::interpreter::Interpreter;
use proptest::prelude::*;

fn observe(source: &str, segments: bool, limits: Limits) -> Observation {
    route_observation::observe(
        source,
        |interp| interp.set_segments_enabled(segments),
        limits,
    )
}

fn assert_same(source: &str, limits: Limits) {
    assert_eq!(
        observe(source, true, limits),
        observe(source, false, limits),
        "routes disagree on `{source}` under {limits:?}"
    );
}

/// Segments committed while running `source`.
fn segment_runs(source: &str) -> u64 {
    let mut interp = Interpreter::new();
    let before = segment_runs_on_this_thread();
    let _ = crate::agent::block_on(interp.execute(source));
    segment_runs_on_this_thread() - before
}

#[test]
fn hand_picked_programs_agree() {
    for source in [
        // The top level, walked; and the same runs in bodies.
        "1 2 ADD 3 MUL",
        "[ 1/3 2/5 ADD 7 DIV 3 LT TRUE AND NOT ] 'F' DEF F F EQ",
        "[ 'X' BIND X X MUL X 1 SUB DIV ] 'F' DEF 5 F",
        "[ 'X' BIND X 1 ADD 'X' BIND X X MUL ] 'F' DEF 5 F",
        "[ 'x' BIND x 2 MUL ] 'F' DEF 5 F",
        "[ 'N' BIND N 2 DIV N 3 MUL 1 ADD N 2 DIV FLOOR 2 MUL N EQ SELECT ] 'F' DEF 4 F 7 F",
        "[ FLOOR -7/2 ROUND 9 MIN 2 MAX ] 'F' DEF 7/2 F",
        "[ TRUE FALSE EQ 1 1 EQ AND ] 'F' DEF F",
        "[ 3 ADD 4 MUL ] 'F' DEF [ 1 2 ] LENGTH F",
        "[ 3 ADD 4 MUL ] 'F' DEF F",
        "[ 2 ADD 2 MUL ] 'F' DEF 1 0 DIV F 9223372036854775807 F 99999999999999999999999 F",
        "[ 2 ADD 2 MUL ] 'F' DEF NIL F 'a' F [ 1 2 ] F 2 SQRT F TRUE F",
        "[ 1 TRUE EQ 1 2 ADD ] 'F' DEF F",
        "[ FALSE 1 AND 1 2 ADD ] 'F' DEF F",
        "1 2 ADD 3 MUL 1 2 ADD 3 MUL ADD",
        "1/3 2/5 ADD 7 DIV 3 LT TRUE AND NOT",
        "5 'X' BIND X X MUL X 1 SUB DIV",
        "5 'X' BIND X 1 ADD 'X' BIND X X MUL",
        "5 'x' BIND x 2 MUL",
        "4 'N' BIND N 2 DIV N 3 MUL 1 ADD N 2 DIV FLOOR 2 MUL N EQ SELECT",
        "7/2 FLOOR -7/2 ROUND 9 MIN 2 MAX",
        "TRUE FALSE EQ 1 1 EQ AND",
        // Operands from below the run, and a run that underflows.
        "[ 1 2 ] LENGTH 3 ADD 4 MUL",
        "3 ADD 4 MUL",
        "1 ADD",
        // Declined: zero divisor, overflow, BigInt, NIL, String, Vector,
        // irrational, a number beside a truth value.
        "1 0 DIV 2 ADD",
        "9223372036854775807 1 ADD 2 MUL",
        "99999999999999999999999 1 ADD 2 MUL",
        "NIL 1 ADD 2 MUL",
        "1 2 ADD 'a' ADD 3 MUL",
        "[ 1 2 ] 1 ADD 2 MUL",
        "2 SQRT 1 ADD 2 MUL",
        "1 TRUE EQ 1 2 ADD",
        "FALSE 1 AND 1 2 ADD",
        "1 2 ADD 0 LT NOT 3 4 ADD",
        // A name that is not a plain value, or not bound at all.
        "[ 1 2 ] 'V' BIND V 1 ADD 2 MUL",
        "Q 1 ADD 2 MUL",
        "99999999999999999999999 'B' BIND B 1 ADD 2 MUL",
        // A binding a later Word and a later block read.
        "3 'T' BIND T 1 ADD 'U' BIND [ 1 5 9 ] [ U LT ] FILTER",
        "3 'T' BIND T 1 ADD 'U' BIND [ U T ] EXEC ADD",
        "1 2 'A' BIND 'B' BIND A B SUB",
        // Bind names the run must refuse to the dispatch.
        "1 'ADD' BIND 2 3 ADD",
        "[ 1 ] 'F' DEF 1 'F' BIND 2 3 ADD",
        "1 '1X' BIND 2 3 ADD",
        // User Words: inlined, nested, binding in their own frames.
        "[ 2 MUL 1 ADD ] 'F' DEF 5 F 6 F ADD",
        "[ 2 MUL ] 'D' DEF [ D 1 ADD D ] 'G' DEF 3 G G",
        "[ 'X' BIND X X MUL ] 'SQ' DEF 3 SQ 4 SQ ADD",
        "[ 'X' BIND X X MUL ] 'SQ' DEF 2 'X' BIND 3 SQ X ADD",
        "[ ADD ] 'P' DEF 1 2 P 3 P",
        "[ ADD ] 'P' DEF 1 P",
        "[ 0 DIV ] 'Z' DEF 5 Z 1 ADD",
        // A body that reads a name it did not bind is the dispatch's error.
        "[ Y 1 ADD ] 'H' DEF 2 'Y' BIND 3 H 1 ADD",
        // A body the run cannot hold, called between scalar runs.
        "[ [ 7 ] LENGTH ADD ] 'L' DEF 1 2 ADD L 3 MUL 4 ADD",
        // Unfused blocks: compiled lines with names and BIND.
        "1 30 RANGE [ [ 7 ] LENGTH ADD 'X' BIND X 2 DIV FLOOR X 3 MUL X 9 LT SELECT ] MAP",
        "1 30 RANGE [ 'X' BIND [ 7 ] LENGTH X ADD 2 MUL X X MUL SUB ] MAP",
        "5 'T' BIND 1 30 RANGE [ [ 7 ] LENGTH ADD T 2 MUL LT ] FILTER",
        "1 30 RANGE [ [ 7 ] LENGTH ADD 'X' BIND X 2 MUL [ X ] EXEC ADD ] MAP",
        "[ 3 MUL ] 'T3' DEF 1 30 RANGE [ [ 7 ] LENGTH ADD T3 T3 1 ADD ] MAP",
        "[ 1 2 3 ] [ 'X' BIND [ 9 ] 'K' DEF X K ADD ] MAP",
        // `DEF` keeps a body as written, which only the token walk has: a
        // body that binds and defines stays on it.
        "[ 'X' BIND [ 0.5 1e2 ] 'K' DEF X K ADD ] 'W' DEF 3 W",
        "[ 1 2 3 ] [ 'X' BIND [ 0.5 1e2 ] 'K' DEF X K ADD ] MAP",
        // Errors after a run, for the position and the trace.
        "1 2 ADD 3 MUL 'a' ADD",
        "1 2 ADD 3 MUL\n4 5 ADD UNKNOWNWORD",
        "1 2 ADD 3 MUL\n[ [ [ 1 ] ] ] 1 ADD",
    ] {
        assert_same(source, Limits::default());
    }
}

#[test]
fn ceilings_agree_at_their_boundaries() {
    let sources = [
        "[ 2 MUL 1 ADD 3 LT ] 'F' DEF 1 2 ADD F 4 5 ADD F 6 7 ADD F",
        "[ 2 MUL 1 ADD 3 LT ] 'F' DEF 1 20 RANGE [ [ 7 ] LENGTH 'X' BIND X F X 1 ADD 2 MUL F AND ] MAP",
    ];
    for source in sources {
        for steps in [
            0, 1, 2, 3, 5, 8, 9, 10, 11, 12, 14, 40, 79, 80, 81, 100, 150, 400,
        ] {
            let limits = Limits {
                steps: Some(steps),
                ..Limits::default()
            };
            assert_same(source, limits);
        }
        for work in [0, 1, 2, 3, 5, 6, 7, 20, 39, 40, 41, 100] {
            let limits = Limits {
                work: Some(work),
                ..Limits::default()
            };
            assert_same(source, limits);
        }
        for bits in [1, 8, 32, 63, 64, 65] {
            let limits = Limits {
                bits: Some(bits),
                ..Limits::default()
            };
            assert_same(source, limits);
        }
    }
}

/// The equalities above would hold if no segment ever ran; these pin that
/// they do, where they should.
#[test]
fn runs_are_segments() {
    // A body run on every call.
    assert_eq!(segment_runs("[ 2 MUL 1 ADD ] 'F' DEF 5 F 6 F"), 2);
    assert_eq!(segment_runs("[ 'X' BIND X X MUL ] 'SQ' DEF 5 SQ"), 1);
    // The top level runs once, and is walked.
    assert_eq!(segment_runs("1 2 ADD 3 MUL"), 0);
    // One Word alone is quickened, not a segment.
    assert_eq!(segment_runs("[ 2 ADD ] 'F' DEF 5 F"), 0);
    // Declined, then dispatched.
    assert_eq!(segment_runs("[ 0 DIV 2 ADD ] 'F' DEF 5 F"), 0);
    // A block's run, once per element.
    assert_eq!(
        segment_runs("1 10 RANGE [ [ 7 ] LENGTH ADD 'X' BIND X 2 MUL X ADD ] MAP"),
        10
    );
}

/// A Word the line redefines after a segment inlined it: the segment must
/// not run the old body.
#[test]
fn a_dictionary_change_retires_the_segments_lowered_against_it() {
    // The line's own `DEF` moves the dictionary between lowering and the
    // run that inlined `F`.
    assert_same(
        "[ 2 ] 'F' DEF [ 1 2 ] [ [ 10 ] 'F' DEF F 1 ADD 2 MUL ADD ] MAP",
        Limits::default(),
    );
    assert_same(
        "[ 2 ] 'F' DEF [ 1 2 3 ] [ 'X' BIND [ 10 ] 'F' DEF X F ADD F MUL ] MAP",
        Limits::default(),
    );
    assert_same(
        "[ 2 ] 'F' DEF [ F 1 ADD F MUL ] 'G' DEF [ 1 2 3 ] [ [ 9 ] LENGTH G ADD ] MAP",
        Limits::default(),
    );
}

/// A call chain past the depth guard: the dispatch refuses it, so a segment
/// that inlined the whole chain must decline, at any depth it is run from.
#[test]
fn the_call_depth_guard_holds_through_inlining() {
    let depth = crate::interpreter::interpreter_core::MAX_USER_WORD_DEPTH;
    for chain in [depth - 2, depth - 1, depth, depth + 1, depth + 2] {
        let mut source = String::from("[ 1 ADD 2 MUL ] 'W0' DEF\n");
        for k in 1..chain {
            source.push_str(&format!("[ W{} 3 ADD ] 'W{k}' DEF\n", k - 1));
        }
        source.push_str(&format!("1 W{} 2 MUL", chain - 1));
        assert_same(&source, Limits::default());
        source.push_str(&format!(
            "\n[ 1 2 ] [ [ 7 ] LENGTH ADD W{} ] MAP",
            chain - 1
        ));
        assert_same(&source, Limits::default());
    }
}

fn operand() -> impl Strategy<Value = String> {
    prop_oneof![
        6 => (-20i64..20).prop_map(|n| n.to_string()),
        2 => (-9i64..9, 1i64..9).prop_map(|(n, d)| format!("{n}/{d}")),
        1 => Just("0".to_string()),
        1 => Just("9223372036854775807".to_string()),
        1 => Just("-9223372036854775808".to_string()),
        1 => Just("NIL".to_string()),
        1 => Just("TRUE".to_string()),
        1 => Just("FALSE".to_string()),
        1 => Just("'s'".to_string()),
        1 => Just("[ 1 ]".to_string()),
        3 => Just("X".to_string()),
        2 => Just("Y".to_string()),
        2 => Just("F".to_string()),
    ]
}

fn word() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("ADD"),
        Just("SUB"),
        Just("MUL"),
        Just("DIV"),
        Just("LT"),
        Just("GT"),
        Just("EQ"),
        Just("MIN"),
        Just("MAX"),
        Just("AND"),
    ]
}

/// One step: a binary Word on a pushed operand, a unary Word, a SELECT, a
/// `BIND` of the top, or a call of the User Word `F`.
fn step() -> impl Strategy<Value = String> {
    let mask = prop_oneof![Just("TRUE"), Just("FALSE"), Just("1"), Just("X 0 LT")];
    prop_oneof![
        6 => (operand(), word()).prop_map(|(o, w)| format!("{o} {w}")),
        2 => prop_oneof![Just("FLOOR"), Just("ROUND"), Just("NOT")].prop_map(String::from),
        1 => (operand(), operand(), mask).prop_map(|(a, b, m)| format!("{a} {b} {m} SELECT")),
        1 => prop_oneof![Just("'Y' BIND Y"), Just("'X' BIND X X")].prop_map(String::from),
        1 => Just("[ 7 ] LENGTH ADD".to_string()),
    ]
}

fn body() -> impl Strategy<Value = String> {
    prop::collection::vec(step(), 1..8).prop_map(|steps| steps.join(" "))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn random_programs_agree(
        elements in prop::collection::vec(operand(), 1..6),
        callee in body(),
        code in body(),
        mode in 0usize..4,
        steps in prop::option::of(0usize..150),
        work in prop::option::of(0u64..60),
        bits in prop::option::of(1u64..130),
    ) {
        let elements = elements
            .into_iter()
            .map(|e| match e.as_str() {
                "X" | "Y" | "F" => "1".to_string(),
                _ => e,
            })
            .collect::<Vec<_>>()
            .join(" ");
        // `F` reads only what it binds; `X` and `Y` are its caller's.
        let callee = callee.replace(" X", " 2").replace(" Y", " 3").replace(" F", " 4");
        let callee = format!("[ 'X' BIND X {callee} ] 'F' DEF");
        let source = match mode {
            0 => format!("{callee} [ 'X' BIND X {code} ] 'W' DEF [ {elements} ] [ [ 7 ] LENGTH 7 SUB ADD W ] MAP"),
            1 => format!("{callee} [ {elements} ] [ [ 7 ] LENGTH 7 SUB ADD 'X' BIND 1 'Y' BIND X {code} ] MAP"),
            2 => format!("{callee} 2 'Y' BIND [ {elements} ] [ [ 7 ] LENGTH 7 SUB ADD 'X' BIND X {code} ] MAP"),
            _ => format!("{callee} 2 'Y' BIND [ {elements} ] 1 GET 'X' BIND X {code}"),
        };
        let limits = Limits { steps, work, bits };
        prop_assert_eq!(
            observe(&source, true, limits),
            observe(&source, false, limits),
            "`{}` under {:?}", source, limits
        );
    }
}
