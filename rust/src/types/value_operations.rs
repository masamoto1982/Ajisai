use super::fraction::Fraction;
use super::interval::Interval;
use super::{DenseTensor, Interpretation, Token, Value, ValueData};
use crate::error::NilReason;
use crate::interpreter::debug_diagnosis::DebugDiagnosis;
use crate::semantic::{
    AbsenceMetadata, AbsenceOrigin, Capability, Recoverability, SemanticKind, ValueOrigin,
    ValueShape,
};
use std::sync::Arc;

fn absence_origin_for_reason(reason: &NilReason) -> AbsenceOrigin {
    match reason {
        NilReason::EmptySequence => AbsenceOrigin::EmptySequence,
        NilReason::MissingField => AbsenceOrigin::MissingField,
        NilReason::InvalidEncoding => AbsenceOrigin::InvalidEncoding,
        NilReason::InvalidLens => AbsenceOrigin::InvalidLens,
        NilReason::StackUnderflow => AbsenceOrigin::StackUnderflow,
        NilReason::IndexOutOfBounds => AbsenceOrigin::IndexOutOfBounds,
        NilReason::UnknownWord => AbsenceOrigin::UnknownWord,
        NilReason::ExecutionFailure => AbsenceOrigin::ExecutionFailure,
        NilReason::Undecidable => AbsenceOrigin::ComparisonBudget,
        NilReason::NoData => AbsenceOrigin::HostEnvironment,
        NilReason::PortDisconnected => AbsenceOrigin::HostEnvironment,
        NilReason::DivisionByZero => AbsenceOrigin::DivisionByZero,
        NilReason::SpaceExhausted => AbsenceOrigin::SpaceBudget,
    }
}

impl Value {
    #[inline]
    pub fn nil() -> Self {
        Self::nil_literal()
    }

    #[inline]
    pub fn nil_literal() -> Self {
        Self {
            data: ValueData::Nil,
            hint: Interpretation::Nil,
            absence: Some(AbsenceMetadata::literal()),
        }
    }

    #[inline]
    pub fn nil_with_absence(absence: AbsenceMetadata) -> Self {
        Self {
            data: ValueData::Nil,
            hint: Interpretation::Nil,
            absence: Some(absence),
        }
    }

    #[inline]
    pub fn nil_with_reason(reason: NilReason) -> Self {
        let origin = absence_origin_for_reason(&reason);
        Self::nil_with_absence(AbsenceMetadata::with_reason(
            reason,
            origin,
            Recoverability::Unknown,
        ))
    }

    /// Construct the logical truth value `Unknown` (U), SPEC §7.5 / §7.4.1.
    ///
    /// U is its own [`ValueData::Unknown`] variant carrying the
    /// `Interpretation::TruthValue` role — **not** a NIL node. It is a
    /// logical value, distinct at the type level from operational absence, so
    /// no NIL call site can absorb it. Detect it with [`is_unknown`], never by
    /// matching the storage representation.
    #[inline]
    pub fn unknown() -> Self {
        Self {
            data: ValueData::Unknown(None),
            hint: Interpretation::TruthValue,
            absence: None,
        }
    }

    /// The logical truth value `Unknown` (U) carrying the CF-comparison
    /// agreed-prefix diagnosis (SPEC §4.5.0 / §7.4.1). Identical to
    /// [`unknown`] except that U's own diagnostic carrier records a
    /// `DebugDiagnosis` whose `agreed_prefix` is surfaced as
    /// `diagnosis.agreedPrefix`. `word` names the comparison Coreword that
    /// produced U (e.g. `"COMPARE-WITHIN"`, `"LT"`).
    pub fn unknown_with_agreed_prefix(word: Option<&str>, agreed_prefix: usize) -> Self {
        Self {
            data: ValueData::Unknown(Some(Box::new(DebugDiagnosis::comparison_unknown(
                word,
                agreed_prefix,
            )))),
            hint: Interpretation::TruthValue,
            absence: None,
        }
    }

    /// Whether this value is the logical truth value `Unknown` (U).
    ///
    /// This is the single canonical predicate for U. It keys off the
    /// dedicated [`ValueData::Unknown`] variant, so the U/NIL distinction is
    /// a type invariant. All call sites must use this instead of matching the
    /// storage representation.
    #[inline]
    pub fn is_unknown(&self) -> bool {
        matches!(self.data, ValueData::Unknown(_))
    }

    /// Whether this value carries the `TruthValue` interpretation role
    /// (true, false, or unknown). Used at observation boundaries to attach
    /// the `truthValue` axis and the `truthValued` capability.
    #[inline]
    pub fn is_truth_value(&self) -> bool {
        self.hint == Interpretation::TruthValue
    }

    /// The observable `truthValue` axis (SPEC §2.3) under a given effective
    /// interpretation role: `Some("true")`, `Some("false")`, or
    /// `Some("unknown")` for truth-valued values, and `None` otherwise.
    ///
    /// The role is taken as a parameter because a definite boolean produced
    /// by a comparison/logic word carries its `TruthValue` role in the
    /// semantic plane (SPEC §12.2), not on the value's own `hint`. The
    /// logical Unknown (U) is always `unknown` regardless of the role, since
    /// it is detected from its reason. This is the single canonical mapping
    /// from a value to its three-valued logical surface; external consumers
    /// must read this axis rather than the internal NIL representation or
    /// display text.
    pub fn truth_value_for_role(&self, effective: Interpretation) -> Option<&'static str> {
        if self.is_unknown() {
            return Some("unknown");
        }
        // A Boolean is intrinsically truth-valued: it reports its truth on the
        // axis regardless of the effective role, because its data identity —
        // not a semantic-plane role — carries the truth.
        if let ValueData::Boolean(b) = &self.data {
            return Some(if *b { "true" } else { "false" });
        }
        if effective != Interpretation::TruthValue {
            return None;
        }
        match &self.data {
            ValueData::Nil => Some("unknown"),
            ValueData::Scalar(f) => Some(if f.is_zero() { "false" } else { "true" }),
            ValueData::ExactScalar(_) => Some("true"),
            _ => Some(if self.is_truthy() { "true" } else { "false" }),
        }
    }

    /// The `truthValue` axis using the value's own `hint` as the role.
    /// Convenience for values that carry the `TruthValue` role on the value
    /// itself (notably U); the boundary uses
    /// [`truth_value_for_role`] with the effective role.
    pub fn truth_value(&self) -> Option<&'static str> {
        self.truth_value_for_role(self.hint)
    }

    #[inline]
    pub fn nil_inheriting_absence_from(source: &Self) -> Self {
        match source.normalized_absence_metadata() {
            Some(absence) => Self::nil_with_absence(absence),
            None => Self::nil(),
        }
    }

    #[inline]
    pub fn nil_from_diagnosis(
        reason: NilReason,
        origin: AbsenceOrigin,
        recoverability: Recoverability,
        diagnosis: DebugDiagnosis,
    ) -> Self {
        Self::nil_with_absence(AbsenceMetadata::from_diagnosis(
            reason,
            origin,
            recoverability,
            diagnosis,
        ))
    }

    /// Create a reasoned NIL for the Bubble Rule: well-formed operations that
    /// cannot produce a value return Bubble/NIL directly with an explicit
    /// reason.
    #[inline]
    pub fn bubble_with_reason(
        reason: NilReason,
        origin: AbsenceOrigin,
        recoverability: Recoverability,
    ) -> Self {
        Self::nil_with_absence(AbsenceMetadata::with_reason(reason, origin, recoverability))
    }

    #[inline]
    pub fn absence_metadata(&self) -> Option<&AbsenceMetadata> {
        self.absence.as_ref()
    }

    #[inline]
    pub fn normalized_absence_metadata(&self) -> Option<AbsenceMetadata> {
        if !self.is_absent() {
            return None;
        }
        Some(
            self.absence
                .clone()
                .unwrap_or_else(AbsenceMetadata::with_reasonless_unknown),
        )
    }

    #[inline]
    pub fn nil_reason(&self) -> Option<&NilReason> {
        self.absence
            .as_ref()
            .and_then(|absence| absence.reason.as_ref())
    }

    #[inline]
    pub fn nil_diagnosis(&self) -> Option<&DebugDiagnosis> {
        // The logical Unknown (U) carries its comparison diagnosis on its own
        // variant, not in NIL's absence metadata. Surface it here so the
        // `agreedPrefix` accessors keep working for U without U ever holding
        // an operational NIL reason.
        if let ValueData::Unknown(diagnosis) = &self.data {
            return diagnosis.as_deref();
        }
        self.absence
            .as_ref()
            .and_then(|absence| absence.diagnosis.as_ref())
    }

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

    #[inline]
    pub fn from_interval(interval: Interval) -> Self {
        Self {
            data: ValueData::Vector(Arc::new(vec![
                Value::from_fraction(interval.lo),
                Value::from_fraction(interval.hi),
            ])),
            hint: Interpretation::Interval,
            absence: None,
        }
    }

    #[inline]
    pub fn from_datetime(f: Fraction) -> Self {
        Self {
            data: ValueData::Scalar(f),
            hint: Interpretation::Timestamp,
            absence: None,
        }
    }

    /// NIL test: `true` only for the operational absence node
    /// ([`ValueData::Nil`], the Bubble). The logical Unknown (U) is a
    /// separate [`ValueData::Unknown`] variant and is **not** NIL
    /// (`unknown().is_nil() == false`), so the U/NIL firewall (SPEC §7.5 /
    /// §2.3) is now guaranteed by the type rather than by a predicate
    /// convention.
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
            ValueData::Record { .. } => SemanticKind::Record,
            // CS4 PR-2: U is a truth value, not an operational absence. Like a
            // definite Boolean it reports `number` on the coarse `semanticKind`
            // axis (for protocol stability — its distinctness lives in the
            // `truthValue` axis and value identity, SPEC §2.3), never
            // `absence`.
            ValueData::Unknown(_) => SemanticKind::Number,
            ValueData::Nil => SemanticKind::Absence,
            ValueData::CodeBlock(_) => SemanticKind::Code,
            ValueData::ProcessHandle(_) => SemanticKind::Process,
            ValueData::SupervisorHandle(_) => SemanticKind::Supervisor,
        }
    }

    #[inline]
    pub fn shape_kind(&self) -> ValueShape {
        match &self.data {
            ValueData::Boolean(_) => ValueShape::Scalar,
            ValueData::Scalar(_) | ValueData::ExactScalar(_) => ValueShape::Scalar,
            ValueData::Vector(_) => ValueShape::Vector,
            ValueData::Tensor { .. } => ValueShape::Tensor,
            ValueData::Record { .. } => ValueShape::Record,
            // CS4 PR-2: U is a rank-0 scalar truth value (like a Boolean), not
            // an absence.
            ValueData::Unknown(_) => ValueShape::Scalar,
            ValueData::Nil => ValueShape::Absence,
            ValueData::CodeBlock(_) => ValueShape::CodeBlock,
            ValueData::ProcessHandle(_) | ValueData::SupervisorHandle(_) => ValueShape::Handle,
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
            ValueData::Vector(_) | ValueData::Tensor { .. } | ValueData::Record { .. } => {
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
            ValueData::Unknown(_) => {
                capabilities.push(Capability::Diagnosable);
                capabilities.push(Capability::AiExplainable);
            }
            ValueData::Nil => {
                capabilities.push(Capability::NilPassthrough);
                capabilities.push(Capability::Diagnosable);
                capabilities.push(Capability::AiExplainable);
            }
            ValueData::CodeBlock(_) => capabilities.push(Capability::Callable),
            // A boolean's only extra capability is `truthValued`, added below.
            ValueData::Boolean(_) => {}
            ValueData::ProcessHandle(_) | ValueData::SupervisorHandle(_) => {}
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
        matches!(
            self.data,
            ValueData::Vector(_) | ValueData::Tensor { .. } | ValueData::Record { .. }
        )
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
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                Some(std::borrow::Cow::Borrowed(v.as_slice()))
            }
            ValueData::Tensor { data, shape } => Some(std::borrow::Cow::Owned(
                tensor_to_nested_values(data, shape),
            )),
            ValueData::Boolean(_)
            | ValueData::Scalar(_)
            | ValueData::ExactScalar(_)
            | ValueData::Nil
            | ValueData::Unknown(_)
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
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
            ValueData::Scalar(_)
            | ValueData::ExactScalar(_)
            | ValueData::Nil
            | ValueData::Unknown(_) => true,
            ValueData::Vector(rc) => Arc::strong_count(rc) == 1,
            ValueData::Tensor { data, shape } => {
                Arc::strong_count(data) == 1 && Arc::strong_count(shape) == 1
            }
            ValueData::Record { pairs, .. } => Arc::strong_count(pairs) == 1,
            ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => false,
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
            ValueData::Nil | ValueData::Unknown(_) => false,
            ValueData::Scalar(f) => !f.is_zero() && !f.is_nil(),
            // ExactScalar values from AlgebraicSqrt are always non-zero positive
            // irrationals; Gosper nodes conservatively report truthy.
            ValueData::ExactScalar(_) => true,
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                !v.is_empty() && !v.iter().all(|c| !c.is_truthy())
            }
            ValueData::Tensor { data, .. } => {
                !data.is_empty() && !data.iter().all(|f| f.is_zero() || f.is_nil())
            }
            ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => true,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        match &self.data {
            ValueData::Nil => 0,
            // CS4 PR-2: U is a single scalar truth value, so it has length 1
            // like a Boolean (not 0 like an absence). It is not indexable —
            // `get_child`/`child` return `None`, exactly as for a Boolean.
            ValueData::Unknown(_) => 1,
            ValueData::Boolean(_) => 1,
            ValueData::Scalar(_) | ValueData::ExactScalar(_) => 1,
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => v.len(),
            ValueData::Tensor { data, shape } => {
                if shape.is_empty() {
                    data.len()
                } else {
                    shape[0]
                }
            }
            ValueData::CodeBlock(tokens) => tokens.len(),
            ValueData::ProcessHandle(_) | ValueData::SupervisorHandle(_) => 1,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_child(&self, index: usize) -> Option<&Value> {
        match &self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => v.get(index),
            ValueData::Tensor { .. } => None,
            ValueData::Scalar(_) | ValueData::ExactScalar(_) if index == 0 => Some(self),
            ValueData::Boolean(_)
            | ValueData::Scalar(_)
            | ValueData::ExactScalar(_)
            | ValueData::Nil
            | ValueData::Unknown(_)
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
        }
    }

    /// Representation-agnostic child accessor. Works for both `Vector` and
    /// `Tensor` payloads by materializing a sub-Value (Scalar leaf or
    /// sub-Tensor) when the receiver is dense. Cloning is cheap because
    /// inner buffers are reference-counted.
    ///
    /// Prefer this over [`get_child`] when the call site can be reached with
    /// a dense `Tensor` input. Use `get_child` only when the caller is known
    /// to operate on `Record` or already-nested `Vector` payloads.
    pub fn child(&self, index: usize) -> Option<Value> {
        match &self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => v.get(index).cloned(),
            ValueData::Scalar(_) | ValueData::ExactScalar(_) if index == 0 => Some(self.clone()),
            ValueData::Tensor { data, shape } => tensor_child(data, shape, index),
            ValueData::Boolean(_)
            | ValueData::Scalar(_)
            | ValueData::ExactScalar(_)
            | ValueData::Nil
            | ValueData::Unknown(_)
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
        }
    }

    pub fn get_child_mut(&mut self, index: usize) -> Option<&mut Value> {
        if matches!(self.data, ValueData::Tensor { .. }) {
            self.hydrate_tensor_to_vector();
        }
        match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                Arc::make_mut(v).get_mut(index)
            }
            ValueData::Boolean(_)
            | ValueData::Tensor { .. }
            | ValueData::Scalar(_)
            | ValueData::ExactScalar(_)
            | ValueData::Nil
            | ValueData::Unknown(_)
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
        }
    }

    #[inline]
    pub fn first(&self) -> Option<&Value> {
        self.get_child(0)
    }

    #[inline]
    pub fn last(&self) -> Option<&Value> {
        match &self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => v.last(),
            ValueData::Tensor { .. } => None,
            ValueData::Scalar(_) | ValueData::ExactScalar(_) => Some(self),
            ValueData::Nil | ValueData::Unknown(_) => None,
            ValueData::Boolean(_)
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
        }
    }

    /// Convert a `ValueData::Tensor` in-place to a nested `ValueData::Vector`
    /// so that mutating helpers (push/pop/insert/remove/replace) can operate
    /// on a uniform `Vec<Value>` representation.
    fn hydrate_tensor_to_vector(&mut self) {
        let ValueData::Tensor { data, shape } = &self.data else {
            return;
        };
        let children = tensor_to_nested_values(data, shape);
        self.data = ValueData::Vector(Arc::new(children));
    }

    pub fn push_child(&mut self, child: Value) {
        if matches!(self.data, ValueData::Tensor { .. }) {
            self.hydrate_tensor_to_vector();
        }
        match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                Arc::make_mut(v).push(child);
            }
            ValueData::Nil => {
                self.data = ValueData::Vector(Arc::new(vec![child]));
            }
            ValueData::Scalar(f) => {
                let old = Value::from_fraction(f.clone());
                self.data = ValueData::Vector(Arc::new(vec![old, child]));
            }
            ValueData::ExactScalar(_) => {
                // Cannot push_child into an ExactScalar — silently ignore
                // (ExactScalar is always a scalar leaf, never mutated into a vector).
            }
            // CS4 PR-2: pushing into U is a no-op, like a Boolean — U is a
            // scalar truth value, not an empty container to be seeded into a
            // vector (that NIL affordance does not apply to a definite datum).
            ValueData::Boolean(_)
            | ValueData::Unknown(_)
            | ValueData::Tensor { .. }
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => {}
        }
    }

    pub fn pop_child(&mut self) -> Option<Value> {
        if matches!(self.data, ValueData::Tensor { .. }) {
            self.hydrate_tensor_to_vector();
        }
        match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Arc::make_mut(v).pop(),
            ValueData::Boolean(_)
            | ValueData::Tensor { .. }
            | ValueData::Scalar(_)
            | ValueData::ExactScalar(_)
            | ValueData::Nil
            | ValueData::Unknown(_)
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
        }
    }

    pub fn insert_child(&mut self, index: usize, child: Value) {
        if matches!(self.data, ValueData::Tensor { .. }) {
            self.hydrate_tensor_to_vector();
        }
        let v: &mut Vec<Value> = match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Arc::make_mut(v),
            ValueData::Boolean(_)
            | ValueData::Tensor { .. }
            | ValueData::Scalar(_)
            | ValueData::ExactScalar(_)
            | ValueData::Nil
            | ValueData::Unknown(_)
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => return,
        };
        if index <= v.len() {
            v.insert(index, child);
        }
    }

    pub fn remove_child(&mut self, index: usize) -> Option<Value> {
        if matches!(self.data, ValueData::Tensor { .. }) {
            self.hydrate_tensor_to_vector();
        }
        let v: &mut Vec<Value> = match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Arc::make_mut(v),
            ValueData::Boolean(_)
            | ValueData::Tensor { .. }
            | ValueData::Scalar(_)
            | ValueData::ExactScalar(_)
            | ValueData::Nil
            | ValueData::Unknown(_)
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => return None,
        };
        if index < v.len() {
            Some(v.remove(index))
        } else {
            None
        }
    }

    pub fn replace_child(&mut self, index: usize, child: Value) -> Option<Value> {
        if matches!(self.data, ValueData::Tensor { .. }) {
            self.hydrate_tensor_to_vector();
        }
        let v: &mut Vec<Value> = match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Arc::make_mut(v),
            ValueData::Boolean(_)
            | ValueData::Tensor { .. }
            | ValueData::Scalar(_)
            | ValueData::ExactScalar(_)
            | ValueData::Nil
            | ValueData::Unknown(_)
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => return None,
        };
        if index < v.len() {
            Some(std::mem::replace(&mut v[index], child))
        } else {
            None
        }
    }

    #[inline]
    pub fn as_scalar(&self) -> Option<&Fraction> {
        match &self.data {
            ValueData::Scalar(f) => Some(f),
            ValueData::Boolean(_)
            | ValueData::ExactScalar(_)
            | ValueData::Vector(_)
            | ValueData::Tensor { .. }
            | ValueData::Record { .. }
            | ValueData::Nil
            | ValueData::Unknown(_)
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
        }
    }

    #[inline]
    pub fn as_scalar_mut(&mut self) -> Option<&mut Fraction> {
        match &mut self.data {
            ValueData::Scalar(f) => Some(f),
            ValueData::Boolean(_)
            | ValueData::ExactScalar(_)
            | ValueData::Vector(_)
            | ValueData::Tensor { .. }
            | ValueData::Record { .. }
            | ValueData::Nil
            | ValueData::Unknown(_)
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
        }
    }

    #[inline]
    pub fn as_i64(&self) -> Option<i64> {
        self.as_scalar().and_then(|f| f.to_i64())
    }

    #[inline]
    pub fn as_usize(&self) -> Option<usize> {
        self.as_scalar().and_then(|f| f.as_usize())
    }

    #[inline]
    pub fn as_vector(&self) -> Option<&Vec<Value>> {
        match &self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Some(v),
            ValueData::Tensor { .. } => None,
            ValueData::Boolean(_)
            | ValueData::Scalar(_)
            | ValueData::ExactScalar(_)
            | ValueData::Nil
            | ValueData::Unknown(_)
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
        }
    }

    #[inline]
    pub fn as_vector_mut(&mut self) -> Option<&mut Vec<Value>> {
        if matches!(self.data, ValueData::Tensor { .. }) {
            self.hydrate_tensor_to_vector();
        }
        match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Some(Arc::make_mut(v)),
            ValueData::Boolean(_)
            | ValueData::Tensor { .. }
            | ValueData::Scalar(_)
            | ValueData::ExactScalar(_)
            | ValueData::Nil
            | ValueData::Unknown(_)
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
        }
    }

    pub fn collect_fractions_flat(&self) -> Vec<Fraction> {
        let mut buf = Vec::new();
        self.collect_fractions_flat_into(&mut buf);
        buf
    }

    pub fn collect_fractions_flat_into(&self, buf: &mut Vec<Fraction>) {
        match &self.data {
            ValueData::Nil => buf.push(Fraction::nil()),
            ValueData::Scalar(f) => buf.push(f.clone()),
            ValueData::ExactScalar(er) => {
                // Use best rational approximation for ExactScalar in flat collection
                if let Some(f) = er.as_rational() {
                    buf.push(f.clone());
                }
                // non-rational ExactScalars are not representable as a single Fraction
            }
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                for child in v.iter() {
                    child.collect_fractions_flat_into(buf);
                }
            }
            ValueData::Tensor { data, .. } => {
                buf.extend(data.iter());
            }
            // CS4 PR-2: U is a truth value, not numeric content — it flattens
            // to no fraction lane, like a Boolean (NIL flattens to a nil
            // lane). Kept in lock-step with `count_fractions` below so buffer
            // sizing stays exact.
            ValueData::Boolean(_)
            | ValueData::Unknown(_)
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => {}
        }
    }

    pub fn count_fractions(&self) -> usize {
        match &self.data {
            ValueData::Nil => 1,
            ValueData::Scalar(_) | ValueData::ExactScalar(_) => 1,
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                v.iter().map(|c| c.count_fractions()).sum()
            }
            ValueData::Tensor { data, .. } => data.len(),
            // CS4 PR-2: U contributes no fraction lane (see
            // `collect_fractions_flat_into`), matching a Boolean.
            ValueData::Boolean(_)
            | ValueData::Unknown(_)
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => 0,
        }
    }

    pub fn shape(&self) -> Vec<usize> {
        match &self.data {
            // U and NIL are both rank-0 (empty shape), like a Boolean/Scalar.
            ValueData::Nil | ValueData::Unknown(_) => vec![],
            ValueData::Scalar(_) | ValueData::ExactScalar(_) => vec![],
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                if v.is_empty() {
                    vec![0]
                } else {
                    let first_shape: Vec<usize> = v[0].shape();
                    let all_same: bool = v.iter().skip(1).all(|c| c.shape() == first_shape);
                    if all_same && !first_shape.is_empty() {
                        let mut shape = vec![v.len()];
                        shape.extend(first_shape);
                        shape
                    } else {
                        vec![v.len()]
                    }
                }
            }
            ValueData::Tensor { shape, .. } => (**shape).clone(),
            ValueData::Boolean(_)
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => vec![],
        }
    }

    #[inline]
    pub fn is_code_block(&self) -> bool {
        matches!(self.data, ValueData::CodeBlock(_))
    }

    #[inline]
    pub fn as_code_block(&self) -> Option<&Vec<Token>> {
        let ValueData::CodeBlock(tokens) = &self.data else {
            return None;
        };
        Some(tokens)
    }

    pub fn from_code_block(tokens: Vec<Token>) -> Self {
        Self {
            data: ValueData::CodeBlock(tokens),
            hint: Interpretation::Unassigned,
            absence: None,
        }
    }

    pub fn from_process_handle(id: u64) -> Self {
        Self {
            data: ValueData::ProcessHandle(id),
            hint: Interpretation::Unassigned,
            absence: None,
        }
    }

    pub fn as_process_handle(&self) -> Option<u64> {
        match self.data {
            ValueData::ProcessHandle(id) => Some(id),
            _ => None,
        }
    }

    pub fn from_supervisor_handle(id: u64) -> Self {
        Self {
            data: ValueData::SupervisorHandle(id),
            hint: Interpretation::Unassigned,
            absence: None,
        }
    }

    pub fn resolve_default_hint(&self) -> Interpretation {
        match &self.data {
            ValueData::Nil => Interpretation::Nil,
            // CS4 PR-2: U's role is `TruthValue`, like a Boolean — its default
            // rendering role must not fall back to `Nil`.
            ValueData::Unknown(_) | ValueData::Boolean(_) => Interpretation::TruthValue,
            ValueData::Scalar(_) | ValueData::ExactScalar(_) => Interpretation::RawNumber,
            ValueData::Vector(_) | ValueData::Tensor { .. } | ValueData::Record { .. } => {
                Interpretation::Unassigned
            }
            ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => Interpretation::Unassigned,
        }
    }

    /// Construct a dense `Tensor` value. `data.len()` must equal the product of
    /// `shape` (or `shape` may be empty for a flat 1-D buffer; in that case
    /// `[data.len()]` is used).
    /// Wrap a flat `Vec<i64>` as a 1-D pure-integer dense `Tensor` (SoA),
    /// without materializing per-element `Value`s or `Fraction`s. This is the
    /// output constructor for the integer SIMD lane: it keeps the result in
    /// the same dense column representation as its inputs instead of degrading
    /// to an AoS `Vector` (handoff 手1). The `hint` matches `from_tensor` /
    /// `from_children` (`Unassigned`) so downstream interpretation is unchanged.
    pub fn from_int_tensor(numerators: Vec<i64>) -> Self {
        if numerators.is_empty() {
            return Self::nil_with_reason(NilReason::EmptySequence);
        }
        let len = numerators.len();
        let tensor = DenseTensor::from_integers(numerators);
        Self {
            data: ValueData::Tensor {
                data: Arc::new(tensor),
                shape: Arc::new(vec![len]),
            },
            hint: Interpretation::Unassigned,
            absence: None,
        }
    }

    pub fn from_tensor(data: Vec<Fraction>, shape: Vec<usize>) -> Self {
        if data.is_empty() {
            return Self::nil_with_reason(NilReason::EmptySequence);
        }
        let resolved_shape = if shape.is_empty() {
            vec![data.len()]
        } else {
            shape
        };
        let Some(tensor) = DenseTensor::from_fractions(data.clone(), resolved_shape.clone()) else {
            return Self::from_vector_with_hint(
                tensor_fractions_to_nested_values(&data, &resolved_shape),
                Interpretation::Unassigned,
            );
        };
        Self {
            data: ValueData::Tensor {
                data: Arc::new(tensor),
                shape: Arc::new(resolved_shape),
            },
            hint: Interpretation::Unassigned,
            absence: None,
        }
    }

    /// Like [`from_vector_with_hint`] but promotes the value to a dense
    /// `Tensor` when every leaf is a Fraction scalar and the shape is
    /// rectangular. Otherwise the nested form is preserved.
    ///
    /// The `String` display hint suppresses promotion at every level so that
    /// codepoint-based strings retain their nested representation.
    pub fn from_vector_promoted_with_hint(values: Vec<Value>, hint: Interpretation) -> Self {
        if values.is_empty() {
            return Self::nil_with_reason(NilReason::EmptySequence);
        }
        if hint == Interpretation::Text {
            return Self {
                data: ValueData::Vector(Arc::new(values)),
                hint,
                absence: None,
            };
        }
        if let Some((data, shape)) = try_collect_dense(&values) {
            if let Some(tensor) = DenseTensor::from_fractions(data, shape.clone()) {
                return Self {
                    data: ValueData::Tensor {
                        data: Arc::new(tensor),
                        shape: Arc::new(shape),
                    },
                    hint,
                    absence: None,
                };
            }
        }
        Self {
            data: ValueData::Vector(Arc::new(values)),
            hint,
            absence: None,
        }
    }

    /// Convenience wrapper around [`from_vector_promoted_with_hint`] using
    /// `Interpretation::Unassigned`.
    pub fn from_vector_promoted(values: Vec<Value>) -> Self {
        Self::from_vector_promoted_with_hint(values, Interpretation::Unassigned)
    }
}

/// Walk a list of `Value`s and return `(flat data, shape)` if every leaf is a
/// Fraction scalar (or a child Tensor) and the shape is rectangular. Returns
/// `None` if any leaf is non-numeric (NIL, Record, CodeBlock, Vector with
/// String hint, etc.) or if shapes disagree.
fn try_collect_dense(values: &[Value]) -> Option<(Vec<Fraction>, Vec<usize>)> {
    if values.is_empty() {
        return None;
    }
    let first = try_dense_value(&values[0])?;
    let inner_shape = first.1;
    let mut data = first.0;
    for v in values.iter().skip(1) {
        let (cdata, cshape) = try_dense_value(v)?;
        if cshape != inner_shape {
            return None;
        }
        data.extend(cdata);
    }
    let mut shape = vec![values.len()];
    shape.extend(inner_shape);
    Some((data, shape))
}

/// Materialize the i-th child of a dense Tensor as an owned `Value`. For 1-D
/// shape `[n]` the child is a Scalar; for higher rank the child is itself a
/// dense Tensor with the trailing dimensions.
fn tensor_child(data: &DenseTensor, shape: &[usize], index: usize) -> Option<Value> {
    if shape.is_empty() {
        return None;
    }
    let outer = shape[0];
    if index >= outer {
        return None;
    }
    if shape.len() == 1 {
        return data.get_small_fraction(index).map(Value::from_fraction);
    }
    let rest: Vec<usize> = shape[1..].to_vec();
    let stride: usize = rest.iter().product();
    let start = index * stride;
    let slice: Vec<Fraction> = (start..start + stride)
        .map(|lane| data.fraction_or_nil(lane))
        .collect();
    Some(Value::from_tensor(slice, rest))
}

fn try_dense_value(v: &Value) -> Option<(Vec<Fraction>, Vec<usize>)> {
    if v.hint == Interpretation::Text {
        return None;
    }
    match &v.data {
        ValueData::Scalar(f) => Some((vec![f.clone()], Vec::new())),
        ValueData::ExactScalar(_) => None, // ExactScalar cannot be densified into a Fraction tensor
        ValueData::Tensor { data, shape } => Some((data.to_fractions(), (**shape).clone())),
        ValueData::Vector(children) => try_collect_dense(children),
        ValueData::Boolean(_)
        | ValueData::Nil
        | ValueData::Unknown(_)
        | ValueData::Record { .. }
        | ValueData::CodeBlock(_)
        | ValueData::ProcessHandle(_)
        | ValueData::SupervisorHandle(_) => None,
    }
}

fn tensor_fractions_to_nested_values(data: &[Fraction], shape: &[usize]) -> Vec<Value> {
    fn build(data: &[Fraction], shape: &[usize], offset: usize) -> Vec<Value> {
        if shape.is_empty() || shape.len() == 1 {
            let len = shape
                .first()
                .copied()
                .unwrap_or_else(|| data.len().saturating_sub(offset));
            return data[offset..offset + len]
                .iter()
                .cloned()
                .map(Value::from_fraction)
                .collect();
        }
        let outer = shape[0];
        let rest = &shape[1..];
        let stride: usize = rest.iter().product();
        let mut out = Vec::with_capacity(outer);
        for i in 0..outer {
            out.push(Value::from_children(build(data, rest, offset + i * stride)));
        }
        out
    }
    build(data, shape, 0)
}

/// Materialize a dense Tensor (`data` + `shape`) as a tree of nested `Value`s.
/// Used by mutating helpers that need a uniform `Vec<Value>` representation,
/// and by display fallbacks.
pub(super) fn tensor_to_nested_values(data: &DenseTensor, shape: &[usize]) -> Vec<Value> {
    fn build(data: &DenseTensor, shape: &[usize], offset: usize) -> Vec<Value> {
        if shape.is_empty() || shape.len() == 1 {
            let len = shape
                .first()
                .copied()
                .unwrap_or_else(|| data.len().saturating_sub(offset));
            return (offset..offset + len)
                .map(|lane| Value::from_fraction(data.fraction_or_nil(lane)))
                .collect();
        }
        let outer = shape[0];
        let rest = &shape[1..];
        let stride: usize = rest.iter().product();
        let mut out = Vec::with_capacity(outer);
        for i in 0..outer {
            let inner = build(data, rest, offset + i * stride);
            out.push(Value::from_children(inner));
        }
        out
    }
    build(data, shape, 0)
}

#[cfg(test)]
mod vtu_tensor_tests {
    use super::*;

    #[test]
    fn tensor_and_nested_vector_compare_equal_when_flatten_matches() {
        let dense = Value::from_tensor(
            vec![
                Fraction::from(1),
                Fraction::from(2),
                Fraction::from(3),
                Fraction::from(4),
            ],
            vec![2, 2],
        );
        let nested = Value::from_children(vec![
            Value::from_children(vec![Value::from_int(1), Value::from_int(2)]),
            Value::from_children(vec![Value::from_int(3), Value::from_int(4)]),
        ]);
        assert_eq!(dense.data, nested.data);
        assert_eq!(nested.data, dense.data);
    }

    #[test]
    fn tensor_shape_matches_nested_shape() {
        let dense = Value::from_tensor(
            vec![
                Fraction::from(1),
                Fraction::from(2),
                Fraction::from(3),
                Fraction::from(4),
            ],
            vec![2, 2],
        );
        assert_eq!(dense.shape(), vec![2, 2]);
        assert_eq!(dense.count_fractions(), 4);
        assert_eq!(dense.collect_fractions_flat().len(), 4);
    }

    #[test]
    fn tensor_with_different_shape_compares_unequal_to_nested() {
        let dense = Value::from_tensor(
            vec![
                Fraction::from(1),
                Fraction::from(2),
                Fraction::from(3),
                Fraction::from(4),
            ],
            vec![4],
        );
        let nested = Value::from_children(vec![
            Value::from_children(vec![Value::from_int(1), Value::from_int(2)]),
            Value::from_children(vec![Value::from_int(3), Value::from_int(4)]),
        ]);
        assert_ne!(dense.data, nested.data);
    }

    #[test]
    fn tensor_is_vector_predicate_holds() {
        let dense = Value::from_tensor(vec![Fraction::from(1)], vec![1]);
        assert!(dense.is_vector());
        assert!(dense.is_tensor());
    }

    #[test]
    fn tensor_hydrates_to_vector_on_push_child() {
        let mut dense = Value::from_tensor(vec![Fraction::from(1), Fraction::from(2)], vec![2]);
        dense.push_child(Value::from_int(3));
        assert!(matches!(dense.data, ValueData::Vector(_)));
        assert_eq!(dense.len(), 3);
    }

    #[test]
    fn dense_tensor_uses_soa_buffers_and_full_valid_mask() {
        let dense = Value::from_tensor(
            vec![Fraction::from(1), Fraction::new(3.into(), 2.into())],
            vec![2],
        );
        let ValueData::Tensor { data, shape } = dense.data else {
            panic!("expected DenseTensor representation");
        };
        assert_eq!(&*shape, &[2]);
        assert_eq!(data.numerators, vec![1, 3]);
        assert_eq!(data.denominators, vec![1, 2]);
        assert_eq!(data.valid_mask, vec![0b11]);
        assert!(!data.is_pure_integer);
    }

    #[test]
    fn dense_tensor_invalid_lane_uses_mask_without_fraction_cache() {
        let mut tensor = DenseTensor::from_fractions(
            vec![Fraction::from(1), Fraction::from(2), Fraction::from(3)],
            vec![3],
        )
        .expect("small fractions should admit dense representation");
        tensor.clear_valid(1);

        assert_eq!(tensor.valid_mask, vec![0b101]);
        assert_eq!(tensor.get_small_fraction(0), Some(Fraction::from(1)));
        assert_eq!(tensor.get_small_fraction(1), None);
        assert!(tensor.fraction_or_nil(1).is_nil());
        assert_eq!(
            tensor.to_fractions(),
            vec![Fraction::from(1), Fraction::nil(), Fraction::from(3)]
        );
    }

    #[test]
    fn big_fraction_tensor_falls_back_without_losing_shape() {
        use num_bigint::BigInt;

        let big = Fraction::new(BigInt::from(i128::from(i64::MAX) + 1), 1.into());
        let value = Value::from_tensor(vec![big.clone()], vec![1]);
        assert!(matches!(value.data, ValueData::Vector(_)));
        assert_eq!(value.shape(), vec![1]);
        assert_eq!(value.collect_fractions_flat(), vec![big]);
    }

    // -----------------------------------------------------------------------
    // VTU Phase III boundary helpers: as_vector_view / ensure_hydrated
    // -----------------------------------------------------------------------

    #[test]
    fn as_vector_view_borrows_for_vector_owns_for_tensor() {
        use std::borrow::Cow;

        let nested = Value::from_children(vec![Value::from_int(1), Value::from_int(2)]);
        match nested.as_vector_view() {
            Some(Cow::Borrowed(slice)) => {
                assert_eq!(slice.len(), 2);
            }
            other => panic!(
                "expected Cow::Borrowed for Vector, got {:?}",
                other.is_some()
            ),
        }

        let dense = Value::from_tensor(vec![Fraction::from(1), Fraction::from(2)], vec![2]);
        match dense.as_vector_view() {
            Some(Cow::Owned(vec)) => {
                assert_eq!(vec.len(), 2);
                assert_eq!(vec[0].as_scalar().map(|f| f.to_i64().unwrap()), Some(1));
                assert_eq!(vec[1].as_scalar().map(|f| f.to_i64().unwrap()), Some(2));
            }
            other => panic!(
                "expected Cow::Owned for Tensor, got {}",
                if other.is_some() { "Borrowed" } else { "None" }
            ),
        }
    }

    #[test]
    fn as_vector_view_returns_none_for_scalar_and_nil() {
        assert!(Value::from_int(7).as_vector_view().is_none());
        assert!(Value::nil().as_vector_view().is_none());
    }

    #[test]
    fn ensure_hydrated_borrows_non_tensor_in_place() {
        use std::borrow::Cow;

        let nested = Value::from_children(vec![Value::from_int(1)]);
        match nested.ensure_hydrated() {
            Cow::Borrowed(_) => {}
            Cow::Owned(_) => panic!("Vector should not be re-allocated"),
        }

        let scalar = Value::from_int(3);
        match scalar.ensure_hydrated() {
            Cow::Borrowed(_) => {}
            Cow::Owned(_) => panic!("Scalar should be borrowed in place"),
        }
    }

    #[test]
    fn ensure_hydrated_converts_tensor_into_vector_preserving_hint() {
        use std::borrow::Cow;

        let mut dense = Value::from_tensor(
            vec![Fraction::from(1), Fraction::from(2), Fraction::from(3)],
            vec![3],
        );
        dense.hint = Interpretation::RawNumber;
        let hydrated = dense.ensure_hydrated();
        match hydrated {
            Cow::Owned(v) => {
                assert!(matches!(v.data, ValueData::Vector(_)));
                assert_eq!(v.hint, Interpretation::RawNumber);
                assert_eq!(v.len(), 3);
            }
            Cow::Borrowed(_) => panic!("Tensor should hydrate into an owned Vector"),
        }
    }
}
