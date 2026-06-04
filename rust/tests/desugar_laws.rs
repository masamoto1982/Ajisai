//! Phase 2 — syntax / desugar soundness as executable laws.
//!
//! Companion to `algebraic_laws.rs`, encoding
//! `docs/dev/ajisai-formalization-expansion-roadmap.md` Phase 2: the surface
//! desugaring of SPEC §3.9 / §7.0 is *observationally transparent*. Every
//! symbolic alias renders identically to its English-word canonical form, the
//! `PIPE` (`==`) marker is a no-op, the `TOP-EAT` shorthand `;` equals `. ,`,
//! and word names are case-normalized (§3.8). Each law is the compressed form
//! of infinitely many tokenizer conformance cases: if desugaring were not
//! `⟦desugar(s)⟧ = ⟦s⟧`, some generated pair would render differently.
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

    // ── PIPE (`==`) is a no-op visual separator (§6.4) ──
    #[test]
    fn pipe_is_noop(a in small(), b in small()) {
        assert_law("pipe-noop", &format!("{a} {b} == ADD"), &format!("{a} {b} ADD"));
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

// ── AND alias `&` over the three-valued domain {T, F, U} (§7.5) ──
fn truths() -> [(&'static str, &'static str); 3] {
    [
        ("T", "TRUE"),
        ("F", "FALSE"),
        ("U", "'MATH' IMPORT 2 SQRT 2 SQRT SUB 0 EQ"),
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
