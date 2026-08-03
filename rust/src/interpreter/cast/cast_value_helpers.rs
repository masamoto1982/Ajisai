use crate::error::{AjisaiError, Result};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Value, ValueData};

/// Whether a value is a String (LANG.VALUES.DISJOINT).
///
/// This used to *guess*: it walked a Vector's elements and answered "string"
/// when every one of them happened to be a printable codepoint, optionally
/// steered by an `Interpretation::Text` role. That made `[ 65 ]` and `'A'`
/// indistinguishable to every caller, and it was render-time re-guessing of
/// exactly the kind `Interpretation` promises the runtime never does. With
/// String a domain of its own, the question is answered by the tag.
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
    let is_keep_mode: bool = interp.consumption_mode == ConsumptionMode::Keep;

    let hint: Interpretation = interp.stack.last_role();
    let value: Value = if is_keep_mode {
        interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    match convert(&value) {
        Ok(result) => {
            // A unary cast is value-preserving on the semantic plane: the
            // slot keeps its prior plane role.
            // Core casts that do change the role (STR/NUM/…) are re-tagged
            // afterward by `apply_word_hint_override`.
            interp.stack.push_with_role(result, hint);
            Ok(())
        }
        Err(error) => {
            if !is_keep_mode {
                interp.stack.push_with_role(value, hint);
            }
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

pub(crate) fn try_char_from_value(val: &Value) -> Option<char> {
    let f: &Fraction = val.as_scalar()?;
    let code: i64 = f.to_i64()?;
    if !(0..=0x10FFFF).contains(&code) {
        return None;
    }
    char::from_u32(code as u32)
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
            // CS4 PR-2: casting U to a string yields `UNKNOWN` (matching its
            // display and the Boolean `TRUE`/`FALSE` precedent), never `NIL`.
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
            ValueData::Tensor { data, .. } => {
                data.iter().map(|f| format_fraction_to_string(&f)).collect()
            }
            ValueData::Text(s) => vec![s.to_string()],
            ValueData::CodeBlock(_) => vec!["<code>".to_string()],
        }
    }

    collect_fractions(value).join(" ")
}
