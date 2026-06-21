//! Phase 2 — syntax / desugar soundness as executable laws.
//!
//! Companion to `algebraic_laws.rs`, encoding
//! `docs/dev/ajisai-formalization-expansion-roadmap.md` Phase 2: the surface
//! desugaring of SPEC §3.9 / §7.0 is *observationally transparent*. Every
//! symbolic alias renders identically to its English-word canonical form, the
//! `FLOW` (`~`) marker is a no-op, the `TOP-EAT` shorthand `;` equals `. ,`,
//! and word names are case-normalized (§3.8). Each law is the compressed form
//! of infinitely many tokenizer conformance cases: if desugaring were not
//! `⟦desugar(s)⟧ = ⟦s⟧`, some generated pair would render differently.
//!
//! Observation is structured, not a display-string fragment: laws compare stack
//! renders plus semantic axes (including NIL/UNKNOWN absence diagnosis), effect
//! trace, and error category.

mod test_support;

use proptest::prelude::*;
use test_support::observe::{observe_program, ProgramObservation};

fn assert_law(name: &str, lhs: &str, rhs: &str) {
    let l = observe_program(lhs);
    let r = observe_program(rhs);
    assert_eq!(
        l, r,
        "law `{name}` broken:\n  {lhs:?} => {l:#?}\n  {rhs:?} => {r:#?}"
    );
}

fn observed(src: &str) -> ProgramObservation {
    observe_program(src)
}

fn small() -> impl Strategy<Value = i64> {
    -50i64..=50
}
fn nonzero() -> impl Strategy<Value = i64> {
    (1i64..=50).prop_flat_map(|n| prop_oneof![Just(n), Just(-n)])
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    // ── Arithmetic aliases (§3.9 Word alias): + - * / % ──
    #[test]
    fn arith_aliases(a in small(), b in nonzero()) {
        assert_law("alias-add", &format!("{a} {b} +"), &format!("{a} {b} ADD"));
        assert_law("alias-sub", &format!("{a} {b} -"), &format!("{a} {b} SUB"));
        assert_law("alias-mul", &format!("{a} {b} *"), &format!("{a} {b} MUL"));
        assert_law("alias-div", &format!("{a} {b} /"), &format!("{a} {b} DIV"));
        assert_law("alias-mod", &format!("{a} {b} %"), &format!("{a} {b} MOD"));
    }

    // ── Comparison aliases (§3.9): = <> < <= > >= ──
    #[test]
    fn comparison_aliases(a in small(), b in small()) {
        assert_law("alias-eq",  &format!("{a} {b} ="),  &format!("{a} {b} EQ"));
        assert_law("alias-neq", &format!("{a} {b} <>"), &format!("{a} {b} NEQ"));
        assert_law("alias-lt",  &format!("{a} {b} <"),  &format!("{a} {b} LT"));
        assert_law("alias-lte", &format!("{a} {b} <="), &format!("{a} {b} LTE"));
        assert_law("alias-gt",  &format!("{a} {b} >"),  &format!("{a} {b} GT"));
        assert_law("alias-gte", &format!("{a} {b} >="), &format!("{a} {b} GTE"));
    }

    // ── FLOW (`~`) is a no-op visual separator (§6.4) ──
    #[test]
    fn pipe_is_noop(a in small(), b in small()) {
        assert_law("pipe-noop", &format!("{a} {b} ~ ADD"), &format!("{a} {b} ADD"));
    }

    // ── Word-name case normalization (§3.8): add ≡ Add ≡ ADD ──
    #[test]
    fn case_normalization(a in small(), b in small()) {
        assert_law("case-lower", &format!("{a} {b} add"), &format!("{a} {b} ADD"));
        assert_law("case-mixed", &format!("{a} {b} Add"), &format!("{a} {b} ADD"));
    }

    // ── TOP-EAT shorthand `;` ≡ `. ,` ≡ default (§3.9 Modifier sugar) ──
    #[test]
    fn top_eat_shorthand(a in small(), b in small()) {
        assert_law("semicolon-dot-comma", &format!("{a} {b} ; ADD"), &format!("{a} {b} . , ADD"));
        assert_law("semicolon-default",   &format!("{a} {b} ; ADD"), &format!("{a} {b} ADD"));
    }
}

#[test]
fn arithmetic_alias_preserves_nil_absence_metadata() {
    let alias = observed("1 0 /");
    let canonical = observed("1 0 DIV");
    assert_eq!(alias, canonical);
    let top = alias.stack.last().expect("division leaves a value");
    let absence = top
        .axes
        .absence
        .as_ref()
        .expect("division by zero projects structured NIL");
    assert_eq!(absence.reason, Some("divisionByZero"));
    // Origin is still a structured field and is compared above through the
    // full ProgramObservation equality; the current runtime tags this as the
    // execution site rather than the arithmetic domain site.
    assert!(!absence.origin.is_empty());
    assert!(
        alias.effects.is_empty(),
        "arithmetic sugar must not emit effects"
    );
    assert_eq!(alias.error_category, None);
}

#[test]
fn comparison_alias_preserves_unknown_diagnosis() {
    let lhs = "'math' IMPORT 2 SQRT 1 ADD 2 SQRT 1 ADD SUB 0 =";
    let rhs = "'math' IMPORT 2 SQRT 1 ADD 2 SQRT 1 ADD SUB 0 EQ";
    let alias = observed(lhs);
    let canonical = observed(rhs);
    assert_eq!(alias, canonical);
    let top = alias.stack.last().expect("comparison leaves a value");
    assert_eq!(top.axes.truth_value, Some("unknown"));
    let absence = top
        .axes
        .absence
        .as_ref()
        .expect("logical UNKNOWN carries structured metadata");
    assert_eq!(absence.reason, Some("logicallyUnknown"));
    assert!(
        absence.diagnosis.is_some(),
        "UNKNOWN comparison should preserve AI-readable diagnosis"
    );
    assert_eq!(alias.effects, canonical.effects);
    assert_eq!(alias.error_category, None);
}

#[test]
fn alias_error_category_is_observationally_transparent() {
    let alias = observed("+");
    let canonical = observed("ADD");
    assert_eq!(alias, canonical);
    assert_eq!(alias.error_category, Some("stackUnderflow"));
    assert!(alias.stack.is_empty());
    assert!(alias.effects.is_empty());
}

// ── AND alias `&` over the three-valued domain {T, F, U} (§7.5) ──
fn truths() -> [(&'static str, &'static str); 3] {
    [
        ("T", "TRUE"),
        ("F", "FALSE"),
        ("U", "'MATH' IMPORT 2 SQRT 1 ADD 2 SQRT 1 ADD SUB 0 EQ"),
    ]
}

#[test]
fn and_alias_over_k3() {
    for (na, a) in truths() {
        for (nb, b) in truths() {
            assert_law(
                &format!("alias-and[{na},{nb}]"),
                &format!("{a} {b} &"),
                &format!("{a} {b} AND"),
            );
        }
    }
}
