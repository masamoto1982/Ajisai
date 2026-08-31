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
fn comparison_budget_undecidable_protocol_strings() {
    // SPEC §7.4.1 requires the comparison-budget NIL to be tagged
    // with `reason = "undecidable"` and `origin =
    // "comparisonBudget"`. The runtime constructs this via
    // `Value::nil_with_reason_unknown(NilReason::Undecidable)` and the
    // origin is derived through `absence_origin_for_reason`.
    assert_eq!(NilReason::Undecidable.as_protocol_str(), "undecidable");
    assert_eq!(
        AbsenceOrigin::ComparisonBudget.as_protocol_str(),
        "comparisonBudget"
    );
}

#[test]
fn domain_miss_protocol_strings() {
    // SPEC §5 names the classification: "SQRT of a negative rational is a
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
    let all = [
        NilReason::DivisionByZero,
        NilReason::EmptySequence,
        NilReason::MissingField,
        NilReason::InvalidEncoding,
        NilReason::InvalidLens,
        NilReason::StackUnderflow,
        NilReason::IndexOutOfBounds,
        NilReason::UnknownWord,
        NilReason::ExecutionFailure,
        NilReason::Undecidable,
        NilReason::SpaceExhausted,
        NilReason::DomainMiss,
        NilReason::NotAvailable,
        NilReason::Literal,
    ];
    let mut seen: Vec<&str> = Vec::new();
    for reason in &all {
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
fn unknown_advertises_truth_valued_capability() {
    // LANG.VALUES.TRUTH: the logical Unknown (U) is observed through the
    // `truthValue` axis as `unknown` and advertises the `truthValued`
    // capability. U has no dedicated `ValueData` variant — it is `Nil`
    // data carrying the `TruthValue` hint. `AND`/`OR`/`NOT` construct it at
    // the interpreter level (`interpreter::logic::as_unknown`); this test
    // stays at the `Value` level, so it still builds one directly.
    use crate::types::{Interpretation, Value};
    let mut u = Value::nil();
    u.hint = Interpretation::TruthValue;
    assert_eq!(u.truth_value(), Some("unknown"));
    assert!(u.has_capability(Capability::TruthValued));
}
#[test]
fn definite_truth_values_expose_truth_value_axis() {
    use crate::types::{Interpretation, Value};
    let mut t = Value::from_bool(true);
    t.hint = Interpretation::TruthValue;
    let mut f = Value::from_bool(false);
    f.hint = Interpretation::TruthValue;
    assert_eq!(t.truth_value(), Some("true"));
    assert_eq!(f.truth_value(), Some("false"));
    assert!(t.has_capability(Capability::TruthValued));
    // A plain number is not truth-valued.
    assert_eq!(Value::from_int(1).truth_value(), None);
    assert!(!Value::from_int(1).has_capability(Capability::TruthValued));
}

#[test]
fn nil_with_reason_undecidable_routes_to_comparison_budget_origin() {
    // `nil_with_reason` is the runtime's primary entry point for
    // building reasoned NIL values. Verify the §7.4.1 reason/origin
    // pairing is preserved end-to-end.
    use crate::types::Value;
    let v = Value::nil_with_reason_unknown(NilReason::Undecidable);
    let absence = v.absence_metadata().expect("nil carries absence");
    assert_eq!(absence.reason, Some(NilReason::Undecidable));
    assert_eq!(absence.origin, AbsenceOrigin::ComparisonBudget);
}
