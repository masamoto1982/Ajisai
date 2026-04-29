pub mod arena;
pub mod display;
#[path = "flow-token.rs"]
pub mod flow_token;
pub mod fraction;
#[path = "fraction-arithmetic.rs"]
mod fraction_arithmetic;
#[cfg(test)]
#[path = "fraction-mcdc-tests.rs"]
mod fraction_mcdc_tests;
pub mod interval;
#[path = "value-operations.rs"]
mod value_operations;

use self::fraction::Fraction;
use crate::error::NilReason;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

pub use flow_token::{FlowResult, FlowToken};

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
    ProcessHandle(u64),
    SupervisorHandle(u64),
}

#[derive(Debug, Clone)]
pub struct Value {
    pub data: ValueData,
    pub hint: DisplayHint,
    pub nil_reason: Option<NilReason>,
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
    Deprecated,
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
    pub const INPUT_HELPER: Self = Self { bits: 0b0001_0000_0000 };

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
