use crate::error::{AjisaiError, Result};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::{DisplayHint, Value, ValueData};

pub(crate) fn is_string_value(val: &Value) -> bool {
    is_string_value_with_hint(val, DisplayHint::Auto)
}

pub(crate) fn is_string_value_with_hint(val: &Value, hint: DisplayHint) -> bool {
    match hint {
        DisplayHint::Number | DisplayHint::Boolean | DisplayHint::DateTime | DisplayHint::Nil => {
            return false;
        }
        DisplayHint::String | DisplayHint::Auto => {}
    }
    let children: &Vec<Value> = match &val.data {
        ValueData::Vector(v) if !v.is_empty() => v,
        ValueData::Vector(_) => return false,
        ValueData::Scalar(_) => return false,
        ValueData::Nil => return false,
        ValueData::Record { .. } => return false,
        ValueData::CodeBlock(_) => return false,
    };
    children.iter().all(|child| check_char_scalar(child))
}

fn check_char_scalar(child: &Value) -> bool {
    let f: &Fraction = match &child.data {
        ValueData::Scalar(f) => f,
        ValueData::Vector(_) => return false,
        ValueData::Nil => return false,
        ValueData::Record { .. } => return false,
        ValueData::CodeBlock(_) => return false,
    };
    let n: i64 = match f.to_i64() {
        Some(n) if n >= 0 && n <= 0x10FFFF => n,
        Some(_) => return false,
        None => return false,
    };
    match char::from_u32(n as u32) {
        Some(c) => !c.is_control() || c == '\n' || c == '\r' || c == '\t',
        None => false,
    }
}

pub(crate) fn is_boolean_value(_val: &Value) -> bool {
    false
}

pub(crate) fn is_number_value(val: &Value) -> bool {
    val.is_scalar()
}

pub(crate) fn is_datetime_value(_val: &Value) -> bool {
    false
}

pub(crate) fn apply_unary_cast(interp: &mut Interpreter, convert: fn(&Value, DisplayHint) -> Result<Value>) -> Result<()> {
    let is_keep_mode: bool = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let hint: DisplayHint = interp.semantic_registry.lookup_last_hint();
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
                    interp.stack.push(result);
                    Ok(())
                }
                Err(error) => {
                    if !is_keep_mode {
                        interp.stack.push(value);
                    }
                    Err(error)
                }
            }
        }
        OperationTargetMode::Stack => {
            if interp.stack.is_empty() {
                return Err(AjisaiError::StackUnderflow);
            }

            if is_keep_mode {
                let originals: Vec<Value> = interp.stack.to_vec();
                let hints: Vec<DisplayHint> = (0..originals.len())
                    .map(|idx| interp.semantic_registry.lookup_hint_at(idx))
                    .collect();
                let mut converted: Vec<Value> = Vec::with_capacity(originals.len());
                for (idx, value) in originals.iter().enumerate() {
                    converted.push(convert(value, hints[idx])?);
                }
                interp.stack.extend(converted);
                Ok(())
            } else {
                let originals: Vec<Value> = interp.stack.drain(..).collect();
                let hints: Vec<DisplayHint> = (0..originals.len())
                    .map(|idx| interp.semantic_registry.lookup_hint_at(idx))
                    .collect();
                let mut converted: Vec<Value> = Vec::with_capacity(originals.len());

                for (idx, value) in originals.iter().enumerate() {
                    match convert(value, hints[idx]) {
                        Ok(result) => converted.push(result),
                        Err(error) => {
                            interp.stack = originals;
                            return Err(error);
                        }
                    }
                }

                interp.stack = converted;
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
    if code < 0 || code > 0x10FFFF { return None; }
    char::from_u32(code as u32)
}

/// 値を文字列表現に変換する（内部ヘルパー）
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

    if is_string_value(value) {
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

    // ベクタの場合
    fn collect_fractions(val: &Value) -> Vec<String> {
        match &val.data {
            ValueData::Nil => vec!["NIL".to_string()],
            ValueData::Scalar(f) => vec![format_fraction_to_string(f)],
            ValueData::Vector(children)
            | ValueData::Record {
                pairs: children, ..
            } => children.iter().flat_map(|c| collect_fractions(c)).collect(),
            ValueData::CodeBlock(_) => vec!["<code>".to_string()],
        }
    }

    collect_fractions(value).join(" ")
}
