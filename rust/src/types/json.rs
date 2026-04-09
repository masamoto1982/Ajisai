use serde_json;
use std::collections::HashMap;
use std::rc::Rc;
use crate::types::{Value, ValueData};
use crate::types::fraction::Fraction;
use crate::error::Result;
use num_traits::ToPrimitive;

pub fn deserialize_json_to_value(json_val: serde_json::Value, depth: usize) -> Result<Value> {
    match json_val {
        serde_json::Value::Null => Ok(Value::nil()),

        serde_json::Value::Bool(b) => Ok(Value::from_bool(b)),

        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::from_int(i))
            } else if let Some(f) = n.as_f64() {
                let s = format!("{}", f);
                let frac = Fraction::from_str(&s).unwrap_or_else(|_| {
                    Fraction::from(f as i64)
                });
                Ok(Value::from_fraction(frac))
            } else {
                Ok(Value::nil())
            }
        }

        serde_json::Value::String(s) => Ok(Value::from_string(&s)),

        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                return Ok(Value::nil());
            }
            let mut values = Vec::with_capacity(arr.len());
            for item in arr {
                values.push(deserialize_json_to_value(item, depth + 1)?);
            }
            Ok(Value {
                data: ValueData::Vector(Rc::new(values)),
            })
        }

        serde_json::Value::Object(map) => {
            if map.is_empty() {
                return Ok(Value::nil());
            }
            let mut pairs = Vec::with_capacity(map.len());
            let mut index = HashMap::with_capacity(map.len());
            for (key, val) in map {
                index.insert(key.clone(), pairs.len());
                let key_val = Value::from_string(&key);
                let val_val = deserialize_json_to_value(val, depth + 1)?;
                pairs.push(Value {
                    data: ValueData::Vector(Rc::new(vec![key_val, val_val])),
                });
            }
            Ok(Value {
                data: ValueData::Record { pairs: Rc::new(pairs), index },
            })
        }
    }
}

pub fn serialize_value_to_json(val: &Value) -> serde_json::Value {
    match &val.data {
        ValueData::Nil => serde_json::Value::Null,

        ValueData::Scalar(f) => {
            if f.is_integer() {
                if let Some(i) = f.to_i64() {
                    return serde_json::Value::Number(serde_json::Number::from(i));
                }
            }
            if let (Some(n), Some(d)) = (f.numerator().to_i64(), f.denominator().to_i64()) {
                let float_val = n as f64 / d as f64;
                if let Some(num) = serde_json::Number::from_f64(float_val) {
                    return serde_json::Value::Number(num);
                }
            }
            serde_json::Value::Null
        }

        ValueData::Record { pairs, .. } => {
            let mut map = serde_json::Map::new();
            for pair in pairs.iter() {
                if let ValueData::Vector(kv) = &pair.data {
                    if kv.len() == 2 {
                        let key = extract_string_content_from_value(&kv[0]);
                        let val_json = serialize_value_to_json(&kv[1]);
                        map.insert(key, val_json);
                    }
                }
            }
            serde_json::Value::Object(map)
        }

        ValueData::Vector(children) => {
            if is_string_like(children) {
                let s: String = children.iter().filter_map(|c| {
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
                return serde_json::Value::String(s);
            }

            if is_json_object(children) {
                let mut map = serde_json::Map::new();
                for pair in children.iter() {
                    if let ValueData::Vector(kv) = &pair.data {
                        if kv.len() == 2 {
                            let key = extract_string_content_from_value(&kv[0]);
                            let val_json = serialize_value_to_json(&kv[1]);
                            map.insert(key, val_json);
                        }
                    }
                }
                return serde_json::Value::Object(map);
            }

            let arr: Vec<serde_json::Value> = children.iter().map(|c| serialize_value_to_json(c)).collect();
            serde_json::Value::Array(arr)
        }

        ValueData::CodeBlock(_) => serde_json::Value::Null,
    }
}

fn is_json_object(children: &[Value]) -> bool {
    if children.is_empty() {
        return false;
    }
    children.iter().all(|child| {
        if let ValueData::Vector(kv) = &child.data {
            kv.len() == 2 && is_string_value(&kv[0])
        } else {
            false
        }
    })
}

fn is_string_like(children: &[Value]) -> bool {
    children.len() > 1 && children.iter().all(|c| {
        if let ValueData::Scalar(f) = &c.data {
            if let Some(n) = f.to_i64() {
                if n >= 0 && n <= 0x10FFFF {
                    if let Some(ch) = char::from_u32(n as u32) {
                        !ch.is_control() || ch == '\n' || ch == '\r' || ch == '\t'
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    })
}

fn is_string_value(val: &Value) -> bool {
    if let ValueData::Vector(chars) = &val.data {
        is_string_like(chars)
    } else {
        false
    }
}

fn extract_string_content_from_value(val: &Value) -> String {
    if let ValueData::Vector(chars) = &val.data {
        chars.iter().filter_map(|c| {
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
        }).collect()
    } else {
        format!("{}", val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_json_to_value_null() {
        let val = deserialize_json_to_value(serde_json::Value::Null, 1).unwrap();
        assert!(val.is_nil());
    }

    #[test]
    fn test_deserialize_json_to_value_bool() {
        let val = deserialize_json_to_value(serde_json::Value::Bool(true), 1).unwrap();
        assert!(!val.as_scalar().unwrap().is_zero());
    }

    #[test]
    fn test_deserialize_json_to_value_integer() {
        let val = deserialize_json_to_value(serde_json::json!(42), 1).unwrap();
        assert_eq!(val.as_scalar().unwrap().to_i64(), Some(42));
    }

    #[test]
    fn test_deserialize_json_to_value_float() {
        let val = deserialize_json_to_value(serde_json::json!(1.5), 1).unwrap();
        let f = val.as_scalar().unwrap();
        assert!(f.is_integer() == false);
    }

    #[test]
    fn test_deserialize_json_to_value_string() {
        let val = deserialize_json_to_value(serde_json::json!("hello"), 1).unwrap();
        assert!(val.is_vector());
    }

    #[test]
    fn test_deserialize_json_to_value_array() {
        let val = deserialize_json_to_value(serde_json::json!([1, 2, 3]), 1).unwrap();
        assert!(val.is_vector());
        assert_eq!(val.len(), 3);
    }

    #[test]
    fn test_deserialize_json_to_value_empty_array() {
        let val = deserialize_json_to_value(serde_json::json!([]), 1).unwrap();
        assert!(val.is_nil());
    }

    #[test]
    fn test_deserialize_json_to_value_object() {
        let val = deserialize_json_to_value(serde_json::json!({"name": "Ajisai"}), 1).unwrap();
        assert!(val.is_vector());
        if let ValueData::Record { pairs, .. } = &val.data {
            assert_eq!(pairs.len(), 1);
            if let ValueData::Vector(kv) = &pairs[0].data {
                assert_eq!(kv.len(), 2);
            }
        }
    }

    #[test]
    fn test_deserialize_json_to_value_deep_nesting() {
        let deep = serde_json::json!([[[[[[[[[[1]]]]]]]]]]);
        let result = deserialize_json_to_value(deep, 1);
        assert!(result.is_ok());

        let deeper = serde_json::json!([[[[[[[[[[[1]]]]]]]]]]]);
        let result = deserialize_json_to_value(deeper, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_value_to_json_nil() {
        let val = Value::nil();
        assert_eq!(serialize_value_to_json(&val), serde_json::Value::Null);
    }

    #[test]
    fn test_serialize_value_to_json_integer() {
        let val = Value::from_int(42);
        assert_eq!(serialize_value_to_json(&val), serde_json::json!(42));
    }

    #[test]
    fn test_serialize_value_to_json_bool() {


        let val = Value::from_bool(true);
        assert_eq!(serialize_value_to_json(&val), serde_json::json!(1));
    }

    #[test]
    fn test_serialize_value_to_json_string() {
        let val = Value::from_string("hello");
        assert_eq!(serialize_value_to_json(&val), serde_json::json!("hello"));
    }

    #[test]
    fn test_serialize_value_to_json_array() {


        let val = Value {
            data: ValueData::Vector(Rc::new(vec![
                Value::from_int(-1),
                Value::from_int(-2),
                Value::from_int(-3),
            ])),
        };
        assert_eq!(serialize_value_to_json(&val), serde_json::json!([-1, -2, -3]));
    }

    #[test]
    fn test_roundtrip_array() {



        let json = serde_json::json!([-1, -2, -3]);
        let val = deserialize_json_to_value(json.clone(), 1).unwrap();
        let back = serialize_value_to_json(&val);
        assert_eq!(back, json);
    }

    #[test]
    fn test_roundtrip_object() {
        let json = serde_json::json!({"name": "Ajisai", "version": 1});
        let val = deserialize_json_to_value(json.clone(), 1).unwrap();
        let back = serialize_value_to_json(&val);
        let back_obj = back.as_object().unwrap();
        assert_eq!(back_obj.get("name"), Some(&serde_json::json!("Ajisai")));
        assert_eq!(back_obj.get("version"), Some(&serde_json::json!(1)));
    }
}
