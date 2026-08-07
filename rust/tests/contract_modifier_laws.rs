//! Property-based contract / modifier / mass-conservation laws (Phase 3 ⭐).
//!
//! Encodes the algebraic content of
//! `docs/dev/ajisai-mathematical-formalization.md` §9-quater E (Phase 3):
//!
//! 1. **Modifier combinators** (`SPEC §6`): `⟦μ·w⟧ = κ_consume ∘ δ_region ∘
//!    base(w)`. default consumption is the identity and `KEEP` is bifurcation.
//! 2. **Coreword contracts** (`SPEC §7.14`): the `partiality` / `nil_policy` /
//!    `safety_level` lattices, with contract absence = conformance violation.
//! 3. **Static mass conservation** (`SPEC §13`): consumption/production as a
//!    resource (linear) discipline, observed here via stack-depth deltas
//!    (`depth(KEEP w) − depth(w) = arity`).
//!
//! Every law was checked against the reference implementation with a throwaway
//! probe before being written (roadmap §1.2-(T) discipline). Probe findings are
//! recorded as §9-quater E.5 findings; the two that are tracked oracles
//! are asserted as guarded
//! invariants so a future drift is loud.

mod test_support;

use ajisai_core::coreword_registry::{
    get_builtin_word_registry, get_coreword_metadata, NilPolicy, Partiality, Purity, SafetyLevel,
};
use proptest::prelude::*;
use test_support::generators::small;
use test_support::observe::{render, run, run_err};

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

    /// `KEEP` is bifurcation (§13.2): operands are retained *and* the result is
    /// pushed. Observationally `a b KEEP w == (a b) ++ (a b w)`. The sugar
    /// `,,`≡KEEP (SPEC §6.2).
    #[test]
    fn keep_is_bifurcation(a in small(), b in small(), w in binary_arith()) {
        let mut expected = obs(&format!("{a} {b}"));
        expected.extend(obs(&format!("{a} {b} {w}")));
        prop_assert_eq!(&expected, &obs(&format!("{a} {b} KEEP {w}")));
        prop_assert_eq!(&expected, &obs(&format!("{a} {b} KEEP {w}")));
    }

    /// **Mass conservation / bifurcation arity** (§13.1/§13.2): for a binary
    /// word the only stack-mass difference between `KEEP` and default consumption is the two
    /// retained operands, so `depth(KEEP w) − depth(w) = arity = 2`.
    #[test]
    fn keep_minus_eat_equals_arity(a in small(), b in small(), w in binary_arith()) {
        let eat = depth(&format!("{a} {b} {w}")) as i64;
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

// ─────────────────── KEEP composes with abstraction (§6.2) ──────────────────

/// `KEEP` modifies the *call*, not the body. A User Word is the only place the
/// two readings can differ, and the body reading is unsound: it let the
/// modifier reach the first consuming Word inside the body, so the body's own
/// literals were preserved as if the caller had written them. Each case below
/// returned a silently wrong stack before the boundary reading:
/// `5 KEEP TWICE` answered `5 2 10`.
#[test]
fn keep_preserves_the_operands_of_a_user_word_call_not_its_body_literals() {
    assert_eq!(obs("{ 2 * } 'TWICE' DEF 5 KEEP TWICE"), vec!["5/1", "10/1"]);
    assert_eq!(obs("{ 1 - } 'DEC' DEF 5 KEEP DEC"), vec!["5/1", "4/1"]);
    // A binary User Word retains both operands, matching the Core Word law.
    assert_eq!(
        obs("{ + } 'PLUS' DEF 3 5 KEEP PLUS"),
        vec!["3/1", "5/1", "8/1"]
    );
    // Reaching below the operands it was handed is still one operand region:
    // the body eats two and leaves one, and both eaten operands come back.
    assert_eq!(
        obs("{ + 5 - } 'F' DEF 3 5 KEEP F"),
        vec!["3/1", "5/1", "3/1"]
    );
    // A Word that consumes nothing has no operand region, so `KEEP` retains
    // nothing extra.
    assert_eq!(obs("{ 7 } 'SEVEN' DEF 3 KEEP SEVEN"), vec!["3/1", "7/1"]);
}

/// The bifurcation law of `keep_is_bifurcation` holds for User Words too:
/// `KEEP w` observes as the operands followed by the plain call's result.
#[test]
fn keep_is_bifurcation_across_abstraction() {
    let mut expected = obs("3 5");
    expected.extend(obs("{ + } 'PLUS' DEF 3 5 PLUS"));
    assert_eq!(expected, obs("{ + } 'PLUS' DEF 3 5 KEEP PLUS"));
}

/// With `KEEP` settled at the call boundary, a total identity Word makes
/// `KEEP` a total duplicator — one that works on the value domains no
/// arithmetic or vector trick reaches (Text, truth values, code blocks).
#[test]
fn keep_over_a_total_identity_word_copies_any_value() {
    let id = "{ 1 COLLECT [ 0 ] GET } 'ID' DEF ";
    assert_eq!(
        obs(&format!("{id} 'hello' KEEP ID")),
        vec!["'hello'", "'hello'"]
    );
    assert_eq!(obs(&format!("{id} TRUE KEEP ID")), vec!["TRUE", "TRUE"]);
    assert_eq!(obs(&format!("{id} 7 KEEP ID")), vec!["7/1", "7/1"]);
}

/// `KEEP` on a higher-order Word retains the collection it walks — and only
/// that. The block says what the call *is* rather than being data the call
/// read, and `FOLD`'s accumulator has been folded into the answer, so neither
/// comes back. All five higher-order Words follow the one rule: `ANY` and
/// `ALL` used to ignore the modifier outright and answer with a bare Boolean.
#[test]
fn keep_on_a_higher_order_word_retains_the_collection() {
    assert_eq!(
        obs("[ 1 2 ] KEEP { 2 MUL } MAP"),
        vec!["[ 1/1 2/1 ]", "[ 2/1 4/1 ]"]
    );
    assert_eq!(
        obs("[ 1 2 3 ] KEEP { 1 GT } FILTER"),
        vec!["[ 1/1 2/1 3/1 ]", "[ 2/1 3/1 ]"]
    );
    assert_eq!(
        obs("[ 1 2 3 ] 0 KEEP { ADD } FOLD"),
        vec!["[ 1/1 2/1 3/1 ]", "6/1"]
    );
    assert_eq!(
        obs("[ 1 2 3 ] KEEP { 1 GT } ANY"),
        vec!["[ 1/1 2/1 3/1 ]", "TRUE"]
    );
    assert_eq!(
        obs("[ 1 2 3 ] KEEP { 1 GT } ALL"),
        vec!["[ 1/1 2/1 3/1 ]", "FALSE"]
    );
    // Every other Word keeps all of its operands, the `GET` of the Reference
    // included — the higher-order rule is the one exception, not the norm.
    assert_eq!(
        obs("[ 10 3 ] KEEP [ 0 ] GET"),
        vec!["[ 10/1 3/1 ]", "[ 0/1 ]", "10/1"]
    );
}

/// Bifurcation has no exception for a Word whose result is empty. `BIND`,
/// `DEF`, `DEL` and `LOOKUP` answer with nothing, so under `KEEP` the whole
/// observation is their operands, unchanged and in order — and the effect
/// still happens. All four used to ignore the modifier outright, which made
/// `5 KEEP 'X' BIND` indistinguishable from the plain call.
#[test]
fn keep_preserves_the_operands_of_a_word_that_answers_with_nothing() {
    assert_eq!(obs("9 8 5 KEEP 'X' BIND"), vec!["9/1", "8/1", "5/1", "'X'"]);
    // The name is bound as well as kept: the modifier changes the stack, not
    // what the Word did.
    assert_eq!(obs("5 KEEP 'X' BIND X X ADD"), vec!["5/1", "'X'", "10/1"]);
    assert_eq!(
        obs("[ 1 2 ] KEEP [ 'A' 'B' ] BIND B"),
        vec!["[ 1/1 2/1 ]", "[ 'A' 'B' ]", "2/1"]
    );
    assert_eq!(obs("{ 7 } 'W' KEEP DEF W"), vec!["{ 7 }", "'W'", "7/1"]);
    assert_eq!(obs("{ 7 } 'W' DEF 'W' KEEP DEL"), vec!["'W'"]);
}

/// A `KEEP`-ed call that fails reports the failure. The modifier does not
/// convert malformed use into a stack that looks like a successful bifurcation.
#[test]
fn a_failed_keep_call_reports_the_failure() {
    // `EXEC` on a number is malformed use, so the call errors out.
    let message = run_err("{ EXEC } 'BOOM' DEF 5 KEEP BOOM");
    assert!(!message.is_empty());
}

// ─────────────────── projecting words bubble domain misses ──────────────────

/// `Projecting`/`CreatesNil` words project a well-formed domain miss onto NIL
/// rather than raising (SPEC §7.14, Bubble Rule §11.2): division by zero and an
/// out-of-range `GET` both yield NIL, not an error.
#[test]
fn projecting_words_bubble_domain_misses() {
    assert_eq!(obs("1 0 DIV"), vec!["NIL"]);
    assert_eq!(obs("1 0 /"), vec!["NIL"]);
    // GET declares `consumption: eat` (LANG.MODIFIERS.CONSUMPTION): both
    // operands leave the stack and the projected NIL is all that remains.
    assert_eq!(obs("[ 1 2 3 ] 9 GET"), vec!["NIL"]);
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
        // The NIL policy is no longer checked against a hand-written list of
        // admissible values: it is generated from the schema's own enum, so
        // every value the specification admits is a variant and no other value
        // is representable. What is worth asserting is that the declaration
        // reaches the runtime at all.
        assert!(
            !m.nil_policy.as_spec_str().is_empty(),
            "{} declares no NIL policy",
            m.name
        );
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

/// Safety-level lattice: `A` (the strongest) implies a Word that contributes
/// no effects of its own and always lands somewhere; effectful words sit
/// strictly above `B`.
///
/// `A` used to also imply *deterministic*, on the reasoning that the strongest
/// safety class must be reproducible. The canonical declarations show those are
/// independent axes, and reading them made three counterexamples visible at
/// once: `KEEP` and `VENT` are safety `A` and `stateRelative` —
/// they compute nothing and touch no value, but what they *do* is change how
/// the next Word runs. `VENT` also broke the "A must be pure" half by
/// declaring `conditional`, the class the hand-written vocabulary could not
/// express. None of that makes them unsafe, which is what `A` is about; it
/// makes determinism the wrong question to ask here, so the clause is gone
/// rather than weakened.
#[test]
fn safety_lattice_is_monotone() {
    for m in get_builtin_word_registry() {
        if m.safety_level == SafetyLevel::A {
            assert!(
                matches!(m.purity, Purity::Pure | Purity::Conditional),
                "{} A must contribute no effects of its own, got {:?}",
                m.name,
                m.purity
            );
            assert!(
                m.effects.is_empty(),
                "{} A must declare no effects, got {:?}",
                m.name,
                m.effects
            );
            // `A` is reserved for *total* words. `Projecting` is total by
            // projection (failures land on NIL), so it qualifies; `Partial`
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
        if m.purity == Purity::Effectful {
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

    // DIV declares `passthroughThenProject`, not `createsNil`: a NIL operand
    // passes through unchanged, and it is a *well-formed* operand pair with a
    // zero divisor that projects onto a fresh reasoned NIL. The hand-written
    // table could not say both, so it said only the second.
    let div = c("DIV");
    assert_eq!(div.partiality, Partiality::Projecting);
    assert_eq!(div.nil_policy, NilPolicy::PassthroughThenProject);
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
