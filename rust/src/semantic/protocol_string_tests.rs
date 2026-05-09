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
    assert_eq!(
        ValueOrigin::SafeProjection.as_protocol_str(),
        "safeProjection"
    );
}

#[test]
fn absence_and_diagnosis_protocol_strings_do_not_use_debug_names() {
    assert_eq!(
        AbsenceOrigin::SafeProjection.as_protocol_str(),
        "safeProjection"
    );
    assert_eq!(Recoverability::Recoverable.as_protocol_str(), "recoverable");
    assert_eq!(
        NilReason::SafeCaught(Box::new(ErrorCategory::DivisionByZero)).as_protocol_str(),
        "safeCaught"
    );
    assert_eq!(
        ErrorCategory::DivisionByZero.as_protocol_str(),
        "divisionByZero"
    );
    assert_eq!(ErrorPhase::ResolveWord.as_protocol_str(), "resolveWord");
    assert_eq!(ErrorLocusKind::CoreWord.as_protocol_str(), "coreWord");
    assert_eq!(
        CauseClass::TypoOrUnknownName.as_protocol_str(),
        "typoOrUnknownName"
    );
}
