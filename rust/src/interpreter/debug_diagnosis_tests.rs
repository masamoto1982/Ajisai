//! Test suite for `crate::interpreter::debug_diagnosis`.

use crate::error::ErrorCategory;
use crate::interpreter::debug_diagnosis::{
    classify_locus, DebugDiagnosis, ErrorLocusKind, ErrorPhase,
};

/// The registry names a Core Word; a User Word is named where the live
/// dictionary is. A name the registry does not hold stays unknown until then,
/// and becomes a User Word only if the dictionary holds it.
#[test]
fn a_user_word_locus_is_named_from_the_live_dictionary() {
    assert_eq!(classify_locus(Some("ADD")).kind, ErrorLocusKind::CoreWord);
    assert_eq!(classify_locus(Some("DOUBLE")).kind, ErrorLocusKind::Unknown);

    let mut diagnosis = DebugDiagnosis::from_error_category(
        ErrorPhase::ExecuteWord,
        Some("DOUBLE"),
        Some(&ErrorCategory::StackUnderflow),
        None,
        0,
        0,
        None,
    );
    diagnosis.with_user_vocabulary(["DOUBLE", "TWICE"].into_iter());
    assert_eq!(diagnosis.where_.kind, ErrorLocusKind::UserWord);

    let mut unknown = DebugDiagnosis::from_error_category(
        ErrorPhase::ResolveWord,
        Some("FROB"),
        Some(&ErrorCategory::UnknownWord),
        None,
        0,
        0,
        None,
    );
    unknown.with_user_vocabulary(["DOUBLE"].into_iter());
    assert_eq!(unknown.where_.kind, ErrorLocusKind::Unknown);
}

#[tokio::test]
async fn a_failing_user_word_reports_a_user_word_locus() {
    let mut interp = crate::interpreter::Interpreter::new();
    assert!(interp.execute("[ 1 ADD ] 'W' DEF W").await.is_err());
    let trace = interp.drain_error_flow_trace();
    let diagnosis = trace
        .iter()
        .rev()
        .find_map(|event| event.diagnosis.as_ref())
        .expect("the failure carries a diagnosis");
    assert_eq!(diagnosis.where_.word.as_deref(), Some("W"));
    assert_eq!(diagnosis.where_.kind, ErrorLocusKind::UserWord);
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
