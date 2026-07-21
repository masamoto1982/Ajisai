use crate::error::{AjisaiError, Result};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Stack, Value, ValueData};

pub(crate) fn is_string_value(val: &Value) -> bool {
    is_string_value_with_hint(val, Interpretation::Unassigned)
}

pub(crate) fn is_string_value_with_hint(val: &Value, hint: Interpretation) -> bool {
    match hint {
        Interpretation::RawNumber
        | Interpretation::ContinuedFraction
        | Interpretation::Interval
        | Interpretation::TruthValue
        | Interpretation::Timestamp
        | Interpretation::Nil => {
            return false;
        }
        Interpretation::Text | Interpretation::Unassigned => {}
    }
    let children: &Vec<Value> = match &val.data {
        ValueData::Vector(v) if !v.is_empty() => v,
        ValueData::Vector(_) => return false,
        ValueData::Tensor { data, .. } => {
            if data.is_empty() {
                return false;
            }
            return data.iter().all(|f| {
                let n: i64 = match f.to_i64() {
                    Some(n) if (0..=0x10FFFF).contains(&n) => n,
                    _ => return false,
                };
                match char::from_u32(n as u32) {
                    Some(c) => !c.is_control() || c == '\n' || c == '\r' || c == '\t',
                    None => false,
                }
            });
        }
        ValueData::Scalar(_) => return false,
        ValueData::ExactScalar(_) => return false,
        ValueData::Nil => return false,
        ValueData::Record { .. } => return false,
        ValueData::Boolean(_)
        | ValueData::CodeBlock(_)
        | ValueData::ProcessHandle(_)
        | ValueData::SupervisorHandle(_) => return false,
    };
    children.iter().all(check_char_scalar)
}

fn check_char_scalar(child: &Value) -> bool {
    let f: &Fraction = match &child.data {
        ValueData::Scalar(f) => f,
        ValueData::ExactScalar(_) => return false,
        ValueData::Vector(_) => return false,
        ValueData::Tensor { .. } => return false,
        ValueData::Nil => return false,
        ValueData::Record { .. } => return false,
        ValueData::Boolean(_)
        | ValueData::CodeBlock(_)
        | ValueData::ProcessHandle(_)
        | ValueData::SupervisorHandle(_) => return false,
    };
    let n: i64 = match f.to_i64() {
        Some(n) if (0..=0x10FFFF).contains(&n) => n,
        Some(_) => return false,
        None => return false,
    };
    match char::from_u32(n as u32) {
        Some(c) => !c.is_control() || c == '\n' || c == '\r' || c == '\t',
        None => false,
    }
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
    convert: fn(&Value, Interpretation) -> Result<Value>,
) -> Result<()> {
    let is_keep_mode: bool = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
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

            match convert(&value, hint) {
                Ok(result) => {
                    // A unary cast is value-preserving on the semantic plane: the
                    // slot keeps its prior plane role (e.g. `>CF` retagging).
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
        OperationTargetMode::Stack => {
            if interp.stack.is_empty() {
                return Err(AjisaiError::StackUnderflow);
            }

            // Roles are captured before any drain so a value-preserving cast can
            // carry each slot's prior plane role onto its converted value.
            let hints: Vec<Interpretation> = interp.stack.roles().to_vec();
            if is_keep_mode {
                let originals: Vec<Value> = interp.stack.to_vec();
                let mut converted: Vec<Value> = Vec::with_capacity(originals.len());
                for (idx, value) in originals.iter().enumerate() {
                    converted.push(convert(value, hints[idx])?);
                }
                for (idx, result) in converted.into_iter().enumerate() {
                    interp.stack.push_with_role(result, hints[idx]);
                }
                Ok(())
            } else {
                let originals: Vec<Value> = interp.stack.drain(..).collect();
                let mut converted: Vec<Value> = Vec::with_capacity(originals.len());

                for (idx, value) in originals.iter().enumerate() {
                    match convert(value, hints[idx]) {
                        Ok(result) => converted.push(result),
                        Err(error) => {
                            interp.stack = Stack::from_values_and_roles(originals, hints);
                            return Err(error);
                        }
                    }
                }

                interp.stack = Stack::from_values_and_roles(converted, hints);
                Ok(())
            }
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

#[cfg(test)]
pub(crate) fn format_value_to_string_repr(value: &Value) -> String {
    format_value_to_string_repr_with_hint(value, Interpretation::Unassigned)
}

pub(crate) fn format_value_to_string_repr_with_hint(value: &Value, hint: Interpretation) -> String {
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

    if is_string_value_with_hint(value, hint) {
        return crate::interpreter::value_extraction_helpers::value_as_string(value)
            .unwrap_or_default();
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
            ValueData::Vector(children)
            | ValueData::Record {
                pairs: children, ..
            } => children.iter().flat_map(|c| collect_fractions(c)).collect(),
            ValueData::Tensor { data, .. } => {
                data.iter().map(|f| format_fraction_to_string(&f)).collect()
            }
            ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => vec!["<code>".to_string()],
        }
    }

    collect_fractions(value).join(" ")
}
