//! Test suite for `crate::interpreter::debug_diagnosis`.

use crate::error::ErrorCategory;
use crate::interpreter::debug_diagnosis::{
    classify_locus, DebugDiagnosis, ErrorLocusKind, ErrorPhase,
};

#[test]
fn qualified_word_is_classified_as_a_user_dictionary_word() {
    let locus = classify_locus(Some("EXAMPLE@DOUBLE"));
    assert_eq!(locus.kind, ErrorLocusKind::UserWord);
    assert_eq!(locus.dictionary.as_deref(), Some("EXAMPLE"));
}

/// A user Word is only knowable at the failure site, so the suggestion it
/// produces arrives after the checks were written. The spelling check
/// names the candidates, so it has to be rewritten against the list that
/// won — otherwise the diagnosis says in one line that nothing was close
/// and in another that `TWICE` was.
#[test]
fn a_user_word_found_late_reaches_the_spelling_check_too() {
    let mut diagnosis = DebugDiagnosis::from_error_category(
        ErrorPhase::ResolveWord,
        Some("TWICA"),
        Some(&ErrorCategory::UnknownWord),
        None,
        0,
        0,
        None,
    );
    let before = diagnosis
        .next_checks
        .iter()
        .find(|c| c.code == "checkSpelling")
        .expect("an unresolved name gets a spelling check")
        .detail
        .en
        .clone();
    assert!(!before.contains("TWICE"), "{before}");

    let user_words = ["TWICE".to_string()];
    diagnosis.with_user_vocabulary(user_words.iter().map(String::as_str));

    assert_eq!(diagnosis.candidates, vec!["TWICE".to_string()]);
    let after = &diagnosis
        .next_checks
        .iter()
        .find(|c| c.code == "checkSpelling")
        .expect("the spelling check survives the re-rank")
        .detail
        .en;
    assert!(after.contains("TWICE"), "{after}");
}

/// `1 + 2` fails inside `ADD`, a name the program never wrote. The canonical
/// name stays the answer to "which Word failed" — the diagnosis classifies on
/// it — so the spelling is recorded beside it.
#[test]
fn an_alias_spelling_is_recorded_beside_the_word_it_resolved_to() {
    let diagnosis = DebugDiagnosis::from_error_category(
        ErrorPhase::ExecuteWord,
        Some("ADD"),
        Some(&ErrorCategory::StackUnderflow),
        None,
        1,
        1,
        None,
    )
    .with_source_word(Some("+"));

    assert_eq!(diagnosis.where_.word.as_deref(), Some("ADD"));
    assert!(
        diagnosis.evidence.iter().any(|e| e == "sourceWord=+"),
        "{:?}",
        diagnosis.evidence
    );
}

/// A spelling that is not an alias of *this* Word says nothing. The alias
/// table is one-directional and a name that merely differs in case is not an
/// alias, so neither may put words in the reader's mouth.
#[test]
fn only_an_alias_of_the_failing_word_is_recorded() {
    let unrelated = DebugDiagnosis::from_error_category(
        ErrorPhase::ExecuteWord,
        Some("ADD"),
        Some(&ErrorCategory::StackUnderflow),
        None,
        1,
        1,
        None,
    )
    .with_source_word(Some("-"));
    assert!(
        !unrelated
            .evidence
            .iter()
            .any(|e| e.starts_with("sourceWord=")),
        "{:?}",
        unrelated.evidence
    );

    let case_only = DebugDiagnosis::from_error_category(
        ErrorPhase::ExecuteWord,
        Some("ADD"),
        Some(&ErrorCategory::StackUnderflow),
        None,
        1,
        1,
        None,
    )
    .with_source_word(Some("add"));
    assert!(
        !case_only
            .evidence
            .iter()
            .any(|e| e.starts_with("sourceWord=")),
        "{:?}",
        case_only.evidence
    );
}
