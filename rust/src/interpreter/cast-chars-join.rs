use crate::error::{AjisaiError, Result};
use crate::interpreter::cast::cast_value_helpers::{
    is_boolean_value, is_datetime_value, is_number_value, is_string_value, try_char_from_value,
};
use crate::interpreter::value_extraction_helpers::value_as_string;
use crate::interpreter::{Interpreter, OperationTargetMode};
use crate::types::{Value, ValueData};

pub fn op_chars(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            if val.is_nil() {
                interp.stack.push(val);
                return Err(AjisaiError::from("CHARS: expected String, got Nil"));
            }

            if is_string_value(&val) {
                let s = value_as_string(&val).unwrap_or_default();
                if s.is_empty() {
                    interp.stack.push(val);
                    return Err(AjisaiError::from("CHARS: expected non-empty String"));
                }

                let chars: Vec<Value> = s
                    .chars()
                    .map(|c| Value::from_string(&c.to_string()))
                    .collect();

                interp.stack.push(Value::from_vector(chars));
                return Ok(());
            }

            if is_number_value(&val) {
                interp.stack.push(val);
                return Err(AjisaiError::from(
                    "CHARS: expected String, got Number",
                ));
            }

            if is_boolean_value(&val) {
                interp.stack.push(val);
                return Err(AjisaiError::from(
                    "CHARS: expected String, got Boolean",
                ));
            }

            interp.stack.push(val);
            Err(AjisaiError::from("CHARS: expected String input"))
        }
        OperationTargetMode::Stack => {
            let stack_len = interp.stack.len();
            if stack_len == 0 {
                return Err(AjisaiError::StackUnderflow);
            }

            let mut results = Vec::with_capacity(stack_len);
            let elements: Vec<Value> = interp.stack.drain(..).collect();

            for elem in elements {
                if elem.is_nil() {
                    interp.stack = results;
                    interp.stack.push(elem);
                    return Err(AjisaiError::from("CHARS: expected String, got Nil"));
                }

                if is_string_value(&elem) {
                    let s = value_as_string(&elem).unwrap_or_default();
                    if s.is_empty() {
                        interp.stack = results;
                        interp.stack.push(elem);
                        return Err(AjisaiError::from("CHARS: expected non-empty String"));
                    }
                    let chars: Vec<Value> = s
                        .chars()
                        .map(|c| Value::from_string(&c.to_string()))
                        .collect();
                    results.push(Value::from_vector(chars));
                    continue;
                }

                if is_number_value(&elem) {
                    interp.stack = results;
                    interp.stack.push(elem);
                    return Err(AjisaiError::from(
                        "CHARS: expected String, got Number",
                    ));
                }

                if is_boolean_value(&elem) {
                    interp.stack = results;
                    interp.stack.push(elem);
                    return Err(AjisaiError::from(
                        "CHARS: expected String, got Boolean",
                    ));
                }

                interp.stack = results;
                interp.stack.push(elem);
                return Err(AjisaiError::from("CHARS: expected String input"));
            }

            interp.stack = results;
            Ok(())
        }
    }
}

pub fn op_join(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            if val.is_nil() {
                interp.stack.push(val);
                return Err(AjisaiError::from("JOIN: expected Vector, got Nil"));
            }

            if let ValueData::Vector(children) = &val.data {
                if children.is_empty() {
                    interp.stack.push(val);
                    return Err(AjisaiError::from(
                        "JOIN: expected non-empty Vector",
                    ));
                }

                let mut result = String::new();
                for (i, elem) in children.iter().enumerate() {
                    if is_string_value(elem) {
                        if let Some(s) = value_as_string(elem) {
                            result.push_str(&s);
                            continue;
                        }
                    }

                    if is_number_value(elem) {
                        match try_char_from_value(elem) {
                            Some(c) => { result.push(c); continue; }
                            None => {
                                interp.stack.push(val);
                                return Err(AjisaiError::from(format!(
                                    "JOIN: invalid character code at index {}",
                                    i
                                )));
                            }
                        }
                    }

                    let type_name = if elem.is_nil() {
                        "nil"
                    } else if is_boolean_value(elem) {
                        "boolean"
                    } else {
                        "other format"
                    };
                    interp.stack.push(val);
                    return Err(AjisaiError::from(format!(
                        "JOIN: all elements must be strings, found {} at index {}",
                        type_name, i
                    )));
                }

                interp.stack.push(Value::from_string(&result));
                return Ok(());
            }

            let type_name = if is_string_value(&val) {
                "String"
            } else if is_number_value(&val) {
                "Number"
            } else if is_boolean_value(&val) {
                "Boolean"
            } else if is_datetime_value(&val) {
                "DateTime"
            } else {
                "other format"
            };
            interp.stack.push(val);
            Err(AjisaiError::from(format!(
                "JOIN: expected Vector, got {}",
                type_name
            )))
        }
        OperationTargetMode::Stack => {
            // スタック上の各ベクタに対してJOINを適用
            let stack_len = interp.stack.len();
            if stack_len == 0 {
                return Err(AjisaiError::StackUnderflow);
            }

            let mut results = Vec::with_capacity(stack_len);
            let elements: Vec<Value> = interp.stack.drain(..).collect();

            for elem in elements {
                // NILの場合
                if elem.is_nil() {
                    interp.stack = results;
                    interp.stack.push(elem);
                    return Err(AjisaiError::from("JOIN: requires vector format, got Nil"));
                }

                // ベクタの場合
                if let ValueData::Vector(children) = &elem.data {
                    if children.is_empty() {
                        interp.stack = results;
                        interp.stack.push(elem);
                        return Err(AjisaiError::from(
                            "JOIN: empty vector has no strings to join",
                        ));
                    }

                    let mut result_str = String::new();
                    for (i, v) in children.iter().enumerate() {
                        // 文字列の場合
                        if is_string_value(v) {
                            if let Some(s) = value_as_string(v) {
                                result_str.push_str(&s);
                                continue;
                            }
                        }

                        // 数値の場合（文字コードとして解釈）
                        if is_number_value(v) {
                            if let Some(f) = v.as_scalar() {
                                if let Some(code) = f.to_i64() {
                                    if code >= 0 && code <= 0x10FFFF {
                                        if let Some(c) = char::from_u32(code as u32) {
                                            result_str.push(c);
                                            continue;
                                        }
                                    }
                                }
                            }
                            interp.stack = results;
                            interp.stack.push(elem);
                            return Err(AjisaiError::from(format!(
                                "JOIN: invalid character code at index {}",
                                i
                            )));
                        }

                        // その他の型
                        let type_name = if v.is_nil() {
                            "nil"
                        } else if is_boolean_value(v) {
                            "boolean"
                        } else {
                            "other format"
                        };
                        interp.stack = results;
                        interp.stack.push(elem);
                        return Err(AjisaiError::from(format!(
                            "JOIN: all elements must be strings, found {} at index {}",
                            type_name, i
                        )));
                    }

                    results.push(Value::from_string(&result_str));
                    continue;
                }

                // 単一要素の場合（ベクタではない）
                let type_name = if is_string_value(&elem) {
                    "String"
                } else if is_number_value(&elem) {
                    "Number"
                } else if is_boolean_value(&elem) {
                    "Boolean"
                } else if is_datetime_value(&elem) {
                    "DateTime"
                } else {
                    "other format"
                };
                interp.stack = results;
                interp.stack.push(elem);
                return Err(AjisaiError::from(format!(
                    "JOIN: requires vector format, got {}",
                    type_name
                )));
            }

            interp.stack = results;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::cast::cast_value_helpers::is_string_value;
    use crate::interpreter::value_extraction_helpers::value_as_string;

    #[tokio::test]
    async fn test_chars_basic() {
        let mut interp = Interpreter::new();
        // 直接文字列を使用（'hello' は文字列、[ 'hello' ] はベクター）
        interp.execute("'hello' CHARS JOIN").await.unwrap();
        assert_eq!(interp.stack.len(), 1);

        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "hello");
        }
    }

    #[tokio::test]
    async fn test_chars_structure_error() {
        // In new architecture, [ 42 ] is a Vector (treated as string).
        // CHARS interprets scalar 42 as Unicode codepoint '*', so it succeeds.
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 42 ] CHARS").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_join_basic() {
        let mut interp = Interpreter::new();
        interp
            .execute("[ 'h' 'e' 'l' 'l' 'o' ] JOIN")
            .await
            .unwrap();
        assert_eq!(interp.stack.len(), 1);

        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "hello");
        }
    }

    #[tokio::test]
    async fn test_join_empty_error() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ ] JOIN").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_chars_join_roundtrip() {
        let mut interp = Interpreter::new();
        // 直接文字列を使用（'hello' は文字列、[ 'hello' ] はベクター）
        interp.execute("'hello' CHARS JOIN").await.unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "hello");
        }
    }

    #[tokio::test]
    async fn test_chars_reverse_join() {
        let mut interp = Interpreter::new();
        // 直接文字列を使用（'hello' は文字列、[ 'hello' ] はベクター）
        interp.execute("'hello' CHARS REVERSE JOIN").await.unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "olleh");
        }
    }

    #[tokio::test]
    async fn test_nil_pushes_constant() {
        // NILは常に定数としてNILをプッシュする
        let mut interp = Interpreter::new();
        let result = interp.execute("NIL").await;
        assert!(result.is_ok());
        assert_eq!(interp.stack.len(), 1);

        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil());
        }
    }

    #[tokio::test]
    async fn test_nil_multiple() {
        // NILを複数回呼ぶと複数のNILがスタックに積まれる
        let mut interp = Interpreter::new();
        let result = interp.execute("NIL NIL NIL").await;
        assert!(result.is_ok());
        assert_eq!(interp.stack.len(), 3);

        for val in interp.stack.iter() {
            assert!(val.is_nil());
        }
    }
}
