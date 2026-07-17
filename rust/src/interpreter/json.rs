use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::semantic::{AbsenceOrigin, Recoverability};
use crate::types::arena::{
    arena_node_to_json, arena_to_value, json_to_arena_node, value_to_arena, ValueArena,
};
use crate::types::{Interpretation, Value, ValueData};
use std::collections::HashMap;
use std::sync::Arc;

fn extract_stack_value(
    interp: &mut Interpreter,
    keep_mode: bool,
    from_top: usize,
) -> Result<Value> {
    if keep_mode {
        if interp.stack.len() <= from_top {
            return Err(AjisaiError::StackUnderflow);
        }
        Ok(interp.stack[interp.stack.len() - 1 - from_top].clone())
    } else {
        if from_top != 0 {
            return Err(AjisaiError::from("Internal error: invalid consume access"));
        }
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)
    }
}

pub fn op_parse(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;

    let val = extract_stack_value(interp, is_keep, 0)?;

    let json_str = extract_string_content_from_value(&val);

    // Text that cannot be decoded as JSON projects onto a reasoned
    // Bubble/NIL — the same projection NUM uses for unparseable numeric text
    // (SPEC §11.2, reason = invalidEncoding). PARSE is registered as Pure,
    // so the failure is reported only through this value, never via an
    // output side effect.
    let parsed = serde_json::from_str::<serde_json::Value>(&json_str)
        .ok()
        .and_then(|json_val| {
            let mut arena = ValueArena::new();
            let root = json_to_arena_node(&mut arena, json_val).ok()?;
            Some(arena_to_value(&arena, root))
        });
    interp.stack.push(parsed.unwrap_or_else(|| {
        Value::bubble_with_reason(
            NilReason::InvalidEncoding,
            AbsenceOrigin::InvalidEncoding,
            Recoverability::Recoverable,
        )
    }));
    Ok(())
}

pub fn op_stringify(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;

    let val = extract_stack_value(interp, is_keep, 0)?;

    let (arena, root_id) = value_to_arena(&val);
    let json_val = arena_node_to_json(&arena, root_id);
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

    let val = extract_stack_value(interp, is_keep, 0)?;

    let text = format!("{}", val);
    interp.io_output_buffer.push_str(&text);
    Ok(())
}

pub fn op_json_get(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;

    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let key_val = extract_stack_value(interp, is_keep, 0)?;
    let obj_val = if is_keep {
        extract_stack_value(interp, true, 1)?
    } else {
        extract_stack_value(interp, false, 0)?
    };

    let key_str = extract_string_content_from_value(&key_val);

    let (pairs, shape) = match &obj_val.data {
        ValueData::Record { pairs, shape } => (pairs.as_slice(), Some(shape)),
        ValueData::Vector(v) => (v.as_slice(), None),
        _ => {
            interp.stack.push(Value::nil());
            return Ok(());
        }
    };

    if let Some(shape) = shape {
        if let Some(idx) = shape.slot(&key_str) {
            if let Some(pair) = pairs.get(idx) {
                if let ValueData::Vector(kv) = &pair.data {
                    if kv.len() == 2 {
                        interp.stack.push(kv[1].clone());
                        return Ok(());
                    }
                }
            }
        }
    } else {
        for pair in pairs {
            if let ValueData::Vector(kv) = &pair.data {
                if kv.len() == 2 {
                    let k = extract_string_content_from_value(&kv[0]);
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

    let obj_val = extract_stack_value(interp, is_keep, 0)?;

    let pairs = match &obj_val.data {
        ValueData::Record { pairs, .. } => pairs.as_slice(),
        ValueData::Vector(v) => v.as_slice(),
        _ => {
            interp.stack.push(Value::nil());
            return Ok(());
        }
    };

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
            data: ValueData::Vector(Arc::new(keys)),
            hint: Interpretation::Unassigned,
            absence: None,
        });
    }

    Ok(())
}

pub fn op_json_set(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;

    if interp.stack.len() < 3 {
        return Err(AjisaiError::StackUnderflow);
    }

    let new_value = extract_stack_value(interp, is_keep, 0)?;
    let key_val = if is_keep {
        extract_stack_value(interp, true, 1)?
    } else {
        extract_stack_value(interp, false, 0)?
    };
    let obj_val = if is_keep {
        extract_stack_value(interp, true, 2)?
    } else {
        extract_stack_value(interp, false, 0)?
    };

    let key_str = extract_string_content_from_value(&key_val);

    let (old_pairs, old_shape) = match &obj_val.data {
        ValueData::Record { pairs, shape } => (Some(pairs.as_slice()), Some(shape)),
        ValueData::Vector(v) => (Some(v.as_slice()), None),
        _ => (None, None),
    };

    if let Some(old_pairs) = old_pairs {
        let mut new_pairs: Vec<Value> = Vec::with_capacity(old_pairs.len() + 1);
        let mut new_index: HashMap<String, usize> = old_shape
            .map(|shape| shape.mapping().clone())
            .unwrap_or_default();
        let found_idx = if let Some(shape) = old_shape {
            shape.slot(&key_str)
        } else {
            old_pairs.iter().position(|pair| {
                if let ValueData::Vector(kv) = &pair.data {
                    if kv.len() == 2 {
                        return extract_string_content_from_value(&kv[0]) == key_str;
                    }
                }
                false
            })
        };

        for (i, pair) in old_pairs.iter().enumerate() {
            if Some(i) == found_idx {
                if let ValueData::Vector(kv) = &pair.data {
                    if kv.len() == 2 {
                        new_pairs.push(Value {
                            data: ValueData::Vector(Arc::new(vec![
                                kv[0].clone(),
                                new_value.clone(),
                            ])),
                            hint: Interpretation::Unassigned,
                            absence: None,
                        });
                        continue;
                    }
                }
            }
            new_pairs.push(pair.clone());
        }

        if found_idx.is_none() {
            new_index.insert(key_str.clone(), new_pairs.len());
            new_pairs.push(Value {
                data: ValueData::Vector(Arc::new(vec![Value::from_string(&key_str), new_value])),
                hint: Interpretation::Unassigned,
                absence: None,
            });
        }

        if old_shape.is_none() {
            new_index.clear();
            for (i, pair) in new_pairs.iter().enumerate() {
                if let ValueData::Vector(kv) = &pair.data {
                    if kv.len() == 2 {
                        let k = extract_string_content_from_value(&kv[0]);
                        new_index.insert(k, i);
                    }
                }
            }
        }

        interp.stack.push(Value {
            data: ValueData::Record {
                pairs: Arc::new(new_pairs),
                shape: crate::types::record_shape::intern_record_shape(new_index),
            },
            hint: Interpretation::Unassigned,
            absence: None,
        });
    } else {
        let pairs = Arc::new(vec![Value {
            data: ValueData::Vector(Arc::new(vec![Value::from_string(&key_str), new_value])),
            hint: Interpretation::Unassigned,
            absence: None,
        }]);
        interp.stack.push(Value {
            data: ValueData::Record {
                pairs,
                shape: crate::types::record_shape::record_shape_from_ordered_keys(
                    [key_str.clone()],
                ),
            },
            hint: Interpretation::Unassigned,
            absence: None,
        });
    }

    Ok(())
}

pub fn op_json_export(interp: &mut Interpreter) -> Result<()> {
    interp.run_hosted_effect_schema(
        "JSON@EXPORT",
        crate::interpreter::HostCapability::JsonExport,
        |interp| {
            let is_keep = interp.consumption_mode == ConsumptionMode::Keep;

            let val = extract_stack_value(interp, is_keep, 0)?;

            let (arena, root_id) = value_to_arena(&val);
            let json_val = arena_node_to_json(&arena, root_id);
            let json_compact =
                serde_json::to_string(&json_val).unwrap_or_else(|_| "null".to_string());
            interp
                .output_buffer
                .push_str(&format!("JSONEXPORT:{}\n", json_compact));
            Ok(crate::interpreter::HostEffect::JsonExport(json_compact))
        },
    )
}

/// Borrow the `[key, value]` pairs of a JSON object. Both the canonical
/// `Record` form and a raw vector-of-pairs are accepted; anything else
/// (scalar, NIL, code block, ...) is not an object and yields `None`.
fn object_pairs(val: &Value) -> Option<&[Value]> {
    match &val.data {
        ValueData::Record { pairs, .. } => Some(pairs.as_slice()),
        ValueData::Vector(v) => Some(v.as_slice()),
        _ => None,
    }
}

fn pair_key(pair: &Value) -> Option<String> {
    if let ValueData::Vector(kv) = &pair.data {
        if kv.len() == 2 {
            return Some(extract_string_content_from_value(&kv[0]));
        }
    }
    None
}

/// Build a canonical `Record` from a list of `[key, value]` pairs, deriving
/// the key index from the final pair order.
fn build_record(pairs: Vec<Value>) -> Value {
    let mut index: HashMap<String, usize> = HashMap::new();
    for (i, pair) in pairs.iter().enumerate() {
        if let Some(k) = pair_key(pair) {
            index.insert(k, i);
        }
    }
    Value {
        data: ValueData::Record {
            pairs: Arc::new(pairs),
            shape: crate::types::record_shape::intern_record_shape(index),
        },
        hint: Interpretation::Unassigned,
        absence: None,
    }
}

pub fn op_json_has(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;

    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let key_val = extract_stack_value(interp, is_keep, 0)?;
    let obj_val = if is_keep {
        extract_stack_value(interp, true, 1)?
    } else {
        extract_stack_value(interp, false, 0)?
    };

    let key_str = extract_string_content_from_value(&key_val);
    let found = object_pairs(&obj_val).is_some_and(|pairs| {
        pairs
            .iter()
            .any(|pair| pair_key(pair).as_deref() == Some(key_str.as_str()))
    });

    interp.stack.push(Value::from_bool(found));
    interp
        .semantic_registry
        .push_hint(Interpretation::TruthValue);
    Ok(())
}

pub fn op_json_values(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;

    let obj_val = extract_stack_value(interp, is_keep, 0)?;

    let Some(pairs) = object_pairs(&obj_val) else {
        interp.stack.push(Value::nil());
        return Ok(());
    };

    let mut values = Vec::new();
    for pair in pairs {
        if let ValueData::Vector(kv) = &pair.data {
            if kv.len() == 2 {
                values.push(kv[1].clone());
            }
        }
    }

    if values.is_empty() {
        interp.stack.push(Value::nil());
    } else {
        interp.stack.push(Value {
            data: ValueData::Vector(Arc::new(values)),
            hint: Interpretation::Unassigned,
            absence: None,
        });
    }
    Ok(())
}

pub fn op_json_merge(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;

    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let overlay_val = extract_stack_value(interp, is_keep, 0)?;
    let base_val = if is_keep {
        extract_stack_value(interp, true, 1)?
    } else {
        extract_stack_value(interp, false, 0)?
    };

    let mut merged: Vec<Value> = Vec::new();
    let mut position: HashMap<String, usize> = HashMap::new();

    for source in [&base_val, &overlay_val] {
        let Some(pairs) = object_pairs(source) else {
            continue;
        };
        for pair in pairs {
            let Some(key) = pair_key(pair) else {
                continue;
            };
            if let Some(&idx) = position.get(&key) {
                merged[idx] = pair.clone();
            } else {
                position.insert(key, merged.len());
                merged.push(pair.clone());
            }
        }
    }

    interp.stack.push(build_record(merged));
    Ok(())
}

pub fn op_json_delete(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;

    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let key_val = extract_stack_value(interp, is_keep, 0)?;
    let obj_val = if is_keep {
        extract_stack_value(interp, true, 1)?
    } else {
        extract_stack_value(interp, false, 0)?
    };

    let key_str = extract_string_content_from_value(&key_val);
    let Some(pairs) = object_pairs(&obj_val) else {
        interp.stack.push(Value::nil());
        return Ok(());
    };

    let kept: Vec<Value> = pairs
        .iter()
        .filter(|pair| pair_key(pair).as_deref() != Some(key_str.as_str()))
        .cloned()
        .collect();

    interp.stack.push(build_record(kept));
    Ok(())
}

fn extract_string_content_from_value(val: &Value) -> String {
    if let Some(view) = val.as_vector_view() {
        if view.iter().all(|c| matches!(c.data, ValueData::Scalar(_))) {
            return view
                .iter()
                .filter_map(|c| {
                    if let ValueData::Scalar(f) = &c.data {
                        f.to_i64().and_then(|n| {
                            if (0..=0x10FFFF).contains(&n) {
                                char::from_u32(n as u32)
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    }
                })
                .collect();
        }
    }
    format!("{}", val)
}
