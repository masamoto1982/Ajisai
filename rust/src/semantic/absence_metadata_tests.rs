//! Test suite for `crate::semantic::absence`.

use crate::error::NilReason;
use crate::semantic::AbsenceOrigin;
use crate::types::Value;

#[test]
fn nil_literal_has_diagnostic_absence_semantics() {
    let value = Value::nil_literal();
    let absence = value
        .absence_metadata()
        .expect("literal NIL has absence metadata");

    assert!(value.is_absent());
    assert!(value.is_nil());
    assert_eq!(absence.origin, AbsenceOrigin::Literal);
    // A written NIL carries a reason like every other NIL (LANG.VALUES.NIL);
    // `literal` is the one that fits — nothing failed to produce it.
    assert_eq!(absence.reason, Some(NilReason::Literal));
}
