use crate::error::{AjisaiError, Result};
use crate::interpreter::cast::cast_value_helpers::{
    is_boolean_value, is_number_value, try_char_from_value,
};
use crate::interpreter::Interpreter;
use crate::types::Value;

pub fn op_chars(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let Some(text) = val.as_text() else {
        let got = describe_domain(&val);
        interp.stack.push(val);
        return Err(AjisaiError::from(format!(
            "CHARS: expected String, got {got}"
        )));
    };

    // `[ ]` is still inexpressible, so an empty String has no vector to split
    // into. This is the one place the String domain now outruns the Vector
    // one; it resolves when the empty Vector is admitted.
    if text.is_empty() {
        interp.stack.push(val);
        return Err(AjisaiError::from("CHARS: expected non-empty String"));
    }

    let chars: Vec<Value> = text
        .chars()
        .map(|c| Value::from_string(&c.to_string()))
        .collect();
    interp.stack.push(Value::from_vector(chars));
    Ok(())
}

pub fn op_join(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let Some(children) = val.as_vector_view().map(|v| v.into_owned()) else {
        let got = describe_domain(&val);
        interp.stack.push(val);
        return Err(AjisaiError::from(format!(
            "JOIN: expected Vector, got {got}"
        )));
    };

    if children.is_empty() {
        interp.stack.push(val);
        return Err(AjisaiError::from("JOIN: expected non-empty Vector"));
    }

    let mut result = String::new();
    for (i, elem) in children.iter().enumerate() {
        // A String element contributes its content; an integer element is
        // taken as a code point, which is what `core-join-mixed-text-and-
        // codepoints` pins. Accepting both is a Word-level choice about JOIN's
        // input domains, not a claim that the two are the same value.
        if let Some(text) = elem.as_text() {
            result.push_str(text);
            continue;
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
    Ok(())
}

/// The operand's domain, named for an error message.
fn describe_domain(val: &Value) -> &'static str {
    if val.is_nil() {
        "Nil"
    } else if val.is_text() {
        "String"
    } else if is_number_value(val) {
        "Number"
    } else if is_boolean_value(val) {
        "Boolean"
    } else if val.is_vector() {
        "Vector"
    } else {
        "other format"
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

        interp.execute("'hello' CHARS JOIN").await.unwrap();
        assert_eq!(interp.stack.len(), 1);

        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "hello");
        }
    }

    #[tokio::test]
    async fn test_chars_rejects_a_vector() {
        // `[ 42 ]` is a Vector of one number. It used to satisfy CHARS because
        // a codepoint vector *was* a String; CHARS' contract registers
        // `nonText`, and now that is what happens.
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 42 ] CHARS").await;
        assert!(result.is_err(), "CHARS must reject a Vector");
    }

    #[tokio::test]
    async fn test_chars_yields_one_character_strings() {
        let mut interp = Interpreter::new();
        interp.execute("'ab' CHARS").await.unwrap();
        let val = interp.stack.last().expect("a result");
        let items = val.as_vector_view().expect("a vector");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].as_text(), Some("a"));
        assert_eq!(items[1].as_text(), Some("b"));
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
