//! Scalar construction and domain classification for [`Value`], and when a
//! dense tensor and a nested `Vector` are the same value.
//!
//! `Value` has two representations for one thing — a rectangular numeric
//! collection is either `ValueData::Tensor` (struct-of-arrays columns) or
//! `ValueData::Vector` (a tree of child `Value`s) — and which one a program
//! ends up holding is a storage decision no clause of the specification makes.
//! LANG.STACK.ORDER makes value identity a semantic question, so the two forms
//! must answer `EQ` and hash alike whenever they hold the same content. This
//! module is that correspondence, kept in one place because `PartialEq` and
//! `Hash` have to agree about it and drift apart the moment they are written
//! twice.
//!
//! An absent lane is where the correspondence is easiest to get wrong. A NIL
//! child of a `Vector` carries its reason on the child `Value`; the same lane
//! of a `Tensor` carries it in the tensor's own per-lane absence map. Under
//! LANG.VALUES.NIL the reason is the whole observable content of an absence,
//! so both sides read it and neither reads more: `Value`'s equality is
//! `data == data && nil_reason == nil_reason`, and the rest of an
//! `AbsenceMetadata` is provenance.

use super::fraction::Fraction;
use super::value_tensor::tensor_to_nested_values;
use super::{DenseTensor, RecordData, Value, ValueData};
use crate::error::NilReason;
use crate::semantic::AbsenceMetadata;
use std::sync::Arc;

pub(super) fn tensor_eq_vector(data: &DenseTensor, shape: &[usize], v: &[Value]) -> bool {
    // A dense tensor is always rectangular, so a ragged nested vector (no
    // well-defined rectangular shape) can never equal one. `nested_vector_shape`
    // returns `None` for ragged structures, which fails the comparison here
    // rather than colliding with the dense shape via a count-only fallback.
    let Some(nested_shape) = nested_vector_shape(v) else {
        return false;
    };
    if nested_shape != shape {
        return false;
    }
    let mut idx = 0usize;
    nested_flatten_matches(v, data, &mut idx) && idx == data.len()
}

/// The rectangular shape of a nested vector, or `None` when the structure is
/// ragged (sibling elements with differing shapes, or mixed scalar/vector
/// siblings). Used only for dense-tensor equality, which requires a
/// rectangular counterpart.
fn nested_vector_shape(v: &[Value]) -> Option<Vec<usize>> {
    if v.is_empty() {
        return Some(vec![0]);
    }
    let first_shape = element_rect_shape(&v[0])?;
    for child in v.iter().skip(1) {
        if element_rect_shape(child)? != first_shape {
            return None;
        }
    }
    let mut s = vec![v.len()];
    s.extend(first_shape);
    Some(s)
}

/// Rectangular shape of a single value, or `None` for non-numeric leaves or
/// ragged sub-structures.
fn element_rect_shape(value: &Value) -> Option<Vec<usize>> {
    match &value.data {
        ValueData::Scalar(_) | ValueData::ExactScalar(_) | ValueData::Nil(_) => Some(Vec::new()),
        // A String is not a numeric leaf, so it has no rectangular element
        // shape and forces the structural (non-dense) path, like a Boolean.
        ValueData::Text(_) => None,
        ValueData::Tensor { shape, .. } => Some((**shape).clone()),
        ValueData::Vector(items) => nested_vector_shape(items),
        ValueData::Boolean(_) | ValueData::Symbol(_) | ValueData::Record(_) => None,
    }
}

fn nested_flatten_matches(v: &[Value], data: &DenseTensor, idx: &mut usize) -> bool {
    for child in v {
        match &child.data {
            ValueData::Scalar(f) => {
                if *idx >= data.len() || data.fraction_or_nil(*idx) != *f {
                    return false;
                }
                *idx += 1;
            }
            // A NIL matches an absent lane carrying the same reason and the
            // same `userDeclared` detail. All three halves matter: `is_valid`
            // alone would let a NIL equal a number (an absent lane's numerator
            // is the dividend it holds), the lane alone would let `NIL(divisionByZero)` equal
            // a written `NIL`, and the reason alone would let `'a' ABSENT`
            // equal `'b' ABSENT` across representations — which the same two
            // values as scalars (`Value::eq`) and as two tensors
            // (`DenseTensor::eq`) both already refuse.
            ValueData::Nil(_) => {
                if *idx >= data.len()
                    || data.is_valid(*idx)
                    || data.lane_reason(*idx) != child.nil_reason().copied()
                    || data.lane_detail(*idx) != child.absence_detail()
                {
                    return false;
                }
                *idx += 1;
            }
            // ExactScalar cannot equal a dense-tensor Fraction element
            ValueData::ExactScalar(_) => return false,
            ValueData::Vector(inner) => {
                if !nested_flatten_matches(inner, data, idx) {
                    return false;
                }
            }
            ValueData::Tensor {
                data: inner_data, ..
            } => {
                for lane in 0..inner_data.len() {
                    if *idx >= data.len()
                        || data.fraction_or_nil(*idx) != inner_data.fraction_or_nil(lane)
                        || data.lane_reason(*idx) != inner_data.lane_reason(lane)
                        || data.lane_detail(*idx) != inner_data.lane_detail(lane)
                    {
                        return false;
                    }
                    *idx += 1;
                }
            }
            _ => return false,
        }
    }
    true
}

/// Flatten a nested `Vector` into `(shape, leaves, lane reasons)` the same way
/// `nested_flatten_matches` walks it against a dense tensor's lanes:
/// `Scalar` contributes its `Fraction`, `Nil` an absent lane and the reason
/// for it, `Tensor` its own dense lanes, `Vector` recurses, and anything else
/// (`ExactScalar`, `Boolean`, `Text`, `Symbol`) fails the flatten — mirroring
/// exactly which leaves `nested_flatten_matches` is willing to match against a
/// tensor lane. Used only to make [`ValueData`]'s `Hash` agree with the
/// `Vector`/`Tensor` cross-equality in `PartialEq`: a value that *can*
/// equal a dense tensor must hash the way that tensor does.
pub(super) type DenseFlatten = (
    Vec<usize>,
    Vec<Option<Fraction>>,
    Vec<(usize, Option<NilReason>)>,
);

pub(super) fn dense_flatten(v: &[Value]) -> Option<DenseFlatten> {
    let shape = nested_vector_shape(v)?;
    let mut leaves = Vec::new();
    let mut reasons = Vec::new();
    if collect_dense_leaves(v, &mut leaves, &mut reasons) {
        Some((shape, leaves, reasons))
    } else {
        None
    }
}

/// The absent lanes of a dense tensor and the reason for each, in lane order
/// — the hash counterpart of what [`DenseTensor`]'s `PartialEq` compares. An
/// absent lane always contributes an entry, `None` when it carries no reason,
/// so a reasonless absence cannot hash like a reasoned one.
pub(super) fn dense_lane_reasons(data: &DenseTensor) -> Vec<(usize, Option<NilReason>)> {
    (0..data.len())
        .filter(|index| !data.is_valid(*index))
        .map(|index| (index, data.lane_reason(index)))
        .collect()
}

fn collect_dense_leaves(
    v: &[Value],
    out: &mut Vec<Option<Fraction>>,
    reasons: &mut Vec<(usize, Option<NilReason>)>,
) -> bool {
    for child in v {
        match &child.data {
            ValueData::Scalar(f) => out.push(Some(f.clone())),
            // An absent leaf is `None` whether or not it holds a dividend:
            // the pair is not part of its identity, so it must not reach the
            // hash.
            ValueData::Nil(_) => {
                reasons.push((out.len(), child.nil_reason().copied()));
                out.push(None);
            }
            ValueData::Vector(inner) => {
                if !collect_dense_leaves(inner, out, reasons) {
                    return false;
                }
            }
            ValueData::Tensor { data, .. } => {
                let offset = out.len();
                reasons.extend(
                    dense_lane_reasons(data)
                        .into_iter()
                        .map(|(index, reason)| (index + offset, reason)),
                );
                out.extend((0..data.len()).map(|lane| data.get_small_fraction(lane)));
            }
            _ => return false,
        }
    }
    true
}

impl Value {
    #[inline]
    /// A numeric leaf from a rational.
    ///
    /// A `Fraction` whose denominator is 0 is absent — the pair the dense
    /// tensor lanes store for an absent lane, not a rational. It becomes
    /// `ValueData::Nil` holding that pair, rather than a `Scalar` wrapping an
    /// unreadable number: this is the one place fraction-level absence is
    /// translated into value-level absence, so a lane rehydrated out of a
    /// tensor answers [`Value::is_nil`] like any other NIL. A pair carries
    /// presence and a dividend, not a reason, so the absence reads back
    /// reasonless here.
    pub fn from_fraction(f: Fraction) -> Self {
        if f.is_nil() {
            // A `Fraction` records that it is absent and nothing about why, so
            // this is a *reasonless* NIL — not `nil_literal()`, which claims
            // the program wrote it (`spec/outcomes.json`: "a NIL the program
            // wrote rather than computed"). Where the reason is known it is
            // stored beside the lane, and `Value::from_dense_lane` is the
            // materialization that reads it.
            return Self::absent_number(f, AbsenceMetadata::with_reasonless_unknown());
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
        // A rational takes the `Fraction` path — through `from_fraction`, so
        // that `Rational(nil)` (what the nil-propagating exact arithmetic
        // answers for an absent operand) becomes `ValueData::Nil` here as it
        // does everywhere else, never a `Scalar` wrapping an absent pair
        // that displays as `NIL` while answering `NIL?` with FALSE.
        if let Some(f) = er.as_rational() {
            return Self::from_fraction(f.clone());
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
        matches!(self.data, ValueData::Nil(_))
    }

    /// As [`is_nil`]: operational-absence test.
    #[inline]
    pub fn is_absent(&self) -> bool {
        matches!(self.data, ValueData::Nil(_))
    }

    /// As [`is_nil`]: operational-absence test, and deliberately *not*
    /// narrower than one.
    ///
    /// UNKNOWN is a NIL read in truth position (LANG.VALUES.TRUTH), so it is
    /// an operational NIL like any other: `NIL?` answers TRUE for it and
    /// `NIL-REASON` reports the reason it arrived with.
    #[inline]
    pub fn is_operational_nil(&self) -> bool {
        matches!(self.data, ValueData::Nil(_))
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
            ValueData::Nil(_) => "NIL",
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
            | ValueData::Nil(_)
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
                    self.absence.as_deref().cloned(),
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
            ValueData::Nil(_) => false,
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
