//! Test suite for `crate::semantic::absence`.

use crate::error::{ErrorCategory, NilReason};
use crate::semantic::{AbsenceOrigin, Recoverability, SemanticKind, ValueShape};
use crate::types::Value;

#[test]
fn nil_literal_has_diagnostic_absence_semantics() {
    let value = Value::nil_literal();
    let absence = value
        .absence_metadata()
        .expect("literal NIL has absence metadata");

    assert!(value.is_absent());
    assert!(value.is_nil());
    assert_eq!(value.semantic_kind(), SemanticKind::Absence);
    assert_eq!(value.shape_kind(), ValueShape::Absence);
    assert_eq!(absence.origin, AbsenceOrigin::Literal);
    assert!(absence.reason.is_none());
}

#[test]
fn nil_with_safe_caught_reason_preserves_caught_category() {
    let value = Value::nil_with_reason(NilReason::SafeCaught(Box::new(
        ErrorCategory::DivisionByZero,
    )));
    let absence = value
        .absence_metadata()
        .expect("SAFE NIL has absence metadata");
    let reason = absence.reason.as_ref().expect("SAFE NIL has reason");

    assert_eq!(reason.as_protocol_str(), "safeCaught");
    assert_eq!(
        reason.caught_category().map(ErrorCategory::as_protocol_str),
        Some("divisionByZero")
    );
    assert_eq!(absence.origin, AbsenceOrigin::SafeProjection);
    assert_eq!(absence.recoverability, Recoverability::Unknown);
}
