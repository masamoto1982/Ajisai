use crate::error::{AjisaiError, Result};
use crate::interpreter::cast::cast_value_helpers::is_string_value;
use crate::interpreter::value_extraction_helpers::value_as_string;
use crate::interpreter::Interpreter;
use crate::types::Value;

fn pop_string(interp: &mut Interpreter) -> Result<String> {
    let val = interp.stack.pop().ok_or(AjisaiError::stack_underflow())?;
    if is_string_value(&val) {
        return Ok(value_as_string(&val).unwrap_or_default());
    }
    let got = val.domain_name();
    interp.stack.push(val);
    Err(AjisaiError::declared(
        "nonText",
        format!("expected a String, got {got}"),
    ))
}

pub fn op_trim(interp: &mut Interpreter) -> Result<()> {
    let s = pop_string(interp)?;
    interp.stack.push(Value::from_string(s.trim()));
    Ok(())
}

/// `UPPER`: Unicode's default, locale-independent upper-case mapping,
/// applied character by character — the table is Unicode's own, which no
/// definition over `CHARS` and `JOIN` could carry, so the Word is native.
/// `'straße' UPPER` is `'STRASSE'`.
pub fn op_upper(interp: &mut Interpreter) -> Result<()> {
    let s = pop_string(interp)?;
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
    let s = pop_string(interp)?;
    let mapped: String = s.chars().flat_map(char::to_lowercase).collect();
    interp.stack.push(Value::from_string(&mapped));
    Ok(())
}

pub fn op_tokenize(interp: &mut Interpreter) -> Result<()> {
    let sep_val = interp.stack.pop().ok_or(AjisaiError::stack_underflow())?;
    let src_val = interp.stack.pop().ok_or(AjisaiError::stack_underflow());
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

    if !is_string_value(&src_val) {
        let got = src_val.domain_name();
        restore(interp, src_val, sep_val);
        return Err(AjisaiError::declared(
            "nonText",
            format!("expected a String, got {got}"),
        ));
    }
    if !is_string_value(&sep_val) {
        let got = sep_val.domain_name();
        restore(interp, src_val, sep_val);
        return Err(AjisaiError::declared(
            "nonText",
            format!("expected a String separator, got {got}"),
        ));
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
    async fn trim_passes_an_absent_operand_through() {
        let mut interp = Interpreter::new();
        interp.execute("0 0 DIV TRIM NIL-REASON").await.unwrap();
        let reason = interp
            .stack
            .last()
            .and_then(|v| v.as_text().map(str::to_string));
        assert_eq!(reason.as_deref(), Some("divisionByZero"));
    }
}
