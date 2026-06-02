//! Test suite for `crate::semantic::absence`.

use crate::semantic::{AbsenceOrigin, SemanticKind, ValueShape};
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
