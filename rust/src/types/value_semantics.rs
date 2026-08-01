//! Scalar construction and semantic classification for [`Value`].
//!
//! Invariant: storage representation is translated to public semantic axes here;
//! callers do not infer semantic kind, shape, capability, or origin themselves.

use super::fraction::Fraction;
use super::value_tensor::tensor_to_nested_values;
use super::{DenseTensor, Interpretation, Value, ValueData};
use crate::error::NilReason;
use crate::semantic::{AbsenceOrigin, Capability, SemanticKind, ValueOrigin, ValueShape};
use std::sync::Arc;

impl Value {
    #[inline]
    pub fn from_fraction(f: Fraction) -> Self {
        Self {
            data: ValueData::Scalar(f),
            hint: Interpretation::RawNumber,
            absence: None,
        }
    }

    #[inline]
    pub fn from_int(n: i64) -> Self {
        Self {
            data: ValueData::Scalar(Fraction::from(n)),
            hint: Interpretation::RawNumber,
            absence: None,
        }
    }

    #[inline]
    pub fn from_bool(b: bool) -> Self {
        Self {
            data: ValueData::Boolean(b),
            hint: Interpretation::TruthValue,
            absence: None,
        }
    }

    /// The definite truth value carried by a Boolean data value, or `None`
    /// for any non-Boolean value. This is the data-plane truth accessor:
    /// unlike [`Value::is_truthy`] it never coerces a number, vector, or
    /// other shape into a truth value. The logical Unknown (U) is not a
    /// Boolean, so it returns `None` here (test it with [`Value::is_unknown`]).
    #[inline]
    pub fn as_truth(&self) -> Option<bool> {
        match &self.data {
            ValueData::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn from_string(s: &str) -> Self {
        let mut children: Vec<Value> = Vec::with_capacity(s.chars().count());
        for c in s.chars() {
            children.push(Value::from_int(c as u32 as i64));
        }
        if children.is_empty() {
            return Self::nil_with_reason(NilReason::EmptySequence);
        }
        Self {
            data: ValueData::Vector(Arc::new(children)),
            hint: Interpretation::Text,
            absence: None,
        }
    }

    pub fn from_symbol(s: &str) -> Self {
        Self::from_string(s)
    }

    #[inline]
    pub fn from_children(children: Vec<Value>) -> Self {
        Self {
            data: ValueData::Vector(Arc::new(children)),
            hint: Interpretation::Unassigned,
            absence: None,
        }
    }

    #[inline]
    pub fn from_children_with_hint(children: Vec<Value>, hint: Interpretation) -> Self {
        Self {
            data: ValueData::Vector(Arc::new(children)),
            hint,
            absence: None,
        }
    }

    pub fn from_vector(values: Vec<Value>) -> Self {
        if values.is_empty() {
            return Self::nil_with_reason(NilReason::EmptySequence);
        }
        Self {
            data: ValueData::Vector(Arc::new(values)),
            hint: Interpretation::Unassigned,
            absence: None,
        }
    }

    pub fn from_vector_with_hint(values: Vec<Value>, hint: Interpretation) -> Self {
        if values.is_empty() {
            return Self::nil_with_reason(NilReason::EmptySequence);
        }
        Self {
            data: ValueData::Vector(Arc::new(values)),
            hint,
            absence: None,
        }
    }

    #[inline]
    pub fn from_exact_real(er: crate::types::exact::ExactReal) -> Self {
        // If the ExactReal is already rational, use the fast Fraction path.
        if let Some(f) = er.as_rational() {
            return Self {
                data: ValueData::Scalar(f.clone()),
                hint: Interpretation::RawNumber,
                absence: None,
            };
        }
        Self {
            data: ValueData::ExactScalar(er),
            hint: Interpretation::RawNumber,
            absence: None,
        }
    }

    #[inline]
    pub fn from_number(f: Fraction) -> Self {
        Self::from_fraction(f)
    }

    /// NIL test: `true` only for the operational absence node
    /// ([`ValueData::Nil`], the Bubble).
    #[inline]
    pub fn is_nil(&self) -> bool {
        matches!(self.data, ValueData::Nil)
    }

    /// As [`is_nil`]: operational-absence test. The logical Unknown (U) is
    /// not absent.
    #[inline]
    pub fn is_absent(&self) -> bool {
        matches!(self.data, ValueData::Nil)
    }

    /// Operational NIL only: an operational absence node that is, by
    /// construction, never the logical truth value Unknown (U). Retained as
    /// the intent-revealing name at the firewall boundary (SPEC §7.5 / §2.3);
    /// now identical to [`is_nil`] because U has its own variant.
    #[inline]
    pub fn is_operational_nil(&self) -> bool {
        matches!(self.data, ValueData::Nil)
    }

    #[inline]
    pub fn semantic_kind(&self) -> SemanticKind {
        match &self.data {
            // A definite boolean is truth-valued, not numeric; its truth is
            // observed through the `truthValue` axis and `truthValued`
            // capability (SPEC §2.3). It reports `number` on the coarse
            // `semanticKind` axis only for protocol stability — distinctness
            // from a number lives in value identity (`TRUE 1 EQ` is false),
            // not in this axis.
            ValueData::Boolean(_) => SemanticKind::Number,
            ValueData::Scalar(_) | ValueData::ExactScalar(_) => SemanticKind::Number,
            ValueData::Vector(_) | ValueData::Tensor { .. } => SemanticKind::Collection,
            // CS4 PR-2: U is a truth value, not an operational absence. Like a
            // definite Boolean it reports `number` on the coarse `semanticKind`
            // axis (for protocol stability — its distinctness lives in the
            // `truthValue` axis and value identity, SPEC §2.3), never
            // `absence`.
            ValueData::Nil => SemanticKind::Absence,
            ValueData::CodeBlock(_) => SemanticKind::Code,
        }
    }

    #[inline]
    pub fn shape_kind(&self) -> ValueShape {
        match &self.data {
            ValueData::Boolean(_) => ValueShape::Scalar,
            ValueData::Scalar(_) | ValueData::ExactScalar(_) => ValueShape::Scalar,
            ValueData::Vector(_) => ValueShape::Vector,
            ValueData::Tensor { .. } => ValueShape::Tensor,
            // CS4 PR-2: U is a rank-0 scalar truth value (like a Boolean), not
            // an absence.
            ValueData::Nil => ValueShape::Absence,
            ValueData::CodeBlock(_) => ValueShape::CodeBlock,
        }
    }

    pub fn capabilities(&self) -> Vec<Capability> {
        let mut capabilities = vec![
            Capability::StackItem,
            Capability::Serializable,
            Capability::Displayable,
        ];
        match &self.data {
            ValueData::Scalar(_) => {
                capabilities.push(Capability::Numeric);
                capabilities.push(Capability::ExactNumeric);
                capabilities.push(Capability::UserEditable);
            }
            ValueData::ExactScalar(_) => {
                capabilities.push(Capability::Numeric);
                capabilities.push(Capability::ExactNumeric);
            }
            ValueData::Vector(_) | ValueData::Tensor { .. } => {
                capabilities.push(Capability::Iterable);
                capabilities.push(Capability::Indexable);
                capabilities.push(Capability::UserEditable);
            }
            // CS4 PR-2: U carries a diagnosis (its `agreedPrefix`) and is
            // AI-explainable, but it is emphatically **not** a NIL-passthrough
            // value — advertising `NilPassthrough` would contradict the very
            // firewall that keeps U from being absorbed as an operational NIL
            // (SPEC §2.3 / §7.5). It gains `truthValued` below via
            // `is_truth_value`.
            ValueData::Nil => {
                capabilities.push(Capability::NilPassthrough);
                capabilities.push(Capability::Diagnosable);
                capabilities.push(Capability::AiExplainable);
            }
            ValueData::CodeBlock(_) => capabilities.push(Capability::Callable),
            // A boolean's only extra capability is `truthValued`, added below.
            ValueData::Boolean(_) => {}
        }
        // Truth-valued values (true / false / unknown) advertise the
        // `truthValued` capability so consumers know to read the
        // `truthValue` axis (SPEC §2.3, §12.2). This covers definite
        // booleans (Scalar + TruthValue role) and the logical U.
        if self.is_truth_value() {
            capabilities.push(Capability::TruthValued);
        }
        capabilities
    }

    pub fn has_capability(&self, capability: Capability) -> bool {
        self.capabilities().contains(&capability)
    }

    pub fn origin(&self) -> ValueOrigin {
        match self.absence_metadata().map(|metadata| &metadata.origin) {
            Some(AbsenceOrigin::Literal) => ValueOrigin::Literal,
            Some(AbsenceOrigin::NilPropagation) => ValueOrigin::NilPropagation,
            Some(AbsenceOrigin::HostEnvironment) => ValueOrigin::HostEnvironment,
            _ => ValueOrigin::Unknown,
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
            | ValueData::Scalar(_)
            | ValueData::ExactScalar(_)
            | ValueData::Nil
            | ValueData::CodeBlock(_) => None,
        }
    }

    /// Return a `Cow<Value>` that is guaranteed to use a non-`Tensor`
    /// representation. `Tensor` values are converted into a nested
    /// `ValueData::Vector` (preserving `hint` and `absence`); every other
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
                std::borrow::Cow::Owned(Value {
                    data: ValueData::Vector(Arc::new(children)),
                    hint: self.hint,
                    absence: self.absence.clone(),
                })
            }
            _ => std::borrow::Cow::Borrowed(self),
        }
    }

    #[inline]
    pub fn is_uniquely_owned(&self) -> bool {
        match &self.data {
            ValueData::Boolean(_) => true,
            ValueData::Scalar(_) | ValueData::ExactScalar(_) | ValueData::Nil => true,
            ValueData::Vector(rc) => Arc::strong_count(rc) == 1,
            ValueData::Tensor { data, shape } => {
                Arc::strong_count(data) == 1 && Arc::strong_count(shape) == 1
            }
            ValueData::CodeBlock(_) => false,
        }
    }

    #[inline]
    pub fn is_truthy(&self) -> bool {
        match &self.data {
            ValueData::Boolean(b) => *b,
            // CS4 PR-2 (reviewed): `is_truthy` is a total two-valued
            // coercion. U is neither definitely true nor false, so it
            // conservatively collapses to `false` — the same result as NIL,
            // hence the shared arm. Control words that must honour the third
            // value branch on `is_unknown()` first (e.g. COND), never here.
            ValueData::Nil => false,
            ValueData::Scalar(f) => !f.is_zero() && !f.is_nil(),
            // ExactScalar values from AlgebraicSqrt are always non-zero positive
            // irrationals; Gosper nodes conservatively report truthy.
            ValueData::ExactScalar(_) => true,
            ValueData::Vector(v) => !v.is_empty() && !v.iter().all(|c| !c.is_truthy()),
            ValueData::Tensor { data, .. } => {
                !data.is_empty() && !data.iter().all(|f| f.is_zero() || f.is_nil())
            }
            ValueData::CodeBlock(_) => true,
        }
    }
}
