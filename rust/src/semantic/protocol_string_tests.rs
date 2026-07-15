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
    assert_eq!(ValueOrigin::Computed.as_protocol_str(), "computed");
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
    // `Value::nil_with_reason(NilReason::Undecidable)` and the
    // origin is derived through `absence_origin_for_reason`.
    assert_eq!(NilReason::Undecidable.as_protocol_str(), "undecidable");
    assert_eq!(
        AbsenceOrigin::ComparisonBudget.as_protocol_str(),
        "comparisonBudget"
    );
}

#[test]
fn logically_unknown_protocol_strings() {
    // SPEC §7.5 / §2.3: the logical Unknown (U) carries the
    // `logicallyUnknown` reason internally and the `truthValued`
    // capability; it is observed through the `truthValue` axis as
    // `unknown`.
    assert_eq!(
        NilReason::LogicallyUnknown.as_protocol_str(),
        "logicallyUnknown"
    );
    assert_eq!(Capability::TruthValued.as_protocol_str(), "truthValued");
}

#[test]
fn unknown_value_exposes_truth_value_axis_and_capability() {
    use crate::types::Value;
    let u = Value::unknown();
    assert!(u.is_unknown());
    assert_eq!(u.truth_value(), Some("unknown"));
    assert!(u.has_capability(Capability::TruthValued));
    // The internal NIL representation carries the LogicallyUnknown reason,
    // but consumers must read the truthValue axis, not this reason.
    let absence = u.absence_metadata().expect("U carries absence metadata");
    assert_eq!(absence.reason, Some(NilReason::LogicallyUnknown));
    assert_eq!(absence.origin, AbsenceOrigin::ComparisonBudget);
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
    let v = Value::nil_with_reason(NilReason::Undecidable);
    let absence = v.absence_metadata().expect("nil carries absence");
    assert_eq!(absence.reason, Some(NilReason::Undecidable));
    assert_eq!(absence.origin, AbsenceOrigin::ComparisonBudget);
}
