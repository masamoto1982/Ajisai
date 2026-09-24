//! Property-based contract / consumption / mass-conservation laws (Phase 3 ⭐).
//!
//! Encodes the algebraic content of the consumption / contract /
//! mass-conservation model (Phase 3):
//!
//! 1. **Consumption** (`LANG.STACK.CONSUMPTION`): every Word consumes the
//!    operands it reads; a value is reused only by naming it with `BIND`.
//! 2. **Coreword contracts** (`LANG.CONTRACT.REGISTRY`): the `partiality` / `nil_policy` /
//!    `safety_level` lattices, with contract absence = conformance violation.
//! 3. **Static mass conservation** (`LANG.STACK.CONSUMPTION`): consumption/production as a
//!    resource (linear) discipline, observed here via stack-depth deltas.
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
use test_support::observe::{render, run};

// ─────────────────────────── observation helpers ───────────────────────────

/// Whole-stack rendering (one value per element), the conformance observation.
fn obs(src: &str) -> Vec<String> {
    run(src).iter().map(render).collect()
}

/// Stack depth after running `src` (mass observation).
fn depth(src: &str) -> usize {
    run(src).len()
}

/// Total binary scalar→scalar words (never error / NIL on integer operands).
fn binary_arith() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("ADD"), Just("MUL"), Just("SUB")]
}

// ───────────────────────── consumption (§6, LANG.STACK.CONSUMPTION) ─────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// **Mass conservation** (LANG.MACHINE.WORD/LANG.STACK.CONSUMPTION): a binary word consumes
    /// both operands it reads and produces one result, so a value beneath its
    /// operands is untouched and the depth drops by exactly one.
    #[test]
    fn binary_words_consume_both_operands(a in small(), b in small(), w in binary_arith()) {
        let below = obs(&format!("9 {a} {b} {w}"));
        prop_assert_eq!(below.len(), 2);
        prop_assert_eq!(&below[0], "9/1");
    }

    // ──────────── partiality contract ↔ observable behavior (LANG.CONTRACT.REGISTRY) ──────────

    /// A `Total` word never errors on well-shaped input: it always leaves a
    /// value (Hoare `ensures` discharged), here over total binary arithmetic.
    #[test]
    fn total_words_do_not_error(a in small(), b in small(), w in binary_arith()) {
        prop_assert_eq!(depth(&format!("{a} {b} {w}")), 1);
    }
}

// ─────────────────── consumption across abstraction (LANG.STACK.CONSUMPTION) ──────────────────

/// A User Word call consumes its operands like a Core Word; reusing a value is
/// done by naming it with `BIND`.
#[test]
fn a_user_word_call_consumes_its_operands() {
    assert_eq!(obs("[ 2 * ] 'TWICE' DEF 5 TWICE"), vec!["10/1"]);
    assert_eq!(obs("[ + ] 'PLUS' DEF 3 5 PLUS"), vec!["8/1"]);
    assert_eq!(
        obs("[ 2 * ] 'TWICE' DEF 5 'N' BIND N N TWICE"),
        vec!["5/1", "10/1"]
    );
}

// ───────────────── projecting words project onto NIL for domain misses ──────

/// `Projecting`/`CreatesNil` words project a well-formed domain miss onto NIL
/// rather than raising (LANG.CONTRACT.REGISTRY, NIL Projection Rule LANG.FAILURE.PROJECT): division by
/// zero and an out-of-range `GET` both yield NIL, not an error.
#[test]
fn projecting_words_project_onto_nil_for_domain_misses() {
    assert_eq!(obs("1 0 DIV"), vec!["NIL"]);
    assert_eq!(obs("1 0 /"), vec!["NIL"]);
    // GET consumes what it reads (LANG.STACK.CONSUMPTION): both
    // operands leave the stack and the projected NIL is all that remains.
    assert_eq!(obs("[ 1 2 3 ] 9 GET"), vec!["NIL"]);
}

// ──────────────────────── contract lattice laws (LANG.CONTRACT.REGISTRY) ─────────────────────

/// Every built-in carries a contract reachable by its own name, with all three
/// classification fields in their declared domains. A Coreword without a
/// contract entry is a conformance violation (LANG.CONTRACT.REGISTRY).
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
            SafetyLevel::A | SafetyLevel::B | SafetyLevel::D
        ));
    }
}

/// Safety-level lattice: `A` (the strongest) implies a Word that contributes
/// no effects of its own and always lands somewhere; effectful words sit
/// strictly above `B`.
///
/// `A` used to also imply *deterministic*, on the reasoning that the strongest
/// safety class must be reproducible. The canonical declarations show those are
/// independent axes: `OR-NIL` is safety `A` and `stateRelative` — it computes
/// nothing and touches no value, but what it *does* is change how the next
/// Word runs. `OR-NIL` also broke the "A must be pure" half by declaring
/// `conditional`, the class the hand-written vocabulary could not express.
/// None of that makes it unsafe, which is what `A` is about; it
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
                m.safety_level == SafetyLevel::D,
                "{} has effects but safety {:?}",
                m.name,
                m.safety_level
            );
        }
        if m.purity == Purity::Effectful {
            assert!(
                m.safety_level == SafetyLevel::D,
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
        "LANG.CONTRACT.REGISTRY: safety A must be total, but these are A+Partial: {a_but_partial:?}"
    );
}

/// Concrete LANG.CONTRACT.REGISTRY anchor contracts (the narrative examples of LANG.CONTRACT.REGISTRY, pinned as
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

    // EQ/LT declare a blanket `passthrough` and are total: a NIL operand
    // passes through unchanged, and order and equality decide over every
    // number the language holds (LANG.VALUES.EXACT), so they project nothing
    // of their own.
    for cmp in ["EQ", "LT"] {
        let m = c(cmp);
        assert_eq!(m.partiality, Partiality::Total, "{cmp}");
        assert_eq!(m.nil_policy, NilPolicy::Passthrough, "{cmp}");
        assert_eq!(m.safety_level, SafetyLevel::A, "{cmp}");
    }

    // `AND`/`NOT` declare `kleeneAbsorbing`, not a blanket `passthrough`:
    // a NIL operand does not always survive to the result (FALSE absorbs it
    // into `AND`), so the primitive must decide instead of a
    // generic projection (LANG.VALUES.TRUTH).
    for logic in ["AND", "NOT"] {
        let m = c(logic);
        assert_eq!(m.partiality, Partiality::Total, "{logic}");
        assert_eq!(m.nil_policy, NilPolicy::KleeneAbsorbing, "{logic}");
        assert_eq!(m.safety_level, SafetyLevel::A, "{logic}");
    }
}
