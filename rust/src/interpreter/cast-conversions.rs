use crate::error::{AjisaiError, Result};
use crate::interpreter::cast::cast_value_helpers::{
    apply_unary_cast, format_fraction_to_string, format_value_to_string_repr,
    is_boolean_value, is_number_value, is_string_value_with_hint,
};
use crate::interpreter::value_extraction_helpers::{create_number_value, value_as_string};
use crate::interpreter::{Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::{DisplayHint, Value};

fn convert_value_to_string(val: &Value, hint: DisplayHint) -> Result<Value> {
    if val.is_nil() {
        return Ok(Value::nil());
    }

    if is_string_value_with_hint(val, hint) {
        return Ok(val.clone());
    }

    if is_number_value(val) {
        if let Some(f) = val.as_scalar() {
            let string_repr = format_fraction_to_string(f);
            return Ok(Value::from_string(&string_repr));
        }
    }

    let string_repr = format_value_to_string_repr(val);
    Ok(Value::from_string(&string_repr))
}

pub fn op_str(interp: &mut Interpreter) -> Result<()> {
    apply_unary_cast(interp, convert_value_to_string)
}

fn convert_value_to_number(val: &Value, hint: DisplayHint) -> Result<Value> {
    if is_string_value_with_hint(val, hint) {
        let s = value_as_string(val).unwrap_or_default();
        match Fraction::from_str(&s) {
            Ok(fraction) => return Ok(create_number_value(fraction)),
            Err(_) => return Ok(Value::nil()),
        }
    }

    if is_number_value(val) {
        return Ok(val.clone());
    }
    if is_boolean_value(val) {
        return Err(AjisaiError::from(
            "NUM: expected String, got Boolean",
        ));
    }
    if val.is_nil() {
        return Err(AjisaiError::from("NUM: expected String, got Nil"));
    }
    Err(AjisaiError::from("NUM: expected String input"))
}

pub fn op_num(interp: &mut Interpreter) -> Result<()> {
    apply_unary_cast(interp, convert_value_to_number)
}

fn convert_value_to_boolean(val: &Value, hint: DisplayHint) -> Result<Value> {
    if is_boolean_value(val) {
        return Ok(val.clone());
    }
    if is_string_value_with_hint(val, hint) {
        let s = value_as_string(val).unwrap_or_default();
        let upper = s.to_uppercase();
        if upper == "TRUE" {
            return Ok(Value::from_bool(true));
        } else if upper == "FALSE" {
            return Ok(Value::from_bool(false));
        } else {
            return Ok(Value::nil());
        }
    }
    if is_number_value(val) {
        if let Some(f) = val.as_scalar() {
            return Ok(Value::from_bool(!f.is_zero()));
        }
    }
    if val.is_nil() {
        return Err(AjisaiError::from(
            "BOOL: expected String or Number, got Nil",
        ));
    }
    Err(AjisaiError::from("BOOL: expected String or Number input"))
}

pub fn op_bool(interp: &mut Interpreter) -> Result<()> {
    apply_unary_cast(interp, convert_value_to_boolean)
}

pub fn op_nil(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported {
            word: "NIL".into(),
            mode: "Stack".into(),
        });
    }

    let hint: DisplayHint = interp.semantic_registry.lookup_last_hint();
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    if val.is_nil() {
        interp.stack.push(val);
        return Ok(());
    }

    if is_string_value_with_hint(&val, hint) {
        let s = value_as_string(&val).unwrap_or_default();
        let upper = s.to_uppercase();
        if upper == "NIL" {
            interp.stack.push(Value::nil());
            return Ok(());
        } else {
            let err_msg = format!("NIL: cannot parse '{}' as nil (expected 'nil')", s);
            interp.stack.push(val);
            return Err(AjisaiError::from(err_msg));
        }
    }

    if is_boolean_value(&val) {
        interp.stack.push(val);
        return Err(AjisaiError::from(
            "NIL: expected String, got Boolean",
        ));
    }

    if is_number_value(&val) {
        interp.stack.push(val);
        return Err(AjisaiError::from(
            "NIL: expected String, got Number",
        ));
    }

    interp.stack.push(val);
    Err(AjisaiError::from("NIL: expected String input"))
}


fn convert_codepoint_to_char(val: &Value, hint: DisplayHint) -> Result<Value> {
    if is_number_value(val) {
        if let Some(f) = val.as_scalar() {
            if let Some(code) = f.to_i64() {
                if code >= 0 && code <= 0x10FFFF {
                    if let Some(c) = char::from_u32(code as u32) {
                        return Ok(Value::from_string(&c.to_string()));
                    }
                }
                return Err(AjisaiError::from(format!(
                    "CHR: {} is not a valid Unicode code point (valid range: 0-0x10FFFF, excluding surrogates)",
                    code
                )));
            } else {
                let frac_str = format_fraction_to_string(f);
                return Err(AjisaiError::from(format!(
                    "CHR: requires an integer, got {}",
                    frac_str
                )));
            }
        }
    }
    if is_string_value_with_hint(val, hint) {
        return Err(AjisaiError::from("CHR: expected Number, got String"));
    }
    if is_boolean_value(val) {
        return Err(AjisaiError::from("CHR: expected Number, got Boolean"));
    }
    if val.is_nil() {
        return Err(AjisaiError::from("CHR: expected Number, got Nil"));
    }
    Err(AjisaiError::from("CHR: expected Number input"))
}

pub fn op_chr(interp: &mut Interpreter) -> Result<()> {
    apply_unary_cast(interp, convert_codepoint_to_char)
}
