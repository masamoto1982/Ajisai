//! Verification of registry joins, profiles, safety, and declared contracts.
//! AQ-VER-007 — Coreword purity / safe-preview integrity tests.
//!
//! These tests are linked from `docs/quality/TRACEABILITY_MATRIX.md`
//! to AQ-REQ-007 ("Built-in word purity classification and `safe_preview`
//! gating remain self-consistent"). Test names are prefixed with their
//! verification ID so that a `cargo test aq_ver_007` invocation runs
//! the full coreword-registry coverage subset.

use super::{get_builtin_word_registry, is_safe_preview_word, Determinism, Purity};

#[test]
fn aq_ver_007_a_metadata_exists_for_all_builtin_words() {
    let registry = get_builtin_word_registry();
    assert!(!registry.is_empty(), "registry must not be empty");
    for word in registry {
        assert!(!word.name.is_empty(), "name must not be empty");
        assert!(
            !word.category.is_empty(),
            "{} has empty category",
            word.name
        );
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
/// Determinism used to be asserted here too, on the reasoning that a pure
/// Word must be reproducible. The canonical declarations disagree, and they
/// are right: `EAT` and `KEEP` are `pure` — they compute nothing and touch
/// no value — yet `stateRelative`, because what they do is change the
/// consumption mode the *next* Word runs under. Purity and determinism are
/// separate axes in the specification, and the hand-written table conflated
/// them by carrying determinism as a bool that nobody had a reason to set
/// false for a pure Word.
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
        vec!["MAP", "FILTER", "FOLD", "ANY", "ALL", "EXEC", "OR-NIL"],
        "the conditional Words are the higher-order ones plus EXEC/OR-NIL"
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
            !word.is_deterministic(),
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

/// The vocabulary currently holds no `observational` Word: `LOOKUP` was the
/// only one, and looking a Word up is the host's job now, not a program's. The
/// loop below therefore has no subjects today, and it is kept deliberately —
/// `observational` remains a contract a future Word may declare, and this is
/// where what that declaration obliges is written down. A Word that reads the
/// session's state must say which state (`effects`), must be `stateRelative`
/// rather than deterministic — reproducible for one interpreter snapshot is not
/// the same as reproducible — and must stay out of auto preview, where it would
/// run against a snapshot the reader never asked about.
#[test]
fn aq_ver_007_d_observational_words_read_state_and_do_not_auto_preview() {
    let registry = get_builtin_word_registry();
    for word in registry
        .iter()
        .filter(|w| w.purity == Purity::Observational)
    {
        assert!(
            !word.effects.is_empty(),
            "{} observational words must declare effects",
            word.name
        );
        assert_eq!(
            word.determinism,
            Determinism::StateRelative,
            "{} observes state rather than the host",
            word.name
        );
        assert!(
            !word.safe_preview,
            "{} observational words must not run in auto preview",
            word.name
        );
    }
}

/// AQ-VER-007-E — MC/DC truth table for `is_safe_preview_word`.
///
/// The decision under test is logically:
///
/// ```text
/// metadata_present(name) && metadata_safe_preview(name)
/// ```
///
/// implemented in `is_safe_preview_word` via
/// `get_coreword_metadata(name).map(|w| w.safe_preview).unwrap_or(false)`.
/// We exercise all three reachable rows (the `metadata_present == false`
/// row collapses both `safe_preview` cases to the `unwrap_or(false)`
/// short-circuit, so it is covered by a single unknown-name probe):
///
/// | row | metadata_present | safe_preview | expected | rationale                          |
/// |-----|------------------|--------------|----------|------------------------------------|
/// | 1   | true             | true         | true     | known pure word (e.g. `ADD`)       |
/// | 2   | true             | false        | false    | known effectful word (e.g. `PRINT`)|
/// | 3   | true             | false        | false    | known observable word (e.g. `NOW`) |
/// | 4   | false            | n/a          | false    | unknown name → unwrap_or(false)    |
///
/// Rows 1 vs 2 demonstrate independent effect of `safe_preview`;
/// rows 1 vs 4 demonstrate independent effect of `metadata_present`.
#[test]
fn aq_ver_007_e_is_safe_preview_word_decision_truth_table() {
    // Row 1: metadata present, safe_preview=true → true.
    assert!(
        is_safe_preview_word("ADD"),
        "row1: pure builtin ADD must be safe preview"
    );
    // Row 2: metadata present, safe_preview=false (effectful) → false.
    assert!(
        !is_safe_preview_word("PRINT"),
        "row2: effectful builtin PRINT must not be safe preview"
    );
    // Row 3: metadata present, safe_preview=false (observable) → false.
    assert!(
        !is_safe_preview_word("NOW"),
        "row3: observable builtin NOW must not be safe preview"
    );
    // Row 4: metadata absent → unwrap_or(false) short-circuit.
    assert!(
        !is_safe_preview_word("__AJISAI_NO_SUCH_WORD__"),
        "row4: unknown name must default to false"
    );

    // Case-insensitive lookup also reaches the safe_preview=true arm,
    // confirming that the upper-casing inside get_coreword_metadata
    // does not flip the decision.
    assert!(
        is_safe_preview_word("add"),
        "row1 (lowercase): case-insensitive lookup must still be safe preview"
    );
}
