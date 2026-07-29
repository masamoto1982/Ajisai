use crate::error::{AjisaiError, Result};
use crate::interpreter::cast::cast_value_helpers::{
    is_boolean_value, is_datetime_value, is_number_value, is_string_value, try_char_from_value,
};
use crate::interpreter::value_extraction_helpers::value_as_string;
use crate::interpreter::Interpreter;
use crate::types::Stack;
use crate::types::Value;

pub fn op_chars(interp: &mut Interpreter) -> Result<()> {
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
        return Err(AjisaiError::from("CHARS: expected String, got Number"));
    }

    if is_boolean_value(&val) {
        interp.stack.push(val);
        return Err(AjisaiError::from("CHARS: expected String, got Boolean"));
    }

    interp.stack.push(val);
    Err(AjisaiError::from("CHARS: expected String input"))
}

pub fn op_join(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    if val.is_nil() {
        interp.stack.push(val);
        return Err(AjisaiError::from("JOIN: expected Vector, got Nil"));
    }

    if let Some(children) = val.as_vector_view() {
        if children.is_empty() {
            interp.stack.push(val);
            return Err(AjisaiError::from("JOIN: expected non-empty Vector"));
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
                    Some(c) => {
                        result.push(c);
                        continue;
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::cast::cast_value_helpers::is_string_value;
    use crate::interpreter::value_extraction_helpers::value_as_string;

    #[tokio::test]
    async fn test_chars_basic() {
        let mut interp = Interpreter::new();

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

        interp.execute("'hello' CHARS REVERSE JOIN").await.unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "olleh");
        }
    }

    #[tokio::test]
    async fn test_nil_pushes_constant() {
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
        let mut interp = Interpreter::new();
        let result = interp.execute("NIL NIL NIL").await;
        assert!(result.is_ok());
        assert_eq!(interp.stack.len(), 3);

        for val in interp.stack.iter() {
            assert!(val.is_nil());
        }
    }
}
