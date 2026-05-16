use crate::types::arena::{NodeId, NodeKind, ValueArena};
use crate::types::fraction::Fraction;
use crate::types::{DisplayHint, Value};
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
pub(crate) struct UserWordData {
    pub(crate) dictionary: Option<String>,
    pub(crate) name: String,
    pub(crate) definition: Option<String>,
    pub(crate) description: Option<String>,
}

pub(crate) fn bracket_chars_for_depth(depth: usize) -> (char, char) {
    let _ = depth;
    ('[', ']')
}

pub(crate) fn build_bracket_structure_from_shape(shape: &[usize]) -> String {
    fn build_level(shape: &[usize], depth: usize) -> String {
        let (open, close) = bracket_chars_for_depth(depth);
        if shape.len() == 1 {
            let empty = format!("{} {}", open, close);
            (0..shape[0])
                .map(|_| empty.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            let inner = build_level(&shape[1..], depth + 1);
            let one_element = format!("{} {} {}", open, inner, close);
            (0..shape[0])
                .map(|_| one_element.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        }
    }
    if shape.is_empty() {
        return "[ ]".to_string();
    }
    build_level(shape, 0)
}

pub(crate) fn is_vector_value(val: &Value) -> bool {
    val.is_vector()
}

fn fraction_display_source_from_js(num_obj: &js_sys::Object) -> Option<String> {
    js_sys::Reflect::get(num_obj, &"displaySource".into())
        .ok()
        .and_then(|value| value.as_string())
        .filter(|source| !source.is_empty())
}

fn apply_fraction_display_source(mut fraction: Fraction, num_obj: &js_sys::Object) -> Fraction {
    if let Some(source) = fraction_display_source_from_js(num_obj) {
        fraction = fraction.with_display_source(&source);
    }
    fraction
}

pub(crate) fn js_value_to_value(js_val: JsValue) -> Result<Value, String> {
    let obj = js_sys::Object::from(js_val);
    let type_str = js_sys::Reflect::get(&obj, &"type".into())
        .map_err(|_| "Failed to get 'type' property".to_string())?
        .as_string()
        .ok_or("Type not string")?;
    let value_js = js_sys::Reflect::get(&obj, &"value".into())
        .map_err(|_| "Failed to get 'value' property".to_string())?;

    match type_str.as_str() {
        "number" => {
            let num_obj = js_sys::Object::from(value_js);
            let num_str = js_sys::Reflect::get(&num_obj, &"numerator".into())
                .map_err(|_| "No numerator".to_string())?
                .as_string()
                .ok_or("Numerator not string")?;
            let den_str = js_sys::Reflect::get(&num_obj, &"denominator".into())
                .map_err(|_| "No denominator".to_string())?
                .as_string()
                .ok_or("Denominator not string")?;
            let fraction = apply_fraction_display_source(
                Fraction::new(
                    BigInt::from_str(&num_str).map_err(|e| e.to_string())?,
                    BigInt::from_str(&den_str).map_err(|e| e.to_string())?,
                ),
                &num_obj,
            );
            Ok(Value::from_fraction(fraction))
        }
        "datetime" => {
            let num_obj = js_sys::Object::from(value_js);
            let num_str = js_sys::Reflect::get(&num_obj, &"numerator".into())
                .map_err(|_| "No numerator".to_string())?
                .as_string()
                .ok_or("Numerator not string")?;
            let den_str = js_sys::Reflect::get(&num_obj, &"denominator".into())
                .map_err(|_| "No denominator".to_string())?
                .as_string()
                .ok_or("Denominator not string")?;
            let fraction = apply_fraction_display_source(
                Fraction::new(
                    BigInt::from_str(&num_str).map_err(|e| e.to_string())?,
                    BigInt::from_str(&den_str).map_err(|e| e.to_string())?,
                ),
                &num_obj,
            );
            Ok(Value::from_datetime(fraction))
        }
        "string" => {
            let s = value_js.as_string().ok_or("Value not string")?;
            Ok(Value::from_string(&s))
        }
        "boolean" => {
            let b = value_js.as_bool().ok_or("Value not boolean")?;
            Ok(Value::from_bool(b))
        }
        "symbol" => {
            let s = value_js.as_string().ok_or("Value not string")?;
            Ok(Value::from_symbol(&s))
        }
        "vector" => {
            let js_array = js_sys::Array::from(&value_js);
            let mut vec = Vec::new();
            for i in 0..js_array.length() {
                vec.push(js_value_to_value(js_array.get(i))?);
            }
            Ok(Value::from_vector(vec))
        }
        "tensor" => {
            let tensor_obj = js_sys::Object::from(value_js);

            let data_js = js_sys::Reflect::get(&tensor_obj, &"data".into())
                .map_err(|_| "No data in tensor".to_string())?;
            let data_array = js_sys::Array::from(&data_js);
            let mut fractions = Vec::new();
            for i in 0..data_array.length() {
                let frac_obj = js_sys::Object::from(data_array.get(i));
                let num_str = js_sys::Reflect::get(&frac_obj, &"numerator".into())
                    .map_err(|_| "No numerator in tensor data".to_string())?
                    .as_string()
                    .ok_or("Numerator not string")?;
                let den_str = js_sys::Reflect::get(&frac_obj, &"denominator".into())
                    .map_err(|_| "No denominator in tensor data".to_string())?
                    .as_string()
                    .ok_or("Denominator not string")?;
                let fraction = apply_fraction_display_source(
                    Fraction::new(
                        BigInt::from_str(&num_str).map_err(|e| e.to_string())?,
                        BigInt::from_str(&den_str).map_err(|e| e.to_string())?,
                    ),
                    &frac_obj,
                );
                fractions.push(fraction);
            }

            let children: Vec<Value> = fractions.into_iter().map(Value::from_fraction).collect();

            Ok(Value::from_children(children))
        }
        "nil" => Ok(Value::nil()),
        "process_handle" => {
            let id = value_js.as_f64().ok_or("Process handle id is not number")? as u64;
            Ok(Value::from_process_handle(id))
        }
        "supervisor_handle" => {
            let id = value_js
                .as_f64()
                .ok_or("Supervisor handle id is not number")? as u64;
            Ok(Value::from_supervisor_handle(id))
        }
        _ => Err(format!("Unknown type: {}", type_str)),
    }
}

fn set_prop(obj: &js_sys::Object, key: &str, value: &JsValue) {
    js_sys::Reflect::set(obj, &key.into(), value).unwrap();
}

fn diagnosis_to_protocol_js(
    diagnosis: &crate::interpreter::debug_diagnosis::DebugDiagnosis,
) -> JsValue {
    let obj = js_sys::Object::new();
    set_prop(&obj, "when", &diagnosis.when.as_protocol_str().into());
    set_prop(&obj, "why", &diagnosis.why.as_protocol_str().into());
    set_prop(&obj, "summary", &diagnosis.summary.clone().into());

    let where_obj = js_sys::Object::new();
    set_prop(
        &where_obj,
        "kind",
        &diagnosis.where_.kind.as_protocol_str().into(),
    );
    if let Some(word) = &diagnosis.where_.word {
        set_prop(&where_obj, "word", &word.clone().into());
    }
    if let Some(module) = &diagnosis.where_.module {
        set_prop(&where_obj, "module", &module.clone().into());
    }
    if let Some(dictionary) = &diagnosis.where_.dictionary {
        set_prop(&where_obj, "dictionary", &dictionary.clone().into());
    }
    set_prop(&obj, "where", &where_obj.into());

    let evidence_arr = js_sys::Array::new();
    for item in &diagnosis.evidence {
        evidence_arr.push(&JsValue::from_str(item));
    }
    set_prop(&obj, "evidence", &evidence_arr.into());

    let checks_arr = js_sys::Array::new();
    for c in &diagnosis.next_checks {
        let check_obj = js_sys::Object::new();
        set_prop(&check_obj, "label", &c.label.clone().into());
        set_prop(&check_obj, "detail", &c.detail.clone().into());
        checks_arr.push(&check_obj);
    }
    set_prop(&obj, "nextChecks", &checks_arr.into());
    obj.into()
}

fn absence_to_protocol_js(absence: &crate::semantic::AbsenceMetadata) -> JsValue {
    let obj = js_sys::Object::new();
    if let Some(reason) = &absence.reason {
        set_prop(&obj, "reason", &reason.as_protocol_str().into());
        if let Some(category) = reason.caught_category() {
            set_prop(&obj, "caughtCategory", &category.as_protocol_str().into());
        }
    }
    set_prop(&obj, "origin", &absence.origin.as_protocol_str().into());
    set_prop(
        &obj,
        "recoverability",
        &absence.recoverability.as_protocol_str().into(),
    );
    if let Some(diagnosis) = &absence.diagnosis {
        set_prop(&obj, "diagnosis", &diagnosis_to_protocol_js(diagnosis));
    }
    obj.into()
}

fn value_semantics_to_js(value: &Value) -> JsValue {
    let obj = js_sys::Object::new();
    set_prop(
        &obj,
        "semanticKind",
        &value.semantic_kind().as_protocol_str().into(),
    );
    set_prop(&obj, "shape", &value.shape_kind().as_protocol_str().into());
    let capabilities = js_sys::Array::new();
    for capability in value.capabilities() {
        capabilities.push(&JsValue::from_str(capability.as_protocol_str()));
    }
    set_prop(&obj, "capabilities", &capabilities.into());
    set_prop(&obj, "origin", &value.origin().as_protocol_str().into());
    if let Some(absence) = value.normalized_absence_metadata() {
        set_prop(&obj, "absence", &absence_to_protocol_js(&absence));
    }
    obj.into()
}

fn set_fraction_display_source_prop(obj: &js_sys::Object, fraction: &crate::types::fraction::Fraction) {
    if let Some(source) = fraction.display_source() {
        set_prop(obj, "displaySource", &source.into());
    }
}

fn set_value_common_fields(obj: &js_sys::Object, value: &Value, hint: DisplayHint) {
    let hint_str: &str = match hint {
        DisplayHint::Auto => "auto",
        DisplayHint::Number => "number",
        DisplayHint::Interval => "interval",
        DisplayHint::String => "string",
        DisplayHint::Boolean => "boolean",
        DisplayHint::DateTime => "datetime",
        DisplayHint::Nil => "nil",
    };
    set_prop(obj, "displayHint", &hint_str.into());
    set_prop(obj, "semantics", &value_semantics_to_js(value));
}

pub(crate) fn value_to_js(value: &Value, external_hint_opt: Option<DisplayHint>) -> JsValue {
    let obj = js_sys::Object::new();
    let effective_hint = external_hint_opt.unwrap_or(value.hint);
    set_value_common_fields(&obj, value, effective_hint);

    match &value.data {
        crate::types::ValueData::Nil => {
            set_prop(&obj, "type", &"nil".into());
            set_prop(&obj, "value", &JsValue::NULL);
        }
        crate::types::ValueData::Scalar(f) => {
            let scalar_type = match effective_hint {
                DisplayHint::Boolean => "boolean",
                DisplayHint::DateTime => "datetime",
                DisplayHint::String => "string",
                _ => "number",
            };
            set_prop(&obj, "type", &scalar_type.into());
            match scalar_type {
                "boolean" => set_prop(&obj, "value", &(!f.is_zero()).into()),
                "string" => {
                    let as_char = f
                        .to_i64()
                        .and_then(|n| char::from_u32(n as u32))
                        .map(|c| c.to_string())
                        .unwrap_or_default();
                    set_prop(&obj, "value", &as_char.into());
                }
                _ => {
                    let num_obj = js_sys::Object::new();
                    set_prop(&num_obj, "numerator", &f.numerator().to_string().into());
                    set_prop(&num_obj, "denominator", &f.denominator().to_string().into());
                    set_fraction_display_source_prop(&num_obj, f);
                    set_prop(&obj, "value", &num_obj.into());
                }
            }
        }
        crate::types::ValueData::Vector(children) => {
            if effective_hint == DisplayHint::String {
                let text = children
                    .iter()
                    .filter_map(|child| match &child.data {
                        crate::types::ValueData::Scalar(codepoint) => {
                            codepoint.to_i64().and_then(|n| char::from_u32(n as u32))
                        }
                        _ => None,
                    })
                    .collect::<String>();
                set_prop(&obj, "type", &"string".into());
                set_prop(&obj, "value", &text.into());
            } else {
                let child_hint = match effective_hint {
                    DisplayHint::Boolean => Some(DisplayHint::Boolean),
                    _ => None,
                };
                let js_array = js_sys::Array::new();
                for child in children.iter() {
                    js_array.push(&value_to_js(child, child_hint));
                }
                set_prop(&obj, "type", &"vector".into());
                set_prop(&obj, "value", &js_array.into());
            }
        }
        crate::types::ValueData::Tensor { data, shape } => {
            if effective_hint == DisplayHint::String && shape.len() <= 1 {
                let text: String = data
                    .iter()
                    .filter_map(|f| f.to_i64().and_then(|n| char::from_u32(n as u32)))
                    .collect();
                set_prop(&obj, "type", &"string".into());
                set_prop(&obj, "value", &text.into());
            } else {
                let tensor_values = data.to_fractions();
                let js_array = tensor_data_to_js_array(&tensor_values, shape);
                set_prop(&obj, "type", &"vector".into());
                set_prop(&obj, "value", &js_array.into());
            }
        }
        crate::types::ValueData::Record { pairs, .. } => {
            let js_array = js_sys::Array::new();
            for pair in pairs.iter() {
                js_array.push(&value_to_js(pair, None));
            }
            set_prop(&obj, "type", &"vector".into());
            set_prop(&obj, "value", &js_array.into());
        }
        crate::types::ValueData::CodeBlock(_) => {
            set_prop(&obj, "type", &"nil".into());
            set_prop(&obj, "value", &JsValue::NULL);
        }
        crate::types::ValueData::ProcessHandle(id) => {
            set_prop(&obj, "type", &"process_handle".into());
            set_prop(&obj, "value", &(*id as f64).into());
        }
        crate::types::ValueData::SupervisorHandle(id) => {
            set_prop(&obj, "type", &"supervisor_handle".into());
            set_prop(&obj, "value", &(*id as f64).into());
        }
    }
    obj.into()
}

fn tensor_data_to_js_array(
    data: &[crate::types::fraction::Fraction],
    shape: &[usize],
) -> js_sys::Array {
    let arr = js_sys::Array::new();
    if shape.is_empty() || shape.len() == 1 {
        for f in data {
            let num_obj = js_sys::Object::new();
            js_sys::Reflect::set(
                &num_obj,
                &"numerator".into(),
                &f.numerator().to_string().into(),
            )
            .unwrap();
            js_sys::Reflect::set(
                &num_obj,
                &"denominator".into(),
                &f.denominator().to_string().into(),
            )
            .unwrap();
            set_fraction_display_source_prop(&num_obj, f);
            let elem = js_sys::Object::new();
            js_sys::Reflect::set(&elem, &"type".into(), &"number".into()).unwrap();
            js_sys::Reflect::set(&elem, &"value".into(), &num_obj).unwrap();
            js_sys::Reflect::set(&elem, &"displayHint".into(), &"number".into()).unwrap();
            let element_value = Value::from_fraction(f.clone());
            js_sys::Reflect::set(
                &elem,
                &"semantics".into(),
                &value_semantics_to_js(&element_value),
            )
            .unwrap();
            arr.push(&elem);
        }
    } else {
        let outer = shape[0];
        let rest = &shape[1..];
        let stride: usize = rest.iter().product();
        for i in 0..outer {
            let inner = tensor_data_to_js_array(&data[i * stride..(i + 1) * stride], rest);
            let elem = js_sys::Object::new();
            js_sys::Reflect::set(&elem, &"type".into(), &"vector".into()).unwrap();
            js_sys::Reflect::set(&elem, &"value".into(), &inner).unwrap();
            js_sys::Reflect::set(&elem, &"displayHint".into(), &"auto".into()).unwrap();
            arr.push(&elem);
        }
    }
    arr
}

#[allow(dead_code)]
pub(crate) fn arena_node_to_js(
    arena: &ValueArena,
    root_id: NodeId,
    external_hint_opt: Option<DisplayHint>,
) -> JsValue {
    let obj = js_sys::Object::new();
    // external_hint_opt が無い場合は必ず Arena 側の hint を参照する。
    // 子ノード再帰では None を渡し、各 NodeId の明示 hint を尊重する。
    let effective_hint = resolve_effective_hint(arena, root_id, external_hint_opt);

    let hint_str: &str = match effective_hint {
        DisplayHint::Auto => "auto",
        DisplayHint::Number => "number",
        DisplayHint::Interval => "interval",
        DisplayHint::String => "string",
        DisplayHint::Boolean => "boolean",
        DisplayHint::DateTime => "datetime",
        DisplayHint::Nil => "nil",
    };
    js_sys::Reflect::set(&obj, &"displayHint".into(), &hint_str.into()).unwrap();

    match arena.kind(root_id) {
        NodeKind::Nil => {
            js_sys::Reflect::set(&obj, &"type".into(), &"nil".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &JsValue::NULL).unwrap();
        }
        NodeKind::Scalar(f) => {
            let scalar_type = match effective_hint {
                DisplayHint::Boolean => "boolean",
                DisplayHint::DateTime => "datetime",
                DisplayHint::String => "string",
                _ => "number",
            };
            js_sys::Reflect::set(&obj, &"type".into(), &scalar_type.into()).unwrap();
            match scalar_type {
                "boolean" => {
                    js_sys::Reflect::set(&obj, &"value".into(), &(!f.is_zero()).into()).unwrap();
                }
                "string" => {
                    let as_char = f
                        .to_i64()
                        .and_then(|n| char::from_u32(n as u32))
                        .map(|c| c.to_string())
                        .unwrap_or_default();
                    js_sys::Reflect::set(&obj, &"value".into(), &as_char.into()).unwrap();
                }
                _ => {
                    let num_obj = js_sys::Object::new();
                    js_sys::Reflect::set(
                        &num_obj,
                        &"numerator".into(),
                        &f.numerator().to_string().into(),
                    )
                    .unwrap();
                    js_sys::Reflect::set(
                        &num_obj,
                        &"denominator".into(),
                        &f.denominator().to_string().into(),
                    )
                    .unwrap();
                    set_fraction_display_source_prop(&num_obj, f);
                    js_sys::Reflect::set(&obj, &"value".into(), &num_obj).unwrap();
                }
            }
        }
        NodeKind::Vector { children } => {
            if effective_hint == DisplayHint::String {
                let text = children
                    .iter()
                    .filter_map(|child| match arena.kind(*child) {
                        NodeKind::Scalar(codepoint) => {
                            codepoint.to_i64().and_then(|n| char::from_u32(n as u32))
                        }
                        _ => None,
                    })
                    .collect::<String>();
                js_sys::Reflect::set(&obj, &"type".into(), &"string".into()).unwrap();
                js_sys::Reflect::set(&obj, &"value".into(), &text.into()).unwrap();
            } else {
                let child_external: Option<DisplayHint> = match effective_hint {
                    DisplayHint::Boolean => Some(DisplayHint::Boolean),
                    _ => None,
                };
                let js_array = js_sys::Array::new();
                for child in children {
                    js_array.push(&arena_node_to_js(arena, *child, child_external));
                }
                js_sys::Reflect::set(&obj, &"type".into(), &"vector".into()).unwrap();
                js_sys::Reflect::set(&obj, &"value".into(), &js_array).unwrap();
            }
        }
        NodeKind::Tensor { data, shape } => {
            // Hydrate a dense Tensor at the WASM boundary so the GUI/TS layer
            // can keep treating values uniformly as nested Vectors.
            if effective_hint == DisplayHint::String && shape.len() <= 1 {
                let text: String = data
                    .iter()
                    .filter_map(|f| f.to_i64().and_then(|n| char::from_u32(n as u32)))
                    .collect();
                js_sys::Reflect::set(&obj, &"type".into(), &"string".into()).unwrap();
                js_sys::Reflect::set(&obj, &"value".into(), &text.into()).unwrap();
            } else {
                let js_array = tensor_data_to_js_array(data, shape);
                js_sys::Reflect::set(&obj, &"type".into(), &"vector".into()).unwrap();
                js_sys::Reflect::set(&obj, &"value".into(), &js_array).unwrap();
            }
        }
        NodeKind::Record { pairs, .. } => {
            let js_array = js_sys::Array::new();
            for pair_id in pairs {
                js_array.push(&arena_node_to_js(arena, *pair_id, None));
            }
            js_sys::Reflect::set(&obj, &"type".into(), &"vector".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &js_array).unwrap();
        }
        NodeKind::CodeBlock(_) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"nil".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &JsValue::NULL).unwrap();
        }
        NodeKind::ProcessHandle(id) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"process_handle".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &(*id as f64).into()).unwrap();
        }
        NodeKind::SupervisorHandle(id) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"supervisor_handle".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &(*id as f64).into()).unwrap();
        }
    }

    obj.into()
}

#[allow(dead_code)]
fn resolve_effective_hint(
    arena: &ValueArena,
    root_id: NodeId,
    external_hint_opt: Option<DisplayHint>,
) -> DisplayHint {
    external_hint_opt.unwrap_or_else(|| arena.hint(root_id))
}

pub(crate) fn extract_display_hint_from_js(js_val: &JsValue) -> DisplayHint {
    let obj = js_sys::Object::from(js_val.clone());
    let hint_js = js_sys::Reflect::get(&obj, &"displayHint".into()).unwrap_or(JsValue::UNDEFINED);
    match hint_js.as_string().as_deref() {
        Some("number") => DisplayHint::Number,
        Some("interval") => DisplayHint::Interval,
        Some("string") => DisplayHint::String,
        Some("boolean") => DisplayHint::Boolean,
        Some("datetime") => DisplayHint::DateTime,
        Some("nil") => DisplayHint::Nil,
        _ => DisplayHint::Auto,
    }
}

#[cfg(test)]
mod test_input_helper {
    use super::{build_bracket_structure_from_shape, resolve_effective_hint};
    use crate::types::arena::ValueArena;
    use crate::types::DisplayHint;

    #[test]
    fn test_build_bracket_structure_from_shape() {
        assert_eq!(build_bracket_structure_from_shape(&[1]), "[ ]");
        assert_eq!(build_bracket_structure_from_shape(&[2]), "[ ] [ ]");
        assert_eq!(build_bracket_structure_from_shape(&[3]), "[ ] [ ] [ ]");

        assert_eq!(build_bracket_structure_from_shape(&[1, 1]), "[ [ ] ]");
        assert_eq!(build_bracket_structure_from_shape(&[1, 2]), "[ [ ] [ ] ]");
        assert_eq!(
            build_bracket_structure_from_shape(&[1, 3]),
            "[ [ ] [ ] [ ] ]"
        );
        assert_eq!(
            build_bracket_structure_from_shape(&[2, 3]),
            "[ [ ] [ ] [ ] ] [ [ ] [ ] [ ] ]"
        );

        assert_eq!(
            build_bracket_structure_from_shape(&[1, 1, 1]),
            "[ [ [ ] ] ]"
        );
        assert_eq!(
            build_bracket_structure_from_shape(&[1, 1, 2]),
            "[ [ [ ] [ ] ] ]"
        );
        assert_eq!(
            build_bracket_structure_from_shape(&[1, 2, 3]),
            "[ [ [ ] [ ] [ ] ] [ [ ] [ ] [ ] ] ]"
        );
        assert_eq!(
            build_bracket_structure_from_shape(&[2, 2, 3]),
            "[ [ [ ] [ ] [ ] ] [ [ ] [ ] [ ] ] ] [ [ [ ] [ ] [ ] ] [ [ ] [ ] [ ] ] ]"
        );

        assert_eq!(
            build_bracket_structure_from_shape(&[1, 1, 1, 1]),
            "[ [ [ [ ] ] ] ]"
        );
    }

    #[test]
    fn effective_hint_prefers_external_otherwise_uses_arena() {
        let mut arena = ValueArena::new();
        let id = arena.alloc_string("AB");
        assert_eq!(
            resolve_effective_hint(&arena, id, None),
            DisplayHint::String
        );
        assert_eq!(
            resolve_effective_hint(&arena, id, Some(DisplayHint::Number)),
            DisplayHint::Number
        );
    }
}

// AQ-VER-003: WASM boundary MC/DC tests for QL-B pure helpers.
//
// Scope: the JS-bridge conversion layer is reachable natively only for
// its pure helpers (`resolve_effective_hint`,
// `build_bracket_structure_from_shape`). JsValue-based entry points
// (`js_value_to_value`, `arena_node_to_js`, `extract_display_hint_from_js`)
// exercise `wasm_bindgen` runtime glue and are verified by the
// `cargo check --target wasm32-unknown-unknown` step in
// `.github/workflows/test.yml` (AQ-REQ-003). They are intentionally not
// asserted here.
//
// Trace: docs/quality/TRACEABILITY_MATRIX.md, requirement AQ-REQ-003.
#[cfg(test)]
mod mcdc_tests {
    use super::{build_bracket_structure_from_shape, resolve_effective_hint};
    use crate::types::arena::ValueArena;
    use crate::types::DisplayHint;

    // AQ-VER-003-A
    // DUT: `resolve_effective_hint`
    //     external_hint_opt.unwrap_or_else(|| arena.hint(root_id))
    //
    // One atomic condition C = external_hint_opt.is_some().
    //   row 1: C=T -> return external value verbatim
    //   row 2: C=F -> fall back to arena hint
    //
    // Additional row 3 pins that C=T ignores the arena hint even when
    // the external value disagrees — this matters because a caller
    // passing an explicit hint must win over arena state.
    mod aq_ver_003_a_resolve_effective_hint {
        use super::*;

        #[test]
        fn row1_some_external_is_returned_verbatim() {
            let mut arena = ValueArena::new();
            let id = arena.alloc_nil(DisplayHint::Number);
            assert_eq!(
                resolve_effective_hint(&arena, id, Some(DisplayHint::Boolean)),
                DisplayHint::Boolean,
            );
        }

        #[test]
        fn row2_none_falls_back_to_arena_hint() {
            let mut arena = ValueArena::new();
            let id = arena.alloc_nil(DisplayHint::DateTime);
            assert_eq!(
                resolve_effective_hint(&arena, id, None),
                DisplayHint::DateTime,
            );
        }

        #[test]
        fn external_hint_wins_even_when_arena_disagrees() {
            // Guards against a regression where the fallback arm is
            // evaluated eagerly and overwrites the external value.
            let mut arena = ValueArena::new();
            let id = arena.alloc_nil(DisplayHint::Number);
            assert_eq!(
                resolve_effective_hint(&arena, id, Some(DisplayHint::String)),
                DisplayHint::String,
            );
        }
    }

    // AQ-VER-003-B
    // DUT: `build_bracket_structure_from_shape`
    //
    // Outer decision: `if shape.is_empty()` — one atomic condition.
    //   row 1: empty shape -> literal "[ ]"
    //   row 2: non-empty shape -> recurse
    //
    // Inner decision (in `build_level`): `if shape.len() == 1`.
    //   row 3: tail dimension -> emit `[ ]` repeated `shape[0]` times
    //   row 4: non-tail dimension -> wrap the inner level
    //
    // The existing `test_build_bracket_structure_from_shape` covers
    // several combinations in row 3/4 already. This module adds the
    // outer-empty boundary (row 1), which was previously untested, and
    // asserts the leaf-count invariant to make the MC/DC intent explicit.
    mod aq_ver_003_b_bracket_structure {
        use super::*;

        #[test]
        fn row1_empty_shape_returns_single_pair() {
            assert_eq!(build_bracket_structure_from_shape(&[]), "[ ]");
        }

        #[test]
        fn row2_single_dim_emits_n_leaves() {
            // Complements row 1 by flipping `shape.is_empty()`.
            let out = build_bracket_structure_from_shape(&[4]);
            assert_eq!(out, "[ ] [ ] [ ] [ ]");
            assert_eq!(
                out.matches("[ ]").count(),
                4,
                "leaf count must equal shape[0] on the tail dimension"
            );
        }

        #[test]
        fn row3_row4_multi_dim_wraps_inner_levels() {
            // Non-tail dimension wraps tail output in brackets.
            // Shape [2, 3]: 2 outer frames, each containing 3 leaves.
            let out = build_bracket_structure_from_shape(&[2, 3]);
            assert_eq!(out, "[ [ ] [ ] [ ] ] [ [ ] [ ] [ ] ]");
            assert_eq!(
                out.matches("[ ]").count(),
                6,
                "leaf count must equal the product of non-head dims"
            );
        }
    }
}
