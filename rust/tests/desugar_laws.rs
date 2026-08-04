//! Phase 2 — syntax / desugar soundness as executable laws.
//!
//! Companion to `algebraic_laws.rs`, encoding
//! `docs/dev/ajisai-formalization-expansion-roadmap.md` Phase 2: the surface
//! desugaring of SPEC §3.9 / §7.0 is *observationally transparent*. Every
//! symbolic alias renders identically to its English-word canonical form, and
//! word names are case-normalized (§3.8). Each law is the compressed form
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
        // Every symbol is one character; LTE, GTE and NEQ are reached by name.
        assert_law("alias-eq", &format!("{a} {b} ="), &format!("{a} {b} EQ"));
        assert_law("alias-lt", &format!("{a} {b} <"), &format!("{a} {b} LT"));
        assert_law("alias-gt", &format!("{a} {b} >"), &format!("{a} {b} GT"));
    }

    // ── An unallocated symbol is not a silent no-op ──
    //
    // Desugaring is semantics-preserving (LANG.SOURCE.DESUGAR), which cuts both
    // ways: a symbol the language has not allocated must not quietly disappear
    // from a program, and neither must a retired spelling. Each of these reaches
    // the dictionary as an ordinary name and fails there: `~` was never
    // allocated, `&` no longer spells AND, and `<=`, `>=`, `<>` and `,,` are
    // two-character spellings that no longer exist.
    #[test]
    fn an_unallocated_symbol_is_not_a_silent_noop(a in small(), b in small()) {
        for symbol in ["~", "&", "<=", ">=", "<>", ",,"] {
            let observation = observed(&format!("{a} {b} {symbol} ADD"));
            prop_assert_eq!(
                observation.error_category,
                Some("unknownWord"),
                "`{}` must reach the dictionary as a name",
                symbol
            );
        }
    }

    // ── Word-name case normalization (§3.8): add ≡ Add ≡ ADD ──
    #[test]
    fn case_normalization(a in small(), b in small()) {
        assert_law("case-lower", &format!("{a} {b} add"), &format!("{a} {b} ADD"));
        assert_law("case-mixed", &format!("{a} {b} Add"), &format!("{a} {b} ADD"));
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
fn comparison_alias_decides_composed_equality_identically() {
    // The bare relations are total over the admitted domain (SPEC §4.2.7 /
    // §7.4): (√2+1)−(√2+1) EQ 0 decides TRUE, and the `=` alias observes
    // identically. (This law formerly pinned the UNKNOWN diagnosis here;
    // with comparison total over D, UNKNOWN is confined to COMPARE-WITHIN,
    // which has no alias sugar to desugar.)
    let lhs = "2 SQRT 1 ADD 2 SQRT 1 ADD SUB 0 =";
    let rhs = "2 SQRT 1 ADD 2 SQRT 1 ADD SUB 0 EQ";
    let alias = observed(lhs);
    let canonical = observed(rhs);
    assert_eq!(alias, canonical);
    let top = alias.stack.last().expect("comparison leaves a value");
    assert_eq!(top.axes.truth_value, Some("true"));
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
