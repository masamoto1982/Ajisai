//! Property-based contract / modifier / mass-conservation laws (Phase 3 ⭐).
//!
//! Encodes the algebraic content of
//! `docs/dev/ajisai-mathematical-formalization.md` §9-quater E (Phase 3):
//!
//! 1. **Modifier combinators** (`SPEC §6`): `⟦μ·w⟧ = κ_consume ∘ δ_region ∘
//!    base(w)`. `TOP`/`EAT` are the identity defaults; `KEEP` is bifurcation
//!    (§13.2); `STAK` is the top-`count` fold.
//! 2. **Coreword contracts** (`SPEC §7.14`): the `partiality` / `nil_policy` /
//!    `safety_level` lattices, with contract absence = conformance violation.
//! 3. **Static mass conservation** (`SPEC §13`): consumption/production as a
//!    resource (linear) discipline, observed here via stack-depth deltas
//!    (`depth(KEEP w) − depth(EAT w) = arity`).
//!
//! Every law was checked against the reference implementation with a throwaway
//! probe before being written (roadmap §1.2-(T) discipline). Probe findings are
//! recorded as §9-quater E.5 findings; the two that are tracked oracles
//! (E2 = GCD/LCM safety/partiality mismatch) are asserted as guarded
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

    /// `TOP` and `EAT` are the identity defaults of the modifier algebra: the
    /// bare word and every default-modifier spelling render the same stack
    /// (SPEC §6.1/§6.2; the sugar `.`≡TOP, `,`≡EAT per §6.1/§6.2 tables).
    #[test]
    fn default_modifiers_are_identities(a in small(), b in small(), w in binary_arith()) {
        let bare = obs(&format!("{a} {b} {w}"));
        for variant in ["TOP", "EAT", "TOP EAT", ".", ",", ". ,"] {
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

    /// `STAK` folds the top `count` items with the word (SPEC §6.1: the stack is
    /// the operand). For an associative scalar word this equals the left-nested
    /// binary fold: `x1 x2 x3 3 STAK ADD ≡ x1 x2 ADD x3 ADD`. The sugar `..`≡STAK.
    #[test]
    fn stak_folds_top_count(xs in prop::collection::vec(small(), 2..5), add_or_mul in prop_oneof![Just("ADD"), Just("MUL")]) {
        let n = xs.len();
        let items = xs.iter().map(i64::to_string).collect::<Vec<_>>().join(" ");
        // left-nested binary fold reference
        let mut folded = format!("{} {}", xs[0], xs[1]);
        folded.push_str(&format!(" {add_or_mul}"));
        for x in &xs[2..] {
            folded.push_str(&format!(" {x} {add_or_mul}"));
        }
        prop_assert_eq!(
            obs(&format!("{items} {n} STAK {add_or_mul}")),
            obs(&folded)
        );
        // `..` sugar agrees with canonical STAK.
        prop_assert_eq!(
            obs(&format!("{items} {n} STAK {add_or_mul}")),
            obs(&format!("{items} {n} .. {add_or_mul}"))
        );
    }

    /// `STAK KEEP` retains the folded operands and pushes the result, so the
    /// resulting depth is `n + 1` for `n` folded items (probe-confirmed).
    #[test]
    fn stak_keep_retains_operands(xs in prop::collection::vec(small(), 2..5)) {
        let n = xs.len();
        let items = xs.iter().map(i64::to_string).collect::<Vec<_>>().join(" ");
        prop_assert_eq!(depth(&format!("{items} {n} STAK KEEP ADD")), n + 1);
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

/// **Finding E2 (resolved).** SPEC §7.14 defines safety `A` as "total, pure,
/// deterministic". `GCD`/`LCM` genuinely raise on non-integer input (they are
/// `Partial`), so they were corrected from `A` to `B` ("partial but with
/// explicit error categories"). The invariant now holds with no exceptions:
/// **no** word is both safety `A` and `Partial`, and `GCD`/`LCM` are `B`. This
/// guards against regressing the contract.
#[test]
fn safety_a_partial_invariant_holds_and_gcd_lcm_are_b() {
    let a_but_partial: Vec<&str> = get_builtin_word_registry()
        .iter()
        .filter(|m| m.safety_level == SafetyLevel::A && m.partiality == Partiality::Partial)
        .map(|m| m.name.as_str())
        .collect();
    assert!(
        a_but_partial.is_empty(),
        "SPEC §7.14: safety A must be total, but these are A+Partial: {a_but_partial:?}"
    );
    for w in ["GCD", "LCM"] {
        let m = get_coreword_metadata(w).unwrap_or_else(|| panic!("no contract {w}"));
        assert_eq!(m.partiality, Partiality::Partial, "{w}");
        assert_eq!(m.safety_level, SafetyLevel::B, "{w} must be safety B");
    }
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
