use crate::error::NilReason;
use crate::interpreter::debug_diagnosis::DebugDiagnosis;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbsenceOrigin {
    Literal,
    /// Division by zero (or by a value indistinguishable from zero within the
    /// comparison budget) produced a Bubble/NIL under the Bubble Rule
    /// (SPEC §11.2). Used together with `NilReason::DivisionByZero`.
    ///
    /// Every construction path reaches this through
    /// `absence_origin_for_reason`, which is the sole derivation of an origin
    /// from a reason: `DIV` and `POW` by way of `Value::bubble_with_reason`,
    /// `nil_with_reason` for the rest. Call sites cannot name an origin
    /// directly, so a reason and its origin cannot drift apart.
    DivisionByZero,
    NilPropagation,
    EmptySequence,
    MissingField,
    InvalidEncoding,
    InvalidLens,
    StackUnderflow,
    IndexOutOfBounds,
    UnknownWord,
    ExecutionFailure,
    /// Continued-fraction comparison exhausted its partial-quotient
    /// budget without resolving the order of the two operands per
    /// SPEC §7.4.1. Used together with `NilReason::Undecidable`.
    ComparisonBudget,
    /// A well-formed generative operation exceeded the space water level
    /// (`max_materialized_elements`) and was projected to a Bubble/NIL under the
    /// Bubble Rule (SPEC §11.2). Used together with
    /// `NilReason::SpaceExhausted`.
    SpaceBudget,
    /// A well-formed operation was applied outside its domain — `SQRT` of a
    /// negative rational, the "well-formed domain miss" of SPEC §5 — and was
    /// projected to a Bubble/NIL under the Bubble Rule (SPEC §11.2). Used
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
    pub diagnosis: Option<DebugDiagnosis>,
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

    pub fn from_diagnosis(
        reason: NilReason,
        origin: AbsenceOrigin,
        recoverability: Recoverability,
        diagnosis: DebugDiagnosis,
    ) -> Self {
        Self {
            reason: Some(reason),
            origin,
            recoverability,
            diagnosis: Some(diagnosis),
        }
    }
}
