//! Stack value type.
//!
//! In Phase 1, every observable value is a continued-fraction number or Nil.
//! The semantic-plane contract requires that the WASM boundary exposes
//! protocol fields rather than Rust enum variants, so this enum is internal.

use crate::cf::ContinuedFraction;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Number(ContinuedFraction),
    Nil,
}

impl Value {
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil) || matches!(self, Value::Number(cf) if cf.is_nil())
    }

    pub fn nested_display(&self) -> String {
        match self {
            Value::Nil => "Nil".to_string(),
            Value::Number(cf) => cf.nested_display(),
        }
    }

    pub fn rational_display(&self) -> String {
        match self {
            Value::Nil => "Nil".to_string(),
            Value::Number(cf) => cf.rational_display(),
        }
    }
}
