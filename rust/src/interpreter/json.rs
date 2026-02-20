use crate::interpreter::{Interpreter, ConsumptionMode};
use crate::types::{Value, ValueData, DisplayHint};
use crate::types::json::{from_json, to_json};
use crate::error::{AjisaiError, Result};

pub fn op_parse(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;

    let val = if is_keep {
        interp.stack.last().cloned().ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    let json_str = value_to_string_content(&val);

    match serde_json::from_str::<serde_json::Value>(&json_str) {
        Ok(json_val) => {
            match from_json(json_val, 1) {
                Ok(parsed) => {
                    interp.stack.push(parsed);
                    Ok(())
                }
                Err(AjisaiError::DimensionLimitExceeded { depth }) => {
                    interp.output_buffer.push_str(
                        &format!("PARSE error: ネスト上限（10次元）を超過しました (depth {})\n", depth)
                    );
                    interp.stack.push(Value::nil());
                    Ok(())
                }
                Err(e) => {
                    interp.output_buffer.push_str(&format!("PARSE error: {}\n", e));
                    interp.stack.push(Value::nil());
                    Ok(())
                }
            }
        }
        Err(e) => {
            interp.output_buffer.push_str(&format!("PARSE error: {}\n", e));
            interp.stack.push(Value::nil());
            Ok(())
        }
    }
}

pub fn op_stringify(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;

    let val = if is_keep {
        interp.stack.last().cloned().ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    let json_val = to_json(&val);
    let json_str = serde_json::to_string(&json_val).unwrap_or_else(|_| "null".to_string());
    interp.stack.push(Value::from_string(&json_str));
    Ok(())
}

pub fn op_input(interp: &mut Interpreter) -> Result<()> {
    let text = interp.input_buffer.clone();
    interp.stack.push(Value::from_string(&text));
    Ok(())
}

pub fn op_output(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;

    let val = if is_keep {
        interp.stack.last().cloned().ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    let text = format!("{}", val);
    interp.io_output_buffer.push_str(&text);
    Ok(())
}

pub fn op_json_get(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;

    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let key_val = if is_keep {
        interp.stack[interp.stack.len() - 1].clone()
    } else {
        interp.stack.pop().unwrap()
    };

    let obj_val = if is_keep {
        interp.stack[interp.stack.len() - if is_keep { 2 } else { 1 }].clone()
    } else {
        interp.stack.pop().unwrap()
    };

    let key_str = value_to_string_content(&key_val);

    if let ValueData::Vector(pairs) = &obj_val.data {
        for pair in pairs {
            if let ValueData::Vector(kv) = &pair.data {
                if kv.len() == 2 {
                    let k = value_to_string_content(&kv[0]);
                    if k == key_str {
                        interp.stack.push(kv[1].clone());
                        return Ok(());
                    }
                }
            }
        }
    }

    interp.stack.push(Value::nil());
    Ok(())
}

pub fn op_json_keys(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;

    let obj_val = if is_keep {
        interp.stack.last().cloned().ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    if let ValueData::Vector(pairs) = &obj_val.data {
        let mut keys = Vec::new();
        for pair in pairs {
            if let ValueData::Vector(kv) = &pair.data {
                if kv.len() == 2 {
                    keys.push(kv[0].clone());
                }
            }
        }
        if keys.is_empty() {
            interp.stack.push(Value::nil());
        } else {
            interp.stack.push(Value {
                data: ValueData::Vector(keys),
                display_hint: DisplayHint::Auto,
                audio_hint: None,
            });
        }
    } else {
        interp.stack.push(Value::nil());
    }

    Ok(())
}

pub fn op_json_set(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;

    if interp.stack.len() < 3 {
        return Err(AjisaiError::StackUnderflow);
    }

    let new_value = if is_keep {
        interp.stack[interp.stack.len() - 1].clone()
    } else {
        interp.stack.pop().unwrap()
    };

    let key_val = if is_keep {
        interp.stack[interp.stack.len() - if is_keep { 2 } else { 1 }].clone()
    } else {
        interp.stack.pop().unwrap()
    };

    let obj_val = if is_keep {
        interp.stack[interp.stack.len() - if is_keep { 3 } else { 1 }].clone()
    } else {
        interp.stack.pop().unwrap()
    };

    let key_str = value_to_string_content(&key_val);

    if let ValueData::Vector(pairs) = &obj_val.data {
        let mut new_pairs: Vec<Value> = Vec::with_capacity(pairs.len() + 1);
        let mut found = false;

        for pair in pairs {
            if let ValueData::Vector(kv) = &pair.data {
                if kv.len() == 2 {
                    let k = value_to_string_content(&kv[0]);
                    if k == key_str {
                        new_pairs.push(Value {
                            data: ValueData::Vector(vec![kv[0].clone(), new_value.clone()]),
                            display_hint: DisplayHint::Auto,
                            audio_hint: None,
                        });
                        found = true;
                        continue;
                    }
                }
            }
            new_pairs.push(pair.clone());
        }

        if !found {
            new_pairs.push(Value {
                data: ValueData::Vector(vec![
                    Value::from_string(&key_str),
                    new_value,
                ]),
                display_hint: DisplayHint::Auto,
                audio_hint: None,
            });
        }

        interp.stack.push(Value {
            data: ValueData::Vector(new_pairs),
            display_hint: DisplayHint::Auto,
            audio_hint: None,
        });
    } else {
        let pairs = vec![Value {
            data: ValueData::Vector(vec![
                Value::from_string(&key_str),
                new_value,
            ]),
            display_hint: DisplayHint::Auto,
            audio_hint: None,
        }];
        interp.stack.push(Value {
            data: ValueData::Vector(pairs),
            display_hint: DisplayHint::Auto,
            audio_hint: None,
        });
    }

    Ok(())
}

fn value_to_string_content(val: &Value) -> String {
    if let ValueData::Vector(chars) = &val.data {
        if val.display_hint == DisplayHint::String || chars.iter().all(|c| matches!(c.data, ValueData::Scalar(_))) {
            return chars.iter().filter_map(|c| {
                if let ValueData::Scalar(f) = &c.data {
                    f.to_i64().and_then(|n| {
                        if n >= 0 && n <= 0x10FFFF {
                            char::from_u32(n as u32)
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            }).collect();
        }
    }
    format!("{}", val)
}
