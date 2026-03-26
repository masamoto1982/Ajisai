use crate::error::{AjisaiError, Result};
use crate::interpreter::{AsyncAction, Interpreter};
use crate::types::{Token, Value, ValueData};

fn format_scalar_to_source(val: &Value) -> Result<String> {
    let Some(f) = val.as_scalar() else {
        return Err(AjisaiError::create_structure_error(
            "scalar",
            "non-scalar value",
        ));
    };

    if f.is_integer() {
        return Ok(f.numerator().to_string());
    }
    Ok(format!("{}/{}", f.numerator(), f.denominator()))
}

fn format_token_to_source(token: &Token) -> String {
    match token {
        Token::Number(n) => n.to_string(),
        Token::String(s) => format!("'{}'", s),
        Token::Symbol(s) => s.to_string(),
        Token::VectorStart => "[".to_string(),
        Token::VectorEnd => "]".to_string(),
        Token::CodeBlockStart => ":".to_string(),
        Token::CodeBlockEnd => ";".to_string(),
        Token::Pipeline => "==".to_string(),
        Token::NilCoalesce => "=>".to_string(),
        Token::ChevronBranch => ">>".to_string(),
        Token::ChevronDefault => ">>>".to_string(),
        Token::SafeMode => "~".to_string(),
        Token::LineBreak => "\n".to_string(),
    }
}

fn format_value_to_source_inner(val: &Value, depth: usize) -> Result<String> {
    match &val.data {
        ValueData::Nil => Ok("NIL".to_string()),
        ValueData::Scalar(_) => format_scalar_to_source(val),
        ValueData::CodeBlock(tokens) => {
            let token_strs: Vec<String> = tokens.iter().map(format_token_to_source).collect();
            Ok(format!(": {} ;", token_strs.join(" ")))
        }
        ValueData::Vector(children)
        | ValueData::Record {
            pairs: children, ..
        } => {
            let inner: Vec<String> = children
                .iter()
                .map(|c| format_value_to_source_inner(c, depth + 1))
                .collect::<Result<Vec<_>>>()?;
            let joined: String = inner.join(" ");
            if depth > 0 {
                Ok(format!("[ {} ]", joined))
            } else {
                Ok(joined)
            }
        }
    }
}

pub fn format_vector_to_source(val: &Value) -> Result<String> {
    format_value_to_source_inner(val, 0)
}

pub fn execute_vector_as_code(interp: &mut Interpreter, val: &Value) -> Result<()> {
    let source: String = format_vector_to_source(val)?;

    let tokens: Vec<Token> = crate::tokenizer::tokenize(&source)
        .map_err(|e| AjisaiError::from(format!("EXECUTE_VECTOR: expected valid tokens, got {}", e)))?;

    let (_, action): (usize, Option<AsyncAction>) = interp.execute_section_core(&tokens, 0)?;

    if action.is_some() {
        return Err(AjisaiError::from(
            "EXECUTE_VECTOR: expected synchronous execution, got async operation",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Value;

    #[test]
    fn test_format_vector_to_source_simple() {
        let val: Value = Value::from_vector(vec![
            Value::from_int(1),
            Value::from_int(2),
            Value::from_int(3),
        ]);
        let source: String = format_vector_to_source(&val).unwrap();
        assert_eq!(source, "1 2 3");
    }

    #[test]
    fn test_format_vector_to_source_with_nested() {
        let val: Value = Value::from_vector(vec![
            Value::from_vector(vec![Value::from_int(2)]),
            Value::from_string("*"),
        ]);
        let source: String = format_vector_to_source(&val).unwrap();
        assert_eq!(source, "[ 2 ] [ 42 ]");
    }

    #[test]
    fn test_format_vector_to_source_symbols() {
        let val: Value = Value::from_vector(vec![Value::from_string("DUP"), Value::from_string("*")]);
        let source: String = format_vector_to_source(&val).unwrap();
        assert_eq!(source, "[ 68 85 80 ] [ 42 ]");
    }

    #[test]
    fn test_format_vector_to_source_nil() {
        let val: Value = Value::nil();
        let source: String = format_vector_to_source(&val).unwrap();
        assert_eq!(source, "NIL");
    }

    #[test]
    fn test_format_vector_to_source_fraction() {
        use crate::types::fraction::Fraction;
        use num_bigint::BigInt;
        let val: Value = Value::from_vector(vec![Value::from_fraction(Fraction::new(
            BigInt::from(1),
            BigInt::from(3),
        ))]);
        let source: String = format_vector_to_source(&val).unwrap();
        assert_eq!(source, "1/3");
    }
}
