use crate::error::{AjisaiError, Result};
use crate::interpreter::Interpreter;
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};
use num_bigint::BigInt;
use num_traits::ToPrimitive;

#[inline]
pub(crate) fn is_vector_value(val: &Value) -> bool {
    matches!(&val.data, ValueData::Vector(_) | ValueData::Tensor { .. })
}

pub(crate) fn value_as_string(val: &Value) -> Option<String> {
    fn collect_chars(val: &Value) -> Vec<char> {
        match &val.data {
            ValueData::Text(s) => s.chars().collect(),
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
            ValueData::Vector(children) => children.iter().flat_map(collect_chars).collect(),
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
            ValueData::Boolean(_) | ValueData::Symbol(_) | ValueData::Record(_) => vec![],
        }
    }

    let chars = collect_chars(val);
    if chars.is_empty() {
        None
    } else {
        Some(chars.into_iter().collect())
    }
}

/// An operand that is not an integer, described for the message the caller
/// raises. Not an `AjisaiError`: GET, TAKE, PUT, COLLECT and RANGE each
/// declare their own condition for it (`invalidIndex`, `invalidCount`,
/// `nonInteger`, `invalidRange`), so the caller names it.
#[derive(Debug, Clone)]
pub(crate) struct NotAnInteger {
    pub got: String,
}

impl NotAnInteger {
    fn of(value: &Value) -> Self {
        NotAnInteger {
            got: crate::types::display::describe_operand(value),
        }
    }
}

fn extract_integer_bigint(value: &Value) -> std::result::Result<BigInt, NotAnInteger> {
    match &value.data {
        ValueData::Scalar(f) if f.is_integer() => Ok(f.numerator()),
        // A one-element Vector stands for its element: `[ 2 ]` is the index 2.
        ValueData::Vector(children) if children.len() == 1 => extract_integer_bigint(&children[0]),
        ValueData::Tensor { data, .. } if data.len() == 1 => match data.get_small_fraction(0) {
            Some(fraction) if fraction.is_integer() => Ok(fraction.numerator()),
            Some(fraction) => Err(NotAnInteger::of(&Value::from_fraction(fraction))),
            None => Err(NotAnInteger {
                got: "NIL".to_string(),
            }),
        },
        _ => Err(NotAnInteger::of(value)),
    }
}

pub(crate) fn extract_integer_from_value(value: &Value) -> std::result::Result<i64, NotAnInteger> {
    extract_integer_bigint(value)?
        .to_i64()
        .ok_or_else(|| NotAnInteger::of(value))
}

pub(crate) fn extract_bigint_from_value(
    value: &Value,
) -> std::result::Result<BigInt, NotAnInteger> {
    extract_integer_bigint(value)
}

/// The Word name a value denotes. A name is a String
/// (`{ ... } 'INC' DEF`), so only the String domain supplies one.
///
/// This used to flatten the value to a list of fractions and decode each as a
/// codepoint, which accepted `[ 73 78 67 ]` as the name `INC` just as readily
/// as `'INC'` — a Vector of numbers naming a Word. Reading the String domain
/// closes that, and `nonText` is the honest error for everything else — DEF
/// and DEL both declare it (`spec/words.json`) for exactly this call.
pub(crate) fn extract_word_name_from_value(value: &Value) -> Result<String> {
    if value.is_nil() {
        return Err(AjisaiError::declared(
            "nonText",
            "expected a name (String), got NIL",
        ));
    }

    match value.as_text() {
        Some(name) => Ok(name.to_uppercase()),
        None => Err(AjisaiError::declared(
            "nonText",
            format!("expected a name (String), got {}", value.domain_name()),
        )),
    }
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

pub(crate) fn extract_operands(interp: &mut Interpreter, count: usize) -> Result<Vec<Value>> {
    if interp.stack.len() < count {
        return Err(AjisaiError::StackUnderflow);
    }

    let values: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();
    if values.len() != count {
        return Err(AjisaiError::StackUnderflow);
    }
    Ok(values)
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
    interp.stack.pop();
    interp.stack.push(inherited);
    true
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
    interp.stack.pop();
    interp.stack.pop();
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
