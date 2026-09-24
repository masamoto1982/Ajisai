use crate::error::NilReason;
use crate::interpreter::debug_diagnosis::DebugDiagnosis;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbsenceOrigin {
    Literal,
    /// Division by zero produced a reasoned NIL under the NIL Projection Rule
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
    /// The program declared the absence itself (`ABSENT`).
    UserDeclared,
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
    /// The text a program gave `ABSENT`, for a `userDeclared` reason. Part
    /// of the value: `Value`'s equality compares it beside the reason. Held
    /// behind a *thin* pointer (`Arc<String>`, not `Arc<str>`) so the
    /// envelope stays within `value_layout_tests`' pointer-sized budget.
    pub detail: Option<std::sync::Arc<String>>,
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
    /// The declared text, when this absence carries one.
    #[inline]
    pub fn detail_text(&self) -> Option<&str> {
        self.detail.as_deref().map(String::as_str)
    }

    pub fn literal() -> Self {
        Self {
            reason: Some(NilReason::Literal),
            detail: None,
            origin: AbsenceOrigin::Literal,
            recoverability: Recoverability::Unknown,
            diagnosis: None,
        }
    }

    pub fn with_reasonless_unknown() -> Self {
        Self {
            reason: None,
            detail: None,
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
            detail: None,
            origin,
            recoverability,
            diagnosis: None,
        }
    }

    /// The absence `ABSENT` produces: reason `userDeclared`, with the
    /// program's text as its detail. Recoverable, because a caller can choose
    /// a fallback for it exactly as for a projected absence.
    pub fn user_declared(detail: &str) -> Self {
        Self {
            reason: Some(NilReason::UserDeclared),
            detail: Some(std::sync::Arc::new(detail.to_string())),
            origin: AbsenceOrigin::UserDeclared,
            recoverability: Recoverability::Recoverable,
            diagnosis: None,
        }
    }
}
