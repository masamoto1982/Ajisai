//! Operational absence and truth-role behavior for [`Value`].
//!
//! Invariant: a [`NilReason`] chooses its [`AbsenceOrigin`] in exactly one place,
//! and every reasoned NIL constructor routes through that exhaustive mapping.

use super::{Interpretation, Value, ValueData};
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
        NilReason::EmptySequence => AbsenceOrigin::EmptySequence,
        NilReason::MissingField => AbsenceOrigin::MissingField,
        NilReason::InvalidEncoding => AbsenceOrigin::InvalidEncoding,
        NilReason::InvalidLens => AbsenceOrigin::InvalidLens,
        NilReason::StackUnderflow => AbsenceOrigin::StackUnderflow,
        NilReason::IndexOutOfBounds => AbsenceOrigin::IndexOutOfBounds,
        NilReason::UnknownWord => AbsenceOrigin::UnknownWord,
        NilReason::ExecutionFailure => AbsenceOrigin::ExecutionFailure,
        NilReason::Undecidable => AbsenceOrigin::ComparisonBudget,
        NilReason::DivisionByZero => AbsenceOrigin::DivisionByZero,
        NilReason::SpaceExhausted => AbsenceOrigin::SpaceBudget,
        NilReason::DomainMiss => AbsenceOrigin::DomainMiss,
        NilReason::NotAvailable => AbsenceOrigin::NotAvailable,
        NilReason::Literal => AbsenceOrigin::Literal,
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
            hint: Interpretation::Nil,
            absence: Some(AbsenceMetadata::literal()),
        }
    }

    #[inline]
    pub fn nil_with_absence(absence: AbsenceMetadata) -> Self {
        Self {
            data: ValueData::Nil,
            hint: Interpretation::Nil,
            absence: Some(absence),
        }
    }

    #[inline]
    pub fn nil_with_reason(reason: NilReason) -> Self {
        let origin = absence_origin_for_reason(&reason);
        Self::nil_with_absence(AbsenceMetadata::with_reason(
            reason,
            origin,
            Recoverability::Unknown,
        ))
    }

    /// Construct the logical truth value `Unknown` (U), SPEC §7.5 / §7.4.1.
    ///
    /// U is its own [`ValueData::Unknown`] variant carrying the
    /// `Interpretation::TruthValue` role — **not** a NIL node. It is a
    /// logical value, distinct at the type level from operational absence, so
    /// no NIL call site can absorb it. Detect it with [`is_unknown`], never by
    /// matching the storage representation.
    #[inline]
    /// Whether this value carries the `TruthValue` interpretation role. Used at
    /// observation boundaries to attach the `truthValue` axis.
    pub fn is_truth_value(&self) -> bool {
        self.hint == Interpretation::TruthValue
    }

    /// The observable `truthValue` axis (SPEC §2.3) under a given effective
    /// interpretation role: `Some("true")`, `Some("false")`, or
    /// `Some("unknown")` for truth-valued values, and `None` otherwise.
    ///
    /// The role is taken as a parameter because a definite boolean produced
    /// by a comparison/logic word carries its `TruthValue` role in the
    /// semantic plane (SPEC §12.2), not on the value's own `hint`. The
    /// logical Unknown (U) is always `unknown` regardless of the role, since
    /// it is detected from its reason. This is the single canonical mapping
    /// from a value to its three-valued logical surface; external consumers
    /// must read this axis rather than the internal NIL representation or
    /// display text.
    pub fn truth_value_for_role(&self, effective: Interpretation) -> Option<&'static str> {
        // A Boolean is intrinsically truth-valued: it reports its truth on the
        // axis regardless of the effective role, because its data identity —
        // not a semantic-plane role — carries the truth.
        if let ValueData::Boolean(b) = &self.data {
            return Some(if *b { "true" } else { "false" });
        }
        if effective != Interpretation::TruthValue {
            return None;
        }
        match &self.data {
            ValueData::Nil => Some("unknown"),
            ValueData::Scalar(f) => Some(if f.is_zero() { "false" } else { "true" }),
            ValueData::ExactScalar(_) => Some("true"),
            _ => Some(if self.is_truthy() { "true" } else { "false" }),
        }
    }

    /// The `truthValue` axis using the value's own `hint` as the role.
    /// Convenience for values that carry the `TruthValue` role on the value
    /// itself (notably U); the boundary uses
    /// [`truth_value_for_role`] with the effective role.
    pub fn truth_value(&self) -> Option<&'static str> {
        self.truth_value_for_role(self.hint)
    }

    #[inline]
    pub fn nil_inheriting_absence_from(source: &Self) -> Self {
        match source.normalized_absence_metadata() {
            Some(absence) => Self::nil_with_absence(absence),
            None => Self::nil(),
        }
    }

    /// Create a reasoned NIL for the Bubble Rule: well-formed operations that
    /// cannot produce a value return Bubble/NIL directly with an explicit
    /// reason.
    ///
    /// The origin follows from the reason via [`absence_origin_for_reason`] and
    /// is deliberately not a parameter. Recoverability is, because it genuinely
    /// varies for one reason — a disconnected port is `Fatal` while an empty
    /// read buffer is `Retryable` — and cannot be read off the reason alone.
    #[inline]
    pub fn bubble_with_reason(reason: NilReason, recoverability: Recoverability) -> Self {
        let origin = absence_origin_for_reason(&reason);
        Self::nil_with_absence(AbsenceMetadata::with_reason(reason, origin, recoverability))
    }

    #[inline]
    pub fn absence_metadata(&self) -> Option<&AbsenceMetadata> {
        self.absence.as_ref()
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
