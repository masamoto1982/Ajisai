//! Stack value type.
//!
//! Observable values are continued-fraction numbers, Nil, or rank-N tensors
//! whose elements are themselves continued fractions. Tensors carry an
//! optional `display_hint` that survives across the WASM boundary; the
//! shell uses it to render strings (rank-1 tensors of UTF-8 byte values)
//! distinctly from generic numeric vectors.
//!
//! The semantic-plane contract requires that the WASM boundary exposes
//! protocol fields rather than Rust enum variants, so this enum is
//! internal.

use crate::cf::ContinuedFraction;
use num_traits::ToPrimitive;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Number(ContinuedFraction),
    Nil,
    Tensor {
        shape: Vec<usize>,
        data: Vec<ContinuedFraction>,
        display_hint: Option<String>,
    },
}

impl Value {
    pub fn is_nil(&self) -> bool {
        match self {
            Value::Nil => true,
            Value::Number(cf) => cf.is_nil(),
            Value::Tensor { .. } => false,
        }
    }

    pub fn nested_display(&self) -> String {
        match self {
            Value::Nil => "Nil".to_string(),
            Value::Number(cf) => cf.nested_display(),
            Value::Tensor { shape, data, display_hint } => {
                if display_hint.as_deref() == Some("string") && shape.len() == 1 {
                    decode_string_tensor(data).unwrap_or_else(|| format!("Tensor{:?}", shape))
                } else {
                    format!("Tensor{:?}", shape)
                }
            }
        }
    }

    pub fn rational_display(&self) -> String {
        match self {
            Value::Nil => "Nil".to_string(),
            Value::Number(cf) => cf.rational_display(),
            Value::Tensor { shape, data, display_hint } => {
                if display_hint.as_deref() == Some("string") && shape.len() == 1 {
                    if let Some(s) = decode_string_tensor(data) {
                        return format!("'{}'", escape_string_literal(&s));
                    }
                }
                format_tensor(shape, data)
            }
        }
    }
}

/// Re-escape inner `'` as `''` so the rendered form is itself a valid
/// string literal in Ajisai source.
fn escape_string_literal(s: &str) -> String {
    s.replace('\'', "''")
}

fn decode_string_tensor(data: &[ContinuedFraction]) -> Option<String> {
    let mut bytes = Vec::with_capacity(data.len());
    for cf in data {
        let (p, q) = cf.to_ratio()?;
        if q != num_bigint::BigInt::from(1u32) {
            return None;
        }
        let byte = p.to_u8()?;
        bytes.push(byte);
    }
    String::from_utf8(bytes).ok()
}

fn format_tensor(shape: &[usize], data: &[ContinuedFraction]) -> String {
    if shape.is_empty() {
        return "[]".to_string();
    }
    let parts: Vec<String> = data.iter().map(|cf| cf.rational_display()).collect();
    format!("[{}]", parts.join(" "))
}
