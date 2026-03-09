pub mod display;
pub mod fraction;
pub mod json;

use self::fraction::Fraction;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// ValueExt: Generic extension trait for module-specific value metadata
// ---------------------------------------------------------------------------

/// Trait for module-specific metadata attached to values.
/// Modules (e.g., MUSIC) can implement this to carry domain-specific
/// information through the value pipeline without coupling the core types.
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

#[derive(Debug)]
pub struct Value {
    pub data: ValueData,
    pub display_hint: DisplayHint,
    pub ext: Option<Box<dyn ValueExt>>,
}

impl Clone for Value {
    fn clone(&self) -> Self {
        Value {
            data: self.data.clone(),
            display_hint: self.display_hint,
            ext: self.ext.clone(),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data && self.display_hint == other.display_hint
    }
}

impl Value {
    #[inline]
    pub fn nil() -> Self {
        Self {
            data: ValueData::Nil,
            display_hint: DisplayHint::Nil,
            ext: None,
        }
    }

    #[inline]
    pub fn from_fraction(f: Fraction) -> Self {
        Self {
            data: ValueData::Scalar(f),
            display_hint: DisplayHint::Number,
            ext: None,
        }
    }

    #[inline]
    pub fn from_int(n: i64) -> Self {
        Self {
            data: ValueData::Scalar(Fraction::from(n)),
            display_hint: DisplayHint::Number,
            ext: None,
        }
    }

    #[inline]
    pub fn from_bool(b: bool) -> Self {
        Self {
            data: ValueData::Scalar(Fraction::from(if b { 1 } else { 0 })),
            display_hint: DisplayHint::Boolean,
            ext: None,
        }
    }

    pub fn from_string(s: &str) -> Self {
        let mut children: Vec<Value> = Vec::with_capacity(s.chars().count());
        for c in s.chars() {
            children.push(Value::from_int(c as u32 as i64));
        }

        if children.is_empty() {
            return Self {
                data: ValueData::Nil,
                display_hint: DisplayHint::String,
                ext: None,
            };
        }

        Self {
            data: ValueData::Vector(Rc::new(children)),
            display_hint: DisplayHint::String,
            ext: None,
        }
    }

    pub fn from_symbol(s: &str) -> Self {
        Self::from_string(s)
    }

    #[inline]
    pub fn from_children(children: Vec<Value>) -> Self {
        Self {
            data: ValueData::Vector(Rc::new(children)),
            display_hint: DisplayHint::Auto,
            ext: None,
        }
    }

    pub fn from_vector(values: Vec<Value>) -> Self {
        if values.is_empty() {
            return Self::nil();
        }

        Self {
            data: ValueData::Vector(Rc::new(values)),
            display_hint: DisplayHint::Auto,
            ext: None,
        }
    }

    #[inline]
    pub fn from_number(f: Fraction) -> Self {
        Self::from_fraction(f)
    }

    #[inline]
    pub fn from_datetime(f: Fraction) -> Self {
        Self {
            data: ValueData::Scalar(f),
            display_hint: DisplayHint::DateTime,
            ext: None,
        }
    }

    #[inline]
    pub fn with_hint(mut self, hint: DisplayHint) -> Self {
        self.display_hint = hint;
        self
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
        matches!(
            self.data,
            ValueData::Vector(_) | ValueData::Record { .. }
        )
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
            _ => None,
        }
    }

    pub fn get_child_mut(&mut self, index: usize) -> Option<&mut Value> {
        match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                Rc::make_mut(v).get_mut(index)
            }
            _ => None,
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
                self.display_hint = DisplayHint::Auto;
            }
            ValueData::Scalar(f) => {
                let old = Value::from_fraction(f.clone());
                self.data = ValueData::Vector(Rc::new(vec![old, child]));
                self.display_hint = DisplayHint::Auto;
            }
            ValueData::CodeBlock(_) => {}
        }
    }

    pub fn pop_child(&mut self) -> Option<Value> {
        match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                Rc::make_mut(v).pop()
            }
            _ => None,
        }
    }

    pub fn insert_child(&mut self, index: usize, child: Value) {
        let v = match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Rc::make_mut(v),
            _ => return,
        };
        if index <= v.len() {
            v.insert(index, child);
        }
    }

    pub fn remove_child(&mut self, index: usize) -> Option<Value> {
        let v = match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Rc::make_mut(v),
            _ => return None,
        };
        if index < v.len() {
            Some(v.remove(index))
        } else {
            None
        }
    }

    pub fn replace_child(&mut self, index: usize, child: Value) -> Option<Value> {
        let v = match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Rc::make_mut(v),
            _ => return None,
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
            _ => None,
        }
    }

    #[inline]
    pub fn as_scalar_mut(&mut self) -> Option<&mut Fraction> {
        match &mut self.data {
            ValueData::Scalar(f) => Some(f),
            _ => None,
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
            _ => None,
        }
    }

    #[inline]
    pub fn as_vector_mut(&mut self) -> Option<&mut Vec<Value>> {
        match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                Some(Rc::make_mut(v))
            }
            _ => None,
        }
    }

    pub fn flatten_fractions(&self) -> Vec<Fraction> {
        match &self.data {
            ValueData::Nil => vec![Fraction::nil()],
            ValueData::Scalar(f) => vec![f.clone()],
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                v.iter().flat_map(|c| c.flatten_fractions()).collect()
            }
            ValueData::CodeBlock(_) => vec![],
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
                    let first_shape = v[0].shape();
                    let all_same = v.iter().skip(1).all(|c| c.shape() == first_shape);
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
                display_hint: DisplayHint::Number,
                ext: None,
            };
        }
        Self {
            data: ValueData::Vector(Rc::new(v.into_iter().map(Value::from_fraction).collect())),
            display_hint: DisplayHint::Number,
            ext: None,
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
                display_hint: DisplayHint::Auto,
                ext: None,
            };
        }
        Self {
            data: ValueData::Vector(Rc::new(v.into_iter().map(Value::from_fraction).collect())),
            display_hint: DisplayHint::Auto,
            ext: None,
        }
    }

    #[inline]
    pub fn with_ext(mut self, ext: Box<dyn ValueExt>) -> Self {
        self.ext = Some(ext);
        self
    }

    #[inline]
    pub fn get_ext<T: ValueExt>(&self) -> Option<&T> {
        self.ext.as_ref()?.as_any().downcast_ref::<T>()
    }

    #[inline]
    pub fn get_ext_mut<T: ValueExt>(&mut self) -> Option<&mut T> {
        self.ext.as_mut()?.as_any_mut().downcast_mut::<T>()
    }

    #[inline]
    pub fn is_code_block(&self) -> bool {
        matches!(self.data, ValueData::CodeBlock(_))
    }

    #[inline]
    pub fn as_code_block(&self) -> Option<&Vec<Token>> {
        if let ValueData::CodeBlock(tokens) = &self.data {
            Some(tokens)
        } else {
            None
        }
    }

    pub fn from_code_block(tokens: Vec<Token>) -> Self {
        Self {
            data: ValueData::CodeBlock(tokens),
            display_hint: DisplayHint::Auto,
            ext: None,
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

    pub fn from_depth(depth: usize) -> Self {
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
}

pub type Stack = Vec<Value>;

pub const MAX_VISIBLE_DIMENSIONS: usize = 9;

// ---------------------------------------------------------------------------
// Fractional Dataflow: FlowToken
// ---------------------------------------------------------------------------
//
// A FlowToken tracks the UTXO-style consumption chain for a value flowing
// through the pipeline.  Each operation consumes some fraction and hands
// the remainder to the next stage.
//
// Conservation law:  total == Σ consumed_i + remaining   (at every point)
//
// The interpreter creates a FlowToken when a value enters the pipeline and
// threads it through successive operations.

use std::sync::atomic::{AtomicU64, Ordering};

static FLOW_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// A token that tracks fraction consumption through the dataflow pipeline.
///
/// Modelled after the UTXO (Unspent Transaction Output) pattern:
/// each operation consumes part of the remaining fraction, and the
/// unconsumed remainder is forwarded to the next operation.
///
/// Bifurcation: when `,,` is used, the flow mass is split into child
/// branches rather than copied.  Each child carries a fraction of the
/// parent mass (MVP: equal 1/2 : 1/2 split).
#[derive(Debug, Clone, PartialEq)]
pub struct FlowToken {
    /// Unique identifier for this flow chain
    pub id: u64,
    /// The original total entering this chain
    pub total: Fraction,
    /// The fraction still available for consumption
    pub remaining: Fraction,
    /// Display interpretation hint (carried along the flow)
    pub hint: DisplayHint,
    /// Logical shape of the flow bundle
    pub shape: Vec<usize>,
    /// If this token was created by bifurcation, the parent flow ID
    pub parent_flow_id: Option<u64>,
    /// Child flow IDs created by bifurcation from this token
    pub child_flow_ids: Vec<u64>,
    /// The mass ratio this branch received (numerator, denominator)
    pub mass_ratio: (u64, u64),
}

impl FlowToken {
    /// Create a new flow token from a `Value`, starting a fresh chain.
    pub fn from_value(value: &Value) -> Self {
        let total = Self::value_total(value);
        let shape = value.shape();
        let hint = value.display_hint;
        FlowToken {
            id: FLOW_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            total: total.clone(),
            remaining: total,
            hint,
            shape,
            parent_flow_id: None,
            child_flow_ids: Vec::new(),
            mass_ratio: (1, 1),
        }
    }

    /// Compute the "total" fraction mass of a value (sum of all scalar leaves).
    fn value_total(value: &Value) -> Fraction {
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
                    let child_total = Self::value_total(child);
                    // Use absolute values so mixed-sign vectors don't cancel out
                    acc = acc.add(&child_total.abs());
                }
                acc
            }
            ValueData::CodeBlock(_) => Fraction::from(0),
        }
    }

    /// Consume `amount` from this token, returning (consumed, remainder_token).
    ///
    /// Returns `Err(OverConsumption)` if `amount > remaining`.
    pub fn consume(&self, amount: &Fraction) -> std::result::Result<(Fraction, FlowToken), crate::error::AjisaiError> {
        if amount > &self.remaining {
            return Err(crate::error::AjisaiError::OverConsumption {
                requested: format!("{}", amount),
                remaining: format!("{}", self.remaining),
            });
        }
        let new_remaining = self.remaining.sub(amount);
        Ok((
            amount.clone(),
            FlowToken {
                id: self.id,
                total: self.total.clone(),
                remaining: new_remaining,
                hint: self.hint,
                shape: self.shape.clone(),
                parent_flow_id: self.parent_flow_id,
                child_flow_ids: self.child_flow_ids.clone(),
                mass_ratio: self.mass_ratio,
            },
        ))
    }

    /// Check that the conservation law holds for this token given a list of
    /// consumed amounts.
    pub fn verify_conservation(&self, consumed: &[Fraction]) -> std::result::Result<(), crate::error::AjisaiError> {
        let mut sum = Fraction::from(0);
        for c in consumed {
            sum = sum.add(&c.abs());
        }
        let reconstructed = sum.add(&self.remaining);
        if reconstructed != self.total {
            return Err(crate::error::AjisaiError::Custom(format!(
                "Conservation violation: total={}, Σconsumed + remaining = {}",
                self.total, reconstructed,
            )));
        }
        Ok(())
    }

    /// Assert that this token has been fully consumed (remainder == 0).
    pub fn assert_complete(&self, context: &str) -> std::result::Result<(), crate::error::AjisaiError> {
        if !self.remaining.is_zero() {
            return Err(crate::error::AjisaiError::UnconsumedLeak {
                remainder: format!("{}", self.remaining),
                context: context.to_string(),
            });
        }
        Ok(())
    }

    /// Whether this token has any remaining fraction to consume.
    pub fn is_exhausted(&self) -> bool {
        self.remaining.is_zero()
    }

    /// Bifurcate this flow into `n` child branches with equal mass distribution.
    ///
    /// Each child receives `remaining / n` of the parent's remaining mass.
    /// The parent token is updated to record the child flow IDs and its
    /// remaining mass becomes zero (fully distributed to children).
    ///
    /// Returns `(updated_parent, Vec<child_tokens>)`.
    pub fn bifurcate(&self, n: usize) -> std::result::Result<(FlowToken, Vec<FlowToken>), crate::error::AjisaiError> {
        if n == 0 {
            return Err(crate::error::AjisaiError::Custom(
                "Bifurcation requires at least 1 branch".to_string(),
            ));
        }
        if self.remaining.is_zero() {
            let children: Vec<FlowToken> = (0..n)
                .map(|_| {
                    FlowToken {
                        id: FLOW_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
                        total: Fraction::from(0),
                        remaining: Fraction::from(0),
                        hint: self.hint,
                        shape: self.shape.clone(),
                        parent_flow_id: Some(self.id),
                        child_flow_ids: Vec::new(),
                        mass_ratio: (1, n as u64),
                    }
                })
                .collect();
            let child_ids: Vec<u64> = children.iter().map(|c| c.id).collect();
            let parent = FlowToken {
                id: self.id,
                total: self.total.clone(),
                remaining: Fraction::from(0),
                hint: self.hint,
                shape: self.shape.clone(),
                parent_flow_id: self.parent_flow_id,
                child_flow_ids: child_ids,
                mass_ratio: self.mass_ratio,
            };
            return Ok((parent, children));
        }

        let denom = Fraction::from(n as i64);
        let child_mass = self.remaining.div(&denom);

        let mut children = Vec::with_capacity(n);
        let mut child_ids = Vec::with_capacity(n);

        for _ in 0..n {
            let child_id = FLOW_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
            child_ids.push(child_id);
            children.push(FlowToken {
                id: child_id,
                total: child_mass.clone(),
                remaining: child_mass.clone(),
                hint: self.hint,
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
            hint: self.hint,
            shape: self.shape.clone(),
            parent_flow_id: self.parent_flow_id,
            child_flow_ids: child_ids,
            mass_ratio: self.mass_ratio,
        };

        Ok((parent, children))
    }

    /// Verify the bifurcation conservation law: sum of child masses == parent remaining.
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

/// Result of a single operation in the consumed/remainder model.
#[derive(Debug, Clone)]
pub struct FlowResult {
    /// The output value produced by the operation
    pub output: Value,
    /// The remainder token after consumption
    pub remainder: FlowToken,
    /// How much was consumed in this step
    pub consumed: Fraction,
}
