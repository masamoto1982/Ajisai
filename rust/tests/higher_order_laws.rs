//! Phase 4 — higher-order / control words as recursion schemes (executable laws).
//!
//! Encodes `docs/dev/ajisai-formalization-expansion-roadmap.md` Phase 4: the
//! control vocabulary of SPEC §7.7 obeys the algebraic laws of its categorical
//! models — `MAP` is a functor lift, `FOLD` a catamorphism, `FILTER` a
//! predicate restriction, `ANY`/`ALL` existential/universal quantifiers,
//! `EXEC`/`EVAL` reflection of `⟦·⟧`, and `COND` a K3-honest guarded case in
//! which a `unknown` (U) guard does not fire (§7.4.3).
//!
//! Observation matches the conformance runner: whole-stack `Value::to_string`.

use ajisai_core::interpreter::Interpreter;
use proptest::prelude::*;

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

fn assert_law(name: &str, lhs: &str, rhs: &str) {
    let l = eval(lhs);
    let r = eval(rhs);
    assert_eq!(
        l, r,
        "law `{name}` broken:\n  {lhs:?} => {l}\n  {rhs:?} => {r}"
    );
}

fn small() -> impl Strategy<Value = i64> {
    -50i64..=50
}

/// Render a slice of integers as an Ajisai vector literal `[ a b c ]`.
fn vlit(xs: &[i64]) -> String {
    let body = xs
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    format!("[ {body} ]")
}

/// A non-empty vector (empty vectors are NIL in Ajisai, SPEC §4.5).
fn vec_ne() -> impl Strategy<Value = Vec<i64>> {
    prop::collection::vec(small(), 1..=5)
}
/// Exactly three operands, for fold/triple laws.
fn triple() -> impl Strategy<Value = (i64, i64, i64)> {
    (small(), small(), small())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    // ── MAP is a functor: identity and composition (fusion) ──

    /// `MAP id = id`: an identity block leaves the vector unchanged.
    #[test]
    fn map_identity(xs in vec_ne()) {
        let v = vlit(&xs);
        assert_law("map-id-empty-block", &format!("{v} {{ }} MAP"), &v);
        assert_law("map-id-dot-block", &format!("{v} {{ . }} MAP"), &v);
    }

    /// Map fusion `MAP g ∘ MAP f = MAP (g∘f)` (functoriality of the lift).
    #[test]
    fn map_fusion(xs in vec_ne()) {
        let v = vlit(&xs);
        assert_law(
            "map-fusion",
            &format!("{v} {{ 2 * }} MAP {{ 1 + }} MAP"),
            &format!("{v} {{ 2 * 1 + }} MAP"),
        );
    }

    // ── FOLD is a catamorphism over the additive/multiplicative monoid ──

    /// Left fold with `ADD` from `0` equals the explicit left-associated sum.
    #[test]
    fn fold_add_catamorphism((a, b, c) in triple()) {
        assert_law(
            "fold-add",
            &format!("[ {a} {b} {c} ] 0 {{ ADD }} FOLD"),
            &format!("{a} {b} ADD {c} ADD"),
        );
    }

    /// Left fold with `MUL` from `1` equals the explicit left-associated product.
    #[test]
    fn fold_mul_catamorphism((a, b, c) in triple()) {
        assert_law(
            "fold-mul",
            &format!("[ {a} {b} {c} ] 1 {{ MUL }} FOLD"),
            &format!("{a} {b} MUL {c} MUL"),
        );
    }

    // ── EXEC / EVAL reflect ⟦·⟧ ──

    /// `EXEC` of a block is its inline expansion.
    #[test]
    fn exec_is_inline(a in small(), b in small()) {
        assert_law("exec-inline", &format!("{a} {b} {{ ADD }} EXEC"), &format!("{a} {b} ADD"));
    }

    /// `EVAL` of source text reflects the meaning function: `⟦EVAL(STR p)⟧ = ⟦p⟧`.
    #[test]
    fn eval_reflects_meaning(a in small(), b in small()) {
        assert_law("eval-reflection", &format!("'{a} {b} ADD' EVAL"), &format!("{a} {b} ADD"));
    }
}

// ── FILTER restriction laws (fixed vectors avoid empty→NIL results) ──

#[test]
fn filter_is_idempotent() {
    for v in ["[ 1 2 3 4 5 ]", "[ 5 4 3 2 1 ]", "[ 3 1 4 1 5 ]"] {
        assert_law(
            "filter-idempotent",
            &format!("{v} {{ 2 > }} FILTER {{ 2 > }} FILTER"),
            &format!("{v} {{ 2 > }} FILTER"),
        );
    }
}

#[test]
fn filter_predicates_commute() {
    for v in ["[ 1 2 3 4 5 ]", "[ 5 4 3 2 1 ]"] {
        assert_law(
            "filter-commute",
            &format!("{v} {{ 2 > }} FILTER {{ 4 < }} FILTER"),
            &format!("{v} {{ 4 < }} FILTER {{ 2 > }} FILTER"),
        );
    }
}

// ── ANY / ALL De Morgan duality: ALL p ≡ ¬ ANY ¬p ──

#[test]
fn any_is_count_positive_projection() {
    let cases = [
        ("[ 1 2 3 4 5 ]", "2 >"),
        ("[ 1 2 3 ]", "5 >"),
        ("[ -1 0 1 ]", "0 >"),
    ];
    for (v, p) in cases {
        assert_law(
            &format!("any-count-positive[{v};{p}]"),
            &format!("{v} {{ {p} }} ANY"),
            &format!("{v} {{ {p} }} COUNT [ 0 ] GT"),
        );
    }
}

#[test]
fn all_any_de_morgan() {
    let cases = [
        ("[ 1 2 3 ]", "0 >"),
        ("[ 1 2 3 ]", "2 >"),
        ("[ 1 2 3 ]", "5 >"),
        ("[ -1 -2 -3 ]", "0 <"),
    ];
    for (v, p) in cases {
        assert_law(
            &format!("all-is-not-any-not[{v};{p}]"),
            &format!("{v} {{ {p} }} ALL"),
            &format!("{v} {{ {p} NOT }} ANY NOT"),
        );
    }
}

// ── SCAN exposes the catamorphism's intermediate accumulators ──

#[test]
fn scan_yields_prefix_sums() {
    assert_law(
        "scan-prefix-sums",
        "[ 1 2 3 4 ] 0 { ADD } SCAN",
        "[ 1 3 6 10 ]",
    );
}

// ── COND is K3-honest: a U guard does not fire (§7.4.3) ──
//
// A guard reducing to `unknown` (an undecidable CF comparison) must fall
// through exactly like a `false` guard, while a definite `true` fires.

#[test]
fn cond_u_guard_does_not_fire() {
    let u_guard = "{ 2 SQRT 1 ADD 2 SQRT 1 ADD SUB 0 EQ }";
    let prog = |guard: &str| {
        format!("'MATH' IMPORT [ 1 ] {guard} {{ 'fired' }} {{ IDLE }} {{ 'else' }} COND")
    };
    // U guard falls through to the else clause, identically to a FALSE guard.
    assert_law("cond-u-like-false", &prog(u_guard), &prog("{ FALSE }"));
    // And the else branch is what actually runs (not the guarded body).
    assert_eq!(
        eval(&prog(u_guard)),
        eval("'else'"),
        "U guard should reach else"
    );
    // A definite TRUE guard, by contrast, fires its body.
    assert_eq!(
        eval(&prog("{ TRUE }")),
        eval("'fired'"),
        "TRUE guard should fire"
    );
}
