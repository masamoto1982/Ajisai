//! Verification of declared contracts, profiles, and registry uniqueness.

use super::{
    collect_duplicate_entries, get_builtin_word_registry, get_coreword_metadata,
    get_hosted_profile_words, NilPolicy, Partiality, Purity, SafetyLevel, WordProfile,
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
        assert!(
            matches!(
                word.safety_level,
                SafetyLevel::A | SafetyLevel::B | SafetyLevel::D
            ),
            "{} must declare safety_level",
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
    assert_eq!(div.safety_level, SafetyLevel::B);

    let add = get_coreword_metadata("ADD").expect("ADD must be in registry");
    assert_eq!(add.partiality, Partiality::Total);
    assert_eq!(add.nil_policy, NilPolicy::Passthrough);
    assert_eq!(add.safety_level, SafetyLevel::A);
}

#[test]
fn aq_ver_contract_f_comparison_words_project_undecidable_to_unknown() {
    // SPEC §7.14: all six comparison primitives are
    // Projecting/PassthroughThenProject/B. They are Projecting because a
    // Tier 2 (`PI`) pair can exhaust its comparison-refinement budget
    // (§7.4.1) without deciding — that genuine incomparability projects onto
    // the logical `Unknown` (U), a reasoned NIL tagged `TruthValue` so it
    // reads as U rather than as an ordinary absence. They are
    // PassthroughThenProject because they still pass a NIL operand through
    // first (§7.12), and only then may project the budget-exhaustion case.
    for name in &["EQ", "NEQ", "LT", "LTE", "GT", "GTE"] {
        let meta =
            get_coreword_metadata(name).unwrap_or_else(|| panic!("{} must be in registry", name));
        assert_eq!(
            meta.partiality,
            Partiality::Projecting,
            "{} must be Projecting (SPEC §7.14)",
            name
        );
        assert_eq!(
            meta.nil_policy,
            NilPolicy::PassthroughThenProject,
            "{} must be PassthroughThenProject (SPEC §7.14)",
            name
        );
        assert_eq!(
            meta.safety_level,
            SafetyLevel::B,
            "{} must be SafetyLevel B (SPEC §9.4)",
            name
        );
    }
}

#[test]
fn aq_ver_contract_g_rounding_modulo_create_nil_under_undecidable() {
    // MOD/FLOOR/ROUND operate on ExactScalar (CF) operands whose
    // partial-quotient budget can exhaust, yielding an Undecidable NIL
    // (SPEC §7.4.1). They are therefore Projecting/CreatesNil/B, matching
    // DIV and the comparison words. ADD/SUB/MUL stay Total because their
    // CF arithmetic always yields a value (never a budget miss).
    for name in &["MOD", "FLOOR", "ROUND"] {
        let meta =
            get_coreword_metadata(name).unwrap_or_else(|| panic!("{} must be in registry", name));
        assert_eq!(
            meta.partiality,
            Partiality::Projecting,
            "{} must be Projecting (SPEC §7.4.1)",
            name
        );
        assert_eq!(
            meta.nil_policy,
            NilPolicy::PassthroughThenProject,
            "{} passes a NIL through and projects a budget miss (SPEC §7.4.1)",
            name
        );
        assert_eq!(
            meta.safety_level,
            SafetyLevel::B,
            "{} must be SafetyLevel B",
            name
        );
    }

    // ADD/SUB/MUL stay plain `passthrough`: their ExactReal arithmetic
    // always produces a value, so there is no projection to declare.
    for name in &["ADD", "SUB", "MUL"] {
        let meta =
            get_coreword_metadata(name).unwrap_or_else(|| panic!("{} must be in registry", name));
        assert_eq!(
            meta.nil_policy,
            NilPolicy::Passthrough,
            "{} must stay Passthrough (CF arithmetic is total)",
            name
        );
    }
}

#[test]
fn aq_ver_contract_i_nil_diagnostic_accessors_consume_nil() {
    // SPEC §4.5.0 / §7.15: the five diagnostic absence accessors inspect a
    // NIL rather than propagate it, so their nil_policy is ConsumesNil (the
    // OR-NIL-family "inspect or branch on NIL" classification). They are pure,
    // total, safety-A observations that retain their inspection target, so
    // their mass contract is Dynamic (net +1, like the LENGTH/GET
    // inspection words of §7.1.1 — a Fixed contract would mis-model the
    // retained operand for the static depth analyzer).
    for name in &["NIL?", "NIL-REASON"] {
        let meta =
            get_coreword_metadata(name).unwrap_or_else(|| panic!("{} must be in registry", name));
        assert_eq!(
            meta.nil_policy,
            NilPolicy::ConsumeNil,
            "{} must be consumeNil (SPEC §4.5.0)",
            name
        );
        assert_eq!(
            meta.purity,
            Purity::Pure,
            "{} must be Pure (SPEC §7.15)",
            name
        );
        assert_eq!(
            meta.partiality,
            Partiality::Total,
            "{} must be Total — a well-formed observation never raises (SPEC §4.5.0)",
            name
        );
        assert_eq!(
            meta.safety_level,
            SafetyLevel::A,
            "{} must be SafetyLevel A (pure, total, deterministic)",
            name
        );
        // The declared arity is 1 in, 2 out under `consumption: retain`:
        // the inspected value stays and the answer is pushed above it.
        // The hand-written table called this `Dynamic` because a bare
        // `Fixed` could not model a retained operand — but 1->2 models it
        // exactly, and calling it dynamic disengaged the static analyzer
        // from a Word whose arity the specification pins.
        assert_eq!(
            meta.mass,
            super::MassContract::Fixed {
                consumes: 1,
                produces: 2
            },
            "{} declares a pinned 1 -> 2 arity",
            name
        );
    }
}

#[test]
fn aq_ver_contract_c_effectful_words_have_d_safety() {
    let registry = get_builtin_word_registry();
    for word in registry.iter().filter(|w| w.purity == Purity::Effectful) {
        assert!(
            matches!(word.safety_level, SafetyLevel::D),
            "{} effectful words must have safety_level D, got {:?}",
            word.name,
            word.safety_level
        );
    }
}

#[test]
fn aq_ver_contract_e_builtin_spec_stability_matches_safety_level() {
    // Three-layer documentation model §5.3: stability label must agree
    // with the §7.14 contract metadata declared on each `BuiltinSpec`.
    // The mapping is:
    //   safety_level A or B          -> "stable"
    //   safety_level D                -> "experimental"
    // This test catches drift between BuiltinSpec.stability and the
    // registry contract.
    for spec in crate::builtins::builtin_specs() {
        let meta = get_coreword_metadata(spec.name)
            .unwrap_or_else(|| panic!("{} must be in registry", spec.name));
        let expected = match meta.safety_level {
            SafetyLevel::A | SafetyLevel::B => "stable",
            SafetyLevel::D => "experimental",
        };
        assert_eq!(
            spec.stability, expected,
            "{}: BuiltinSpec.stability = {:?} but safety_level = {:?} maps to {:?}",
            spec.name, spec.stability, meta.safety_level, expected
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

#[test]
fn aq_ver_profile_a_print_is_the_only_hosted_word() {
    // Output is the only *hosted* effect (LANG.EFFECTS.OUTPUT), so PRINT is the
    // only Word outside the Core profile. DEF/DEL are effectful as well, but
    // their effect stays inside the machine, so they keep the Core profile.
    let hosted = get_hosted_profile_words();
    assert_eq!(
        hosted.iter().map(|w| w.name.as_str()).collect::<Vec<_>>(),
        vec!["PRINT"],
        "PRINT must be the only Hosted-profile Word"
    );
}

#[test]
fn aq_ver_profile_b_core_profile_excludes_print() {
    for word in get_builtin_word_registry()
        .iter()
        .filter(|word| word.profile == WordProfile::Core)
    {
        assert_ne!(
            word.name, "PRINT",
            "PRINT is the effectful Word and must not be Core-profile"
        );
    }
}
