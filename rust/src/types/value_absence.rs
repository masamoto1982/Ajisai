//! Operational absence and the truth observation for [`Value`], and the
//! dense-storage decision that keeps an absent lane's reason.
//!
//! Invariant: a [`NilReason`] chooses its [`AbsenceOrigin`] in exactly one place,
//! and every reasoned NIL constructor routes through that exhaustive mapping.
//!
//! Deciding whether a value can be stored densely, and flattening it when it
//! can ([`try_collect_dense`]), is the storage half of the two
//! representations `value_identity` reconciles: this module answers "may these
//! children be columns", and that one answers "is the result still the same
//! value". They constrain each other — promoting something whose dense form
//! could not be read back as the value that went in is exactly how a
//! representation loses information — so the arms here and the arms there
//! are meant to be read as a pair.

use super::fraction::Fraction;
use super::{Value, ValueData};
use crate::error::NilReason;
use crate::semantic::{AbsenceMetadata, AbsenceOrigin, Recoverability};
use std::collections::BTreeMap;

/// The single derivation of an absence origin from a NIL reason.
///
/// Nothing else may compute one. Call sites used to pass an origin alongside a
/// reason, which gave the pairing two sources and let them disagree: `DIV` by
/// zero reported `reason = divisionByZero` with `origin = executionFailure`,
/// contradicting `AbsenceOrigin::DivisionByZero`'s own documentation, and
/// `INDEX-OF` did the same to `notFound`. Deriving here means a new reason
/// gets its origin by adding one arm, and gets it everywhere at once.
fn absence_origin_for_reason(reason: &NilReason) -> AbsenceOrigin {
    match reason {
        NilReason::NotFound => AbsenceOrigin::NotFound,
        NilReason::InvalidEncoding => AbsenceOrigin::InvalidEncoding,
        NilReason::IndexOutOfBounds => AbsenceOrigin::IndexOutOfBounds,
        NilReason::DivisionByZero => AbsenceOrigin::DivisionByZero,
        NilReason::SpaceExhausted => AbsenceOrigin::SpaceBudget,
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

/// A rectangular numeric value flattened into the three things a dense tensor
/// stores: the lanes, their shape, and the reason each absent lane is absent.
///
/// The third used to be missing, and so did the lanes it describes:
/// [`try_dense_value`] refused a NIL child outright, because the only thing
/// dense storage could have said about one was that it was absent. Refusing
/// was the right call while that was true — a densified NIL came back claiming
/// the program had written it — and it is why a vector holding an absence kept
/// its nested form. With the reason stored beside the lane the refusal is no
/// longer needed, so a NIL is an ordinary lane again.
pub(super) struct DenseCollect {
    pub(super) data: Vec<Fraction>,
    pub(super) shape: Vec<usize>,
    pub(super) absences: BTreeMap<usize, AbsenceMetadata>,
}

/// Walk a list of `Value`s into a [`DenseCollect`] if every leaf is a numeric
/// lane — a Fraction scalar, a NIL, or a child Tensor — and the shape is
/// rectangular. `None` if any leaf is outside that (Boolean, Text, Symbol,
/// ExactScalar) or if shapes disagree.
///
/// The lanes are appended into one buffer as they are found. Each leaf used to
/// answer with a `DenseCollect` of its own, which meant a scalar — the common
/// leaf, and the only one `MAP` over a flat vector ever produces — was carried
/// from the value to the buffer inside a freshly heap-allocated one-element
/// `Vec` that was consumed and dropped on the next line. Promoting n scalars
/// therefore made n allocations whose entire contents were one `Fraction`.
pub(super) fn try_collect_dense(values: &[Value]) -> Option<DenseCollect> {
    let mut data = Vec::with_capacity(values.len());
    let mut absences = BTreeMap::new();
    let shape = append_dense_values(values, &mut data, &mut absences)?;
    Some(DenseCollect {
        data,
        shape,
        absences,
    })
}

/// Append every value's lanes to `data`, recording absences at their absolute
/// index, and answer with the shape the whole run forms.
///
/// A refusal part-way through leaves junk in the buffers, which is sound
/// because the only caller owns them and drops them on `None`; nothing reads a
/// buffer this returned `None` for.
fn append_dense_values(
    values: &[Value],
    data: &mut Vec<Fraction>,
    absences: &mut BTreeMap<usize, AbsenceMetadata>,
) -> Option<Vec<usize>> {
    if values.is_empty() {
        return None;
    }
    let mut inner_shape: Option<Vec<usize>> = None;
    for value in values {
        let child_shape = append_dense_value(value, data, absences)?;
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
fn append_dense_value(
    value: &Value,
    data: &mut Vec<Fraction>,
    absences: &mut BTreeMap<usize, AbsenceMetadata>,
) -> Option<Vec<usize>> {
    let offset = data.len();
    match &value.data {
        ValueData::Scalar(fraction) => {
            data.push(fraction.clone());
            Some(Vec::new())
        }
        // A NIL is a rank-0 numeric lane: the absence sentinel, and the reason
        // for it stored beside the lane. Storing the reason is what makes this
        // arm possible — see [`DenseCollect`].
        ValueData::Nil => {
            data.push(Fraction::nil());
            if let Some(metadata) = value.absence_metadata() {
                absences.insert(offset, metadata.clone());
            }
            Some(Vec::new())
        }
        // ExactScalar cannot be densified into a Fraction tensor
        ValueData::ExactScalar(_) => None,
        // A Record is not a numeric lane.
        ValueData::Record(_) => None,
        ValueData::Tensor {
            data: tensor,
            shape,
        } => {
            data.extend(tensor.iter());
            for (index, metadata) in tensor.absences() {
                absences.insert(offset + index, metadata.clone());
            }
            Some((**shape).clone())
        }
        ValueData::Vector(children) => append_dense_values(children, data, absences),
        ValueData::Boolean(_) | ValueData::Text(_) | ValueData::Symbol(_) => None,
    }
}
