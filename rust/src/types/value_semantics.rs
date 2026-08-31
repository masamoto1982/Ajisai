//! Scalar construction and semantic classification for [`Value`].
//!
//! Invariant: storage representation is translated to public semantic axes here;
//! callers do not infer semantic kind, shape, capability, or origin themselves.

use super::fraction::Fraction;
use super::value_tensor::tensor_to_nested_values;
use super::{DenseTensor, Interpretation, Value, ValueData};
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
    /// Boolean, so it returns `None` here — U is `Nil` data carrying the
    /// `TruthValue` hint; read it with [`Value::truth_value`] instead.
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
        Self {
            data: ValueData::Text(Arc::from(s)),
            hint: Interpretation::Unassigned,
            absence: None,
        }
    }

    /// The characters of a String value, or `None` for any other domain.
    ///
    /// This is the whole of stringhood now: no element inspection, no
    /// codepoint-range guessing, no hint. A Vector of codepoint Scalars is a
    /// Vector, and answers `None`.
    #[inline]
    pub fn as_text(&self) -> Option<&str> {
        match &self.data {
            ValueData::Text(s) => Some(s),
            _ => None,
        }
    }

    /// A bare Word reference — data until something executes it. See
    /// `ValueData::Symbol`'s doc comment.
    pub fn from_symbol(s: &str) -> Self {
        Self {
            data: ValueData::Symbol(Arc::from(s)),
            hint: Interpretation::Unassigned,
            absence: None,
        }
    }

    #[inline]
    pub fn from_children(children: Vec<Value>) -> Self {
        Self {
            data: ValueData::Vector(Arc::new(children)),
            hint: Interpretation::Unassigned,
            absence: None,
        }
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
        Self {
            data: ValueData::Vector(Arc::new(values)),
            hint: Interpretation::Unassigned,
            absence: None,
        }
    }

    pub fn from_vector_with_hint(values: Vec<Value>, hint: Interpretation) -> Self {
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

    /// Whether a Tier 2 computable real sits anywhere inside this value.
    ///
    /// `Computable`'s `PartialEq`/`Hash` are allocation identity
    /// (`types::exact::computable`), so any structural comparison reaching one
    /// answers from how the value was made rather than from what it denotes —
    /// which LANG.VALUES.DENOTATION forbids. A caller about to decide from
    /// `ValueData` equality asks this first and defers to the budgeted
    /// comparison instead.
    ///
    /// Only a Vector needs the walk: a Tensor stores `i64` numerator/
    /// denominator pairs (`DenseTensor`) and so cannot hold one.
    pub fn carries_computable(&self) -> bool {
        match &self.data {
            ValueData::ExactScalar(er) => er.is_computable(),
            ValueData::Vector(items) => items.iter().any(Value::carries_computable),
            _ => false,
        }
    }

    /// As [`is_nil`]: operational-absence test, and deliberately *not*
    /// narrower than one.
    ///
    /// The logical Unknown (U) is `Nil` data carrying the `TruthValue` hint,
    /// and this briefly excluded that hint, on the theory that U is a truth
    /// value rather than an absence. That conflated two different things
    /// under one name. The U this language actually has arises from a NIL
    /// operand read in truth position (LANG.VALUES.TRUTH): something *is*
    /// absent, so `NIL?` must answer TRUE and `NIL-REASON` must still report
    /// the reason it arrived with. Excluding it here made
    /// `1 0 DIV TRUE AND NIL-REASON` answer `notAvailable` while the host
    /// protocol went on publishing `absence.reason = divisionByZero` for the
    /// very same value — the language contradicting its own protocol, and
    /// LANG.VALUES.NIL ("the reason is the entire observable content of a
    /// NIL") along with it.
    ///
    /// A U that is genuinely *not* an absence — an undecided comparison
    /// between two reals that both exist — would want that firewall. No such
    /// value exists yet (comparison over the exact domain is total,
    /// LANG.VALUES.EXACT), so the distinction belongs to whatever introduces
    /// one, not here.
    #[inline]
    pub fn is_operational_nil(&self) -> bool {
        matches!(self.data, ValueData::Nil)
    }

    #[inline]
    pub fn semantic_kind(&self) -> SemanticKind {
        match &self.data {
            // A definite boolean is truth-valued, not numeric; its truth is
            // observed through the `truthValue` axis and `truthValued`
            // capability (LANG.VALUES.TRUTH). It reports `number` on the coarse
            // `semanticKind` axis only for protocol stability — distinctness
            // from a number lives in value identity (`TRUE 1 EQ` is false),
            // not in this axis.
            ValueData::Boolean(_) => SemanticKind::Number,
            ValueData::Scalar(_) | ValueData::ExactScalar(_) => SemanticKind::Number,
            // A String reports `collection` on this coarse axis, matching
            // the frozen `v1_semantic_kind` (which has always mapped
            // `KernelValue::String` this way). Its distinctness from a Vector
            // lives in value identity — `'A' [ 65 ] EQ` is false — not on a
            // protocol axis, so making String a domain costs V1 nothing.
            ValueData::Text(_) => SemanticKind::Collection,
            ValueData::Vector(_) | ValueData::Tensor { .. } => SemanticKind::Collection,
            // The logical Unknown (U — `Nil` carrying the `TruthValue`
            // hint) reports `absence` on this coarse `semanticKind` axis,
            // same as an operational NIL; its distinctness lives on the
            // `truthValue` axis (LANG.VALUES.TRUTH) and in value identity, not here.
            ValueData::Nil => SemanticKind::Absence,
            // A Symbol is a bare Word reference — the closest existing
            // bucket on this coarse axis is `Code`, though a lone Symbol
            // (distinct from the Vector holding it) is arguably its own
            // thing; unresolved.
            ValueData::Symbol(_) => SemanticKind::Code,
        }
    }

    #[inline]
    pub fn shape_kind(&self) -> ValueShape {
        match &self.data {
            ValueData::Boolean(_) => ValueShape::Scalar,
            ValueData::Scalar(_) | ValueData::ExactScalar(_) => ValueShape::Scalar,
            // As above: `v1_shape` maps a spine String to `vector`.
            ValueData::Text(_) => ValueShape::Vector,
            ValueData::Vector(_) => ValueShape::Vector,
            ValueData::Tensor { .. } => ValueShape::Tensor,
            // The logical Unknown (U — `Nil` carrying the `TruthValue`
            // hint) reports `absence` on this coarse `shape` axis too, same
            // as an operational NIL.
            ValueData::Nil => ValueShape::Absence,
            // `ValueShape::CodeBlock` used to mean "the executable domain";
            // every Vector is executable now, so the `Vector` arm above
            // already covers what this used to distinguish. Kept for a lone
            // Symbol only.
            ValueData::Symbol(_) => ValueShape::CodeBlock,
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
                // Every Vector is potentially executable now (EXEC no longer
                // rejects it) — Callable used to be exclusive to CodeBlock.
                capabilities.push(Capability::Callable);
            }
            // A String keeps the legacy V1 capability set for a string, which
            // `v1_capabilities` still reconstructs from `KernelValue::String`.
            ValueData::Text(_) => {
                capabilities.push(Capability::Iterable);
                capabilities.push(Capability::Indexable);
                capabilities.push(Capability::UserEditable);
            }
            // Every NIL advertises `nilPassthrough`, the logical Unknown (U)
            // included. U used to be excluded here by its `TruthValue` hint,
            // which advertised something untrue the moment `AND`/`OR`/`NOT`
            // made U reachable: `TRUE NIL AND 1 ADD` answers NIL, so U does
            // pass through, and a capability a consumer branches on
            // (LANG.OBSERVATION.FIREWALL) may not say otherwise. U's
            // distinction from an ordinary NIL is the `truthValue` axis it
            // gains below via `is_truth_value`, not a withheld capability.
            ValueData::Nil => {
                capabilities.push(Capability::NilPassthrough);
                capabilities.push(Capability::Diagnosable);
                capabilities.push(Capability::AiExplainable);
            }
            // A lone Symbol has no extra capability of its own — it is data
            // (a Word reference) until the Vector holding it is EXEC'd, at
            // which point the Vector's Callable capability is what applies.
            ValueData::Symbol(_) => {}
            // A boolean's only extra capability is `truthValued`, added below.
            ValueData::Boolean(_) => {}
        }
        // Truth-valued values (true / false / unknown) advertise the
        // `truthValued` capability so consumers know to read the
        // `truthValue` axis (LANG.VALUES.TRUTH). This covers definite
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
            | ValueData::Symbol(_) => None,
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
    pub fn is_truthy(&self) -> bool {
        match &self.data {
            ValueData::Boolean(b) => *b,
            // `is_truthy` is a total two-valued coercion. U (`Nil` carrying
            // the `TruthValue` hint) is neither definitely true nor false,
            // so it conservatively collapses to `false` — the same result as
            // an operational NIL, hence the shared arm. Control words that
            // must honour the third value branch on `is_truth_value()` first
            // (e.g. COND), never here.
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
        }
    }
}
