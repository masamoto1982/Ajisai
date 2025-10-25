// rust/src/interpreter/control.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType};
// `BigInt::one()` を使用するために `One` トレイトをスコープに入れる
use num_traits::{One, ToPrimitive};
use std::fmt::Write; // for write!

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
        // ★ 修正: TIMES/WAIT は 'WORD' ではなく [ 'WORD' ] (シンボル) を取るべき
        // ...だったが、高階関数 (MAP) との整合性をとるため、 'WORD' (文字列) を取る
        ValueType::String(s) => s.clone(),
        _ => return Err(AjisaiError::type_error("string 'name'", "other type")),
    };

    let upper_name = word_name.to_uppercase();
    
    if let Some(def) = interp.dictionary.get(&upper_name) {
        if def.is_builtin {
            return Err(AjisaiError::from("TIMES can only be used with custom words"));
        }
    } else {
        return Err(AjisaiError::UnknownWord(word_name));
    }

    // ★ デバッグログ
    writeln!(interp.debug_buffer, "[CONTROL] Executing '{}' {} times", upper_name, count).unwrap();

    for i in 0..count {
        writeln!(interp.debug_buffer, "[CONTROL] TIMES iteration {}/{}", i + 1, count).unwrap();
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
        ValueType::String(s) => s.clone(),
        _ => return Err(AjisaiError::type_error("string 'name'", "other type")),
    };

    let upper_name = word_name.to_uppercase();
    
    if let Some(def) = interp.dictionary.get(&upper_name) {
        if def.is_builtin {
            return Err(AjisaiError::from("WAIT can only be used with custom words"));
        }
    } else {
        return Err(AjisaiError::UnknownWord(word_name));
    }

    // ★ デバッグバッファに書き込む
    writeln!(interp.debug_buffer, "[CONTROL] Waiting {}ms before executing '{}'", delay_ms, word_name).unwrap();
    
    // (実際の待機処理は async execute に移動する必要があるが、
    //  現在の execute_word_sync は同期的であるため、ここではログ出力のみ行う)
    
    interp.execute_word_sync(&upper_name)?;

    Ok(())
}
