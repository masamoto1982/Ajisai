use crate::error::{AjisaiError, Result};
use crate::interpreter::cast::cast_value_helpers::{
    is_boolean_value, is_number_value, is_string_value,
};
use crate::interpreter::value_extraction_helpers::value_as_string;
use crate::interpreter::Interpreter;
use crate::types::Value;

fn type_name_of(val: &Value) -> &'static str {
    if val.is_nil() {
        "Nil"
    } else if is_string_value(val) {
        "String"
    } else if is_number_value(val) {
        "Number"
    } else if is_boolean_value(val) {
        "Boolean"
    } else if val.as_vector_view().is_some() {
        "Vector"
    } else {
        "other format"
    }
}

fn pop_string(interp: &mut Interpreter, word: &str) -> Result<String> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    if val.is_nil() {
        let err = AjisaiError::declared("nonText", format!("{}: expected String, got Nil", word));
        interp.stack.push(val);
        return Err(err);
    }
    if is_string_value(&val) {
        return Ok(value_as_string(&val).unwrap_or_default());
    }
    let tn = type_name_of(&val);
    interp.stack.push(val);
    Err(AjisaiError::declared(
        "nonText",
        format!("{}: expected String, got {}", word, tn),
    ))
}

pub fn op_trim(interp: &mut Interpreter) -> Result<()> {
    let s = pop_string(interp, "TRIM")?;
    interp.stack.push(Value::from_string(s.trim()));
    Ok(())
}

/// `UPPER`: Unicode's default, locale-independent upper-case mapping,
/// applied character by character — the table is Unicode's own, which no
/// definition over `CHARS` and `JOIN` could carry, so the Word is native.
/// `'straße' UPPER` is `'STRASSE'`.
pub fn op_upper(interp: &mut Interpreter) -> Result<()> {
    let s = pop_string(interp, "UPPER")?;
    let mapped: String = s.chars().flat_map(char::to_uppercase).collect();
    interp.stack.push(Value::from_string(&mapped));
    Ok(())
}

/// `LOWER`: the default lower-case mapping, character by character. No
/// context- or language-specific rule (final sigma, Turkish dotless i) is
/// applied — `str::to_lowercase` would apply the final-sigma rule — so the
/// same text lowers the same way wherever it is run, and `CHARS LOWER` per
/// character agrees with `LOWER` of the whole.
pub fn op_lower(interp: &mut Interpreter) -> Result<()> {
    let s = pop_string(interp, "LOWER")?;
    let mapped: String = s.chars().flat_map(char::to_lowercase).collect();
    interp.stack.push(Value::from_string(&mapped));
    Ok(())
}

pub fn op_tokenize(interp: &mut Interpreter) -> Result<()> {
    let sep_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let src_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow);
    let src_val = match src_val {
        Ok(v) => v,
        Err(e) => {
            interp.stack.push(sep_val);
            return Err(e);
        }
    };

    let restore = |interp: &mut Interpreter, a: Value, b: Value| {
        interp.stack.push(a);
        interp.stack.push(b);
    };

    if src_val.is_nil() {
        let err = AjisaiError::declared("nonText", "TOKENIZE: expected String, got Nil");
        restore(interp, src_val, sep_val);
        return Err(err);
    }
    if sep_val.is_nil() {
        let err = AjisaiError::declared(
            "nonTextSeparator",
            "TOKENIZE: expected separator String, got Nil",
        );
        restore(interp, src_val, sep_val);
        return Err(err);
    }
    if !is_string_value(&src_val) {
        let tn = type_name_of(&src_val);
        let err =
            AjisaiError::declared("nonText", format!("TOKENIZE: expected String, got {}", tn));
        restore(interp, src_val, sep_val);
        return Err(err);
    }
    if !is_string_value(&sep_val) {
        let tn = type_name_of(&sep_val);
        let err = AjisaiError::declared(
            "nonTextSeparator",
            format!("TOKENIZE: expected separator String, got {}", tn),
        );
        restore(interp, src_val, sep_val);
        return Err(err);
    }

    let src = value_as_string(&src_val).unwrap_or_default();
    let sep = value_as_string(&sep_val).unwrap_or_default();

    // The empty separator splits between every character, the same reading
    // SEARCH and REPLACE give the empty pattern (it matches everywhere). This
    // keeps TOKENIZE total over Text × Text.
    let parts: Vec<Value> = if sep.is_empty() {
        src.chars()
            .map(|c| Value::from_string(&c.to_string()))
            .collect()
    } else {
        src.split(sep.as_str()).map(Value::from_string).collect()
    };
    interp.stack.push(Value::from_vector(parts));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::cast::cast_value_helpers::is_string_value;
    use crate::interpreter::value_extraction_helpers::value_as_string;

    fn top_str(interp: &Interpreter) -> String {
        let v = interp.stack.last().unwrap();
        assert!(is_string_value(v));
        value_as_string(v).unwrap()
    }

    #[tokio::test]
    async fn trim_both() {
        let mut interp = Interpreter::new();
        interp.execute("'  hello  ' TRIM").await.unwrap();
        assert_eq!(top_str(&interp), "hello");
    }

    #[tokio::test]
    async fn tokenize_basic() {
        let mut interp = Interpreter::new();
        interp.execute("'a,b,c' ',' TOKENIZE").await.unwrap();
        let v = interp.stack.last().unwrap();
        let parts = v.as_vector_view().unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(value_as_string(&parts[0]).unwrap(), "a");
        assert_eq!(value_as_string(&parts[1]).unwrap(), "b");
        assert_eq!(value_as_string(&parts[2]).unwrap(), "c");
    }

    #[tokio::test]
    async fn tokenize_no_match() {
        let mut interp = Interpreter::new();
        interp.execute("'abc' ',' TOKENIZE").await.unwrap();
        let v = interp.stack.last().unwrap();
        let parts = v.as_vector_view().unwrap();
        assert_eq!(parts.len(), 1);
        assert_eq!(value_as_string(&parts[0]).unwrap(), "abc");
    }

    #[tokio::test]
    async fn tokenize_empty_separator_splits_characters() {
        let mut interp = Interpreter::new();
        interp.execute("'abc' '' TOKENIZE").await.unwrap();
        let v = interp.stack.last().unwrap();
        let parts = v.as_vector_view().unwrap();
        let got: Vec<String> = parts.iter().map(|p| value_as_string(p).unwrap()).collect();
        assert_eq!(got, vec!["a", "b", "c"]);
    }

    #[tokio::test]
    async fn trim_nil_rejected() {
        let mut interp = Interpreter::new();
        let r = interp.execute("NIL TRIM").await;
        assert!(r.is_err());
    }
}
