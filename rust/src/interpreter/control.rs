// rust/src/interpreter/control.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType};
// `BigInt::one()` を使用するために `One` トレイトをスコープに入れる
use num_traits::{One, ToPrimitive};

pub(crate) fn execute_times(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::from("TIMES requires word name and count. Usage: 'WORD' [ n ] TIMES"));
    }

    let count_val = interp.stack.pop().unwrap();
    let name_val = interp.stack.pop().unwrap();

    let count = match &count_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) if n.denominator == num_bigint::BigInt::one() => {
                    n.numerator.to_i64().ok_or_else(|| AjisaiError::from("Count too large"))?
                },
                _ => return Err(AjisaiError::type_error("integer", "other type")),
            }
        },
        _ => return Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    };

    let word_name = match &name_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::String(s) => s.clone(),
                _ => return Err(AjisaiError::type_error("string", "other type")),
            }
        },
        _ => return Err(AjisaiError::type_error("single-element vector with string", "other type")),
    };

    let upper_name = word_name.to_uppercase();
    
    if let Some(def) = interp.dictionary.get(&upper_name) {
        if def.is_builtin {
            return Err(AjisaiError::from("TIMES can only be used with custom words"));
        }
    } else {
        return Err(AjisaiError::UnknownWord(word_name));
    }

    for _ in 0..count {
        interp.execute_word_sync(&upper_name)?;
    }

    Ok(())
}

pub(crate) fn execute_wait(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::from("WAIT requires word name and delay. Usage: 'WORD' [ ms ] WAIT"));
    }

    let delay_val = interp.stack.pop().unwrap();
    let name_val = interp.stack.pop().unwrap();

    let delay_ms = match &delay_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) if n.denominator == num_bigint::BigInt::one() => {
                    n.numerator.to_u64().ok_or_else(|| AjisaiError::from("Delay too large"))?
                },
                _ => return Err(AjisaiError::type_error("integer", "other type")),
            }
        },
        _ => return Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    };

    let word_name = match &name_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::String(s) => s.clone(),
                _ => return Err(AjisaiError::type_error("string", "other type")),
            }
        },
        _ => return Err(AjisaiError::type_error("single-element vector with string", "other type")),
    };

    let upper_name = word_name.to_uppercase();
    
    if let Some(def) = interp.dictionary.get(&upper_name) {
        if def.is_builtin {
            return Err(AjisaiError::from("WAIT can only be used with custom words"));
        }
    } else {
        return Err(AjisaiError::UnknownWord(word_name));
    }

    interp.output_buffer.push_str(&format!("[DEBUG] Would wait {}ms before executing '{}'\n", delay_ms, word_name));
    
    interp.execute_word_sync(&upper_name)?;

    Ok(())
}
