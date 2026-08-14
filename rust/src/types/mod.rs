pub mod arena;
pub(crate) mod code_data;
pub mod display;
pub mod exact;
pub mod fraction;
mod fraction_arithmetic;
#[cfg(test)]
mod fraction_mcdc_tests;
pub mod stack;
mod value_absence;
mod value_children;
mod value_semantics;
mod value_tensor;
// The lossless persistence codec is consumed only by the wasm boundary
// (`snapshot_stack` / `restore_stack_snapshot`) and by its own native property
// tests. Gating it on `any(test, feature = "wasm")` keeps a plain native build
// free of dead code while still running the round-trip tests under `cargo test`.
#[cfg(test)]
mod value_hash_tests;
#[cfg(any(test, feature = "wasm"))]
pub(crate) mod value_persist;
#[cfg(test)]
mod value_persist_tests;
pub(crate) mod value_protocol;
#[cfg(test)]
mod value_protocol_tests;

mod tensor_storage;

use self::fraction::Fraction;
pub use self::stack::Stack;
pub use self::tensor_storage::{DenseTensor, SparseTensor};
use crate::semantic::AbsenceMetadata;
use crate::types::exact::ExactReal;
use std::collections::HashSet;
use std::sync::Arc;

/// Semantic interpretation role assigned to a stack value. This is the
/// meaning the runtime attaches to a value, not a formatting switch:
/// rendering for humans and AI is derived from (data, role).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Interpretation {
    /// Role not yet assigned. Rendered structurally with no heuristic
    /// re-guessing — the runtime never infers meaning at render time.
    #[default]
    Unassigned,
    /// A plain exact-real number.
    RawNumber,
    /// A 2-element vector interpreted as a closed interval.
    Interval,
    /// A scalar interpreted as a truth value.
    TruthValue,
    /// An integer interpreted as a timestamp.
    Timestamp,
    /// A diagnostic absence value.
    Nil,
    /// Canonical AI-readable continued-fraction serialization
    /// (SPEC §4.2.3, §12.2): the nested right-associative form
    /// `( a0 ( a1 ( a2 ) ) )`, with a `...)` truncation marker for
    /// lazy irrationals. Round-trip-safe machine serialization role.
    ContinuedFraction,
}

#[derive(Debug, Clone)]
pub enum ValueData {
    /// A definite logical truth value, `true` or `false`
    /// (LANG.VALUES.TRUTH). A Boolean is a data-plane value distinct from any
    /// number: `TRUE` is not the scalar `1` and `FALSE` is not the scalar `0`,
    /// so `TRUE 1 EQ` is false.
    Boolean(bool),
    Scalar(Fraction),
    /// An exact real value backed by a continued-fraction representation
    /// (e.g. AlgebraicSqrt or a Gosper transform). Constructed only by
    /// `Value::from_exact_real`; use `as_scalar()` for the rational fast path.
    ExactScalar(ExactReal),
    Vector(Arc<Vec<Value>>),
    Tensor {
        data: Arc<DenseTensor>,
        shape: Arc<Vec<usize>>,
    },
    Nil,
    CodeBlock(Vec<Token>),
    /// A String: a sequence of Unicode scalar values, and one of the six
    /// disjoint domains of LANG.VALUES.DISJOINT.
    ///
    /// String used to be encoded as a `Vector` of codepoint `Scalar`s carrying
    /// `Interpretation::Text`, which made the domain a property of a
    /// presentation field rather than of the value. Two consequences were
    /// observable from the language: `'A' [ 65 ] EQ` answered TRUE, because
    /// the encodings were identical and only the hint differed; and
    /// `is_string_value` had to *guess* stringhood by testing whether every
    /// element happened to be a printable codepoint, which is exactly the
    /// render-time re-guessing `Interpretation` promises never to do.
    ///
    /// Holding the content directly settles both: the domain is the tag, and
    /// nothing has to be inferred from the elements.
    Text(Arc<str>),
}

impl PartialEq for ValueData {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ValueData::Boolean(a), ValueData::Boolean(b)) => a == b,
            (ValueData::Scalar(a), ValueData::Scalar(b)) => a == b,
            (ValueData::ExactScalar(a), ValueData::ExactScalar(b)) => a == b,
            (ValueData::Vector(a), ValueData::Vector(b)) => a == b,
            (
                ValueData::Tensor {
                    data: a_data,
                    shape: a_shape,
                },
                ValueData::Tensor {
                    data: b_data,
                    shape: b_shape,
                },
            ) => a_data == b_data && a_shape == b_shape,
            (ValueData::Vector(v), ValueData::Tensor { data, shape })
            | (ValueData::Tensor { data, shape }, ValueData::Vector(v)) => {
                tensor_eq_vector(data, shape, v)
            }
            (ValueData::Nil, ValueData::Nil) => true,
            (ValueData::CodeBlock(a), ValueData::CodeBlock(b)) => a == b,
            // A String equals a String with the same codepoints, and nothing
            // else. It never meets the Vector/Tensor arms above, so `'A'` and
            // `[ 65 ]` now fall to the catch-all and answer false, as
            // LANG.VALUES.DISJOINT requires.
            (ValueData::Text(a), ValueData::Text(b)) => a == b,
            _ => false,
        }
    }
}

fn tensor_eq_vector(data: &DenseTensor, shape: &[usize], v: &[Value]) -> bool {
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
        ValueData::Scalar(_) | ValueData::ExactScalar(_) | ValueData::Nil => Some(Vec::new()),
        // A String is not a numeric leaf, so it has no rectangular element
        // shape and forces the structural (non-dense) path, like a Boolean.
        ValueData::Text(_) => None,
        ValueData::Tensor { shape, .. } => Some((**shape).clone()),
        ValueData::Vector(items) => nested_vector_shape(items),
        // CS4 PR-2: U is not a dense-tensor lane. NIL is (a nil lane, via the
        // valid-mask), so it counts as a rank-0 element here; U is a truth
        // value with no numeric lane, so — like a Boolean — it has no
        // rectangular element shape and forces the structural (non-dense)
        // path.
        ValueData::Boolean(_) | ValueData::CodeBlock(_) => None,
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
                for f in inner_data.iter() {
                    if *idx >= data.len() || data.fraction_or_nil(*idx) != f {
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

/// Flatten a nested `Vector` into `(shape, leaves)` the same way
/// `nested_flatten_matches` walks it against a dense tensor's lanes:
/// `Scalar` contributes its `Fraction`, `Tensor` contributes its own dense
/// lanes, `Vector` recurses, and anything else (`ExactScalar`, `Nil`,
/// `Boolean`, `Text`, `CodeBlock`) fails the flatten — mirroring exactly
/// which leaves `nested_flatten_matches` is willing to match against a
/// tensor lane. Used only to make [`ValueData`]'s `Hash` agree with the
/// `Vector`/`Tensor` cross-equality in `PartialEq`: a value that *can*
/// equal a dense tensor must hash the way that tensor does.
fn dense_flatten(v: &[Value]) -> Option<(Vec<usize>, Vec<Fraction>)> {
    let shape = nested_vector_shape(v)?;
    let mut leaves = Vec::new();
    if collect_dense_leaves(v, &mut leaves) {
        Some((shape, leaves))
    } else {
        None
    }
}

fn collect_dense_leaves(v: &[Value], out: &mut Vec<Fraction>) -> bool {
    for child in v {
        match &child.data {
            ValueData::Scalar(f) => out.push(f.clone()),
            ValueData::Vector(inner) => {
                if !collect_dense_leaves(inner, out) {
                    return false;
                }
            }
            ValueData::Tensor { data, .. } => out.extend(data.iter()),
            _ => return false,
        }
    }
    true
}

/// Discriminant tags for [`ValueData`]'s `Hash`. `DENSE` is shared by
/// `Vector` and `Tensor` deliberately: it is what makes a rectangular
/// numeric `Vector` hash the same as the `Tensor` holding the same data,
/// matching the cross-representation equality `tensor_eq_vector` grants
/// them. `STRUCT_VECTOR` is everything else — a `Vector` that is not a
/// dense-tensor lookalike, hashed structurally instead.
const HASH_TAG_BOOLEAN: u8 = 0;
const HASH_TAG_SCALAR: u8 = 1;
const HASH_TAG_EXACT_SCALAR: u8 = 2;
const HASH_TAG_DENSE: u8 = 3;
const HASH_TAG_STRUCT_VECTOR: u8 = 4;
const HASH_TAG_NIL: u8 = 5;
const HASH_TAG_CODE_BLOCK: u8 = 6;
const HASH_TAG_TEXT: u8 = 7;

impl std::hash::Hash for ValueData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ValueData::Boolean(b) => {
                state.write_u8(HASH_TAG_BOOLEAN);
                b.hash(state);
            }
            ValueData::Scalar(f) => {
                state.write_u8(HASH_TAG_SCALAR);
                f.hash(state);
            }
            ValueData::ExactScalar(e) => {
                state.write_u8(HASH_TAG_EXACT_SCALAR);
                e.hash(state);
            }
            ValueData::Vector(v) => match dense_flatten(v) {
                Some((shape, leaves)) => {
                    state.write_u8(HASH_TAG_DENSE);
                    shape.hash(state);
                    leaves.hash(state);
                }
                None => {
                    state.write_u8(HASH_TAG_STRUCT_VECTOR);
                    v.hash(state);
                }
            },
            ValueData::Tensor { data, shape } => {
                state.write_u8(HASH_TAG_DENSE);
                shape.hash(state);
                let leaves: Vec<Fraction> = data.iter().collect();
                leaves.hash(state);
            }
            ValueData::Nil => state.write_u8(HASH_TAG_NIL),
            ValueData::CodeBlock(tokens) => {
                state.write_u8(HASH_TAG_CODE_BLOCK);
                tokens.hash(state);
            }
            ValueData::Text(s) => {
                state.write_u8(HASH_TAG_TEXT);
                s.hash(state);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Value {
    pub data: ValueData,
    pub hint: Interpretation,
    pub absence: Option<AbsenceMetadata>,
}

impl PartialEq for Value {
    /// Value equality, which must agree with the `EQ` Word: this relation is
    /// what decides nested elements once `EQ` reaches a Vector, so a
    /// disagreement between the two is observable from the language.
    ///
    /// The NIL reason is part of it. LANG.VALUES.NIL makes the reason "the
    /// entire observable content of a NIL", so two NILs are the same value
    /// exactly when their reasons agree. `ValueData::Nil` carries no reason —
    /// it lives in `absence` — so comparing `data` alone decided every pair of
    /// NILs equal, and `[ NIL ] [ 0 ] { 0 DIV } MAP EQ` answered TRUE for a
    /// `literal` absence against a `divisionByZero` one.
    ///
    /// `hint` is *not* part of it. It is a presentation role, and letting it
    /// decide a semantic question would violate LANG.STACK.ORDER. It used to
    /// be compared here for one reason only — String was encoded as a Vector
    /// of codepoints and `Interpretation::Text` was the sole discriminator —
    /// and `ValueData::Text` removed that reason by making the domain the tag.
    /// Two values with the same data and the same NIL reason are now the same
    /// value however they came to be displayed.
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data && self.nil_reason() == other.nil_reason()
    }
}

/// No value here is ever NaN-like (every numeric tier is exact), so `eq` is
/// a true equivalence relation and this marker is safe.
impl Eq for Value {}

/// Hashes exactly what `PartialEq` compares — `data`, then the NIL reason —
/// and nothing `PartialEq` does not (`hint` stays out of both). Required
/// for `UNIQUE` / `TALLY` / `GROUP` to key a `HashMap<Value, _>` rather than
/// re-scan the accumulated result for every element (CS5 collection-word
/// de-quadraticization).
impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
        self.nil_reason().hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Token {
    Number(Arc<str>),
    String(Arc<str>),
    Symbol(Arc<str>),
    VectorStart,
    VectorEnd,
    BlockStart,
    BlockEnd,
    NilCoalesce,
    CondClauseSep,
    LineBreak,
}

#[derive(Debug, Clone)]
pub struct ExecutionLine {
    pub body_tokens: Arc<[Token]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tier {
    Core,
    Standard,
    #[default]
    Contrib,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stability {
    #[default]
    Stable,
    Experimental,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    bits: u32,
}

impl Capabilities {
    pub const PURE: Self = Self { bits: 0b0000_0001 };
    pub const IO: Self = Self { bits: 0b0000_0010 };
    pub const TIME: Self = Self { bits: 0b0000_0100 };
    pub const RANDOM: Self = Self { bits: 0b0000_1000 };
    pub const CRYPTO: Self = Self { bits: 0b0001_0000 };
    pub const SPAWN: Self = Self { bits: 0b0010_0000 };
    pub const EVAL: Self = Self { bits: 0b0100_0000 };
    pub const MUTATES_DICT: Self = Self { bits: 0b1000_0000 };
    pub const INPUT_HELPER: Self = Self {
        bits: 0b0001_0000_0000,
    };

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.bits & other.bits) == other.bits
    }

    pub const fn union(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }
}

impl Default for Capabilities {
    fn default() -> Self {
        Self::PURE
    }
}

impl std::ops::BitOr for Capabilities {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            bits: self.bits | rhs.bits,
        }
    }
}

impl std::ops::BitAnd for Capabilities {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self {
            bits: self.bits & rhs.bits,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WordDefinition {
    pub lines: Arc<[ExecutionLine]>,
    pub is_builtin: bool,
    pub tier: Tier,
    pub stability: Stability,
    pub capabilities: Capabilities,
    pub description: Option<String>,
    pub dependencies: HashSet<String>,
    pub original_source: Option<String>,
    pub namespace: Option<String>,
    pub registration_order: u64,
    pub execution_plans: Option<Arc<crate::interpreter::execution_plan_set::ExecutionPlanSet>>,
}
