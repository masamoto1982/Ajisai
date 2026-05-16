pub mod arena;
pub mod continued_fraction;
pub mod display;
pub mod fraction;
mod fraction_arithmetic;
#[cfg(test)]
mod fraction_mcdc_tests;
pub mod interval;
mod value_operations;

use self::fraction::Fraction;
use crate::error::NilReason;
use crate::semantic::AbsenceMetadata;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug, Clone, Eq)]
pub struct DenseTensor {
    pub numerators: Vec<i64>,
    pub denominators: Vec<i64>,
    pub display_sources: Vec<Option<Arc<str>>>,
    pub valid_mask: Vec<u64>,
    pub shape: Vec<usize>,
    pub is_pure_integer: bool,
}

impl PartialEq for DenseTensor {
    fn eq(&self, other: &Self) -> bool {
        self.numerators == other.numerators
            && self.denominators == other.denominators
            && self.valid_mask == other.valid_mask
            && self.shape == other.shape
            && self.is_pure_integer == other.is_pure_integer
    }
}

impl DenseTensor {
    pub fn from_fractions(data: Vec<Fraction>, shape: Vec<usize>) -> Option<Self> {
        let expected_len = if shape.is_empty() {
            0
        } else {
            shape.iter().product()
        };
        if expected_len != data.len() {
            return None;
        }

        let mut numerators = Vec::with_capacity(data.len());
        let mut denominators = Vec::with_capacity(data.len());
        let mut display_sources = Vec::with_capacity(data.len());
        let mut is_pure_integer = true;
        for fraction in data {
            let source = fraction.display_source.clone();
            let (numerator, denominator) = fraction.extract_i64_pair()?;
            numerators.push(numerator);
            denominators.push(denominator);
            display_sources.push(source);
            is_pure_integer &= denominator == 1;
        }

        let valid_mask_len = numerators.len().div_ceil(64);
        let mut valid_mask = vec![u64::MAX; valid_mask_len];
        if let Some(last) = valid_mask.last_mut() {
            let live_bits = numerators.len() % 64;
            if live_bits != 0 {
                *last = (1u64 << live_bits) - 1;
            }
        }

        Some(Self {
            numerators,
            denominators,
            display_sources,
            valid_mask,
            shape,
            is_pure_integer,
        })
    }

    pub fn len(&self) -> usize {
        self.numerators.len()
    }

    pub fn is_empty(&self) -> bool {
        self.numerators.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = Fraction> + '_ {
        (0..self.len()).map(|index| self.fraction_or_nil(index))
    }

    pub fn get_small_fraction(&self, index: usize) -> Option<Fraction> {
        if !self.is_valid(index) {
            return None;
        }
        let mut fraction = Fraction::new(
            self.numerators[index].into(),
            self.denominators[index].into(),
        );
        fraction.display_source = self.display_sources.get(index).cloned().flatten();
        Some(fraction)
    }

    pub fn fraction_or_nil(&self, index: usize) -> Fraction {
        self.get_small_fraction(index).unwrap_or_else(Fraction::nil)
    }

    pub fn to_fractions(&self) -> Vec<Fraction> {
        self.iter().collect()
    }

    pub fn clear_valid(&mut self, index: usize) {
        if index < self.len() {
            self.valid_mask[index / 64] &= !(1u64 << (index % 64));
        }
    }

    pub fn is_valid(&self, index: usize) -> bool {
        if index >= self.len() {
            return false;
        }
        let Some(word) = self.valid_mask.get(index / 64) else {
            return false;
        };
        ((word >> (index % 64)) & 1) == 1
    }

    pub fn zero_count(&self) -> usize {
        (0..self.len())
            .filter(|&index| self.is_valid(index) && self.numerators[index] == 0)
            .count()
    }

    pub fn nonzero_count(&self) -> usize {
        (0..self.len())
            .filter(|&index| self.is_valid(index) && self.numerators[index] != 0)
            .count()
    }

    pub fn density(&self) -> f64 {
        if self.is_empty() {
            return 0.0;
        }
        self.nonzero_count() as f64 / self.len() as f64
    }

    pub fn is_sparse_candidate(&self) -> bool {
        const MIN_LEN: usize = 64;
        const MAX_DENSITY: f64 = 0.25;

        self.len() >= MIN_LEN && self.density() <= MAX_DENSITY
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparseTensor {
    pub indices: Vec<usize>,
    pub numerators: Vec<i64>,
    pub denominators: Vec<i64>,
    pub display_sources: Vec<Option<Arc<str>>>,
    pub valid_mask: Vec<u64>,
    pub shape: Vec<usize>,
    pub len: usize,
    pub is_pure_integer: bool,
}

impl SparseTensor {
    pub fn from_dense(dense: &DenseTensor) -> Option<Self> {
        let expected_len = if dense.shape.is_empty() {
            dense.len()
        } else {
            dense.shape.iter().product()
        };
        if expected_len != dense.len() {
            return None;
        }
        if (0..dense.len()).any(|index| !dense.is_valid(index)) {
            return None;
        }

        let nonzero_count = dense.nonzero_count();
        let mut indices = Vec::with_capacity(nonzero_count);
        let mut numerators = Vec::with_capacity(nonzero_count);
        let mut denominators = Vec::with_capacity(nonzero_count);
        let mut display_sources = Vec::with_capacity(nonzero_count);

        for index in 0..dense.len() {
            if dense.numerators[index] != 0 {
                indices.push(index);
                numerators.push(dense.numerators[index]);
                denominators.push(dense.denominators[index]);
                display_sources.push(dense.display_sources.get(index).cloned().flatten());
            }
        }

        let valid_mask_len = dense.len().div_ceil(64);
        let mut valid_mask = vec![u64::MAX; valid_mask_len];
        if let Some(last) = valid_mask.last_mut() {
            let live_bits = dense.len() % 64;
            if live_bits != 0 {
                *last = (1u64 << live_bits) - 1;
            }
        }

        Some(Self {
            indices,
            numerators,
            denominators,
            display_sources,
            valid_mask,
            shape: dense.shape.clone(),
            len: dense.len(),
            is_pure_integer: dense.is_pure_integer,
        })
    }

    pub fn to_dense(&self) -> DenseTensor {
        let mut numerators = vec![0; self.len];
        let mut denominators = vec![1; self.len];
        let mut display_sources = vec![None; self.len];
        for (entry, &index) in self.indices.iter().enumerate() {
            if index < self.len {
                numerators[index] = self.numerators[entry];
                denominators[index] = self.denominators[entry];
                display_sources[index] = self.display_sources.get(entry).cloned().flatten();
            }
        }
        DenseTensor {
            numerators,
            denominators,
            display_sources,
            valid_mask: self.valid_mask.clone(),
            shape: self.shape.clone(),
            is_pure_integer: self.is_pure_integer,
        }
    }

    pub fn get_small_fraction(&self, index: usize) -> Option<Fraction> {
        if index >= self.len || !self.is_valid(index) {
            return None;
        }
        let entry = self.indices.binary_search(&index).ok()?;
        let mut fraction = Fraction::new(
            self.numerators[entry].into(),
            self.denominators[entry].into(),
        );
        fraction.display_source = self.display_sources.get(entry).cloned().flatten();
        Some(fraction)
    }

    pub fn fraction_or_zero(&self, index: usize) -> Fraction {
        self.get_small_fraction(index)
            .unwrap_or_else(|| Fraction::new(0.into(), 1.into()))
    }

    pub fn nonzero_count(&self) -> usize {
        self.indices.len()
    }

    pub fn density(&self) -> f64 {
        if self.len == 0 {
            return 0.0;
        }
        self.nonzero_count() as f64 / self.len as f64
    }

    pub fn is_valid(&self, index: usize) -> bool {
        if index >= self.len {
            return false;
        }
        let Some(word) = self.valid_mask.get(index / 64) else {
            return false;
        };
        ((word >> (index % 64)) & 1) == 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TensorLaneId {
    pub tensor_id: u64,
    pub lane: usize,
}

pub type NilReasonRegistry = HashMap<TensorLaneId, NilReason>;

pub trait ValueExt: std::fmt::Debug + 'static {
    fn clone_box(&self) -> Box<dyn ValueExt>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl Clone for Box<dyn ValueExt> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DisplayHint {
    #[default]
    Auto,
    Number,
    Interval,
    String,
    Boolean,
    DateTime,
    Nil,
}

#[derive(Debug, Clone)]
pub enum ValueData {
    Scalar(Fraction),
    Vector(Rc<Vec<Value>>),
    Tensor {
        data: Rc<DenseTensor>,
        shape: Rc<Vec<usize>>,
    },
    Record {
        pairs: Rc<Vec<Value>>,
        index: HashMap<String, usize>,
    },
    Nil,
    CodeBlock(Vec<Token>),
    ProcessHandle(u64),
    SupervisorHandle(u64),
}

impl PartialEq for ValueData {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ValueData::Scalar(a), ValueData::Scalar(b)) => a == b,
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
            (
                ValueData::Record {
                    pairs: ap,
                    index: ai,
                },
                ValueData::Record {
                    pairs: bp,
                    index: bi,
                },
            ) => ap == bp && ai == bi,
            (ValueData::Nil, ValueData::Nil) => true,
            (ValueData::CodeBlock(a), ValueData::CodeBlock(b)) => a == b,
            (ValueData::ProcessHandle(a), ValueData::ProcessHandle(b)) => a == b,
            (ValueData::SupervisorHandle(a), ValueData::SupervisorHandle(b)) => a == b,
            _ => false,
        }
    }
}

fn tensor_eq_vector(data: &DenseTensor, shape: &[usize], v: &[Value]) -> bool {
    let nested_shape = nested_vector_shape(v);
    if nested_shape != shape {
        return false;
    }
    let mut idx = 0usize;
    nested_flatten_matches(v, data, &mut idx) && idx == data.len()
}

fn nested_vector_shape(v: &[Value]) -> Vec<usize> {
    if v.is_empty() {
        return vec![0];
    }
    let first_shape = v[0].shape();
    let all_same = v.iter().skip(1).all(|c| c.shape() == first_shape);
    if all_same && !first_shape.is_empty() {
        let mut s = vec![v.len()];
        s.extend(first_shape);
        s
    } else {
        vec![v.len()]
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

#[derive(Debug, Clone)]
pub struct Value {
    pub data: ValueData,
    pub hint: DisplayHint,
    pub absence: Option<AbsenceMetadata>,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data && self.hint == other.hint
    }
}

pub struct SemanticRegistry {
    pub stack_hints: Vec<DisplayHint>,
    pub flow_hints: HashMap<u64, DisplayHint>,
    pub flow_extensions: HashMap<u64, Box<dyn ValueExt>>,
}

impl std::fmt::Debug for SemanticRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SemanticRegistry")
            .field("stack_hints_len", &self.stack_hints.len())
            .field("flow_hints_len", &self.flow_hints.len())
            .field("flow_extensions_len", &self.flow_extensions.len())
            .finish()
    }
}

impl SemanticRegistry {
    pub fn new() -> Self {
        SemanticRegistry {
            stack_hints: Vec::new(),
            flow_hints: HashMap::new(),
            flow_extensions: HashMap::new(),
        }
    }

    pub fn push_hint(&mut self, hint: DisplayHint) {
        self.stack_hints.push(hint);
    }

    pub fn pop_hint(&mut self) -> DisplayHint {
        self.stack_hints.pop().unwrap_or(DisplayHint::Auto)
    }

    pub fn lookup_hint_at(&self, index: usize) -> DisplayHint {
        self.stack_hints
            .get(index)
            .copied()
            .unwrap_or(DisplayHint::Auto)
    }

    pub fn update_hint_at(&mut self, index: usize, hint: DisplayHint) {
        if index < self.stack_hints.len() {
            self.stack_hints[index] = hint;
        }
    }

    pub fn lookup_last_hint(&self) -> DisplayHint {
        self.stack_hints
            .last()
            .copied()
            .unwrap_or(DisplayHint::Auto)
    }

    pub fn truncate(&mut self, len: usize) {
        self.stack_hints.truncate(len);
    }

    pub fn clear(&mut self) {
        self.stack_hints.clear();
    }

    pub fn len(&self) -> usize {
        self.stack_hints.len()
    }

    pub fn normalize_to_stack_len(&mut self, stack_len: usize) {
        while self.stack_hints.len() < stack_len {
            self.stack_hints.push(DisplayHint::Auto);
        }
        self.stack_hints.truncate(stack_len);
    }

    pub fn collect_last_hints(&mut self, count: usize) -> Vec<DisplayHint> {
        let start = self.stack_hints.len().saturating_sub(count);
        self.stack_hints.drain(start..).collect()
    }

    pub fn extend_hints(&mut self, hints: impl IntoIterator<Item = DisplayHint>) {
        self.stack_hints.extend(hints);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(Arc<str>),
    String(Arc<str>),
    Symbol(Arc<str>),
    VectorStart,
    VectorEnd,
    BlockStart,
    BlockEnd,
    Pipeline,
    NilCoalesce,
    CondClauseSep,
    SafeMode,
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

pub type Stack = Vec<Value>;

#[cfg(test)]
mod sparse_tensor_tests {
    use super::{DenseTensor, SparseTensor};
    use crate::types::fraction::Fraction;

    fn dense_from_i64(values: &[i64], shape: Vec<usize>) -> DenseTensor {
        DenseTensor::from_fractions(values.iter().copied().map(Fraction::from).collect(), shape)
            .expect("small dense tensor should build")
    }

    #[test]
    fn dense_tensor_sparse_density_counts_zero_and_nonzero_lanes() {
        let all_zero = dense_from_i64(&vec![0; 64], vec![64]);
        assert_eq!(all_zero.zero_count(), 64);
        assert_eq!(all_zero.nonzero_count(), 0);
        assert_eq!(all_zero.density(), 0.0);
        assert!(all_zero.is_sparse_candidate());

        let all_nonzero = dense_from_i64(&vec![1; 64], vec![64]);
        assert_eq!(all_nonzero.zero_count(), 0);
        assert_eq!(all_nonzero.nonzero_count(), 64);
        assert_eq!(all_nonzero.density(), 1.0);
        assert!(!all_nonzero.is_sparse_candidate());

        let mixed = dense_from_i64(&[0, 7, 0, -3], vec![4]);
        assert_eq!(mixed.zero_count(), 2);
        assert_eq!(mixed.nonzero_count(), 2);
        assert_eq!(mixed.density(), 0.5);
        assert!(!mixed.is_sparse_candidate());
    }

    #[test]
    fn dense_tensor_sparse_density_does_not_count_invalid_lanes_as_zero() {
        let mut dense = dense_from_i64(&[0, 5, 0, 9], vec![4]);
        dense.clear_valid(0);
        dense.clear_valid(1);
        assert_eq!(dense.zero_count(), 1);
        assert_eq!(dense.nonzero_count(), 1);
        assert_eq!(dense.density(), 0.25);
        assert!(SparseTensor::from_dense(&dense).is_none());
    }

    #[test]
    fn sparse_tensor_round_trips_dense_values_and_shape() {
        let dense = dense_from_i64(&[0, 0, 3, 0, -4, 0], vec![2, 3]);
        let sparse =
            SparseTensor::from_dense(&dense).expect("all-valid dense tensor is sparseable");
        assert_eq!(sparse.shape, vec![2, 3]);
        assert_eq!(sparse.len, 6);
        assert_eq!(sparse.indices, vec![2, 4]);
        assert_eq!(sparse.nonzero_count(), 2);
        assert!(sparse.indices.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(sparse.fraction_or_zero(0), Fraction::from(0_i64));
        assert_eq!(sparse.get_small_fraction(2), Some(Fraction::from(3_i64)));
        assert_eq!(sparse.to_dense(), dense);
    }

    #[test]
    fn sparse_tensor_accepts_all_zero_dense_tensor() {
        let dense = dense_from_i64(&vec![0; 64], vec![8, 8]);
        let sparse =
            SparseTensor::from_dense(&dense).expect("all-zero all-valid tensor is sparseable");
        assert!(sparse.indices.is_empty());
        assert_eq!(sparse.nonzero_count(), 0);
        assert_eq!(sparse.density(), 0.0);
        assert_eq!(sparse.to_dense(), dense);
    }
}
