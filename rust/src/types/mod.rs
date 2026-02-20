pub mod fraction;
pub mod display;

use std::collections::HashSet;
use self::fraction::Fraction;
use serde::Serialize;

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
    Vector(Vec<Value>),
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
        let children: Vec<Value> = s.chars()
            .map(|c| Value::from_int(c as u32 as i64))
            .collect();

        if children.is_empty() {
            return Self {
                data: ValueData::Nil,
                display_hint: DisplayHint::String,
                audio_hint: None,
            };
        }

        Self {
            data: ValueData::Vector(children),
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
            data: ValueData::Vector(children),
            display_hint: DisplayHint::Auto,
            audio_hint: None,
        }
    }

    pub fn from_vector(values: Vec<Value>) -> Self {
        if values.is_empty() {
            return Self::nil();
        }

        Self {
            data: ValueData::Vector(values),
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
        matches!(self.data, ValueData::Vector(_))
    }

    #[inline]
    pub fn is_truthy(&self) -> bool {
        match &self.data {
            ValueData::Nil => false,
            ValueData::Scalar(f) => !f.is_zero() && !f.is_nil(),
            ValueData::Vector(v) => !v.is_empty() && !v.iter().all(|c| !c.is_truthy()),
            ValueData::CodeBlock(_) => true,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        match &self.data {
            ValueData::Nil => 0,
            ValueData::Scalar(_) => 1,
            ValueData::Vector(v) => v.len(),
            ValueData::CodeBlock(tokens) => tokens.len(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_child(&self, index: usize) -> Option<&Value> {
        match &self.data {
            ValueData::Vector(v) => v.get(index),
            ValueData::Scalar(_) if index == 0 => Some(self),
            _ => None,
        }
    }

    pub fn get_child_mut(&mut self, index: usize) -> Option<&mut Value> {
        match &mut self.data {
            ValueData::Vector(v) => v.get_mut(index),
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
            ValueData::Vector(v) => v.last(),
            ValueData::Scalar(_) => Some(self),
            ValueData::Nil => None,
            ValueData::CodeBlock(_) => None,
        }
    }

    pub fn push_child(&mut self, child: Value) {
        match &mut self.data {
            ValueData::Vector(v) => v.push(child),
            ValueData::Nil => {
                self.data = ValueData::Vector(vec![child]);
                self.display_hint = DisplayHint::Auto;
            }
            ValueData::Scalar(f) => {
                let old = Value::from_fraction(f.clone());
                self.data = ValueData::Vector(vec![old, child]);
                self.display_hint = DisplayHint::Auto;
            }
            ValueData::CodeBlock(_) => {}
        }
    }

    pub fn pop_child(&mut self) -> Option<Value> {
        match &mut self.data {
            ValueData::Vector(v) => v.pop(),
            _ => None,
        }
    }

    pub fn insert_child(&mut self, index: usize, child: Value) {
        if let ValueData::Vector(v) = &mut self.data {
            if index <= v.len() {
                v.insert(index, child);
            }
        }
    }

    pub fn remove_child(&mut self, index: usize) -> Option<Value> {
        if let ValueData::Vector(v) = &mut self.data {
            if index < v.len() {
                return Some(v.remove(index));
            }
        }
        None
    }

    pub fn replace_child(&mut self, index: usize, child: Value) -> Option<Value> {
        if let ValueData::Vector(v) = &mut self.data {
            if index < v.len() {
                return Some(std::mem::replace(&mut v[index], child));
            }
        }
        None
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
            ValueData::Vector(v) => Some(v),
            _ => None,
        }
    }

    #[inline]
    pub fn as_vector_mut(&mut self) -> Option<&mut Vec<Value>> {
        match &mut self.data {
            ValueData::Vector(v) => Some(v),
            _ => None,
        }
    }

    pub fn flatten_fractions(&self) -> Vec<Fraction> {
        match &self.data {
            ValueData::Nil => vec![Fraction::nil()],
            ValueData::Scalar(f) => vec![f.clone()],
            ValueData::Vector(v) => {
                v.iter().flat_map(|c| c.flatten_fractions()).collect()
            }
            ValueData::CodeBlock(_) => vec![],
        }
    }

    pub fn shape(&self) -> Vec<usize> {
        match &self.data {
            ValueData::Nil => vec![],
            ValueData::Scalar(_) => vec![],
            ValueData::Vector(v) => {
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
            data: ValueData::Vector(v.into_iter().map(Value::from_fraction).collect()),
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
            data: ValueData::Vector(v.into_iter().map(Value::from_fraction).collect()),
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
    Number(String),
    String(String),
    Symbol(String),
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
    pub body_tokens: Vec<Token>,
}

#[derive(Debug, Clone)]
pub struct WordDefinition {
    pub lines: Vec<ExecutionLine>,
    pub is_builtin: bool,
    pub description: Option<String>,
    pub dependencies: HashSet<String>,
    pub original_source: Option<String>,
}

pub type Stack = Vec<Value>;

pub const MAX_VISIBLE_DIMENSIONS: usize = 9;
