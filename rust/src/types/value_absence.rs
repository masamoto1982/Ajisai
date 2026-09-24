//! Operational absence and the truth observation for [`Value`].
//!
//! Invariant: a [`NilReason`] chooses its [`AbsenceOrigin`] in exactly one place,
//! and every reasoned NIL constructor routes through that exhaustive mapping.

use super::{Value, ValueData};
use crate::error::NilReason;
use crate::semantic::{AbsenceMetadata, AbsenceOrigin, Recoverability};

/// The single derivation of an absence origin from a NIL reason.
///
/// Nothing else may compute one. Call sites used to pass an origin alongside a
/// reason, which gave the pairing two sources and let them disagree: `DIV` by
/// zero reported `reason = divisionByZero` with `origin = executionFailure`,
/// contradicting `AbsenceOrigin::DivisionByZero`'s own documentation, and
/// `INDEX-OF` did the same to `missingField`. Deriving here means a new reason
/// gets its origin by adding one arm, and gets it everywhere at once.
fn absence_origin_for_reason(reason: &NilReason) -> AbsenceOrigin {
    match reason {
        NilReason::MissingField => AbsenceOrigin::MissingField,
        NilReason::InvalidEncoding => AbsenceOrigin::InvalidEncoding,
        NilReason::IndexOutOfBounds => AbsenceOrigin::IndexOutOfBounds,
        NilReason::Undecidable => AbsenceOrigin::ComparisonBudget,
        NilReason::DivisionByZero => AbsenceOrigin::DivisionByZero,
        NilReason::SpaceExhausted => AbsenceOrigin::SpaceBudget,
        NilReason::DomainMiss => AbsenceOrigin::DomainMiss,
        NilReason::NotAvailable => AbsenceOrigin::NotAvailable,
        NilReason::Literal => AbsenceOrigin::Literal,
        NilReason::UserDeclared => AbsenceOrigin::UserDeclared,
    }
}

impl Value {
    #[inline]
    pub fn nil() -> Self {
        Self::nil_literal()
    }

    #[inline]
    pub fn nil_literal() -> Self {
        Self {
            data: ValueData::Nil,
            absence: Some(AbsenceMetadata::literal()),
        }
    }

    #[inline]
    pub fn nil_with_absence(absence: AbsenceMetadata) -> Self {
        Self {
            data: ValueData::Nil,
            absence: Some(absence),
        }
    }

    #[inline]
    pub fn nil_with_reason_unknown(reason: NilReason) -> Self {
        let origin = absence_origin_for_reason(&reason);
        Self::nil_with_absence(AbsenceMetadata::with_reason(
            reason,
            origin,
            Recoverability::Unknown,
        ))
    }

    /// The observable `truthValue` axis (LANG.VALUES.TRUTH,
    /// LANG.OBSERVATION.PROTOCOL): `Some("true")` or `Some("false")` for a
    /// Boolean, `None` for every other value. It is read off the value's
    /// domain alone. UNKNOWN is a NIL (LANG.VALUES.TRUTH) and is observed as
    /// one, with its reason, so it has no entry on this axis.
    pub fn truth_value(&self) -> Option<&'static str> {
        match &self.data {
            ValueData::Boolean(b) => Some(if *b { "true" } else { "false" }),
            _ => None,
        }
    }

    #[inline]
    pub fn nil_inheriting_absence_from(source: &Self) -> Self {
        match source.normalized_absence_metadata() {
            Some(absence) => Self::nil_with_absence(absence),
            None => Self::nil(),
        }
    }

    /// Create a reasoned NIL for the NIL Projection Rule (the specification's
    /// "Bubble Rule"): well-formed operations that cannot produce a value
    /// return a reasoned NIL directly with an explicit reason.
    ///
    /// The origin follows from the reason via [`absence_origin_for_reason`] and
    /// is deliberately not a parameter. Recoverability is, because it genuinely
    /// varies for one reason — a disconnected port is `Fatal` while an empty
    /// read buffer is `Retryable` — and cannot be read off the reason alone.
    #[inline]
    pub fn nil_with_reason(reason: NilReason, recoverability: Recoverability) -> Self {
        let origin = absence_origin_for_reason(&reason);
        Self::nil_with_absence(AbsenceMetadata::with_reason(reason, origin, recoverability))
    }

    /// The NIL `ABSENT` produces: reason `userDeclared`, carrying `detail`.
    #[inline]
    pub fn nil_user_declared(detail: &str) -> Self {
        Self::nil_with_absence(AbsenceMetadata::user_declared(detail))
    }

    #[inline]
    pub fn absence_metadata(&self) -> Option<&AbsenceMetadata> {
        self.absence.as_ref()
    }

    /// The text a `userDeclared` absence carries beside its reason.
    #[inline]
    pub fn absence_detail(&self) -> Option<&str> {
        self.absence.as_ref().and_then(AbsenceMetadata::detail_text)
    }

    #[inline]
    pub fn normalized_absence_metadata(&self) -> Option<AbsenceMetadata> {
        if !self.is_absent() {
            return None;
        }
        Some(
            self.absence
                .clone()
                .unwrap_or_else(AbsenceMetadata::with_reasonless_unknown),
        )
    }

    #[inline]
    pub fn nil_reason(&self) -> Option<&NilReason> {
        self.absence
            .as_ref()
            .and_then(|absence| absence.reason.as_ref())
    }
}
