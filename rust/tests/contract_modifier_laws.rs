//! Property-based contract / modifier / mass-conservation laws (Phase 3 ⭐).
//!
//! Encodes the algebraic content of
//! `docs/dev/ajisai-mathematical-formalization.md` §9-quater E (Phase 3):
//!
//! 1. **Modifier combinators** (`SPEC §6`): `⟦μ·w⟧ = κ_consume ∘ δ_region ∘
//!    base(w)`. `EAT` is the identity default and `KEEP` is bifurcation.
//! 2. **Coreword contracts** (`SPEC §7.14`): the `partiality` / `nil_policy` /
//!    `safety_level` lattices, with contract absence = conformance violation.
//! 3. **Static mass conservation** (`SPEC §13`): consumption/production as a
//!    resource (linear) discipline, observed here via stack-depth deltas
//!    (`depth(KEEP w) − depth(EAT w) = arity`).
//!
//! Every law was checked against the reference implementation with a throwaway
//! probe before being written (roadmap §1.2-(T) discipline). Probe findings are
//! recorded as §9-quater E.5 findings; the two that are tracked oracles
//! are asserted as guarded
//! invariants so a future drift is loud.

mod test_support;

use ajisai_core::coreword_registry::{
    get_builtin_word_registry, get_coreword_metadata, NilPolicy, Partiality, SafetyLevel,
    WordPurity,
};
use proptest::prelude::*;
use test_support::generators::small;
use test_support::observe::{render, run};

// ─────────────────────────── observation helpers ───────────────────────────

/// Whole-stack rendering (one value per element), the conformance observation.
fn obs(src: &str) -> Vec<String> {
    run(src).iter().map(|v| render(v, v.hint)).collect()
}

/// Stack depth after running `src` (mass observation).
fn depth(src: &str) -> usize {
    run(src).len()
}

/// Total binary scalar→scalar words (never error / NIL on integer operands).
fn binary_arith() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("ADD"), Just("MUL"), Just("SUB")]
}

// ───────────────────────── modifier algebra (§6, §13.2) ─────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// `EAT` is the identity default of the one modifier axis: the bare word
    /// and every default-modifier spelling render the same stack
    /// (LANG.MODIFIERS.CONSUMPTION; the sugar `,` ≡ EAT).
    #[test]
    fn default_modifiers_are_identities(a in small(), b in small(), w in binary_arith()) {
        let bare = obs(&format!("{a} {b} {w}"));
        for variant in ["EAT", ","] {
            prop_assert_eq!(&bare, &obs(&format!("{a} {b} {variant} {w}")));
        }
    }

    /// `KEEP` is bifurcation (§13.2): operands are retained *and* the result is
    /// pushed. Observationally `a b KEEP w == (a b) ++ (a b w)`. The sugar
    /// `,,`≡KEEP (SPEC §6.2).
    #[test]
    fn keep_is_bifurcation(a in small(), b in small(), w in binary_arith()) {
        let mut expected = obs(&format!("{a} {b}"));
        expected.extend(obs(&format!("{a} {b} {w}")));
        prop_assert_eq!(&expected, &obs(&format!("{a} {b} KEEP {w}")));
        prop_assert_eq!(&expected, &obs(&format!("{a} {b} ,, {w}")));
    }

    /// **Mass conservation / bifurcation arity** (§13.1/§13.2): for a binary
    /// word the only stack-mass difference between `KEEP` and `EAT` is the two
    /// retained operands, so `depth(KEEP w) − depth(EAT w) = arity = 2`.
    #[test]
    fn keep_minus_eat_equals_arity(a in small(), b in small(), w in binary_arith()) {
        let eat = depth(&format!("{a} {b} EAT {w}")) as i64;
        let keep = depth(&format!("{a} {b} KEEP {w}")) as i64;
        prop_assert_eq!(keep - eat, 2);
    }
    // ──────────── partiality contract ↔ observable behavior (§7.14) ──────────

    /// A `Total` word never errors on well-shaped input: it always leaves a
    /// value (Hoare `ensures` discharged), here over total binary arithmetic.
    #[test]
    fn total_words_do_not_error(a in small(), b in small(), w in binary_arith()) {
        prop_assert_eq!(depth(&format!("{a} {b} {w}")), 1);
    }
}

// ─────────────────── projecting words bubble domain misses ──────────────────

/// `Projecting`/`CreatesNil` words project a well-formed domain miss onto NIL
/// rather than raising (SPEC §7.14, Bubble Rule §11.2): division by zero and an
/// out-of-range `GET` both yield NIL, not an error.
#[test]
fn projecting_words_bubble_domain_misses() {
    assert_eq!(obs("1 0 DIV"), vec!["NIL"]);
    assert_eq!(obs("1 0 /"), vec!["NIL"]);
    // GET is non-consuming (probe finding E3): it keeps its source vector and
    // pushes the projected NIL for an out-of-range index.
    assert_eq!(obs("[ 1 2 3 ] 9 GET"), vec!["[ 1/1 2/1 3/1 ]", "NIL"]);
}

// ──────────────────────── contract lattice laws (§7.14) ─────────────────────

/// Every built-in carries a contract reachable by its own name, with all three
/// classification fields in their declared domains. A Coreword without a
/// contract entry is a conformance violation (SPEC §7.14).
#[test]
fn every_coreword_declares_a_reachable_contract() {
    let reg = get_builtin_word_registry();
    assert!(!reg.is_empty());
    for m in reg {
        assert!(
            get_coreword_metadata(&m.name).is_some(),
            "{} has no reachable contract",
            m.name
        );
        assert!(matches!(
            m.partiality,
            Partiality::Total | Partiality::Partial | Partiality::Projecting
        ));
        assert!(matches!(
            m.nil_policy,
            NilPolicy::Passthrough
                | NilPolicy::CreatesNil
                | NilPolicy::RejectsNil
                | NilPolicy::ConsumesNil
                | NilPolicy::PreservesReason
        ));
        assert!(matches!(
            m.safety_level,
            SafetyLevel::A
                | SafetyLevel::B
                | SafetyLevel::C
                | SafetyLevel::D
                | SafetyLevel::Quarantined
        ));
    }
}

/// Safety-level lattice (§7.14): `A` (the strongest) implies pure and
/// deterministic; effectful words sit strictly above `B`. These hold over the
/// whole registry (probe-confirmed: 0 counterexamples).
#[test]
fn safety_lattice_is_monotone() {
    for m in get_builtin_word_registry() {
        if m.safety_level == SafetyLevel::A {
            assert_eq!(m.purity, WordPurity::Pure, "{} A must be pure", m.name);
            assert!(m.deterministic, "{} A must be deterministic", m.name);
            // SPEC §7.14: A is reserved for *total* words. `Projecting` is total
            // by projection (failures land on NIL), so it qualifies; `Partial`
            // does not (finding E2, resolved).
            assert!(
                matches!(m.partiality, Partiality::Total | Partiality::Projecting),
                "{} A must be total (or total-by-projection), got {:?}",
                m.name,
                m.partiality
            );
        }
        if !m.effects.is_empty() {
            assert!(
                matches!(
                    m.safety_level,
                    SafetyLevel::C | SafetyLevel::D | SafetyLevel::Quarantined
                ),
                "{} has effects but safety {:?}",
                m.name,
                m.safety_level
            );
        }
        if m.purity == WordPurity::Effectful {
            assert!(
                matches!(
                    m.safety_level,
                    SafetyLevel::C | SafetyLevel::D | SafetyLevel::Quarantined
                ),
                "{} effectful but safety {:?}",
                m.name,
                m.safety_level
            );
        }
    }
}

/// Safety `A` means "total, pure, deterministic", so no Word may be both `A`
/// and `Partial`. This guards against regressing the contract.
#[test]
fn safety_a_words_are_total() {
    let a_but_partial: Vec<&str> = get_builtin_word_registry()
        .iter()
        .filter(|m| m.safety_level == SafetyLevel::A && m.partiality == Partiality::Partial)
        .map(|m| m.name.as_str())
        .collect();
    assert!(
        a_but_partial.is_empty(),
        "SPEC §7.14: safety A must be total, but these are A+Partial: {a_but_partial:?}"
    );
}

/// Concrete §7.14 anchor contracts (the narrative examples of §7.14, pinned as
/// machine-checked facts).
#[test]
fn key_word_contracts_match_spec_7_14() {
    let c = |n: &str| get_coreword_metadata(n).unwrap_or_else(|| panic!("no contract {n}"));

    let add = c("ADD");
    assert_eq!(add.partiality, Partiality::Total);
    assert_eq!(add.nil_policy, NilPolicy::Passthrough);
    assert_eq!(add.safety_level, SafetyLevel::A);

    let div = c("DIV");
    assert_eq!(div.partiality, Partiality::Projecting);
    assert_eq!(div.nil_policy, NilPolicy::CreatesNil);
    assert_eq!(div.safety_level, SafetyLevel::B);

    for cmp in ["EQ", "LT"] {
        let m = c(cmp);
        assert_eq!(m.partiality, Partiality::Projecting, "{cmp}");
        assert_eq!(m.nil_policy, NilPolicy::Passthrough, "{cmp}");
        assert_eq!(m.safety_level, SafetyLevel::B, "{cmp}");
    }

    for logic in ["AND", "OR", "NOT"] {
        let m = c(logic);
        assert_eq!(m.partiality, Partiality::Total, "{logic}");
        assert_eq!(m.nil_policy, NilPolicy::Passthrough, "{logic}");
        assert_eq!(m.safety_level, SafetyLevel::A, "{logic}");
    }
}
