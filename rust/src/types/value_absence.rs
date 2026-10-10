//! Operational absence and the truth observation for [`Value`].
//!
//! Invariant: a [`NilReason`] chooses its [`AbsenceOrigin`] in exactly one
//! place, and every reasoned NIL constructor routes through that exhaustive
//! mapping.
//!
//! `try_collect_dense` is test-only: `dense_columns::try_promote_columns`
//! writes the same lanes straight into the tensor's columns and is the one
//! promotion route the runtime takes, while this walk remains as the
//! independently written oracle `dense_columns_tests` checks it against.

#[cfg(test)]
use super::fraction::Fraction;
use super::{Value, ValueData};
use crate::error::NilReason;
use crate::semantic::{AbsenceMetadata, AbsenceOrigin, Recoverability};

/// The single derivation of an absence origin from a NIL reason.
///
/// Nothing else may compute one. Call sites used to pass an origin alongside a
/// reason, which gave the pairing two sources and let them disagree:
/// `INDEX-OF` reported `reason = notFound` with `origin = executionFailure`.
/// Deriving here means a new reason gets its origin by adding one arm, and
/// gets it everywhere at once.
fn absence_origin_for_reason(reason: &NilReason) -> AbsenceOrigin {
    match reason {
        NilReason::NotFound => AbsenceOrigin::NotFound,
        NilReason::InvalidEncoding => AbsenceOrigin::InvalidEncoding,
        NilReason::IndexOutOfBounds => AbsenceOrigin::IndexOutOfBounds,
        NilReason::DomainMiss => AbsenceOrigin::DomainMiss,
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
        Self::new(ValueData::Nil, Some(AbsenceMetadata::literal()))
    }

    /// An absence, with `absence` for why.
    #[inline]
    pub fn nil_with_absence(absence: AbsenceMetadata) -> Self {
        Self::new(ValueData::Nil, Some(absence))
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

    /// The absent `source` carried whole — its reason and the rest of its
    /// absence — for a Word that passes it through
    /// (LANG.FAILURE.PASSTHROUGH). A present `source` gives a written NIL.
    #[inline]
    pub fn nil_inheriting_absence_from(source: &Self) -> Self {
        match source.normalized_absence_metadata() {
            Some(absence) => Self::nil_with_absence(absence),
            None => Self::nil(),
        }
    }

    /// Create a reasoned NIL for the NIL Projection Rule (the specification's
    /// "NIL Projection Rule"): well-formed operations that cannot produce a value
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
        self.absence.as_deref()
    }

    /// The text a `userDeclared` absence carries beside its reason.
    #[inline]
    pub fn absence_detail(&self) -> Option<&str> {
        self.absence
            .as_deref()
            .and_then(AbsenceMetadata::detail_text)
    }

    #[inline]
    pub fn normalized_absence_metadata(&self) -> Option<AbsenceMetadata> {
        if !self.is_absent() {
            return None;
        }
        Some(
            self.absence
                .as_deref()
                .cloned()
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

/// A rectangular numeric value flattened into the two things a dense tensor
/// stores: the lanes and their shape.
#[cfg(test)]
pub(super) struct DenseCollect {
    pub(super) data: Vec<Fraction>,
    pub(super) shape: Vec<usize>,
}

/// Walk a list of `Value`s into a [`DenseCollect`] if every leaf is a numeric
/// lane — a Fraction scalar or a child Tensor — and the shape is rectangular.
/// `None` if any leaf is outside that (NIL, Boolean, Text, Symbol,
/// ExactScalar, Record) or if shapes disagree.
#[cfg(test)]
pub(super) fn try_collect_dense(values: &[Value]) -> Option<DenseCollect> {
    let mut data = Vec::with_capacity(values.len());
    let shape = append_dense_values(values, &mut data)?;
    Some(DenseCollect { data, shape })
}

/// Append every value's lanes to `data`, and answer with the shape the whole
/// run forms.
#[cfg(test)]
fn append_dense_values(values: &[Value], data: &mut Vec<Fraction>) -> Option<Vec<usize>> {
    if values.is_empty() {
        return None;
    }
    let mut inner_shape: Option<Vec<usize>> = None;
    for value in values {
        let child_shape = append_dense_value(value, data)?;
        match &inner_shape {
            None => inner_shape = Some(child_shape),
            Some(expected) if *expected == child_shape => {}
            Some(_) => return None,
        }
    }
    let mut shape = vec![values.len()];
    shape.extend(inner_shape.expect("values is non-empty, so a shape was set"));
    Some(shape)
}

/// Append one value's lanes, answering with its own shape (empty for a leaf).
#[cfg(test)]
fn append_dense_value(value: &Value, data: &mut Vec<Fraction>) -> Option<Vec<usize>> {
    match &value.data {
        ValueData::Scalar(fraction) => {
            data.push(fraction.clone());
            Some(Vec::new())
        }
        ValueData::Tensor {
            data: tensor,
            shape,
        } => {
            data.extend(tensor.iter());
            Some((**shape).clone())
        }
        ValueData::Vector(children) => append_dense_values(children, data),
        // A NIL is not a numeric lane, and nor is anything below.
        ValueData::Nil
        | ValueData::ExactScalar(_)
        | ValueData::Record(_)
        | ValueData::Boolean(_)
        | ValueData::Text(_)
        | ValueData::Symbol(_) => None,
    }
}
