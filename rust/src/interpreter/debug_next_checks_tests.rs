//! Test suite for `crate::interpreter::debug_next_checks`.

use crate::error::ErrorCategory;
use crate::interpreter::debug_diagnosis::CauseClass;
use crate::interpreter::debug_next_checks::build_next_checks;

/// Hiragana, katakana or CJK ideographs — enough to catch a Japanese
/// sentence that leaked into the English locale.
fn is_japanese(c: char) -> bool {
    matches!(c, '\u{3040}'..='\u{30ff}' | '\u{4e00}'..='\u{9fff}')
}

#[test]
fn every_check_carries_a_code_and_both_locales() {
    let classes = [
        CauseClass::Domain,
        CauseClass::StackShape,
        CauseClass::TypoOrUnknownName,
        CauseClass::Environment,
        CauseClass::ValueShape,
        CauseClass::Index,
        CauseClass::VectorLength,
        CauseClass::ShapeMismatch,
        CauseClass::SourceForm,
        CauseClass::ResourceLimit,
        CauseClass::UserLogic,
        CauseClass::ContractViolation,
        CauseClass::Effect,
        CauseClass::NilFlow,
        CauseClass::OptimizerMismatch,
        CauseClass::InternalInvariant,
        CauseClass::Unknown,
    ];
    let categories = [
        None,
        Some(ErrorCategory::DivisionByZero),
        Some(ErrorCategory::ExecutionLimitExceeded),
        Some(ErrorCategory::ResourceLimitExceeded),
        Some(ErrorCategory::RecursionLimitExceeded),
        Some(ErrorCategory::CondExhausted),
        Some(ErrorCategory::ModeUnsupported),
        Some(ErrorCategory::BuiltinProtection),
    ];
    for why in &classes {
        for category in &categories {
            for check in build_next_checks(why, Some("MAP"), category.as_ref(), None) {
                assert!(
                    !check.code.is_empty(),
                    "{why:?} produced a check with no code"
                );
                assert!(!check.title.en.is_empty() && !check.title.ja.is_empty());
                assert!(!check.detail.en.is_empty() && !check.detail.ja.is_empty());
                // The point of splitting the locales was that neither one
                // may carry the other's language.
                assert!(
                    !check.detail.en.chars().any(is_japanese),
                    "the en locale must not carry Japanese text: {}",
                    check.detail.en
                );
                assert!(
                    !check.title.en.chars().any(is_japanese),
                    "the en locale must not carry Japanese text: {}",
                    check.title.en
                );
            }
        }
    }
}

#[test]
fn a_size_ceiling_and_a_step_budget_get_different_advice() {
    let sizes = build_next_checks(
        &CauseClass::ResourceLimit,
        None,
        Some(&ErrorCategory::ResourceLimitExceeded),
        None,
    );
    let steps = build_next_checks(
        &CauseClass::ResourceLimit,
        None,
        Some(&ErrorCategory::ExecutionLimitExceeded),
        None,
    );
    assert_eq!(sizes.first().map(|c| c.code), Some("checkWhichLimit"));
    assert_eq!(steps.first().map(|c| c.code), Some("checkBudgetVsWork"));
}

#[test]
fn a_stack_underflow_names_the_declared_arity_and_a_correct_call() {
    let checks = build_next_checks(&CauseClass::StackShape, Some("TOKENIZE"), None, None);
    let codes: Vec<&str> = checks.iter().map(|c| c.code).collect();
    assert_eq!(codes.first(), Some(&"checkDeclaredArity"));
    assert_eq!(codes.get(1), Some(&"checkDeclaredSyntax"));
    assert!(
        checks[0].detail.en.contains("( 2 -- 1 )"),
        "the declared arity must be written out, not merely referred to: {}",
        checks[0].detail.en
    );
    assert!(
        checks[1].detail.en.contains("TOKENIZE"),
        "a correct call names the Word: {}",
        checks[1].detail.en
    );
}

#[test]
fn an_unclassified_raise_lists_the_declared_conditions_and_a_classified_one_does_not() {
    // `why: unknown` is where the alternative is "read the message", which is
    // what the caller had already read.
    let unclassified = build_next_checks(&CauseClass::Unknown, Some("NUM"), None, None);
    assert_eq!(
        unclassified.first().map(|c| c.code),
        Some("checkDeclaredErrorConditions")
    );
    assert!(unclassified[0].detail.en.contains("nonText"));

    // Where the class *is* decided, its own checks are more specific than a
    // list of every condition the Word declares, so the list stays out.
    let classified = build_next_checks(
        &CauseClass::ShapeMismatch,
        Some("ADD"),
        Some(&ErrorCategory::ShapeMismatch),
        None,
    );
    assert!(classified
        .iter()
        .all(|c| c.code != "checkDeclaredErrorConditions"));
}

#[test]
fn a_projection_names_the_condition_the_registry_declares_for_it() {
    let checks = build_next_checks(
        &CauseClass::Domain,
        Some("SQRT"),
        Some(&ErrorCategory::Custom),
        Some(&crate::error::NilReason::DomainMiss),
    );
    let projection = checks
        .iter()
        .find(|c| c.code == "checkDeclaredProjection")
        .expect("a projecting Word's declared condition must reach the diagnosis");
    assert!(
        projection.detail.en.contains("negativeScalar"),
        "the declared condition must be named: {}",
        projection.detail.en
    );
}

#[test]
fn a_word_with_no_registry_entry_still_gets_its_class_level_checks() {
    // A user Word is not declared, so nothing is derived — but the class-level
    // table must still answer, or an undeclared Word would get no advice at all.
    let checks = build_next_checks(&CauseClass::StackShape, Some("MY-WORD"), None, None);
    assert!(checks.iter().all(|c| !c.code.starts_with("checkDeclared")));
    assert_eq!(checks.first().map(|c| c.code), Some("checkArity"));
}
