use crate::error::{AjisaiError, Result};
use crate::interpreter::cast::cast_value_helpers::{
    is_boolean_value, is_number_value, is_string_value,
};
use crate::interpreter::value_extraction_helpers::value_as_string;
use crate::interpreter::{Interpreter, OperationTargetMode};
use crate::types::Stack;
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
        let err = AjisaiError::from(format!("{}: expected String, got Nil", word));
        interp.stack.push(val);
        return Err(err);
    }
    if is_string_value(&val) {
        return Ok(value_as_string(&val).unwrap_or_default());
    }
    let tn = type_name_of(&val);
    interp.stack.push(val);
    Err(AjisaiError::from(format!(
        "{}: expected String, got {}",
        word, tn
    )))
}

enum TrimSide {
    Both,
    Left,
    Right,
}

fn apply_trim(side: &TrimSide, s: &str) -> String {
    match side {
        TrimSide::Both => s.trim().to_string(),
        TrimSide::Left => s.trim_start().to_string(),
        TrimSide::Right => s.trim_end().to_string(),
    }
}

fn op_trim_generic(interp: &mut Interpreter, word: &str, side: TrimSide) -> Result<()> {
    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let s = pop_string(interp, word)?;
            interp
                .stack
                .push(Value::from_string(&apply_trim(&side, &s)));
            Ok(())
        }
        OperationTargetMode::Stack => {
            if interp.stack.is_empty() {
                return Err(AjisaiError::StackUnderflow);
            }
            let elements: Vec<Value> = interp.stack.drain(..).collect();
            let mut results: Vec<Value> = Vec::with_capacity(elements.len());
            for elem in elements {
                if elem.is_nil() {
                    let err = AjisaiError::from(format!("{}: expected String, got Nil", word));
                    interp.stack = Stack::from_values(results);
                    interp.stack.push(elem);
                    return Err(err);
                }
                if is_string_value(&elem) {
                    let s = value_as_string(&elem).unwrap_or_default();
                    results.push(Value::from_string(&apply_trim(&side, &s)));
                    continue;
                }
                let tn = type_name_of(&elem);
                interp.stack = Stack::from_values(results);
                interp.stack.push(elem);
                return Err(AjisaiError::from(format!(
                    "{}: expected String, got {}",
                    word, tn
                )));
            }
            interp.stack = Stack::from_values(results);
            Ok(())
        }
    }
}

pub fn op_trim(interp: &mut Interpreter) -> Result<()> {
    op_trim_generic(interp, "TRIM", TrimSide::Both)
}

pub fn op_trim_left(interp: &mut Interpreter) -> Result<()> {
    op_trim_generic(interp, "TRIM-LEFT", TrimSide::Left)
}

pub fn op_trim_right(interp: &mut Interpreter) -> Result<()> {
    op_trim_generic(interp, "TRIM-RIGHT", TrimSide::Right)
}

pub fn op_tokenize(interp: &mut Interpreter) -> Result<()> {
    let sep_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let src_val = interp
        .stack
        .pop()
        .ok_or_else(|| AjisaiError::StackUnderflow);
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
        let err = AjisaiError::from("TOKENIZE: expected String, got Nil");
        restore(interp, src_val, sep_val);
        return Err(err);
    }
    if sep_val.is_nil() {
        let err = AjisaiError::from("TOKENIZE: expected separator String, got Nil");
        restore(interp, src_val, sep_val);
        return Err(err);
    }
    if !is_string_value(&src_val) {
        let tn = type_name_of(&src_val);
        let err = AjisaiError::from(format!("TOKENIZE: expected String, got {}", tn));
        restore(interp, src_val, sep_val);
        return Err(err);
    }
    if !is_string_value(&sep_val) {
        let tn = type_name_of(&sep_val);
        let err = AjisaiError::from(format!("TOKENIZE: expected separator String, got {}", tn));
        restore(interp, src_val, sep_val);
        return Err(err);
    }

    let src = value_as_string(&src_val).unwrap_or_default();
    let sep = value_as_string(&sep_val).unwrap_or_default();

    if sep.is_empty() {
        let err = AjisaiError::from("TOKENIZE: separator must be non-empty");
        restore(interp, src_val, sep_val);
        return Err(err);
    }

    let parts: Vec<Value> = src.split(sep.as_str()).map(Value::from_string).collect();
    interp.stack.push(Value::from_vector(parts));
    Ok(())
}

pub fn op_substitute(interp: &mut Interpreter) -> Result<()> {
    let to_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let from_val = match interp.stack.pop() {
        Some(v) => v,
        None => {
            interp.stack.push(to_val);
            return Err(AjisaiError::StackUnderflow);
        }
    };
    let src_val = match interp.stack.pop() {
        Some(v) => v,
        None => {
            interp.stack.push(from_val);
            interp.stack.push(to_val);
            return Err(AjisaiError::StackUnderflow);
        }
    };

    let restore = |interp: &mut Interpreter, a: Value, b: Value, c: Value| {
        interp.stack.push(a);
        interp.stack.push(b);
        interp.stack.push(c);
    };

    let check = |label: &str, v: &Value| -> Option<AjisaiError> {
        if v.is_nil() {
            return Some(AjisaiError::from(format!(
                "SUBSTITUTE: expected {}, got Nil",
                label
            )));
        }
        if !is_string_value(v) {
            return Some(AjisaiError::from(format!(
                "SUBSTITUTE: expected {} as String, got {}",
                label,
                type_name_of(v)
            )));
        }
        None
    };
    if let Some(err) = check("String", &src_val)
        .or_else(|| check("from", &from_val))
        .or_else(|| check("to", &to_val))
    {
        restore(interp, src_val, from_val, to_val);
        return Err(err);
    }

    let src = value_as_string(&src_val).unwrap_or_default();
    let from = value_as_string(&from_val).unwrap_or_default();
    let to = value_as_string(&to_val).unwrap_or_default();

    if from.is_empty() {
        let err = AjisaiError::from("SUBSTITUTE: from pattern must be non-empty");
        restore(interp, src_val, from_val, to_val);
        return Err(err);
    }

    let result = src.replace(from.as_str(), to.as_str());
    interp.stack.push(Value::from_string(&result));
    Ok(())
}

fn op_affix_predicate(
    interp: &mut Interpreter,
    word: &str,
    check: impl Fn(&str, &str) -> bool,
) -> Result<()> {
    let needle_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let hay_val = match interp.stack.pop() {
        Some(v) => v,
        None => {
            interp.stack.push(needle_val);
            return Err(AjisaiError::StackUnderflow);
        }
    };

    let restore = |interp: &mut Interpreter, a: Value, b: Value| {
        interp.stack.push(a);
        interp.stack.push(b);
    };

    let validate = |label: &str, v: &Value| -> Option<AjisaiError> {
        if v.is_nil() {
            return Some(AjisaiError::from(format!(
                "{}: expected {}, got Nil",
                word, label
            )));
        }
        if !is_string_value(v) {
            return Some(AjisaiError::from(format!(
                "{}: expected {} as String, got {}",
                word,
                label,
                type_name_of(v)
            )));
        }
        None
    };
    if let Some(err) = validate("String", &hay_val).or_else(|| validate("affix", &needle_val)) {
        restore(interp, hay_val, needle_val);
        return Err(err);
    }

    let hay = value_as_string(&hay_val).unwrap_or_default();
    let needle = value_as_string(&needle_val).unwrap_or_default();
    interp.stack.push(Value::from_bool(check(&hay, &needle)));
    Ok(())
}

pub fn op_starts_with(interp: &mut Interpreter) -> Result<()> {
    op_affix_predicate(interp, "STARTS-WITH?", |h, n| h.starts_with(n))
}

pub fn op_ends_with(interp: &mut Interpreter) -> Result<()> {
    op_affix_predicate(interp, "ENDS-WITH?", |h, n| h.ends_with(n))
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
    async fn trim_left_only() {
        let mut interp = Interpreter::new();
        interp.execute("'  hello  ' TRIM-LEFT").await.unwrap();
        assert_eq!(top_str(&interp), "hello  ");
    }

    #[tokio::test]
    async fn trim_right_only() {
        let mut interp = Interpreter::new();
        interp.execute("'  hello  ' TRIM-RIGHT").await.unwrap();
        assert_eq!(top_str(&interp), "  hello");
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
    async fn substitute_basic() {
        let mut interp = Interpreter::new();
        interp.execute("'hello' 'l' 'L' SUBSTITUTE").await.unwrap();
        assert_eq!(top_str(&interp), "heLLo");
    }

    #[tokio::test]
    async fn substitute_no_match() {
        let mut interp = Interpreter::new();
        interp.execute("'hello' 'z' 'Z' SUBSTITUTE").await.unwrap();
        assert_eq!(top_str(&interp), "hello");
    }

    #[tokio::test]
    async fn substitute_empty_from_errors() {
        let mut interp = Interpreter::new();
        let r = interp.execute("'hello' '' 'X' SUBSTITUTE").await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn starts_with_true() {
        let mut interp = Interpreter::new();
        interp.execute("'hello' 'he' STARTS-WITH?").await.unwrap();
        assert!(interp.stack.last().unwrap().is_truthy());
    }

    #[tokio::test]
    async fn starts_with_false() {
        let mut interp = Interpreter::new();
        interp.execute("'hello' 'lo' STARTS-WITH?").await.unwrap();
        assert!(!interp.stack.last().unwrap().is_truthy());
    }

    #[tokio::test]
    async fn ends_with_true() {
        let mut interp = Interpreter::new();
        interp.execute("'hello' 'lo' ENDS-WITH?").await.unwrap();
        assert!(interp.stack.last().unwrap().is_truthy());
    }

    #[tokio::test]
    async fn ends_with_false() {
        let mut interp = Interpreter::new();
        interp.execute("'hello' 'he' ENDS-WITH?").await.unwrap();
        assert!(!interp.stack.last().unwrap().is_truthy());
    }

    #[tokio::test]
    async fn trim_nil_rejected() {
        let mut interp = Interpreter::new();
        let r = interp.execute("NIL TRIM").await;
        assert!(r.is_err());
    }
}
