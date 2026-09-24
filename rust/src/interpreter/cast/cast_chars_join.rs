use crate::error::{AjisaiError, Result};
use crate::interpreter::Interpreter;
use crate::types::Value;

pub fn op_chars(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let Some(text) = val.as_text() else {
        let got = val.domain_name();
        interp.stack.push(val);
        return Err(AjisaiError::declared(
            "nonText",
            format!("expected a String, got {got}"),
        ));
    };

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
        let got = val.domain_name();
        interp.stack.push(val);
        return Err(AjisaiError::declared(
            "nonVector",
            format!("expected a Vector, got {got}"),
        ));
    };

    // Only Strings: a String is a value domain of its own, not a Vector of
    // code points (LANG.VALUES.DISJOINT), and CHARS, JOIN's inverse, yields
    // one-character Strings. Taking an integer as a code point here was the
    // one place a number still stood for a character, and nothing produced
    // one to pass.
    let mut result = String::new();
    for (i, elem) in children.iter().enumerate() {
        let Some(text) = elem.as_text() else {
            let got = elem.domain_name();
            interp.stack.push(val);
            return Err(AjisaiError::declared(
                "nonText",
                format!("expected a Vector of Strings, got {got} at index {i}"),
            ));
        };
        result.push_str(text);
    }

    interp.stack.push(Value::from_string(&result));
    Ok(())
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
    async fn test_join_of_the_empty_vector_is_the_empty_string() {
        // Joining nothing is the identity of concatenation. Both endpoints of
        // this used to be inexpressible, so it had to be an error.
        let mut interp = Interpreter::new();
        interp.execute("[ ] JOIN").await.unwrap();
        assert_eq!(interp.stack.last().and_then(|v| v.as_text()), Some(""));
    }

    #[tokio::test]
    async fn test_chars_join_round_trips_the_empty_string() {
        let mut interp = Interpreter::new();
        interp.execute("'' CHARS JOIN").await.unwrap();
        assert_eq!(interp.stack.last().and_then(|v| v.as_text()), Some(""));
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
