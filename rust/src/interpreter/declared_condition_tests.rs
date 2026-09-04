//! Test suite for raises that name a condition their Word's contract declares.
//!
//! A raise written as a bare sentence reaches the caller classified `custom` /
//! `unknown`, which is the one classification that says nothing: the reader is
//! told to "read the message", which is what they had already read. The
//! registry declares, per Word, the conditions it raises under; these pin that
//! a raise names one of them, that the name survives to `why` and to
//! `aiDiagnostic.kind`, and that the declared vocabulary is classified as a
//! vocabulary rather than Word by Word.

use crate::error::ErrorCategory;
use crate::interpreter::debug_declared_checks::cause_class_for_declared_condition;
use crate::interpreter::debug_diagnosis::CauseClass;
use crate::interpreter::error_flow_trace::ErrorFlowEventKind;
use crate::interpreter::Interpreter;
use crate::kernel::generated::GENERATED_WORDS;

/// Run `code`, expect it to raise, and answer the diagnosis of the raise.
async fn raise_diagnosis(code: &str) -> crate::interpreter::debug_diagnosis::DebugDiagnosis {
    let mut interp = Interpreter::new();
    let outcome = interp.execute(code).await;
    assert!(outcome.is_err(), "expected {:?} to raise", code);
    let trace = interp.drain_error_flow_trace();
    trace
        .iter()
        .filter(|e| e.kind == ErrorFlowEventKind::WordError)
        .find_map(|e| e.diagnosis.clone())
        .unwrap_or_else(|| panic!("expected a diagnosis for {:?}", code))
}

/// Every condition the spec declares has to classify, or a Word earns
/// `unknown` for a state its own contract named. The vocabulary is
/// `spec/words.json`'s, so this fails the day the spec grows a condition the
/// mapping does not answer for — which is the moment to answer for it, not
/// after a caller has been told "read the message" in production.
#[test]
fn declared_condition_vocabulary_is_classified() {
    let mut unclassified: Vec<&str> = GENERATED_WORDS
        .iter()
        .flat_map(|word| word.error_when.iter().copied())
        .filter(|condition| {
            matches!(
                cause_class_for_declared_condition(condition),
                CauseClass::Unknown
            )
        })
        .collect();
    unclassified.sort_unstable();
    unclassified.dedup();
    assert!(
        unclassified.is_empty(),
        "these declared conditions have no cause class: {:?}",
        unclassified
    );
}

/// The condition a raise names has to be one the raising Word declares.
/// Naming a condition the contract does not carry would answer in a vocabulary
/// the caller cannot look up.
#[tokio::test]
async fn a_named_condition_is_one_the_word_declares() {
    for (code, word) in [
        ("[ 1 2 ] [ 'X' BIND ] MAP", "MAP"),
        ("[ 1 2 ] [ 0 ] [ 'A' BIND 'B' BIND ] FOLD", "FOLD"),
        ("[ 1 2 ] [ 'X' BIND ] FILTER", "FILTER"),
        ("[ 1 2 ] [ 'X' BIND ] ALL", "ALL"),
        ("[ 1 2 ] [ 'X' BIND ] ANY", "ANY"),
        ("42 PROBE", "PROBE"),
        ("[ 1 2 ] 5 MAP", "MAP"),
        ("TRUE NUM", "NUM"),
        ("NIL NUM", "NUM"),
        ("[ 0 5 0 ] RANGE", "RANGE"),
        ("[ 5 0 1 ] RANGE", "RANGE"),
        ("[ 1 ] [ [ TRUE ] [ FALSE ] [ TRUE ] ] COND", "COND"),
        ("[ 1 ] [ [ 'x' | 1 ] ] COND", "COND"),
    ] {
        let diagnosis = raise_diagnosis(code).await;
        let declared = GENERATED_WORDS
            .iter()
            .find(|w| w.name == word)
            .unwrap_or_else(|| panic!("{} must be a declared Word", word));
        let condition = diagnosis.summary.clone();
        assert!(
            declared
                .error_when
                .iter()
                .any(|c| condition.contains(&format!("({})", c))),
            "{:?} answered a condition {} does not declare ({:?}): {}",
            code,
            word,
            declared.error_when,
            condition
        );
        assert_ne!(
            diagnosis.why,
            CauseClass::Unknown,
            "{:?} left the cause class unknown: {}",
            code,
            condition
        );
    }
}

/// The declared condition reaches the caller in the same vocabulary
/// `word_contract` publishes it in, rather than as the catch-all `custom`.
#[tokio::test]
async fn the_named_condition_is_the_protocol_category() {
    let diagnosis = raise_diagnosis("[ 1 2 ] [ 'X' BIND ] MAP").await;
    assert_eq!(diagnosis.why, CauseClass::ContractViolation);
    assert_eq!(
        ErrorCategory::Declared("blockContractViolation").as_protocol_str(),
        "blockContractViolation"
    );
    let fired = diagnosis
        .next_checks
        .iter()
        .find(|c| c.code == "checkFiredCondition")
        .expect("a named condition earns a check that names it");
    assert!(
        fired.detail.en.contains("blockContractViolation"),
        "the check has to say which condition fired: {}",
        fired.detail.en
    );
    assert!(
        fired.detail.ja.contains("blockContractViolation"),
        "both locales carry the condition: {}",
        fired.detail.ja
    );
}

/// `notExecutable` is a wrong operand and `blockContractViolation` a broken
/// promise, so they must not answer the caller with the same repair.
#[tokio::test]
async fn a_wrong_operand_and_a_broken_block_get_different_repairs() {
    let operand = raise_diagnosis("42 PROBE").await;
    let block = raise_diagnosis("[ 1 2 ] [ 'X' BIND ] MAP").await;
    assert_eq!(operand.why, CauseClass::ValueShape);
    assert_eq!(block.why, CauseClass::ContractViolation);
}
