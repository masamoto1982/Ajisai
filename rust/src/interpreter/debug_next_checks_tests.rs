//! Test suite for `crate::interpreter::debug_next_checks`.

use crate::error::{ErrorCategory, NilReason};
use crate::interpreter::debug_diagnosis::{CauseClass, DebugCheck};
use crate::interpreter::debug_next_checks::build_next_checks;

/// The checks for a case, holding the two inputs this suite does not vary: no
/// spelling candidates, and an empty stack beneath the failing Word. Cases
/// that are *about* either of those call `build_next_checks` directly.
fn checks_for(
    why: &CauseClass,
    word: Option<&str>,
    category: Option<&ErrorCategory>,
    nil_reason: Option<&NilReason>,
) -> Vec<DebugCheck> {
    build_next_checks(why, word, category, nil_reason, &[], 0)
}

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
        Some(ErrorCategory::Declared("protectedWord")),
    ];
    for why in &classes {
        for category in &categories {
            for check in checks_for(why, Some("MAP"), category.as_ref(), None) {
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
    let sizes = checks_for(
        &CauseClass::ResourceLimit,
        None,
        Some(&ErrorCategory::ResourceLimitExceeded),
        None,
    );
    let steps = checks_for(
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
    let checks = checks_for(&CauseClass::StackShape, Some("TOKENIZE"), None, None);
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
fn a_stack_underflow_says_how_many_values_are_missing() {
    // `1 ADD`: the arity line and the locus line each held half of "push one
    // more", four lines apart, and the reader was left to subtract.
    let checks = build_next_checks(&CauseClass::StackShape, Some("ADD"), None, None, &[], 1);
    let arity = &checks[0];
    assert_eq!(arity.code, "checkDeclaredArity");
    for fragment in [
        "( 2 -- 1 )",
        "needs 2 values",
        "the stack held 1",
        "Push 1 more",
    ] {
        assert!(
            arity.detail.en.contains(fragment),
            "expected {fragment:?} in {}",
            arity.detail.en
        );
    }
    assert!(
        arity.detail.ja.contains("あと 1 個積む"),
        "{}",
        arity.detail.ja
    );
}

#[test]
fn an_arity_the_depth_already_meets_keeps_the_plain_declaration() {
    // A Word that underflowed with its operands apparently present failed for
    // a reason this sentence would misdescribe, so it is not said.
    let checks = build_next_checks(&CauseClass::StackShape, Some("ADD"), None, None, &[], 5);
    assert_eq!(checks[0].detail.en, "ADD declares ( 2 -- 1 ).");
}

#[test]
fn the_spelling_check_names_the_candidates_it_has() {
    let named = build_next_checks(
        &CauseClass::TypoOrUnknownName,
        Some("MAPP"),
        None,
        None,
        &["MAP".to_string()],
        0,
    );
    let spelling = named
        .iter()
        .find(|c| c.code == "checkSpelling")
        .expect("an unresolved name gets a spelling check");
    assert!(spelling.detail.en.contains("MAP"), "{}", spelling.detail.en);
}

#[test]
fn the_spelling_check_promises_no_list_when_there_is_none() {
    // `^`, `FOO` and `=` all suggest nothing: a symbol is never a typo of an
    // alphabetic name, and the distance ceiling is deliberately tight. The
    // check used to send the reader to `diagnosis.candidates` regardless —
    // a field no host prints, holding nothing.
    let checks = checks_for(&CauseClass::TypoOrUnknownName, Some("FOO"), None, None);
    let spelling = checks
        .iter()
        .find(|c| c.code == "checkSpelling")
        .expect("an unresolved name gets a spelling check");
    assert!(
        !spelling.detail.en.contains("diagnosis.candidates")
            && !spelling.detail.ja.contains("diagnosis.candidates"),
        "no check may point at a protocol field the reader cannot see: {}",
        spelling.detail.en
    );
    assert!(
        spelling.detail.en.contains("never defined"),
        "say what an empty list means: {}",
        spelling.detail.en
    );
}

#[test]
fn an_unclassified_raise_lists_the_declared_conditions_and_a_classified_one_does_not() {
    // `why: unknown` is where the alternative is "read the message", which is
    // what the caller had already read.
    let unclassified = checks_for(&CauseClass::Unknown, Some("NUM"), None, None);
    assert_eq!(
        unclassified.first().map(|c| c.code),
        Some("checkDeclaredErrorConditions")
    );
    assert!(unclassified[0].detail.en.contains("nonText"));

    // Where the class *is* decided, its own checks are more specific than a
    // list of every condition the Word declares, so the list stays out.
    let classified = checks_for(
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
    let checks = checks_for(
        &CauseClass::Domain,
        Some("SQRT"),
        // No `ErrorCategory` names a domain miss (`error_category_for_nil_reason`
        // answers `None` for it), matching the real call site.
        None,
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
    let checks = checks_for(&CauseClass::StackShape, Some("MY-WORD"), None, None);
    assert!(checks.iter().all(|c| !c.code.starts_with("checkDeclared")));
    assert_eq!(checks.first().map(|c| c.code), Some("checkArity"));
}

/// Every Word-shaped name a check *names to the reader* must be a name the
/// dictionary will actually accept.
///
/// This gate exists because of a defect it would have caught on the day it
/// landed: the zero-division check told both locales to "handle it with SAFE",
/// and `SAFE` is not an Ajisai word — it is the pre-rename spelling of
/// `OR-NIL`. `1 0 /` therefore answered a correct NIL whose diagnosis sent the
/// reader to `1 0 / SAFE`, which fails with `Unknown word: SAFE`, and no check
/// ever named `OR-NIL`. For a language whose stated claim is that a machine can
/// follow a structured diagnosis to a first-attempt repair, a check that names
/// a word the dictionary rejects is worse than a check that names none.
///
/// Prose is not the thing being pinned here: a check may be reworded freely.
/// What may not change is that a name written in Word shape resolves.
mod diagnosis_vocabulary_is_real {
    use super::*;
    use crate::error::NilReason;

    /// Word-shaped: upper-case runs, optionally hyphenated (`OR-NIL`,
    /// `NIL-REASON`), which is exactly the shape Ajisai Words are written in.
    ///
    /// Single letters are excluded: `U` (the truth value) and `A`/`I` as
    /// ordinary English are not Word references, and no Word is one letter.
    fn word_shaped_tokens(text: &str) -> Vec<String> {
        let mut out = Vec::new();
        for raw in text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-')) {
            let token = raw.trim_matches('-');
            if token.len() < 2 {
                continue;
            }
            let has_letter = token.chars().any(|c| c.is_ascii_alphabetic());
            let all_upper = token
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-');
            if has_letter && all_upper {
                out.push(token.to_string());
            }
        }
        out
    }

    /// Names that are written in Word shape but denote something other than a
    /// Word, so the dictionary is the wrong place to look them up.
    ///
    /// Keep this list short and justified. A new entry is a claim that a
    /// reader will not mistake the name for something they can type.
    fn is_not_a_word_reference(token: &str) -> bool {
        matches!(
            token,
            // Type and value names from LANG.VALUES, not callable Words.
            "NIL" | "TRUE" | "FALSE"
            // Specification section ids, which appear verbatim in checks.
            | "LANG"
        ) || token.starts_with("LANG-")
    }

    fn resolves(token: &str) -> bool {
        if crate::kernel::generated::generated_word(token).is_some() {
            return true;
        }
        if crate::surface_forms::lookup_surface_form(token).is_some() {
            return true;
        }
        let canonical = crate::core_word_aliases::canonicalize_core_word_name(token);
        crate::kernel::generated::generated_word(canonical.as_ref()).is_some()
    }

    #[test]
    fn every_word_named_by_a_check_is_in_the_dictionary() {
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
            Some(ErrorCategory::StackUnderflow),
            Some(ErrorCategory::UnknownWord),
            Some(ErrorCategory::DivisionByZero),
            Some(ErrorCategory::VectorLengthMismatch),
            Some(ErrorCategory::ShapeMismatch),
            Some(ErrorCategory::MalformedSource),
            Some(ErrorCategory::Declared("nameConflict")),
            Some(ErrorCategory::ExecutionLimitExceeded),
            Some(ErrorCategory::ResourceLimitExceeded),
            Some(ErrorCategory::RecursionLimitExceeded),
            Some(ErrorCategory::Declared("protectedWord")),
            Some(ErrorCategory::SelfReferentialDefinition),
            Some(ErrorCategory::Declared("divisorEqualsZero")),
        ];
        let reasons = [
            None,
            Some(NilReason::DivisionByZero),
            Some(NilReason::NotFound),
            Some(NilReason::InvalidEncoding),
            Some(NilReason::IndexOutOfBounds),
            Some(NilReason::SpaceExhausted),
            Some(NilReason::DomainMiss),
            Some(NilReason::NotAvailable),
            Some(NilReason::Literal),
        ];

        // `DIV` is a real Word, so a check that interpolates the failing word's
        // own name stays resolvable and the sweep tests the *table's* text.
        for why in &classes {
            for category in &categories {
                for reason in &reasons {
                    let checks = checks_for(why, Some("DIV"), category.as_ref(), reason.as_ref());
                    for check in checks {
                        for text in [
                            &check.detail.en,
                            &check.detail.ja,
                            &check.title.en,
                            &check.title.ja,
                        ] {
                            for token in word_shaped_tokens(text) {
                                if is_not_a_word_reference(&token) {
                                    continue;
                                }
                                assert!(
                                    resolves(&token),
                                    "check `{}` names `{token}`, which no Ajisai Word, alias or \
                                     surface form resolves. A diagnosis may not send a reader to \
                                     a name the dictionary rejects. Text: {text}",
                                    check.code
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// The positive half: the zero-division advice must name the Words that
    /// actually recover a NIL, so the fix is not merely "stopped saying SAFE".
    #[test]
    fn zero_division_advice_names_the_recovery_words() {
        let checks = checks_for(
            &CauseClass::Domain,
            Some("DIV"),
            Some(&ErrorCategory::DivisionByZero),
            Some(&NilReason::DivisionByZero),
        );
        let advice = checks
            .iter()
            .find(|c| c.code == "checkZeroIsExpected")
            .expect("zero-division diagnosis offers a recovery check");
        for word in ["NIL?", "SELECT"] {
            assert!(
                advice.detail.en.contains(word) && advice.detail.ja.contains(word),
                "both locales must name {word}: en={} ja={}",
                advice.detail.en,
                advice.detail.ja
            );
        }
        assert!(
            !advice.detail.en.contains("SAFE") && !advice.detail.ja.contains("SAFE"),
            "SAFE is not an Ajisai Word and must not be advised"
        );
    }
}
