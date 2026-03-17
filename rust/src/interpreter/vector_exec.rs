use crate::error::{AjisaiError, Result};
use crate::interpreter::Interpreter;
use crate::types::{Value, ValueData};

fn scalar_to_source(val: &Value) -> Result<String> {
    if let Some(f) = val.as_scalar() {
        if f.is_integer() {
            return Ok(f.numerator.to_string());
        }
        return Ok(format!("{}/{}", f.numerator, f.denominator));
    }
    Err(AjisaiError::structure_error("scalar", "non-scalar value"))
}

fn token_to_source(token: &crate::types::Token) -> String {
    use crate::types::Token;
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
        Token::ScopeDirective(name) => format!("@{}", name),
    }
}

fn value_to_source_inner(val: &Value, depth: usize) -> Result<String> {
    match &val.data {
        ValueData::Nil => Ok("NIL".to_string()),
        ValueData::Scalar(_) => scalar_to_source(val),
        ValueData::CodeBlock(tokens) => {
            let token_strs: Vec<String> = tokens.iter().map(token_to_source).collect();
            Ok(format!(": {} ;", token_strs.join(" ")))
        }
        ValueData::Vector(children)
        | ValueData::Record {
            pairs: children, ..
        } => {
            let inner: Vec<String> = children
                .iter()
                .map(|c| value_to_source_inner(c, depth + 1))
                .collect::<Result<Vec<_>>>()?;
            let joined = inner.join(" ");
            if depth > 0 {
                Ok(format!("[ {} ]", joined))
            } else {
                Ok(joined)
            }
        }
    }
}

pub fn vector_to_source(val: &Value) -> Result<String> {
    value_to_source_inner(val, 0)
}

pub fn execute_vector_as_code(interp: &mut Interpreter, val: &Value) -> Result<()> {
    let source = vector_to_source(val)?;

    let tokens = crate::tokenizer::tokenize(&source)
        .map_err(|e| AjisaiError::from(format!("Tokenization error: {}", e)))?;

    let (_, action) = interp.execute_section_core(&tokens, 0)?;

    if action.is_some() {
        return Err(AjisaiError::from(
            "Async operations not supported in vector execution",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Value;

    #[test]
    fn test_vector_to_source_simple() {
        // [ 1 2 3 ] → "1 2 3"
        let val = Value::from_vector(vec![
            Value::from_int(1),
            Value::from_int(2),
            Value::from_int(3),
        ]);
        let source = vector_to_source(&val).unwrap();
        assert_eq!(source, "1 2 3");
    }

    #[test]
    fn test_vector_to_source_with_nested() {
        // [ [ 2 ] * ] — from_string("*") now creates Vector([Scalar(42)])
        // so vector_to_source sees two nested vectors: [ 2 ] and [ 42 ]
        let val = Value::from_vector(vec![
            Value::from_vector(vec![Value::from_int(2)]),
            Value::from_string("*"),
        ]);
        let source = vector_to_source(&val).unwrap();
        assert_eq!(source, "[ 2 ] [ 42 ]");
    }

    #[test]
    fn test_vector_to_source_symbols() {
        // from_string("DUP") → Vector([68, 85, 80])
        // from_string("*")   → Vector([42])
        // vector_to_source renders these as nested vectors of codepoints
        let val = Value::from_vector(vec![Value::from_string("DUP"), Value::from_string("*")]);
        let source = vector_to_source(&val).unwrap();
        assert_eq!(source, "[ 68 85 80 ] [ 42 ]");
    }

    #[test]
    fn test_vector_to_source_nil() {
        // NIL → "NIL"
        let val = Value::nil();
        let source = vector_to_source(&val).unwrap();
        assert_eq!(source, "NIL");
    }

    #[test]
    fn test_vector_to_source_fraction() {
        // [ 1/3 ] → "1/3"
        use crate::types::fraction::Fraction;
        use num_bigint::BigInt;
        let val = Value::from_vector(vec![Value::from_fraction(Fraction::new(
            BigInt::from(1),
            BigInt::from(3),
        ))]);
        let source = vector_to_source(&val).unwrap();
        assert_eq!(source, "1/3");
    }
}
