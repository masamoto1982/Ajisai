//! Verification of declared contracts and registry uniqueness.

use super::{
    collect_duplicate_entries, get_builtin_word_registry, get_coreword_metadata, NilPolicy,
    Partiality, Purity,
};

#[test]
fn aq_ver_contract_a_every_word_has_contract_metadata() {
    let registry = get_builtin_word_registry();
    for word in registry {
        assert!(
            matches!(
                word.partiality,
                Partiality::Total | Partiality::Partial | Partiality::Projecting
            ),
            "{} must declare partiality",
            word.name
        );
        // The NIL policy's admissible values are the schema's, generated
        // into the enum, so an invalid one is unrepresentable rather than
        // merely untested — which is the whole reason the list this
        // assertion used to spell out went stale.
        assert!(
            !word.nil_policy.as_spec_str().is_empty(),
            "{} must declare nil_policy",
            word.name
        );
    }
}

/// `DIV` both passes a NIL through and projects a zero divisor onto a
/// fresh reasoned NIL. `passthroughThenProject` is the declaration that
/// says both; `createsNil` — all the hand-written vocabulary could
/// express — said only the second, which is why `1 0 DIV 1 ADD` looked
/// like a Word creating an absence out of nothing rather than one
/// projected NIL flowing into the next.
#[test]
fn aq_ver_contract_b_arithmetic_division_passes_through_then_projects() {
    let div = get_coreword_metadata("DIV").expect("DIV must be in registry");
    assert_eq!(div.partiality, Partiality::Projecting);
    assert_eq!(div.nil_policy, NilPolicy::PassthroughThenProject);

    let add = get_coreword_metadata("ADD").expect("ADD must be in registry");
    assert_eq!(add.partiality, Partiality::Total);
    assert_eq!(add.nil_policy, NilPolicy::Passthrough);
}

#[test]
fn aq_ver_contract_f_comparison_and_rounding_words_are_total() {
    // LANG.CONTRACT.REGISTRY / LANG.VALUES.EXACT: order, equality and integer
    // rounding decide over every number the language holds — the rationals and
    // the algebraic field `SQRT` builds — so the comparison and rounding
    // primitives have no projection to declare. Like ADD/SUB/MUL they pass a
    // NIL operand through (LANG.FAILURE.PASSTHROUGH) and are otherwise total.
    for name in &["EQ", "LT", "GT", "FLOOR", "ROUND", "ADD", "SUB", "MUL"] {
        let meta =
            get_coreword_metadata(name).unwrap_or_else(|| panic!("{} must be in registry", name));
        assert_eq!(
            meta.partiality,
            Partiality::Total,
            "{} must be Total (LANG.VALUES.EXACT)",
            name
        );
        assert_eq!(
            meta.nil_policy,
            NilPolicy::Passthrough,
            "{} must be Passthrough (LANG.FAILURE.PASSTHROUGH)",
            name
        );
    }
}

#[test]
fn aq_ver_contract_i_nil_diagnostic_accessors_consume_nil() {
    // LANG.VALUES.NIL / LANG.OBSERVATION.DIAGNOSIS: the five diagnostic absence accessors inspect a
    // NIL rather than propagate it, so their nil_policy is ConsumesNil (the
    // OR-NIL-family "inspect or branch on NIL" classification). They are pure,
    // observations that consume what they read, like every
    // Word (LANG.STACK.CONSUMPTION), so their mass contract is a pinned 1 -> 1.
    for name in &["NIL?", "NIL-REASON"] {
        let meta =
            get_coreword_metadata(name).unwrap_or_else(|| panic!("{} must be in registry", name));
        assert_eq!(
            meta.nil_policy,
            NilPolicy::ConsumeNil,
            "{} must be consumeNil (LANG.VALUES.NIL)",
            name
        );
        assert_eq!(
            meta.purity,
            Purity::Pure,
            "{} must be Pure (LANG.OBSERVATION.DIAGNOSIS)",
            name
        );
        // Neither raises on any operand. `NIL?` always answers a truth;
        // `NIL-REASON` answers NIL(domainMiss) for a value that is not a NIL,
        // which is a projection, so it is `projecting`.
        let partiality = if *name == "NIL?" {
            Partiality::Total
        } else {
            Partiality::Projecting
        };
        assert_eq!(meta.partiality, partiality, "{}", name);
        // The declared arity is 1 in, 1 out: the inspected value is
        // consumed and the answer takes its place. A program that needs the
        // value afterwards names it with `BIND`.
        assert_eq!(
            meta.mass,
            super::MassContract::Fixed {
                consumes: 1,
                produces: 1
            },
            "{} declares a pinned 1 -> 1 arity",
            name
        );
    }
}

/// The mass contract is the declared stack arity, read through the
/// analyzers' coarser vocabulary. This used to assert that the adapter
/// returned what the hand-written table said; now that there is nothing to
/// disagree with, what is worth asserting is the projection itself — a
/// pinned arity survives, and only the two data-dependent markers collapse.
#[test]
fn aq_ver_contract_f_mass_contract_projects_the_declared_arity() {
    use crate::kernel::generated::{Arity, GENERATED_WORDS};

    let mut pinned = 0_usize;
    for word in GENERATED_WORDS {
        let expected = match (word.stack_inputs, word.stack_outputs) {
            (Arity::Fixed(consumes), Arity::Fixed(produces)) => {
                pinned += 1;
                super::MassContract::Fixed { consumes, produces }
            }
            _ => super::MassContract::Dynamic,
        };
        assert_eq!(
            super::mass_contract(word.name),
            expected,
            "{}: mass_contract must project the declared arity",
            word.name
        );
    }
    assert!(
        pinned >= 53,
        "only {pinned} Words have a pinned arity; the projection has collapsed"
    );
}

/// An alias reaches the same contract as the Word it names.
#[test]
fn aq_ver_contract_f2_mass_contract_canonicalizes_aliases() {
    assert_eq!(super::mass_contract("+"), super::mass_contract("ADD"));
    assert_eq!(
        super::mass_contract("__AJISAI_NO_SUCH_WORD__"),
        super::MassContract::Dynamic
    );
}

#[test]
fn aq_ver_listing_a_no_two_entries_share_a_name() {
    let registry = get_builtin_word_registry();
    let dupes = collect_duplicate_entries(registry);
    assert!(
        dupes.is_empty(),
        "built-in word names must be unique (duplicates: {:?})",
        dupes
    );
}
