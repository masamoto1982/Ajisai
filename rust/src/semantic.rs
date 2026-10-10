//! The semantic vocabulary of an absence — where a NIL came from, whether it
//! can be recovered from, and the metadata a reasoned NIL carries — and the
//! protocol strings every host surface spells them with.

use crate::error::NilReason;
use crate::interpreter::debug_diagnosis::DebugDiagnosis;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbsenceOrigin {
    Literal,
    NilPropagation,
    NotFound,
    InvalidEncoding,
    IndexOutOfBounds,
    /// A well-formed operation was applied outside its domain — `SQRT` of a
    /// negative rational, the "well-formed domain miss" of LANG.FAILURE.PROJECT — and was
    /// projected to NIL under the NIL Projection Rule (LANG.FAILURE.PROJECT). Used
    /// together with `NilReason::DomainMiss`.
    DomainMiss,
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

thread_local! {
    static MINTED_ABSENCES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many reasoned absences this thread has minted so far.
///
/// A reasoned absence is *minted* once, by the Word whose contract projects it
/// (`with_reason`, `user_declared`), and from then on only *carried*: a data
/// operand's absence is copied to the result unchanged
/// (LANG.FAILURE.PASSTHROUGH, `Value::nil_inheriting_absence_from`), an
/// element's absence is moved with the element. The value itself cannot say
/// which of the two happened to it — a passed-through NIL is the same value as
/// the NIL it was passed from (LANG.VALUES.DENOTATION) — so a count of mints is
/// kept beside it. A Word that ran while the count stood still produced no
/// absence, whatever its result carries; that is how the error-flow trace tells
/// the Word that answered `domainMiss` from the ones that merely handed it
/// on (`Interpreter::trace_nil_outcome`). Diagnostic only: nothing a program
/// can observe reads it.
pub fn minted_absence_count() -> u64 {
    MINTED_ABSENCES.with(|count| count.get())
}

fn mint() {
    MINTED_ABSENCES.with(|count| count.set(count.get().wrapping_add(1)));
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
        mint();
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
        mint();
        Self {
            reason: Some(NilReason::UserDeclared),
            detail: Some(std::sync::Arc::new(detail.to_string())),
            origin: AbsenceOrigin::UserDeclared,
            recoverability: Recoverability::Recoverable,
            diagnosis: None,
        }
    }
}

impl AbsenceOrigin {
    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            AbsenceOrigin::Literal => "literal",
            AbsenceOrigin::NilPropagation => "nilPropagation",
            AbsenceOrigin::NotFound => "notFound",
            AbsenceOrigin::InvalidEncoding => "invalidEncoding",
            AbsenceOrigin::IndexOutOfBounds => "indexOutOfBounds",
            AbsenceOrigin::DomainMiss => "domainMiss",
            AbsenceOrigin::HostEnvironment => "hostEnvironment",
            AbsenceOrigin::UserDeclared => "userDeclared",
            AbsenceOrigin::Unknown => "unknown",
        }
    }
}

impl Recoverability {
    pub fn as_protocol_str(self) -> &'static str {
        match self {
            Recoverability::Recoverable => "recoverable",
            Recoverability::Retryable => "retryable",
            Recoverability::Fatal => "fatal",
            Recoverability::Unknown => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AbsenceOrigin, Recoverability};
    use crate::error::{ErrorCategory, NilReason};
    use crate::interpreter::debug_diagnosis::{CauseClass, ErrorLocusKind, ErrorPhase};
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

    #[test]
    fn absence_and_diagnosis_protocol_strings_do_not_use_debug_names() {
        assert_eq!(Recoverability::Recoverable.as_protocol_str(), "recoverable");
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
    fn domain_miss_protocol_strings() {
        // LANG.FAILURE.PROJECT names the classification: "SQRT of a negative rational is a
        // well-formed domain miss". Reason and origin share the spelling because
        // the origin is derived from the reason.
        assert_eq!(NilReason::DomainMiss.as_protocol_str(), "domainMiss");
        assert_eq!(AbsenceOrigin::DomainMiss.as_protocol_str(), "domainMiss");
    }

    /// Every `NilReason` maps to a distinct, lowerCamelCase protocol string. A new
    /// variant that forgot its arm, or reused another's spelling, fails here rather
    /// than in whichever serializer happened to hit it first.
    #[test]
    fn every_nil_reason_has_a_distinct_lower_camel_protocol_string() {
        let mut seen: Vec<&str> = Vec::new();
        for reason in NilReason::ALL {
            let s = reason.as_protocol_str();
            assert!(
                s.starts_with(|c: char| c.is_ascii_lowercase())
                    && s.chars().all(|c| c.is_alphanumeric()),
                "{s} is not a lowerCamelCase protocol string"
            );
            assert!(!seen.contains(&s), "{s} is used by two reasons");
            seen.push(s);
        }
    }

    #[test]
    fn unknown_is_observed_as_a_nil() {
        // LANG.VALUES.TRUTH: UNKNOWN is NIL read in truth position, not a fourth
        // variant, so it is observed as a NIL — no truth axis.
        use crate::types::Value;
        let u = Value::nil_with_reason_unknown(NilReason::DomainMiss);
        assert_eq!(u.truth_value(), None);
        assert!(u.is_nil());
    }
    #[test]
    fn definite_truth_values_expose_truth_value_axis() {
        use crate::types::Value;
        let t = Value::from_bool(true);
        let f = Value::from_bool(false);
        assert_eq!(t.truth_value(), Some("true"));
        assert_eq!(f.truth_value(), Some("false"));
        // A plain number is not truth-valued.
        assert_eq!(Value::from_int(1).truth_value(), None);
    }
}
