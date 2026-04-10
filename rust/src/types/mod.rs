pub mod display;
pub mod fraction;
#[path = "fraction-arithmetic.rs"]
mod fraction_arithmetic;
pub mod json;
#[path = "flow-token.rs"]
pub mod flow_token;
#[path = "value-operations.rs"]
mod value_operations;

use self::fraction::Fraction;
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
