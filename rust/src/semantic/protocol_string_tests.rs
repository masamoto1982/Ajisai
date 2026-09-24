//! Test suite for `crate::semantic::protocol`.

use super::{AbsenceOrigin, Capability, Recoverability, SemanticKind, ValueOrigin, ValueShape};
use crate::error::{ErrorCategory, NilReason};
use crate::interpreter::debug_diagnosis::{CauseClass, ErrorLocusKind, ErrorPhase};

#[test]
fn semantic_axes_use_lower_camel_case_protocol_strings() {
    assert_eq!(SemanticKind::Absence.as_protocol_str(), "absence");
    assert_eq!(ValueShape::CodeBlock.as_protocol_str(), "codeBlock");
    assert_eq!(Capability::ExactNumeric.as_protocol_str(), "exactNumeric");
    assert_eq!(
        Capability::NilPassthrough.as_protocol_str(),
        "nilPassthrough"
    );
    // A live origin: `Value::origin` still produces this one.
    assert_eq!(
        ValueOrigin::NilPropagation.as_protocol_str(),
        "nilPropagation"
    );
}

#[test]
fn absence_and_diagnosis_protocol_strings_do_not_use_debug_names() {
    assert_eq!(
        AbsenceOrigin::DivisionByZero.as_protocol_str(),
        "divisionByZero"
    );
    assert_eq!(Recoverability::Recoverable.as_protocol_str(), "recoverable");
    assert_eq!(
        ErrorCategory::DivisionByZero.as_protocol_str(),
        "divisionByZero"
    );
    assert_eq!(
        ErrorCategory::RecursionLimitExceeded.as_protocol_str(),
        "recursionLimitExceeded"
    );
    assert_eq!(ErrorPhase::ResolveWord.as_protocol_str(), "resolveWord");
    assert_eq!(ErrorLocusKind::CoreWord.as_protocol_str(), "coreWord");
    assert_eq!(
        CauseClass::TypoOrUnknownName.as_protocol_str(),
        "typoOrUnknownName"
    );
}

#[test]
fn domain_miss_protocol_strings() {
    // LANG.FAILURE.PROJECT names the classification: "SQRT of a negative rational is a
    // well-formed domain miss". Reason and origin share the spelling because
    // the origin is derived from the reason.
    assert_eq!(NilReason::DomainMiss.as_protocol_str(), "domainMiss");
    assert_eq!(AbsenceOrigin::DomainMiss.as_protocol_str(), "domainMiss");
}

/// Every `NilReason` maps to a distinct, lowerCamelCase protocol string. A new
/// variant that forgot its arm, or reused another's spelling, fails here rather
/// than in whichever serializer happened to hit it first.
#[test]
fn every_nil_reason_has_a_distinct_lower_camel_protocol_string() {
    let mut seen: Vec<&str> = Vec::new();
    for reason in NilReason::ALL {
        let s = reason.as_protocol_str();
        assert!(
            s.starts_with(|c: char| c.is_ascii_lowercase())
                && s.chars().all(|c| c.is_alphanumeric()),
            "{s} is not a lowerCamelCase protocol string"
        );
        assert!(!seen.contains(&s), "{s} is used by two reasons");
        seen.push(s);
    }
}

#[test]
fn unknown_is_observed_as_a_nil() {
    // LANG.VALUES.TRUTH: UNKNOWN is NIL read in truth position, not a fourth
    // variant, so it is observed as a NIL — no truth axis, no truth
    // capability, and the NIL capabilities every absence has.
    use crate::types::Value;
    let u = Value::nil_with_reason_unknown(NilReason::DomainMiss);
    assert_eq!(u.truth_value(), None);
    assert!(!u.has_capability(Capability::TruthValued));
    assert!(u.has_capability(Capability::NilPassthrough));
}
#[test]
fn definite_truth_values_expose_truth_value_axis() {
    use crate::types::Value;
    let t = Value::from_bool(true);
    let f = Value::from_bool(false);
    assert_eq!(t.truth_value(), Some("true"));
    assert_eq!(f.truth_value(), Some("false"));
    assert!(t.has_capability(Capability::TruthValued));
    // A plain number is not truth-valued.
    assert_eq!(Value::from_int(1).truth_value(), None);
    assert!(!Value::from_int(1).has_capability(Capability::TruthValued));
}
