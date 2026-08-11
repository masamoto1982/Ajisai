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
            for check in build_next_checks(why, Some("MAP"), category.as_ref()) {
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
    );
    let steps = build_next_checks(
        &CauseClass::ResourceLimit,
        None,
        Some(&ErrorCategory::ExecutionLimitExceeded),
    );
    assert_eq!(sizes.first().map(|c| c.code), Some("checkWhichLimit"));
    assert_eq!(steps.first().map(|c| c.code), Some("checkBudgetVsWork"));
}
