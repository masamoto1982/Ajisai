use crate::error::NilReason;
use crate::interpreter::debug_diagnosis::DebugDiagnosis;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbsenceOrigin {
    Literal,
    /// Division by zero (or by a value indistinguishable from zero within the
    /// comparison budget) produced a reasoned NIL under the NIL Projection Rule
    /// (LANG.FAILURE.PROJECT). Used together with `NilReason::DivisionByZero`.
    ///
    /// Every construction path reaches this through
    /// `absence_origin_for_reason`, which is the sole derivation of an origin
    /// from a reason: `DIV` and `POW` by way of `Value::nil_with_reason`,
    /// `nil_with_reason` for the rest. Call sites cannot name an origin
    /// directly, so a reason and its origin cannot drift apart.
    DivisionByZero,
    NilPropagation,
    MissingField,
    InvalidEncoding,
    IndexOutOfBounds,
    /// Continued-fraction comparison exhausted its partial-quotient
    /// budget without resolving the order of the two operands per
    /// LANG.VALUES.EXACT. Used together with `NilReason::Undecidable`.
    ComparisonBudget,
    /// A well-formed generative operation exceeded the space water level
    /// (`max_materialized_elements`) and was projected to NIL under the
    /// NIL Projection Rule (LANG.FAILURE.PROJECT). Used together with
    /// `NilReason::SpaceExhausted`.
    SpaceBudget,
    /// A well-formed operation was applied outside its domain — `SQRT` of a
    /// negative rational, the "well-formed domain miss" of LANG.FAILURE.PROJECT — and was
    /// projected to NIL under the NIL Projection Rule (LANG.FAILURE.PROJECT). Used
    /// together with `NilReason::DomainMiss`.
    DomainMiss,
    /// A diagnostic accessor found nothing to report — the origin paired with
    /// `NilReason::NotAvailable`.
    NotAvailable,
    HostEnvironment,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recoverability {
    Recoverable,
    Retryable,
    Fatal,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsenceMetadata {
    pub reason: Option<NilReason>,
    pub origin: AbsenceOrigin,
    pub recoverability: Recoverability,
    /// Boxed, not inlined. A [`DebugDiagnosis`] is 256 bytes — a summary
    /// string, an evidence list, a next-check list, a candidate list — and
    /// inlining it made `AbsenceMetadata` 264 bytes and `Value` 344, against
    /// `ValueData`'s 72. Every stack slot and every lane of an AoS `Vector`
    /// carried that width, so a vector of a million numbers moved 344 MB to
    /// hold 72 MB of numbers, and the 264 bytes were `None` in all but the
    /// rare absent lane. The diagnosis is the rarest thing a value can carry;
    /// it pays for its own allocation when it exists and costs a pointer when
    /// it does not.
    pub diagnosis: Option<Box<DebugDiagnosis>>,
}

impl AbsenceMetadata {
    pub fn literal() -> Self {
        Self {
            reason: Some(NilReason::Literal),
            origin: AbsenceOrigin::Literal,
            recoverability: Recoverability::Unknown,
            diagnosis: None,
        }
    }

    pub fn with_reasonless_unknown() -> Self {
        Self {
            reason: None,
            origin: AbsenceOrigin::Unknown,
            recoverability: Recoverability::Unknown,
            diagnosis: None,
        }
    }

    pub fn with_reason(
        reason: NilReason,
        origin: AbsenceOrigin,
        recoverability: Recoverability,
    ) -> Self {
        Self {
            reason: Some(reason),
            origin,
            recoverability,
            diagnosis: None,
        }
    }
}
