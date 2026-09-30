use crate::error::{AjisaiError, Result};
use crate::interpreter::Interpreter;
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};

/// Whether a value is a String (LANG.VALUES.DISJOINT).
///
/// String is a domain of its own, so the question is answered by the tag.
pub(crate) fn is_string_value(val: &Value) -> bool {
    val.is_text()
}

pub(crate) fn is_boolean_value(val: &Value) -> bool {
    matches!(val.data, ValueData::Boolean(_))
}

pub(crate) fn is_number_value(val: &Value) -> bool {
    val.is_scalar()
}

pub(crate) fn is_datetime_value(_val: &Value) -> bool {
    false
}

pub(crate) fn apply_unary_cast(
    interp: &mut Interpreter,
    convert: fn(&Value) -> Result<Value>,
) -> Result<()> {
    let value: Value = interp.stack.pop().ok_or(AjisaiError::stack_underflow())?;

    match convert(&value) {
        Ok(result) => {
            interp.stack.push(result);
            Ok(())
        }
        Err(error) => {
            interp.stack.push(value);
            Err(error)
        }
    }
}

pub(crate) fn format_fraction_to_string(f: &Fraction) -> String {
    if f.is_integer() {
        format!("{}", f.numerator())
    } else {
        format!("{}/{}", f.numerator(), f.denominator())
    }
}

pub(crate) fn format_value_to_string_repr(value: &Value) -> String {
    if value.is_nil() {
        return "NIL".to_string();
    }

    if is_boolean_value(value) {
        if let Some(f) = value.as_scalar() {
            return if !f.is_zero() {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            };
        }
    }

    if let Some(text) = value.as_text() {
        return text.to_string();
    }

    if is_datetime_value(value) {
        if let Some(f) = value.as_scalar() {
            return format!("@{}", format_fraction_to_string(f));
        }
    }

    if is_number_value(value) {
        if let Some(f) = value.as_scalar() {
            return format_fraction_to_string(f);
        }
    }

    fn collect_fractions(val: &Value) -> Vec<String> {
        match &val.data {
            ValueData::Nil => vec!["NIL".to_string()],
            ValueData::Boolean(b) => vec![if *b { "TRUE" } else { "FALSE" }.to_string()],
            ValueData::Scalar(f) => vec![format_fraction_to_string(f)],
            ValueData::ExactScalar(er) => {
                use num_bigint::BigInt;
                match er.best_rational_approximation(&BigInt::from(1_000_000u64)) {
                    Some(approx) => vec![format_fraction_to_string(&approx)],
                    None => vec!["NIL".to_string()],
                }
            }
            ValueData::Vector(children) => children.iter().flat_map(collect_fractions).collect(),
            // Through the lane, not its `Fraction`: `format_fraction_to_string`
            // renders the denominator-0 absence sentinel as the unreadable
            // number `0/0`. `[ 1 NIL ] STR` is `'1 NIL'`, and used to be only
            // because a vector holding a NIL was never stored densely.
            ValueData::Tensor { data, .. } => (0..data.len())
                .flat_map(|lane| collect_fractions(&Value::from_dense_lane(data, lane)))
                .collect(),
            ValueData::Text(s) => vec![s.to_string()],
            ValueData::Symbol(name) => vec![name.to_string()],
            // A Record casts as its display form, whole: it is not a sequence
            // of lanes to join.
            ValueData::Record(_) => vec![val.to_string()],
        }
    }

    collect_fractions(value).join(" ")
}
