use crate::error::{AjisaiError, Result};
use crate::types::{Value, ValueData};
use crate::types::fraction::Fraction;
use crate::interpreter::{Interpreter, ConsumptionMode};
#[allow(unused_imports)]
use num_traits::Zero;
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};

#[inline]
pub(crate) fn is_vector_value(val: &Value) -> bool {
    matches!(&val.data, ValueData::Vector(_))
}

#[inline]
pub(crate) fn is_string_value(val: &Value) -> bool {
    // Structurally: a string is a Vector (of scalar codepoints)
    matches!(&val.data, ValueData::Vector(_))
}

pub(crate) fn value_as_string(val: &Value) -> Option<String> {
    fn collect_chars(val: &Value) -> Vec<char> {
        match &val.data {
            ValueData::Nil => vec![],
            ValueData::Scalar(f) => {
                f.to_i64().and_then(|n| {
                    if n >= 0 && n <= 0x10FFFF {
                        char::from_u32(n as u32)
                    } else {
                        None
                    }
                }).map(|c| vec![c]).unwrap_or_default()
            }
            ValueData::Vector(children) | ValueData::Record { pairs: children, .. } => {
                children.iter().flat_map(|c| collect_chars(c)).collect()
            }
            ValueData::CodeBlock(_) => vec![],
        }
    }

    let chars = collect_chars(val);
    if chars.is_empty() {
        None
    } else {
        Some(chars.into_iter().collect())
    }
}

pub(crate) fn get_integer_from_value(value: &Value) -> Result<i64> {
    match &value.data {
        ValueData::Scalar(f) => {
            if f.denominator != BigInt::one() {
                return Err(AjisaiError::structure_error("integer", "fraction"));
            }
            f.numerator.to_i64().ok_or_else(|| AjisaiError::from("Integer value is too large for i64"))
        }
        ValueData::Nil => {
            Err(AjisaiError::structure_error("single-element value with integer", "NIL"))
        }
        ValueData::Vector(children) | ValueData::Record { pairs: children, .. } if children.len() == 1 => {
            get_integer_from_value(&children[0])
        }
        ValueData::Vector(_) | ValueData::Record { .. } => {
            Err(AjisaiError::structure_error("single-element value with integer", "multi-element vector"))
        }
        ValueData::CodeBlock(_) => {
            Err(AjisaiError::structure_error("single-element value with integer", "code block"))
        }
    }
}

pub(crate) fn get_bigint_from_value(value: &Value) -> Result<BigInt> {
    match &value.data {
        ValueData::Scalar(f) => {
            if f.denominator != BigInt::one() {
                return Err(AjisaiError::structure_error("integer", "fraction"));
            }
            Ok(f.numerator.clone())
        }
        ValueData::Nil => {
            Err(AjisaiError::structure_error("single-element value with integer", "NIL"))
        }
        ValueData::Vector(children) | ValueData::Record { pairs: children, .. } if children.len() == 1 => {
            get_bigint_from_value(&children[0])
        }
        ValueData::Vector(_) | ValueData::Record { .. } => {
            Err(AjisaiError::structure_error("single-element value with integer", "multi-element vector"))
        }
        ValueData::CodeBlock(_) => {
            Err(AjisaiError::structure_error("single-element value with integer", "code block"))
        }
    }
}

pub(crate) fn get_word_name_from_value(value: &Value) -> Result<String> {
    if value.is_nil() {
        return Err(AjisaiError::from("Cannot get word name from NIL"));
    }

    let fractions = value.flatten_fractions();
    let chars: String = fractions.iter()
        .filter_map(|f| {
            f.to_i64().and_then(|n| {
                if n >= 0 && n <= 0x10FFFF {
                    char::from_u32(n as u32)
                } else {
                    None
                }
            })
        })
        .collect();

    Ok(chars.to_uppercase())
}

pub(crate) fn normalize_index(index: i64, length: usize) -> Option<usize> {
    let actual_index = if index < 0 {
        let offset = (length as i64) + index;
        if offset < 0 {
            return None;
        }
        offset as usize
    } else {
        index as usize
    };

    if actual_index < length {
        Some(actual_index)
    } else {
        None
    }
}

pub(crate) fn wrap_number(fraction: Fraction) -> Value {
    Value::from_fraction(fraction)
}

pub(crate) fn wrap_datetime(fraction: Fraction) -> Value {
    Value::from_fraction(fraction)
}

pub(crate) fn get_operands(interp: &mut Interpreter, count: usize) -> Result<Vec<Value>> {
    if interp.stack.len() < count {
        return Err(AjisaiError::StackUnderflow);
    }

    match interp.consumption_mode {
        ConsumptionMode::Consume => {
            let mut values = Vec::with_capacity(count);
            for _ in 0..count {
                values.push(interp.stack.pop().unwrap());
            }
            values.reverse();
            Ok(values)
        }
        ConsumptionMode::Keep => {
            let stack_len = interp.stack.len();
            let values: Vec<Value> = interp.stack[stack_len - count..]
                .iter()
                .cloned()
                .collect();
            Ok(values)
        }
    }
}

pub(crate) fn push_result(interp: &mut Interpreter, result: Value) {
    interp.stack.push(result);
}

// ── Fractional Dataflow helpers ──────────────────────────────────────

use crate::types::FlowToken;

/// Wrap `get_operands` with FlowToken creation when flow tracking is on.
/// Returns (operands, Option<Vec<FlowToken>>).
pub(crate) fn get_operands_with_flow(
    interp: &mut Interpreter,
    count: usize,
) -> Result<(Vec<Value>, Option<Vec<FlowToken>>)> {
    let operands = get_operands(interp, count)?;
    let tokens = if interp.flow_tracking {
        Some(operands.iter().map(|v| interp.begin_flow(v)).collect())
    } else {
        None
    };
    Ok((operands, tokens))
}

/// Push a result and record flow consumption when tracking is active.
pub(crate) fn push_flow_result(
    interp: &mut Interpreter,
    result: Value,
    input_flows: Option<&[FlowToken]>,
    consumed_amounts: &[Fraction],
) {
    interp.stack.push(result);

    if let Some(flows) = input_flows {
        for (flow, consumed) in flows.iter().zip(consumed_amounts.iter()) {
            let _ = interp.record_consumption(flow, consumed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_index_positive() {
        assert_eq!(normalize_index(0, 5), Some(0));
        assert_eq!(normalize_index(4, 5), Some(4));
        assert_eq!(normalize_index(5, 5), None);
    }

    #[test]
    fn test_normalize_index_negative() {
        assert_eq!(normalize_index(-1, 5), Some(4));
        assert_eq!(normalize_index(-5, 5), Some(0));
        assert_eq!(normalize_index(-6, 5), None);
    }

    #[test]
    fn test_wrap_number() {
        let frac = Fraction::new(BigInt::from(42), BigInt::one());
        let wrapped = wrap_number(frac.clone());
        assert!(wrapped.is_scalar());
        assert_eq!(wrapped.as_scalar(), Some(&frac));
    }

    #[test]
    fn test_get_integer_from_value() {
        let wrapped = wrap_number(Fraction::new(BigInt::from(42), BigInt::one()));
        let result = get_integer_from_value(&wrapped).unwrap();
        assert_eq!(result, 42);
    }
}
