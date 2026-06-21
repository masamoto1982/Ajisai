use crate::error::{AjisaiError, Result};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};
use num_bigint::BigInt;
use num_traits::ToPrimitive;

#[inline]
pub(crate) fn is_vector_value(val: &Value) -> bool {
    matches!(&val.data, ValueData::Vector(_) | ValueData::Tensor { .. })
}

#[inline]
pub(crate) fn is_string_value(val: &Value) -> bool {
    matches!(&val.data, ValueData::Vector(_))
}

pub(crate) fn value_as_string(val: &Value) -> Option<String> {
    fn collect_chars(val: &Value) -> Vec<char> {
        match &val.data {
            ValueData::Nil => vec![],
            ValueData::Scalar(f) => f
                .to_i64()
                .and_then(|n| {
                    if (0..=0x10FFFF).contains(&n) {
                        char::from_u32(n as u32)
                    } else {
                        None
                    }
                })
                .map(|c| vec![c])
                .unwrap_or_default(),
            ValueData::Vector(children)
            | ValueData::Record {
                pairs: children, ..
            } => children.iter().flat_map(collect_chars).collect(),
            ValueData::Tensor { data, .. } => data
                .iter()
                .filter_map(|f| {
                    f.to_i64().and_then(|n| {
                        if (0..=0x10FFFF).contains(&n) {
                            char::from_u32(n as u32)
                        } else {
                            None
                        }
                    })
                })
                .collect(),
            ValueData::ExactScalar(_) => vec![],
            ValueData::Boolean(_)
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => vec![],
        }
    }

    let chars = collect_chars(val);
    if chars.is_empty() {
        None
    } else {
        Some(chars.into_iter().collect())
    }
}

fn extract_integer_bigint(value: &Value) -> Result<BigInt> {
    match &value.data {
        ValueData::Scalar(f) => {
            if !f.is_integer() {
                return Err(AjisaiError::create_structure_error("integer", "fraction"));
            }
            Ok(f.numerator())
        }
        ValueData::Nil => Err(AjisaiError::create_structure_error(
            "single-element value with integer",
            "NIL",
        )),
        ValueData::Vector(children)
        | ValueData::Record {
            pairs: children, ..
        } if children.len() == 1 => extract_integer_bigint(&children[0]),
        ValueData::Vector(_) | ValueData::Record { .. } => {
            Err(AjisaiError::create_structure_error(
                "single-element value with integer",
                "multi-element vector",
            ))
        }
        ValueData::Tensor { data, .. } => {
            if data.len() == 1 {
                let fraction = data
                    .get_small_fraction(0)
                    .ok_or_else(|| AjisaiError::create_structure_error("integer", "NIL"))?;
                if !fraction.is_integer() {
                    return Err(AjisaiError::create_structure_error("integer", "fraction"));
                }
                Ok(fraction.numerator())
            } else {
                Err(AjisaiError::create_structure_error(
                    "single-element value with integer",
                    "multi-element vector",
                ))
            }
        }
        ValueData::ExactScalar(_) => Err(AjisaiError::create_structure_error(
            "integer",
            "irrational exact real",
        )),
        ValueData::Boolean(_)
        | ValueData::CodeBlock(_)
        | ValueData::ProcessHandle(_)
        | ValueData::SupervisorHandle(_) => Err(AjisaiError::create_structure_error(
            "single-element value with integer",
            "code block",
        )),
    }
}

pub(crate) fn extract_integer_from_value(value: &Value) -> Result<i64> {
    let n = extract_integer_bigint(value)?;
    n.to_i64()
        .ok_or_else(|| AjisaiError::from("Integer value is too large for i64"))
}

pub(crate) fn extract_bigint_from_value(value: &Value) -> Result<BigInt> {
    extract_integer_bigint(value)
}

pub(crate) fn extract_word_name_from_value(value: &Value) -> Result<String> {
    if value.is_nil() {
        return Err(AjisaiError::from("Cannot get word name from NIL"));
    }

    let fractions = value.collect_fractions_flat();
    let chars: String = fractions
        .iter()
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
    // Resolve the bounds check entirely in i64 before narrowing. An in-memory
    // vector length always fits i64, and a previous `index as usize` truncated
    // out-of-range positive indices on 32-bit wasm (e.g. 2^32 + 1 wrapping to a
    // valid-looking small index). Keeping `actual` in [0, length) guarantees the
    // final `as usize` is exact on both 32- and 64-bit targets.
    let len_i64 = length as i64;
    let actual = if index < 0 {
        len_i64.checked_add(index)?
    } else {
        index
    };

    if actual >= 0 && actual < len_i64 {
        Some(actual as usize)
    } else {
        None
    }
}

pub(crate) fn create_number_value(fraction: Fraction) -> Value {
    Value::from_fraction(fraction)
}

pub(crate) fn create_datetime_value(fraction: Fraction) -> Value {
    Value::from_fraction(fraction)
}

pub(crate) fn extract_operands(interp: &mut Interpreter, count: usize) -> Result<Vec<Value>> {
    if interp.stack.len() < count {
        return Err(AjisaiError::StackUnderflow);
    }

    match interp.consumption_mode {
        ConsumptionMode::Consume => {
            let values: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();
            if values.len() != count {
                return Err(AjisaiError::StackUnderflow);
            }
            Ok(values)
        }
        ConsumptionMode::Keep => {
            let stack_len = interp.stack.len();
            let values: Vec<Value> = interp.stack[stack_len - count..].iter().cloned().collect();
            Ok(values)
        }
    }
}

pub(crate) fn push_result(interp: &mut Interpreter, result: Value) {
    interp.stack.push(result);
}

pub(crate) fn nil_passthrough_unary(interp: &mut Interpreter) -> bool {
    let stack_len = interp.stack.len();
    if stack_len == 0 {
        return false;
    }
    if !interp.stack[stack_len - 1].is_operational_nil() {
        return false;
    }
    let inherited = Value::nil_inheriting_absence_from(&interp.stack[stack_len - 1]);
    if interp.consumption_mode == ConsumptionMode::Consume {
        interp.stack.pop();
    }
    interp.stack.push(inherited);
    true
}

pub(crate) fn nil_passthrough_value<'a>(
    items: impl IntoIterator<Item = &'a Value>,
) -> Option<Value> {
    items
        .into_iter()
        .find(|v| v.is_operational_nil())
        .map(Value::nil_inheriting_absence_from)
}

pub(crate) fn nil_passthrough_binary(interp: &mut Interpreter) -> bool {
    let stack_len = interp.stack.len();
    if stack_len < 2 {
        return false;
    }
    let a_nil = interp.stack[stack_len - 2].is_operational_nil();
    let b_nil = interp.stack[stack_len - 1].is_operational_nil();
    if !(a_nil || b_nil) {
        return false;
    }
    let inherited = if a_nil {
        Value::nil_inheriting_absence_from(&interp.stack[stack_len - 2])
    } else {
        Value::nil_inheriting_absence_from(&interp.stack[stack_len - 1])
    };
    if interp.consumption_mode == ConsumptionMode::Consume {
        interp.stack.pop();
        interp.stack.pop();
    }
    interp.stack.push(inherited);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::One;

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
    fn test_create_number_value() {
        let frac = Fraction::new(BigInt::from(42), BigInt::one());
        let wrapped = create_number_value(frac.clone());
        assert!(wrapped.is_scalar());
        assert_eq!(wrapped.as_scalar(), Some(&frac));
    }

    #[test]
    fn test_extract_integer_from_value() {
        let wrapped = create_number_value(Fraction::new(BigInt::from(42), BigInt::one()));
        let result = extract_integer_from_value(&wrapped).unwrap();
        assert_eq!(result, 42);
    }
}
