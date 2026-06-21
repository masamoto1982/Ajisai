//! Property-based algebraic-law conformance.
//!
//! These tests encode the algebraic laws of
//! `docs/dev/ajisai-mathematical-formalization.md` §9.2 as executable
//! properties: instead of enumerating finitely many input/output pairs (as the
//! HTML conformance suite does), each law asserts an equation that must hold for
//! *all* inputs in a generated sample. A law is the compressed form of
//! infinitely many conformance cases, so this file is the "equation-level
//! continuous verification" companion to `tests/conformance/`.
//!
//! Scope: the laws asserted here are consistent with `SPECIFICATION.html`. They
//! include the strong-Kleene K3 logic laws over {TRUE, FALSE, UNKNOWN}, which
//! pass now that truth values are a distinct data-plane kind rendering
//! uniformly as TRUE/FALSE/UNKNOWN (findings B1/B2) and irrationals render as
//! exact nested continued fractions (finding C).
//!
//! Observation: two programs are "equal" when their whole-stack rendering
//! (`Value::to_string`, the same surface the conformance runner observes) is
//! identical.

use ajisai_core::interpreter::Interpreter;
use proptest::prelude::*;

/// Run an Ajisai program and render the whole final stack value-by-value, the
/// same observation the conformance runner uses. Panics on execution error so a
/// malformed law program is loud rather than silently skipped.
fn eval(src: &str) -> String {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("tokio current-thread runtime");
    rt.block_on(async {
        let mut interp = Interpreter::new();
        interp
            .execute(src)
            .await
            .unwrap_or_else(|e| panic!("program failed: {src:?}: {e}"));
        interp
            .get_stack()
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    })
}

/// Assert two Ajisai programs are observationally equal.
fn assert_law(name: &str, lhs: &str, rhs: &str) {
    let l = eval(lhs);
    let r = eval(rhs);
    assert_eq!(
        l, r,
        "law `{name}` broken:\n  {lhs:?} => {l}\n  {rhs:?} => {r}"
    );
}

// Small integer operands keep generated programs cheap while still exercising
// BigInt-backed exact arithmetic across sign and zero.
fn small() -> impl Strategy<Value = i64> {
    -50i64..=50
}
fn nonzero() -> impl Strategy<Value = i64> {
    (1i64..=50).prop_flat_map(|n| prop_oneof![Just(n), Just(-n)])
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    // ─────────────────── Monoid of state transformers (§2) ───────────────────

    /// `IDLE` is the identity of program composition: `p IDLE ≡ p`.
    #[test]
    fn monoid_identity(a in small()) {
        assert_law("monoid-identity", &format!("{a} IDLE"), &format!("{a}"));
    }

    // ─────────────────── Exact-rational field laws (§3, 𝔸) ───────────────────

    #[test]
    fn add_commutative(a in small(), b in small()) {
        assert_law("add-comm", &format!("{a} {b} ADD"), &format!("{b} {a} ADD"));
    }

    #[test]
    fn add_associative(a in small(), b in small(), c in small()) {
        assert_law(
            "add-assoc",
            &format!("{a} {b} ADD {c} ADD"),
            &format!("{a} {b} {c} ADD ADD"),
        );
    }

    #[test]
    fn mul_commutative(a in small(), b in small()) {
        assert_law("mul-comm", &format!("{a} {b} MUL"), &format!("{b} {a} MUL"));
    }

    #[test]
    fn mul_associative(a in small(), b in small(), c in small()) {
        assert_law(
            "mul-assoc",
            &format!("{a} {b} MUL {c} MUL"),
            &format!("{a} {b} {c} MUL MUL"),
        );
    }

    /// Multiplication distributes over addition: `(a + b) · c = a·c + b·c`.
    #[test]
    fn mul_distributes_over_add(a in small(), b in small(), c in small()) {
        assert_law(
            "mul-distrib",
            &format!("{a} {b} ADD {c} MUL"),
            &format!("{a} {c} MUL {b} {c} MUL ADD"),
        );
    }

    /// Additive identity, multiplicative identity, and additive inverse.
    #[test]
    fn field_units_and_inverse(a in small()) {
        assert_law("add-ident", &format!("{a} 0 ADD"), &format!("{a}"));
        assert_law("mul-ident", &format!("{a} 1 MUL"), &format!("{a}"));
        assert_law("self-sub-zero", &format!("{a} {a} SUB"), "0");
    }

    /// Exact division round-trips: `(a / b) · b = a` for `b ≠ 0`. This is the
    /// rational-domain analogue of finding C's `x a ADD a SUB ≡ x`; it holds
    /// here because finite CFs decide and never approximate.
    #[test]
    fn div_mul_roundtrip(a in small(), b in nonzero()) {
        assert_law(
            "div-mul-roundtrip",
            &format!("{a} {b} DIV {b} MUL"),
            &format!("{a}"),
        );
    }

    /// Exact finite comparisons are dual observations over the shared budgeted-order primitive.
    #[test]
    fn comparison_dualities(a in small(), b in small()) {
        assert_law("lt-gt-dual", &format!("{a} {b} LT"), &format!("{b} {a} GT"));
        assert_law("lte-gte-dual", &format!("{a} {b} LTE"), &format!("{b} {a} GTE"));
        assert_law("neq-eq-not", &format!("{a} {b} NEQ"), &format!("{a} {b} EQ NOT"));
    }

    // ─────────────────── Bubble/NIL monad (§5) ───────────────────

    /// NIL passthrough: any arithmetic on a division-by-zero Bubble stays NIL.
    #[test]
    fn nil_passthrough(a in small()) {
        assert_law("nil-passthrough", &format!("1 0 DIV {a} ADD"), "NIL");
    }

    /// VENT handler: a Bubble is replaced by the fallback (verified operand
    /// order `Bubble ^ fallback`, SPEC §6.4), a present value is kept.
    #[test]
    fn or_nil_handler(a in small()) {
        assert_law("or-nil-bubble", &format!("1 0 DIV ^ {a}"), &format!("{a}"));
        // A non-NIL value is its own left-biased result regardless of fallback.
        assert_law("or-nil-present", &format!("{a} ^ 999"), &format!("{a}"));
    }
}

/// MOD is the Euclidean remainder induced by floor division: x - floor(x/y)·y.
#[test]
fn mod_floor_remainder_examples() {
    assert_law("mod-positive", "7 3 MOD", "1");
    assert_law("mod-negative-dividend", "-7 3 MOD", "2");
}

/// Integer projections are exact-real observations, not float round trips.
#[test]
fn integer_projection_examples() {
    assert_law("floor-positive", "7 3 DIV FLOOR", "2");
    assert_law("floor-negative", "-7 3 DIV FLOOR", "-3");
    assert_law("ceil-positive", "7 3 DIV CEIL", "3");
    assert_law("ceil-negative", "-7 3 DIV CEIL", "-2");
    assert_law("round-positive-half", "5 2 DIV ROUND", "3");
    assert_law("round-negative-half", "-5 2 DIV ROUND", "-3");
}

// ─────────────────── Strong Kleene three-valued logic K3 (§4) ───────────────────
//
// K3 laws are checked exhaustively over the truth domain {TRUE, FALSE, U}.
// `U` is produced by an undecidable continued-fraction comparison
// (SPEC §7.4.1): `2 SQRT 1 ADD 2 SQRT 1 ADD SUB 0 EQ` compares the composed
// Gosper value (√2+1) − (√2+1) against 0 and exhausts the budget. (Plain
// √2 − √2 now collapses to an exact 0 in closed form and would decide.)
// Each law renders both sides through the identical path,
// so the equation is independent of how a truth value is displayed (finding B).

/// The three truth-domain generators as Ajisai source fragments.
fn truths() -> [(&'static str, &'static str); 3] {
    [
        ("T", "TRUE"),
        ("F", "FALSE"),
        ("U", "'math' IMPORT 2 SQRT 1 ADD 2 SQRT 1 ADD SUB 0 EQ"),
    ]
}

#[test]
fn k3_double_negation() {
    for (name, t) in truths() {
        assert_law(&format!("double-neg[{name}]"), &format!("{t} NOT NOT"), t);
    }
}

#[test]
fn k3_and_or_commutative() {
    for (na, a) in truths() {
        for (nb, b) in truths() {
            assert_law(
                &format!("and-comm[{na},{nb}]"),
                &format!("{a} {b} AND"),
                &format!("{b} {a} AND"),
            );
            assert_law(
                &format!("or-comm[{na},{nb}]"),
                &format!("{a} {b} OR"),
                &format!("{b} {a} OR"),
            );
        }
    }
}

// De Morgan over {T, F, U}. Truth values now render uniformly as
// TRUE/FALSE/UNKNOWN through every path (finding B fixed), so both sides of
// each law render identically when they denote the same truth value.
#[test]
fn k3_de_morgan() {
    for (na, a) in truths() {
        for (nb, b) in truths() {
            // ¬(a ∧ b) = ¬a ∨ ¬b
            assert_law(
                &format!("de-morgan-and[{na},{nb}]"),
                &format!("{a} {b} AND NOT"),
                &format!("{a} NOT {b} NOT OR"),
            );
            // ¬(a ∨ b) = ¬a ∧ ¬b
            assert_law(
                &format!("de-morgan-or[{na},{nb}]"),
                &format!("{a} {b} OR NOT"),
                &format!("{a} NOT {b} NOT AND"),
            );
        }
    }
}

#[test]
fn k3_associativity_and_idempotence() {
    let ts = truths();
    for (na, a) in ts {
        // Idempotence: a ∧ a = a, a ∨ a = a.
        assert_law(&format!("and-idem[{na}]"), &format!("{a} {a} AND"), a);
        assert_law(&format!("or-idem[{na}]"), &format!("{a} {a} OR"), a);
        for (nb, b) in ts {
            for (nc, c) in ts {
                assert_law(
                    &format!("and-assoc[{na},{nb},{nc}]"),
                    &format!("{a} {b} AND {c} AND"),
                    &format!("{a} {b} {c} AND AND"),
                );
                assert_law(
                    &format!("or-assoc[{na},{nb},{nc}]"),
                    &format!("{a} {b} OR {c} OR"),
                    &format!("{a} {b} {c} OR OR"),
                );
            }
        }
    }
}
