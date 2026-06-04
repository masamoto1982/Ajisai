//! Semantic-domain generators (Phase 1 foundation).
//!
//! Each strategy yields an **Ajisai source program** that leaves exactly one
//! value of a given semantic domain on the stack. Property-based law tests
//! (`observation_laws.rs` and later phases) consume these so that adding a new
//! law for a new domain is "add a generator, write the equation" — the土台
//! the formalization-expansion roadmap §1.2-(T) asks for.
//!
//! Programs are deliberately small (cheap to run thousands of times) while
//! still covering the boundary values the roadmap §4 calls out: zero, sign,
//! NIL, irrationals, the logical Unknown, and empty/short vectors.

use proptest::prelude::*;

/// Small signed integers, including zero and both signs.
pub fn small() -> impl Strategy<Value = i64> {
    -20i64..=20
}

/// A nonzero divisor (keeps `_ _ /` away from the Bubble path when a value is
/// wanted; a separate [`nil_src`] exercises the division-by-zero NIL).
pub fn nonzero() -> impl Strategy<Value = i64> {
    (1i64..=20).prop_flat_map(|n| prop_oneof![Just(n), Just(-n)])
}

/// Pushes a single rational scalar (RawNumber role).
pub fn scalar_src() -> impl Strategy<Value = String> {
    prop_oneof![
        small().prop_map(|n| n.to_string()),
        (small(), nonzero()).prop_map(|(a, b)| format!("{a} {b} /")),
        (small(), small()).prop_map(|(a, b)| format!("{a} {b} ADD")),
        (small(), small()).prop_map(|(a, b)| format!("{a} {b} MUL")),
    ]
}

/// Pushes a single definite truth value (Boolean, TruthValue role).
pub fn boolean_src() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("TRUE".to_string()),
        Just("FALSE".to_string()),
        (small(), small()).prop_map(|(a, b)| format!("{a} {b} LT")),
        (small(), small()).prop_map(|(a, b)| format!("{a} {b} EQ")),
        (small(), small()).prop_map(|(a, b)| format!("{a} {b} GT")),
    ]
}

/// Pushes a reasoned NIL via the Bubble Rule (division by zero, §11.2).
pub fn nil_src() -> impl Strategy<Value = String> {
    small().prop_map(|n| format!("{n} 0 /"))
}

/// Radicands that are **not** perfect squares, so `√n` stays a genuine
/// irrational. (Probe finding: `√4`, `√9` collapse to the rationals `2`, `3`,
/// so a comparison of equal `√(square)` decides `true`, not the logical U.)
fn non_square_radicand() -> impl Strategy<Value = i64> {
    prop::sample::select(vec![2i64, 3, 5, 6, 7, 8, 10, 11, 13])
}

/// Pushes the logical Unknown (U): an undecidable comparison of equal
/// irrationals, `√n √n SUB 0 EQ` (confirmed by probe to render `UNKNOWN`).
pub fn unknown_src() -> impl Strategy<Value = String> {
    non_square_radicand()
        .prop_map(|n| format!("'math' IMPORT {n} MATH@SQRT {n} MATH@SQRT SUB 0 EQ"))
}

/// Pushes an irrational exact-real scalar `√n` (n not a perfect square, so the
/// value is a genuine lazy continued fraction).
pub fn irrational_src() -> impl Strategy<Value = String> {
    non_square_radicand().prop_map(|n| format!("'math' IMPORT {n} MATH@SQRT"))
}

/// Pushes a single vector/tensor literal (1–3 elements).
pub fn vector_src() -> impl Strategy<Value = String> {
    prop::collection::vec(small(), 1..4).prop_map(|xs| {
        let body = xs.iter().map(i64::to_string).collect::<Vec<_>>().join(" ");
        format!("[ {body} ]")
    })
}

/// Pushes a single code block (callable).
pub fn block_src() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("{ 1 ADD }".to_string()),
        Just("{ DUP MUL }".to_string()),
        Just("{ }".to_string()),
    ]
}

/// Union over every semantic domain — the "any well-formed value" generator.
pub fn any_value_src() -> impl Strategy<Value = String> {
    prop_oneof![
        scalar_src(),
        boolean_src(),
        nil_src(),
        unknown_src(),
        irrational_src(),
        vector_src(),
        block_src(),
    ]
}
