//! Scalar construction and domain classification for [`Value`].

use super::fraction::Fraction;
use super::value_tensor::tensor_to_nested_values;
use super::{DenseTensor, RecordData, Value, ValueData};
use crate::semantic::AbsenceMetadata;
use std::sync::Arc;

impl Value {
    #[inline]
    /// A numeric leaf from a rational.
    ///
    /// A `Fraction` whose denominator is 0 is [`Fraction::nil`] — the absence
    /// sentinel the dense tensor lanes store, not a rational. It becomes
    /// `ValueData::Nil` here rather than a `Scalar` wrapping an unreadable
    /// number: this is the one place fraction-level absence is translated into
    /// value-level absence, so a lane rehydrated out of a tensor answers
    /// [`Value::is_nil`] like any other NIL. Lanes carry presence, not a
    /// reason, so the absence reads back as `literal` — the reason a lane was
    /// stored under does not survive densification.
    pub fn from_fraction(f: Fraction) -> Self {
        if f.is_nil() {
            // A `Fraction` records that it is absent and nothing about why, so
            // this is a *reasonless* NIL — not `nil_literal()`, which claims
            // the program wrote it (`spec/outcomes.json`: "a NIL the program
            // wrote rather than computed"). Where the reason is known it is
            // stored beside the lane, and `Value::from_dense_lane` is the
            // materialization that reads it.
            return Self::nil_with_absence(AbsenceMetadata::with_reasonless_unknown());
        }
        Self::new(ValueData::Scalar(f), None)
    }

    #[inline]
    pub fn from_int(n: i64) -> Self {
        Self::new(ValueData::Scalar(Fraction::from(n)), None)
    }

    #[inline]
    pub fn from_bool(b: bool) -> Self {
        Self::new(ValueData::Boolean(b), None)
    }

    /// The definite truth value carried by a Boolean data value, or `None`
    /// for any non-Boolean value. This is the data-plane truth accessor:
    /// unlike [`Value::is_truthy`] it never coerces a number, vector, or
    /// other shape into a truth value. UNKNOWN is a NIL, not a Boolean
    /// (LANG.VALUES.TRUTH), so it returns `None` here.
    #[inline]
    pub fn as_truth(&self) -> Option<bool> {
        match &self.data {
            ValueData::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Build a String value (LANG.VALUES.DISJOINT).
    ///
    /// The empty String is a String. It used to become
    /// `NilReason::EmptySequence`, which made `''` an absence rather than a
    /// value and forced every text Word to carry an empty special case; a
    /// domain with no empty element also cannot be closed under `TRIM` or
    /// `TOKENIZE`. NIL means "no value here", and `''` is a perfectly good
    /// value with no characters in it.
    pub fn from_string(s: &str) -> Self {
        Self::new(ValueData::Text(Arc::from(s)), None)
    }

    /// The characters of a String value, or `None` for any other domain.
    ///
    /// This is the whole of stringhood now: no element inspection, no
    /// codepoint-range guessing. A Vector of codepoint Scalars is a
    /// Vector, and answers `None`.
    #[inline]
    pub fn as_text(&self) -> Option<&str> {
        match &self.data {
            ValueData::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Build a Record value (LANG.RECORDS.STRUCTURE).
    pub fn from_record(record: RecordData) -> Self {
        Self::new(ValueData::Record(Arc::new(record)), None)
    }

    /// The Record behind a Record value, or `None` for any other domain.
    #[inline]
    pub fn as_record(&self) -> Option<&RecordData> {
        match &self.data {
            ValueData::Record(record) => Some(record),
            _ => None,
        }
    }

    /// A bare Word reference — data until something executes it. See
    /// `ValueData::Symbol`'s doc comment.
    pub fn from_symbol(s: &str) -> Self {
        Self::new(ValueData::Symbol(Arc::from(s)), None)
    }

    #[inline]
    pub fn from_children(children: Vec<Value>) -> Self {
        Self::new(ValueData::Vector(Arc::new(children)), None)
    }

    /// Build a Vector value (LANG.VALUES.VECTOR).
    ///
    /// The empty Vector is a Vector. It used to become
    /// `NilReason::EmptySequence`, which made `[ ]` an absence and put NIL to
    /// work as "empty collection" — a second job that collides with
    /// LANG.VALUES.NIL, where a reason is the *whole observable content* of an
    /// absence rather than a stand-in for a value. LANG.VALUES.VECTOR calls a
    /// Vector "an ordered finite collection of values" and makes "order and
    /// length" its whole observable structure; zero is a finite length.
    pub fn from_vector(values: Vec<Value>) -> Self {
        Self::new(ValueData::Vector(Arc::new(values)), None)
    }

    #[inline]
    pub fn from_exact_real(er: crate::types::exact::ExactReal) -> Self {
        // If the ExactReal is already rational, use the fast Fraction path.
        if let Some(f) = er.as_rational() {
            return Self::new(ValueData::Scalar(f.clone()), None);
        }
        Self::new(ValueData::ExactScalar(er), None)
    }

    #[inline]
    pub fn from_number(f: Fraction) -> Self {
        Self::from_fraction(f)
    }

    /// NIL test: `true` only for the operational absence node
    /// ([`ValueData::Nil`]).
    #[inline]
    pub fn is_nil(&self) -> bool {
        matches!(self.data, ValueData::Nil)
    }

    /// As [`is_nil`]: operational-absence test.
    #[inline]
    pub fn is_absent(&self) -> bool {
        matches!(self.data, ValueData::Nil)
    }

    /// As [`is_nil`]: operational-absence test, and deliberately *not*
    /// narrower than one.
    ///
    /// UNKNOWN is a NIL read in truth position (LANG.VALUES.TRUTH), so it is
    /// an operational NIL like any other: `NIL?` answers TRUE for it and
    /// `NIL-REASON` reports the reason it arrived with.
    #[inline]
    pub fn is_operational_nil(&self) -> bool {
        matches!(self.data, ValueData::Nil)
    }

    /// The value's domain, spelled as LANG.VALUES.DISJOINT spells it. Every
    /// error message names an operand's domain through this, so a reader
    /// meets one vocabulary for the seven domains and no other.
    pub fn domain_name(&self) -> &'static str {
        match &self.data {
            ValueData::Scalar(_) | ValueData::ExactScalar(_) => "Scalar",
            ValueData::Boolean(_) => "Boolean",
            ValueData::Text(_) => "String",
            ValueData::Vector(_) | ValueData::Tensor { .. } => "Vector",
            ValueData::Record(_) => "Record",
            ValueData::Nil => "NIL",
            ValueData::Symbol(_) => "Symbol",
        }
    }

    #[inline]
    pub fn is_scalar(&self) -> bool {
        matches!(self.data, ValueData::Scalar(_) | ValueData::ExactScalar(_))
    }

    #[inline]
    pub fn is_vector(&self) -> bool {
        matches!(self.data, ValueData::Vector(_) | ValueData::Tensor { .. })
    }

    /// Whether this value is a String (LANG.VALUES.DISJOINT).
    #[inline]
    pub fn is_text(&self) -> bool {
        matches!(self.data, ValueData::Text(_))
    }

    #[inline]
    pub fn is_tensor(&self) -> bool {
        matches!(self.data, ValueData::Tensor { .. })
    }

    /// Borrow the dense numeric backing of a `Tensor` value as
    /// `(tensor, shape)`. Returns `None` for any other representation.
    /// Use this on hot HOF paths to iterate fraction lanes directly without
    /// materializing per-element `Value`s.
    #[inline]
    pub fn as_dense_tensor(&self) -> Option<(&DenseTensor, &[usize])> {
        match &self.data {
            ValueData::Tensor { data, shape } => Some((data.as_ref(), shape.as_slice())),
            _ => None,
        }
    }

    /// Borrow the children of an iterable `Value` as a `Cow<[Value]>`.
    /// `Vector` and `Record` borrow their backing slice directly; `Tensor`
    /// materializes its children once into an owned `Vec<Value>`. Non-iterable
    /// kinds (`Scalar`, `Nil`, `CodeBlock`, handles) return `None`.
    ///
    /// Use this in non-hot consumers (JSON serialization, sort, structural
    /// helpers) so they only need a single iteration path regardless of
    /// whether the value is `Vector` or `Tensor`. For tight numeric loops
    /// prefer [`as_dense_tensor`] which returns the dense tensor without
    /// materializing per-element `Value`s.
    pub fn as_vector_view(&self) -> Option<std::borrow::Cow<'_, [Value]>> {
        match &self.data {
            ValueData::Vector(v) => Some(std::borrow::Cow::Borrowed(v.as_slice())),
            ValueData::Tensor { data, shape } => Some(std::borrow::Cow::Owned(
                tensor_to_nested_values(data, shape),
            )),
            ValueData::Boolean(_)
            | ValueData::Text(_)
            | ValueData::Scalar(_)
            | ValueData::ExactScalar(_)
            | ValueData::Nil
            | ValueData::Symbol(_)
            | ValueData::Record(_) => None,
        }
    }

    /// Return a `Cow<Value>` that is guaranteed to use a non-`Tensor`
    /// representation. `Tensor` values are converted into a nested
    /// `ValueData::Vector` (preserving `absence`); every other
    /// variant is borrowed in place.
    ///
    /// Useful at user-visible boundaries (PRINT, JSON-EXPORT, GUI hand-off,
    /// error message formatting) where the caller wants to operate on a
    /// uniform `Vector` shape without caring whether the producer happened to
    /// emit a dense `Tensor`.
    pub fn ensure_hydrated(&self) -> std::borrow::Cow<'_, Value> {
        match &self.data {
            ValueData::Tensor { data, shape } => {
                let children = tensor_to_nested_values(data, shape);
                std::borrow::Cow::Owned(Value::new(
                    ValueData::Vector(Arc::new(children)),
                    self.absence.clone(),
                ))
            }
            _ => std::borrow::Cow::Borrowed(self),
        }
    }

    #[inline]
    pub fn is_truthy(&self) -> bool {
        match &self.data {
            ValueData::Boolean(b) => *b,
            // `is_truthy` is a total two-valued coercion. NIL — UNKNOWN in
            // truth position — collapses to `false`. Words that must honour
            // the third value read it before asking for a definite truth
            // (`SELECT`, `AND`/`NOT`), never here.
            ValueData::Nil => false,
            // A String is not a truth value. LANG.VALUES.TRUTH is two-valued
            // over Booleans, and the logic Words reject anything else outright
            // (`nonTruthValue`); this total coercion survives only for legacy
            // internal callers, so a String collapses to `false` rather than
            // inventing an emptiness rule for a domain that has none.
            ValueData::Text(_) => false,
            ValueData::Scalar(f) => !f.is_zero() && !f.is_nil(),
            // ExactScalar values from AlgebraicSqrt are always non-zero positive
            // irrationals; Gosper nodes conservatively report truthy.
            ValueData::ExactScalar(_) => true,
            ValueData::Vector(v) => !v.is_empty() && !v.iter().all(|c| !c.is_truthy()),
            ValueData::Tensor { data, .. } => {
                !data.is_empty() && !data.iter().all(|f| f.is_zero() || f.is_nil())
            }
            ValueData::Symbol(_) => true,
            // A Record is not a truth value either; like a String it collapses
            // to `false` here and is rejected outright by the logic Words.
            ValueData::Record(_) => false,
        }
    }
}
