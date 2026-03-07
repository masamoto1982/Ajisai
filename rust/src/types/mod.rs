pub mod display;
pub mod fraction;
pub mod json;

use self::fraction::Fraction;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WaveformType {
    #[default]
    Sine,
    Square,
    Sawtooth,
    Triangle,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Envelope {
    pub attack: f64,
    pub decay: f64,
    pub sustain: f64,
    pub release: f64,
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.0,
            sustain: 1.0,
            release: 0.01,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AudioHint {
    pub chord: bool,
    pub envelope: Option<Envelope>,
    pub waveform: WaveformType,
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
    JsonObject {
        pairs: Rc<Vec<Value>>,
        index: HashMap<String, usize>,
    },
    Nil,
    CodeBlock(Vec<Token>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    pub data: ValueData,
    pub display_hint: DisplayHint,
    pub audio_hint: Option<AudioHint>,
}

impl Value {
    #[inline]
    pub fn nil() -> Self {
        Self {
            data: ValueData::Nil,
            display_hint: DisplayHint::Nil,
            audio_hint: None,
        }
    }

    #[inline]
    pub fn from_fraction(f: Fraction) -> Self {
        Self {
            data: ValueData::Scalar(f),
            display_hint: DisplayHint::Number,
            audio_hint: None,
        }
    }

    #[inline]
    pub fn from_int(n: i64) -> Self {
        Self {
            data: ValueData::Scalar(Fraction::from(n)),
            display_hint: DisplayHint::Number,
            audio_hint: None,
        }
    }

    #[inline]
    pub fn from_bool(b: bool) -> Self {
        Self {
            data: ValueData::Scalar(Fraction::from(if b { 1 } else { 0 })),
            display_hint: DisplayHint::Boolean,
            audio_hint: None,
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
                audio_hint: None,
            };
        }

        Self {
            data: ValueData::Vector(Rc::new(children)),
            display_hint: DisplayHint::String,
            audio_hint: None,
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
            audio_hint: None,
        }
    }

    pub fn from_vector(values: Vec<Value>) -> Self {
        if values.is_empty() {
            return Self::nil();
        }

        Self {
            data: ValueData::Vector(Rc::new(values)),
            display_hint: DisplayHint::Auto,
            audio_hint: None,
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
            audio_hint: None,
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
            ValueData::Vector(_) | ValueData::JsonObject { .. }
        )
    }

    #[inline]
    pub fn is_truthy(&self) -> bool {
        match &self.data {
            ValueData::Nil => false,
            ValueData::Scalar(f) => !f.is_zero() && !f.is_nil(),
            ValueData::Vector(v) | ValueData::JsonObject { pairs: v, .. } => {
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
            ValueData::Vector(v) | ValueData::JsonObject { pairs: v, .. } => v.len(),
            ValueData::CodeBlock(tokens) => tokens.len(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_child(&self, index: usize) -> Option<&Value> {
        match &self.data {
            ValueData::Vector(v) | ValueData::JsonObject { pairs: v, .. } => v.get(index),
            ValueData::Scalar(_) if index == 0 => Some(self),
            _ => None,
        }
    }

    pub fn get_child_mut(&mut self, index: usize) -> Option<&mut Value> {
        match &mut self.data {
            ValueData::Vector(v) | ValueData::JsonObject { pairs: v, .. } => {
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
            ValueData::Vector(v) | ValueData::JsonObject { pairs: v, .. } => v.last(),
            ValueData::Scalar(_) => Some(self),
            ValueData::Nil => None,
            ValueData::CodeBlock(_) => None,
        }
    }

    pub fn push_child(&mut self, child: Value) {
        match &mut self.data {
            ValueData::Vector(v) | ValueData::JsonObject { pairs: v, .. } => {
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
            ValueData::Vector(v) | ValueData::JsonObject { pairs: v, .. } => {
                Rc::make_mut(v).pop()
            }
            _ => None,
        }
    }

    pub fn insert_child(&mut self, index: usize, child: Value) {
        let v = match &mut self.data {
            ValueData::Vector(v) | ValueData::JsonObject { pairs: v, .. } => Rc::make_mut(v),
            _ => return,
        };
        if index <= v.len() {
            v.insert(index, child);
        }
    }

    pub fn remove_child(&mut self, index: usize) -> Option<Value> {
        let v = match &mut self.data {
            ValueData::Vector(v) | ValueData::JsonObject { pairs: v, .. } => Rc::make_mut(v),
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
            ValueData::Vector(v) | ValueData::JsonObject { pairs: v, .. } => Rc::make_mut(v),
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
            ValueData::Vector(v) | ValueData::JsonObject { pairs: v, .. } => Some(v),
            _ => None,
        }
    }

    #[inline]
    pub fn as_vector_mut(&mut self) -> Option<&mut Vec<Value>> {
        match &mut self.data {
            ValueData::Vector(v) | ValueData::JsonObject { pairs: v, .. } => {
                Some(Rc::make_mut(v))
            }
            _ => None,
        }
    }

    pub fn flatten_fractions(&self) -> Vec<Fraction> {
        match &self.data {
            ValueData::Nil => vec![Fraction::nil()],
            ValueData::Scalar(f) => vec![f.clone()],
            ValueData::Vector(v) | ValueData::JsonObject { pairs: v, .. } => {
                v.iter().flat_map(|c| c.flatten_fractions()).collect()
            }
            ValueData::CodeBlock(_) => vec![],
        }
    }

    pub fn shape(&self) -> Vec<usize> {
        match &self.data {
            ValueData::Nil => vec![],
            ValueData::Scalar(_) => vec![],
            ValueData::Vector(v) | ValueData::JsonObject { pairs: v, .. } => {
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
                audio_hint: None,
            };
        }
        Self {
            data: ValueData::Vector(Rc::new(v.into_iter().map(Value::from_fraction).collect())),
            display_hint: DisplayHint::Number,
            audio_hint: None,
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
                audio_hint: None,
            };
        }
        Self {
            data: ValueData::Vector(Rc::new(v.into_iter().map(Value::from_fraction).collect())),
            display_hint: DisplayHint::Auto,
            audio_hint: None,
        }
    }

    #[inline]
    pub fn with_audio_hint(mut self, hint: AudioHint) -> Self {
        self.audio_hint = Some(hint);
        self
    }

    #[inline]
    pub fn get_audio_hint(&self) -> Option<&AudioHint> {
        self.audio_hint.as_ref()
    }

    #[inline]
    pub fn get_audio_hint_mut(&mut self) -> &mut Option<AudioHint> {
        &mut self.audio_hint
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
            audio_hint: None,
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
            ValueData::Vector(v) | ValueData::JsonObject { pairs: v, .. } => {
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
