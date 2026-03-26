pub mod display;
pub mod fraction;
pub mod json;

use self::fraction::Fraction;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

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
    String,
    Boolean,
    DateTime,
    Nil,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValueData {
    Scalar(Fraction),
    Vector(Rc<Vec<Value>>),
    Record {
        pairs: Rc<Vec<Value>>,
        index: HashMap<String, usize>,
    },
    Nil,
    CodeBlock(Vec<Token>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    pub data: ValueData,
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
        Self {
            stack_hints: Vec::new(),
            flow_hints: HashMap::new(),
            flow_extensions: HashMap::new(),
        }
    }

    #[inline]
    pub fn push_hint(&mut self, hint: DisplayHint) {
        self.stack_hints.push(hint);
    }

    #[inline]
    pub fn pop_hint(&mut self) -> DisplayHint {
        self.stack_hints.pop().unwrap_or(DisplayHint::Auto)
    }

    #[inline]
    pub fn lookup_hint_at(&self, index: usize) -> DisplayHint {
        self.stack_hints
            .get(index)
            .copied()
            .unwrap_or(DisplayHint::Auto)
    }

    #[inline]
    pub fn update_hint_at(&mut self, index: usize, hint: DisplayHint) {
        if index < self.stack_hints.len() {
            self.stack_hints[index] = hint;
        }
    }

    #[inline]
    pub fn lookup_last_hint(&self) -> DisplayHint {
        self.stack_hints
            .last()
            .copied()
            .unwrap_or(DisplayHint::Auto)
    }

    #[inline]
    pub fn truncate(&mut self, len: usize) {
        self.stack_hints.truncate(len);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.stack_hints.clear();
    }

    #[inline]
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
        let start: usize = self.stack_hints.len().saturating_sub(count);
        let hints: Vec<DisplayHint> = self.stack_hints.drain(start..).collect();
        hints
    }

    pub fn extend_hints(&mut self, hints: impl IntoIterator<Item = DisplayHint>) {
        self.stack_hints.extend(hints);
    }

    pub fn insert_hint(&mut self, index: usize, hint: DisplayHint) {
        if index <= self.stack_hints.len() {
            self.stack_hints.insert(index, hint);
        }
    }

    pub fn remove_hint(&mut self, index: usize) -> DisplayHint {
        if index < self.stack_hints.len() {
            self.stack_hints.remove(index)
        } else {
            DisplayHint::Auto
        }
    }
}

impl Value {
    #[inline]
    pub fn nil() -> Self {
        Self {
            data: ValueData::Nil,
        }
    }

    #[inline]
    pub fn from_fraction(f: Fraction) -> Self {
        Self {
            data: ValueData::Scalar(f),
        }
    }

    #[inline]
    pub fn from_int(n: i64) -> Self {
        Self {
            data: ValueData::Scalar(Fraction::from(n)),
        }
    }

    #[inline]
    pub fn from_bool(b: bool) -> Self {
        Self {
            data: ValueData::Scalar(Fraction::from(if b { 1 } else { 0 })),
        }
    }

    pub fn from_string(s: &str) -> Self {
        let mut children: Vec<Value> = Vec::with_capacity(s.chars().count());
        for c in s.chars() {
            children.push(Value::from_int(c as u32 as i64));
        }
        if children.is_empty() {
            return Self::nil();
        }
        Self {
            data: ValueData::Vector(Rc::new(children)),
        }
    }

    pub fn from_symbol(s: &str) -> Self {
        Self::from_string(s)
    }

    #[inline]
    pub fn from_children(children: Vec<Value>) -> Self {
        Self {
            data: ValueData::Vector(Rc::new(children)),
        }
    }

    pub fn from_vector(values: Vec<Value>) -> Self {
        if values.is_empty() {
            return Self::nil();
        }
        Self {
            data: ValueData::Vector(Rc::new(values)),
        }
    }

    #[inline]
    pub fn from_number(f: Fraction) -> Self {
        Self::from_fraction(f)
    }

    #[inline]
    pub fn from_datetime(f: Fraction) -> Self {
        Self::from_fraction(f)
    }

    #[inline]
    pub fn is_nil(&self) -> bool {
        matches!(self.data, ValueData::Nil)
    }

    #[inline]
    pub fn is_scalar(&self) -> bool {
        matches!(self.data, ValueData::Scalar(_))
    }

    #[inline]
    pub fn is_vector(&self) -> bool {
        matches!(self.data, ValueData::Vector(_) | ValueData::Record { .. })
    }

    /// Linear-consumption optimization: check whether this value's underlying
    /// allocation is uniquely owned (Rc strong_count == 1 for Vector/Record).
    /// Scalars and Nil are always "uniquely owned" (no shared heap allocation).
    /// CodeBlocks are treated as not uniquely owned (mutation is not meaningful).
    #[inline]
    pub fn is_uniquely_owned(&self) -> bool {
        match &self.data {
            ValueData::Scalar(_) | ValueData::Nil => true,
            ValueData::Vector(rc) => Rc::strong_count(rc) == 1,
            ValueData::Record { pairs, .. } => Rc::strong_count(pairs) == 1,
            ValueData::CodeBlock(_) => false,
        }
    }

    #[inline]
    pub fn is_truthy(&self) -> bool {
        match &self.data {
            ValueData::Nil => false,
            ValueData::Scalar(f) => !f.is_zero() && !f.is_nil(),
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                !v.is_empty() && !v.iter().all(|c| !c.is_truthy())
            }
            ValueData::CodeBlock(_) => true,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        match &self.data {
            ValueData::Nil => 0,
            ValueData::Scalar(_) => 1,
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => v.len(),
            ValueData::CodeBlock(tokens) => tokens.len(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_child(&self, index: usize) -> Option<&Value> {
        match &self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => v.get(index),
            ValueData::Scalar(_) if index == 0 => Some(self),
            ValueData::Scalar(_) | ValueData::Nil | ValueData::CodeBlock(_) => None,
        }
    }

    pub fn get_child_mut(&mut self, index: usize) -> Option<&mut Value> {
        match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                Rc::make_mut(v).get_mut(index)
            }
            ValueData::Scalar(_) | ValueData::Nil | ValueData::CodeBlock(_) => None,
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
            ValueData::Scalar(_) => Some(self),
            ValueData::Nil => None,
            ValueData::CodeBlock(_) => None,
        }
    }

    pub fn push_child(&mut self, child: Value) {
        match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                Rc::make_mut(v).push(child);
            }
            ValueData::Nil => {
                self.data = ValueData::Vector(Rc::new(vec![child]));
            }
            ValueData::Scalar(f) => {
                let old = Value::from_fraction(f.clone());
                self.data = ValueData::Vector(Rc::new(vec![old, child]));
            }
            ValueData::CodeBlock(_) => {}
        }
    }

    pub fn pop_child(&mut self) -> Option<Value> {
        match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Rc::make_mut(v).pop(),
            ValueData::Scalar(_) | ValueData::Nil | ValueData::CodeBlock(_) => None,
        }
    }

    pub fn insert_child(&mut self, index: usize, child: Value) {
        let v: &mut Vec<Value> = match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Rc::make_mut(v),
            ValueData::Scalar(_) | ValueData::Nil | ValueData::CodeBlock(_) => return,
        };
        if index <= v.len() {
            v.insert(index, child);
        }
    }

    pub fn remove_child(&mut self, index: usize) -> Option<Value> {
        let v: &mut Vec<Value> = match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Rc::make_mut(v),
            ValueData::Scalar(_) | ValueData::Nil | ValueData::CodeBlock(_) => return None,
        };
        if index < v.len() {
            Some(v.remove(index))
        } else {
            None
        }
    }

    pub fn replace_child(&mut self, index: usize, child: Value) -> Option<Value> {
        let v: &mut Vec<Value> = match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Rc::make_mut(v),
            ValueData::Scalar(_) | ValueData::Nil | ValueData::CodeBlock(_) => return None,
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
            ValueData::Vector(_) | ValueData::Record { .. } | ValueData::Nil | ValueData::CodeBlock(_) => None,
        }
    }

    #[inline]
    pub fn as_scalar_mut(&mut self) -> Option<&mut Fraction> {
        match &mut self.data {
            ValueData::Scalar(f) => Some(f),
            ValueData::Vector(_) | ValueData::Record { .. } | ValueData::Nil | ValueData::CodeBlock(_) => None,
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
            ValueData::Scalar(_) | ValueData::Nil | ValueData::CodeBlock(_) => None,
        }
    }

    #[inline]
    pub fn as_vector_mut(&mut self) -> Option<&mut Vec<Value>> {
        match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Some(Rc::make_mut(v)),
            ValueData::Scalar(_) | ValueData::Nil | ValueData::CodeBlock(_) => None,
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
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                for child in v.iter() {
                    child.collect_fractions_flat_into(buf);
                }
            }
            ValueData::CodeBlock(_) => {}
        }
    }

    pub fn count_fractions(&self) -> usize {
        match &self.data {
            ValueData::Nil => 1,
            ValueData::Scalar(_) => 1,
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                v.iter().map(|c| c.count_fractions()).sum()
            }
            ValueData::CodeBlock(_) => 0,
        }
    }

    pub fn shape(&self) -> Vec<usize> {
        match &self.data {
            ValueData::Nil => vec![],
            ValueData::Scalar(_) => vec![],
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
            ValueData::CodeBlock(_) => vec![],
        }
    }

    #[inline]
    pub fn from_numbers(v: Vec<Fraction>) -> Self {
        if v.is_empty() {
            return Self::nil();
        }
        if v.len() == 1 {
            return Self {
                data: ValueData::Scalar(v[0].clone()),
            };
        }
        Self {
            data: ValueData::Vector(Rc::new(v.into_iter().map(Value::from_fraction).collect::<Vec<Value>>())),
        }
    }

    #[inline]
    pub fn from_vec(v: Vec<Fraction>) -> Self {
        if v.is_empty() {
            return Self::nil();
        }
        if v.len() == 1 {
            return Self {
                data: ValueData::Scalar(v[0].clone()),
            };
        }
        Self {
            data: ValueData::Vector(Rc::new(v.into_iter().map(Value::from_fraction).collect::<Vec<Value>>())),
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
        }
    }

    pub fn resolve_default_hint(&self) -> DisplayHint {
        match &self.data {
            ValueData::Nil => DisplayHint::Nil,
            ValueData::Scalar(_) => DisplayHint::Number,
            ValueData::Vector(_) | ValueData::Record { .. } => DisplayHint::Auto,
            ValueData::CodeBlock(_) => DisplayHint::Auto,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(Arc<str>),
    String(Arc<str>),
    Symbol(Arc<str>),
    VectorStart,
    VectorEnd,
    CodeBlockStart,
    CodeBlockEnd,
    ChevronBranch,
    ChevronDefault,
    Pipeline,
    NilCoalesce,
    SafeMode,
    LineBreak,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BracketType {
    Square,
    Curly,
    Round,
}

impl BracketType {
    pub fn opening_char(&self) -> char {
        match self {
            BracketType::Square => '[',
            BracketType::Curly => '{',
            BracketType::Round => '(',
        }
    }
    pub fn closing_char(&self) -> char {
        match self {
            BracketType::Square => ']',
            BracketType::Curly => '}',
            BracketType::Round => ')',
        }
    }

    pub fn resolve_from_depth(depth: usize) -> Self {
        match depth % 3 {
            0 => BracketType::Curly,
            1 => BracketType::Round,
            2 => BracketType::Square,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionLine {
    pub body_tokens: Arc<[Token]>,
}

#[derive(Debug, Clone)]
pub struct WordDefinition {
    pub lines: Arc<[ExecutionLine]>,
    pub is_builtin: bool,
    pub description: Option<String>,
    pub dependencies: HashSet<String>,
    pub original_source: Option<String>,
    pub namespace: Option<String>,
    pub registration_order: u64,
}

pub type Stack = Vec<Value>;

pub const MAX_VISIBLE_DIMENSIONS: usize = 9;

use std::sync::atomic::{AtomicU64, Ordering};

static FLOW_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq)]
pub struct FlowToken {
    pub id: u64,
    pub total: Fraction,
    pub remaining: Fraction,
    pub shape: Vec<usize>,
    pub parent_flow_id: Option<u64>,
    pub child_flow_ids: Vec<u64>,
    pub mass_ratio: (u64, u64),
}

impl FlowToken {
    pub fn from_value(value: &Value) -> Self {
        let total: Fraction = Self::compute_value_total(value);
        let shape: Vec<usize> = value.shape();
        FlowToken {
            id: FLOW_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            total: total.clone(),
            remaining: total,
            shape,
            parent_flow_id: None,
            child_flow_ids: Vec::new(),
            mass_ratio: (1, 1),
        }
    }

    fn compute_value_total(value: &Value) -> Fraction {
        match &value.data {
            ValueData::Nil => Fraction::from(0),
            ValueData::Scalar(f) => {
                if f.is_nil() {
                    Fraction::from(0)
                } else {
                    f.clone()
                }
            }
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                let mut acc = Fraction::from(0);
                for child in v.iter() {
                    let child_total: Fraction = Self::compute_value_total(child);
                    // Use absolute values so mixed-sign vectors don't cancel out
                    acc = acc.add(&child_total.abs());
                }
                acc
            }
            ValueData::CodeBlock(_) => Fraction::from(0),
        }
    }

    pub fn consume(
        &self,
        amount: &Fraction,
    ) -> std::result::Result<(Fraction, FlowToken), crate::error::AjisaiError> {
        if amount > &self.remaining {
            return Err(crate::error::AjisaiError::OverConsumption {
                requested: format!("{}", amount),
                remaining: format!("{}", self.remaining),
            });
        }
        let new_remaining: Fraction = self.remaining.sub(amount);
        Ok((
            amount.clone(),
            FlowToken {
                id: self.id,
                total: self.total.clone(),
                remaining: new_remaining,
                shape: self.shape.clone(),
                parent_flow_id: self.parent_flow_id,
                child_flow_ids: self.child_flow_ids.clone(),
                mass_ratio: self.mass_ratio,
            },
        ))
    }

    pub fn verify_conservation(
        &self,
        consumed: &[Fraction],
    ) -> std::result::Result<(), crate::error::AjisaiError> {
        let mut sum = Fraction::from(0);
        for c in consumed {
            sum = sum.add(&c.abs());
        }
        let reconstructed: Fraction = sum.add(&self.remaining);
        if reconstructed != self.total {
            return Err(crate::error::AjisaiError::Custom(format!(
                "Conservation violation: total={}, Σconsumed + remaining = {}",
                self.total, reconstructed,
            )));
        }
        Ok(())
    }

    pub fn assert_complete(
        &self,
        context: &str,
    ) -> std::result::Result<(), crate::error::AjisaiError> {
        if !self.remaining.is_zero() {
            return Err(crate::error::AjisaiError::UnconsumedLeak {
                remainder: format!("{}", self.remaining),
                context: context.to_string(),
            });
        }
        Ok(())
    }

    pub fn is_exhausted(&self) -> bool {
        self.remaining.is_zero()
    }

    pub fn is_reusable_allocation(&self) -> bool {
        self.remaining == self.total
            && self.parent_flow_id.is_none()
            && self.child_flow_ids.is_empty()
            && self.mass_ratio == (1, 1)
    }

    /// Combined linear-consumption optimization hook: flow-level AND value-level.
    ///
    /// Returns true when BOTH conditions hold:
    /// 1. The flow token is reusable (`remaining == total`, no parent/children, full mass ratio)
    /// 2. The value's underlying allocation has no aliases (Rc strong_count == 1)
    ///
    /// When this returns true, an operator MAY safely perform in-place mutation
    /// on the value's data buffer instead of allocating a new one.
    /// This is a judgment API — it does NOT change execution semantics.
    #[inline]
    pub fn can_update_in_place(&self, value: &Value) -> bool {
        self.is_reusable_allocation() && value.is_uniquely_owned()
    }

    /// Bifurcate this flow into `n` child branches with equal mass distribution.
    ///
    /// Each child receives `remaining / n` of the parent's remaining mass.
    /// The parent token is updated to record the child flow IDs and its
    /// remaining mass becomes zero (fully distributed to children).
    ///
    /// Returns `(updated_parent, Vec<child_tokens>)`.
    pub fn bifurcate(
        &self,
        n: usize,
    ) -> std::result::Result<(FlowToken, Vec<FlowToken>), crate::error::AjisaiError> {
        if n == 0 {
            return Err(crate::error::AjisaiError::Custom(
                "Bifurcation requires at least 1 branch".to_string(),
            ));
        }
        if self.remaining.is_zero() {
            let children: Vec<FlowToken> = (0..n)
                .map(|_| FlowToken {
                    id: FLOW_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
                    total: Fraction::from(0),
                    remaining: Fraction::from(0),
                    shape: self.shape.clone(),
                    parent_flow_id: Some(self.id),
                    child_flow_ids: Vec::new(),
                    mass_ratio: (1, n as u64),
                })
                .collect();
            let child_ids: Vec<u64> = children.iter().map(|c| c.id).collect();
            let parent = FlowToken {
                id: self.id,
                total: self.total.clone(),
                remaining: Fraction::from(0),
                shape: self.shape.clone(),
                parent_flow_id: self.parent_flow_id,
                child_flow_ids: child_ids,
                mass_ratio: self.mass_ratio,
            };
            return Ok((parent, children));
        }

        let denom: Fraction = Fraction::from(n as i64);
        let child_mass: Fraction = self.remaining.div(&denom);

        let mut children = Vec::with_capacity(n);
        let mut child_ids = Vec::with_capacity(n);

        for _ in 0..n {
            let child_id: u64 = FLOW_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
            child_ids.push(child_id);
            children.push(FlowToken {
                id: child_id,
                total: child_mass.clone(),
                remaining: child_mass.clone(),
                shape: self.shape.clone(),
                parent_flow_id: Some(self.id),
                child_flow_ids: Vec::new(),
                mass_ratio: (1, n as u64),
            });
        }

        let parent = FlowToken {
            id: self.id,
            total: self.total.clone(),
            remaining: Fraction::from(0),
            shape: self.shape.clone(),
            parent_flow_id: self.parent_flow_id,
            child_flow_ids: child_ids,
            mass_ratio: self.mass_ratio,
        };

        Ok((parent, children))
    }

    pub fn verify_bifurcation_conservation(
        parent_remaining: &Fraction,
        children: &[FlowToken],
    ) -> std::result::Result<(), crate::error::AjisaiError> {
        let mut sum = Fraction::from(0);
        for child in children {
            sum = sum.add(&child.total);
        }
        if &sum != parent_remaining {
            return Err(crate::error::AjisaiError::Custom(format!(
                "Bifurcation conservation violation: parent remaining={}, sum of children={}",
                parent_remaining, sum,
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FlowResult {
    pub output: Value,
    pub remainder: FlowToken,
    pub consumed: Fraction,
}
