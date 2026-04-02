use crate::types::display::is_string_like;
use crate::types::fraction::Fraction;
use crate::types::{DisplayHint, Value, ValueData};
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

pub(crate) fn is_string_value(val: &Value) -> bool {
    match &val.data {
        ValueData::Vector(children) => !children.is_empty() && is_string_like(children),
        _ => false,
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

pub(crate) fn is_vector_value(val: &Value) -> bool {
    val.is_vector()
}

pub(crate) fn value_as_string(val: &Value) -> String {
    match &val.data {
        ValueData::Vector(children) | ValueData::Record { pairs: children, .. } => {
            children
                .iter()
                .filter_map(|child| {
                    if let ValueData::Scalar(f) = &child.data {
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
                })
                .collect()
        }
        ValueData::Scalar(f) => {
            if let Some(n) = f.to_i64() {
                if n >= 0 && n <= 0x10FFFF {
                    if let Some(c) = char::from_u32(n as u32) {
                        return c.to_string();
                    }
                }
            }
            String::new()
        }
        _ => String::new(),
    }
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
            let fraction = Fraction::new(
                BigInt::from_str(&num_str).map_err(|e| e.to_string())?,
                BigInt::from_str(&den_str).map_err(|e| e.to_string())?,
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
            let fraction = Fraction::new(
                BigInt::from_str(&num_str).map_err(|e| e.to_string())?,
                BigInt::from_str(&den_str).map_err(|e| e.to_string())?,
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
                let fraction = Fraction::new(
                    BigInt::from_str(&num_str).map_err(|e| e.to_string())?,
                    BigInt::from_str(&den_str).map_err(|e| e.to_string())?,
                );
                fractions.push(fraction);
            }

            let children: Vec<Value> = fractions.into_iter().map(Value::from_fraction).collect();

            Ok(Value::from_children(children))
        }
        "nil" => Ok(Value::nil()),
        _ => Err(format!("Unknown type: {}", type_str)),
    }
}

pub(crate) fn value_to_js_value(value: &Value) -> JsValue {
    value_to_js_value_with_hint(value, DisplayHint::Auto)
}

pub(crate) fn value_to_js_value_with_hint(value: &Value, hint: DisplayHint) -> JsValue {
    let obj = js_sys::Object::new();

    if value.is_nil() {
        js_sys::Reflect::set(&obj, &"type".into(), &"nil".into()).unwrap();
        js_sys::Reflect::set(&obj, &"value".into(), &JsValue::NULL).unwrap();
        js_sys::Reflect::set(&obj, &"displayHint".into(), &"nil".into()).unwrap();
        return obj.into();
    }

    let type_str: &str = match hint {
        DisplayHint::String => {
            if value.is_scalar() || is_string_value(value) {
                "string"
            } else if is_vector_value(value) {
                "vector"
            } else {
                "number"
            }
        }
        DisplayHint::Boolean => {
            if value.is_scalar() {
                "boolean"
            } else if is_vector_value(value) {
                "vector"
            } else {
                "number"
            }
        }
        DisplayHint::DateTime => {
            if value.is_scalar() {
                "datetime"
            } else {
                "number"
            }
        }
        DisplayHint::Number => "number",
        DisplayHint::Nil => "nil",
        DisplayHint::Auto => {
            // Fallback to heuristic detection for Auto hint
            if is_datetime_value(value) {
                "datetime"
            } else if is_boolean_value(value) {
                "boolean"
            } else if is_string_value(value) {
                "string"
            } else if is_number_value(value) {
                "number"
            } else if is_vector_value(value) {
                "vector"
            } else if value.is_scalar() {
                "number"
            } else {
                "nil"
            }
        }
    };

    let hint_str: &str = match hint {
        DisplayHint::Auto => "auto",
        DisplayHint::Number => "number",
        DisplayHint::String => "string",
        DisplayHint::Boolean => "boolean",
        DisplayHint::DateTime => "datetime",
        DisplayHint::Nil => "nil",
    };

    js_sys::Reflect::set(&obj, &"type".into(), &type_str.into()).unwrap();
    js_sys::Reflect::set(&obj, &"displayHint".into(), &hint_str.into()).unwrap();

    match type_str {
        "number" | "datetime" => {
            if let Some(f) = value.as_scalar() {
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
                js_sys::Reflect::set(&obj, &"value".into(), &num_obj).unwrap();
            }
        }
        "string" => {
            let s = value_as_string(value);
            js_sys::Reflect::set(&obj, &"value".into(), &s.into()).unwrap();
        }
        "boolean" => {
            if let Some(f) = value.as_scalar() {
                let b = !f.is_zero();
                js_sys::Reflect::set(&obj, &"value".into(), &b.into()).unwrap();
            }
        }
        "vector" => {
            let js_array = js_sys::Array::new();
            if let ValueData::Vector(children) = &value.data {
                for child in children.iter() {
                    js_array.push(&value_to_js_value(child));
                }
            }
            js_sys::Reflect::set(&obj, &"value".into(), &js_array).unwrap();
        }
        _ => {}
    };

    obj.into()
}

pub(crate) fn extract_display_hint_from_js(js_val: &JsValue) -> DisplayHint {
    let obj = js_sys::Object::from(js_val.clone());
    let hint_js = js_sys::Reflect::get(&obj, &"displayHint".into()).unwrap_or(JsValue::UNDEFINED);
    match hint_js.as_string().as_deref() {
        Some("number") => DisplayHint::Number,
        Some("string") => DisplayHint::String,
        Some("boolean") => DisplayHint::Boolean,
        Some("datetime") => DisplayHint::DateTime,
        Some("nil") => DisplayHint::Nil,
        _ => DisplayHint::Auto,
    }
}

#[cfg(test)]
mod test_input_helper {
    use super::build_bracket_structure_from_shape;

    #[test]
    fn test_build_bracket_structure_from_shape() {
        assert_eq!(build_bracket_structure_from_shape(&[1]), "[ ]");
        assert_eq!(build_bracket_structure_from_shape(&[2]), "[ ] [ ]");
        assert_eq!(build_bracket_structure_from_shape(&[3]), "[ ] [ ] [ ]");

        assert_eq!(build_bracket_structure_from_shape(&[1, 1]), "[ [ ] ]");
        assert_eq!(
            build_bracket_structure_from_shape(&[1, 2]),
            "[ [ ] [ ] ]"
        );
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

        // 4D: still [ ] with deeper nesting
        assert_eq!(
            build_bracket_structure_from_shape(&[1, 1, 1, 1]),
            "[ [ [ [ ] ] ] ]"
        );
    }
}
