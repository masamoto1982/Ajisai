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

    if sep.is_empty() {
        // Not `nonTextSeparator`: the separator *is* Text, it just carries no
        // content to split on. TOKENIZE's contract does not name this
        // condition, so `StructureError` is the honest fallback rather than
        // reusing a type-mismatch category for a value-domain one.
        let err = AjisaiError::create_structure_error("a non-empty separator", "the empty string");
        restore(interp, src_val, sep_val);
        return Err(err);
    }

    let parts: Vec<Value> = src.split(sep.as_str()).map(Value::from_string).collect();
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
    async fn tokenize_empty_separator_errors() {
        let mut interp = Interpreter::new();
        let r = interp.execute("'abc' '' TOKENIZE").await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn trim_nil_rejected() {
        let mut interp = Interpreter::new();
        let r = interp.execute("NIL TRIM").await;
        assert!(r.is_err());
    }
}
