//! Verification of registry joins, profiles, safety, and declared contracts.
//! AQ-VER-007 — Coreword purity / safe-preview integrity tests.
//!
//! These tests are linked from `docs/quality/TRACEABILITY_MATRIX.md`
//! to AQ-REQ-007 ("Built-in word purity classification and `safe_preview`
//! gating remain self-consistent"). Test names are prefixed with their
//! verification ID so that a `cargo test aq_ver_007` invocation runs
//! the full coreword-registry coverage subset.

use super::{get_builtin_word_registry, Determinism, Purity};

#[test]
fn aq_ver_007_a_metadata_exists_for_all_builtin_words() {
    let registry = get_builtin_word_registry();
    assert!(!registry.is_empty(), "registry must not be empty");
    for word in registry {
        assert!(!word.name.is_empty(), "name must not be empty");
        assert!(!word.family.is_empty(), "{} has empty family", word.name);
        // Purity is generated from the schema's enum, so "is this a valid
        // class" is a type-level fact now. What still needs asserting is
        // that the declaration reached the registry.
        assert!(
            !word.purity.as_spec_str().is_empty(),
            "{} has no declared purity",
            word.name
        );
    }
}

/// A `pure` Word declares no effects and is safe to preview.
///
/// Determinism is not asserted here: purity and determinism are separate
/// axes in the specification, and a pure Word may still be `stateRelative`.
#[test]
fn aq_ver_007_b_pure_words_declare_no_effects_and_are_safe_to_preview() {
    let registry = get_builtin_word_registry();
    for word in registry.iter().filter(|w| w.purity == Purity::Pure) {
        assert!(
            word.effects.is_empty(),
            "{} pure words must have no effects",
            word.name
        );
        assert!(
            word.safe_preview,
            "{} pure words must be safe preview",
            word.name
        );
    }
}

/// The `conditional` class the hand-written vocabulary could not express:
/// a Word whose purity is that of the block it is given. It contributes no
/// effects of its own, so it must declare none — but it is never
/// `deterministic`, because what it runs is decided at runtime.
#[test]
fn aq_ver_007_b2_conditional_words_borrow_their_purity_from_their_block() {
    let conditional: Vec<&str> = get_builtin_word_registry()
        .iter()
        .filter(|w| w.purity == Purity::Conditional)
        .map(|w| w.name.as_str())
        .collect();
    assert_eq!(
        conditional,
        vec!["MAP", "FILTER", "FOLD", "SCAN", "EXEC"],
        "the conditional Words are the higher-order ones plus EXEC"
    );
    for word in get_builtin_word_registry()
        .iter()
        .filter(|w| w.purity == Purity::Conditional)
    {
        assert!(
            word.effects.is_empty(),
            "{} contributes no effects of its own",
            word.name
        );
        assert!(
            word.determinism != Determinism::Deterministic,
            "{} runs a block chosen at runtime, so it is not deterministic",
            word.name
        );
    }
}

#[test]
fn aq_ver_007_c_effectful_words_must_not_be_safe_preview() {
    let registry = get_builtin_word_registry();
    for word in registry.iter().filter(|w| w.purity == Purity::Effectful) {
        assert!(
            !word.safe_preview,
            "{} effectful words must disable safe preview",
            word.name
        );
        assert!(
            !word.effects.is_empty(),
            "{} effectful words must declare effects",
            word.name
        );
    }
}
