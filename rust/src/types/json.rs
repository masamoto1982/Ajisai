use serde_json;
use crate::types::{Value, ValueData, DisplayHint, MAX_VISIBLE_DIMENSIONS};
use crate::types::fraction::Fraction;
use crate::error::{Result, AjisaiError};
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};

pub fn from_json(json_val: serde_json::Value, depth: usize) -> Result<Value> {
    match json_val {
        serde_json::Value::Null => Ok(Value::nil()),

        serde_json::Value::Bool(b) => Ok(Value::from_bool(b)),

        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::from_int(i))
            } else if let Some(f) = n.as_f64() {
                let s = format!("{}", f);
                let frac = Fraction::from_str(&s).unwrap_or_else(|_| {
                    Fraction {
                        numerator: BigInt::from(f as i64),
                        denominator: BigInt::one(),
                    }
                });
                Ok(Value::from_fraction(frac))
            } else {
                Ok(Value::nil())
            }
        }

        serde_json::Value::String(s) => Ok(Value::from_string(&s)),

        serde_json::Value::Array(arr) => {
            if depth > MAX_VISIBLE_DIMENSIONS + 1 {
                return Err(AjisaiError::DimensionLimitExceeded { depth });
            }
            if arr.is_empty() {
                return Ok(Value::nil());
            }
            let mut values = Vec::with_capacity(arr.len());
            for item in arr {
                values.push(from_json(item, depth + 1)?);
            }
            Ok(Value {
                data: ValueData::Vector(values),
                display_hint: DisplayHint::Auto,
                audio_hint: None,
            })
        }

        serde_json::Value::Object(map) => {
            if depth > MAX_VISIBLE_DIMENSIONS + 1 {
                return Err(AjisaiError::DimensionLimitExceeded { depth });
            }
            if map.is_empty() {
                return Ok(Value::nil());
            }
            let mut pairs = Vec::with_capacity(map.len());
            for (key, val) in map {
                let key_val = Value::from_string(&key);
                let val_val = from_json(val, depth + 1)?;
                pairs.push(Value {
                    data: ValueData::Vector(vec![key_val, val_val]),
                    display_hint: DisplayHint::Auto,
                    audio_hint: None,
                });
            }
            Ok(Value {
                data: ValueData::Vector(pairs),
                display_hint: DisplayHint::Auto,
                audio_hint: None,
            })
        }
    }
}

pub fn to_json(val: &Value) -> serde_json::Value {
    match &val.data {
        ValueData::Nil => serde_json::Value::Null,

        ValueData::Scalar(f) => {
            if val.display_hint == DisplayHint::Boolean {
                return serde_json::Value::Bool(!f.is_zero());
            }
            if f.is_integer() {
                if let Some(i) = f.to_i64() {
                    return serde_json::Value::Number(serde_json::Number::from(i));
                }
            }
            if let (Some(n), Some(d)) = (f.numerator.to_i64(), f.denominator.to_i64()) {
                let float_val = n as f64 / d as f64;
                if let Some(num) = serde_json::Number::from_f64(float_val) {
                    return serde_json::Value::Number(num);
                }
            }
            serde_json::Value::Null
        }

        ValueData::Vector(children) => {
            if val.display_hint == DisplayHint::String {
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
                for pair in children {
                    if let ValueData::Vector(kv) = &pair.data {
                        if kv.len() == 2 {
                            let key = value_to_string_content(&kv[0]);
                            let val_json = to_json(&kv[1]);
                            map.insert(key, val_json);
                        }
                    }
                }
                return serde_json::Value::Object(map);
            }

            let arr: Vec<serde_json::Value> = children.iter().map(|c| to_json(c)).collect();
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
            kv.len() == 2 && matches!(kv[0].display_hint, DisplayHint::String)
        } else {
            false
        }
    })
}

fn value_to_string_content(val: &Value) -> String {
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
    fn test_from_json_null() {
        let val = from_json(serde_json::Value::Null, 1).unwrap();
        assert!(val.is_nil());
    }

    #[test]
    fn test_from_json_bool() {
        let val = from_json(serde_json::Value::Bool(true), 1).unwrap();
        assert_eq!(val.display_hint, DisplayHint::Boolean);
        assert!(!val.as_scalar().unwrap().is_zero());
    }

    #[test]
    fn test_from_json_integer() {
        let val = from_json(serde_json::json!(42), 1).unwrap();
        assert_eq!(val.as_scalar().unwrap().to_i64(), Some(42));
    }

    #[test]
    fn test_from_json_float() {
        let val = from_json(serde_json::json!(1.5), 1).unwrap();
        let f = val.as_scalar().unwrap();
        assert!(f.is_integer() == false);
    }

    #[test]
    fn test_from_json_string() {
        let val = from_json(serde_json::json!("hello"), 1).unwrap();
        assert_eq!(val.display_hint, DisplayHint::String);
        assert!(val.is_vector());
    }

    #[test]
    fn test_from_json_array() {
        let val = from_json(serde_json::json!([1, 2, 3]), 1).unwrap();
        assert!(val.is_vector());
        assert_eq!(val.len(), 3);
    }

    #[test]
    fn test_from_json_empty_array() {
        let val = from_json(serde_json::json!([]), 1).unwrap();
        assert!(val.is_nil());
    }

    #[test]
    fn test_from_json_object() {
        let val = from_json(serde_json::json!({"name": "Ajisai"}), 1).unwrap();
        assert!(val.is_vector());
        if let ValueData::Vector(pairs) = &val.data {
            assert_eq!(pairs.len(), 1);
            if let ValueData::Vector(kv) = &pairs[0].data {
                assert_eq!(kv.len(), 2);
                assert_eq!(kv[0].display_hint, DisplayHint::String);
            }
        }
    }

    #[test]
    fn test_from_json_nest_limit() {
        let deep = serde_json::json!([[[[[[[[[[1]]]]]]]]]]);
        let result = from_json(deep, 1);
        assert!(result.is_ok());

        let too_deep = serde_json::json!([[[[[[[[[[[1]]]]]]]]]]]);
        let result = from_json(too_deep, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_to_json_nil() {
        let val = Value::nil();
        assert_eq!(to_json(&val), serde_json::Value::Null);
    }

    #[test]
    fn test_to_json_integer() {
        let val = Value::from_int(42);
        assert_eq!(to_json(&val), serde_json::json!(42));
    }

    #[test]
    fn test_to_json_bool() {
        let val = Value::from_bool(true);
        assert_eq!(to_json(&val), serde_json::json!(true));
    }

    #[test]
    fn test_to_json_string() {
        let val = Value::from_string("hello");
        assert_eq!(to_json(&val), serde_json::json!("hello"));
    }

    #[test]
    fn test_to_json_array() {
        let val = Value {
            data: ValueData::Vector(vec![
                Value::from_int(1),
                Value::from_int(2),
                Value::from_int(3),
            ]),
            display_hint: DisplayHint::Auto,
            audio_hint: None,
        };
        assert_eq!(to_json(&val), serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn test_roundtrip_array() {
        let json = serde_json::json!([1, 2, 3]);
        let val = from_json(json.clone(), 1).unwrap();
        let back = to_json(&val);
        assert_eq!(back, json);
    }

    #[test]
    fn test_roundtrip_object() {
        let json = serde_json::json!({"name": "Ajisai", "version": 1});
        let val = from_json(json.clone(), 1).unwrap();
        let back = to_json(&val);
        let back_obj = back.as_object().unwrap();
        assert_eq!(back_obj.get("name"), Some(&serde_json::json!("Ajisai")));
        assert_eq!(back_obj.get("version"), Some(&serde_json::json!(1)));
    }
}
