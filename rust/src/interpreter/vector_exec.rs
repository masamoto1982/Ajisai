use crate::interpreter::Interpreter;
use crate::types::{Value, ValueData, DisplayHint};
use crate::error::{AjisaiError, Result};


pub fn vector_to_source(val: &Value) -> Result<String> {
    fn value_to_code(val: &Value, depth: usize) -> Result<String> {
        match &val.data {
            ValueData::Nil => Ok("NIL".to_string()),

            ValueData::Scalar(f) => {
                // 数値として出力
                if f.is_integer() {
                    Ok(f.numerator.to_string())
                } else {
                    Ok(format!("{}/{}", f.numerator, f.denominator))
                }
            }

            ValueData::CodeBlock(tokens) => {
                // CodeBlockはソースコードとして表示
                use crate::types::Token;
                let token_strs: Vec<String> = tokens.iter().map(|t| {
                    match t {
                        Token::Number(n) => n.clone(),
                        Token::String(s) => format!("'{}'", s),
                        Token::Symbol(s) => s.clone(),
                        Token::VectorStart => "[".to_string(),
                        Token::VectorEnd => "]".to_string(),
                        Token::CodeBlockStart => ":".to_string(),
                        Token::CodeBlockEnd => ";".to_string(),
                        Token::Pipeline => "==".to_string(),
                        Token::NilCoalesce => "=>".to_string(),
                        _ => String::new(),
                    }
                }).collect();
                Ok(format!(": {} ;", token_strs.join(" ")))
            }

            ValueData::Vector(children) => {
                // DisplayHint::String の場合はシンボルとして出力
                if val.display_hint == DisplayHint::String {
                    // 文字列をシンボル名として解釈
                    let chars: String = children.iter()
                        .filter_map(|c| {
                            c.as_scalar()
                                .and_then(|f| f.to_i64())
                                .and_then(|n| char::from_u32(n as u32))
                        })
                        .collect();
                    return Ok(chars);
                }

                // 通常のVector: 再帰的に処理
                // depth > 0 の場合は括弧で囲む
                let inner: Vec<String> = children.iter()
                    .map(|c| value_to_code(c, depth + 1))
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

    value_to_code(val, 0)
}

pub fn execute_vector_as_code(interp: &mut Interpreter, val: &Value) -> Result<()> {
    let source = vector_to_source(val)?;

    let tokens = crate::tokenizer::tokenize(&source)
        .map_err(|e| AjisaiError::from(format!("Tokenization error: {}", e)))?;

    let (_, action) = interp.execute_section_core(&tokens, 0)?;

    if action.is_some() {
        return Err(AjisaiError::from("Async operations not supported in vector execution"));
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
        // [ [ 2 ] * ] → "[ 2 ] *"
        let val = Value::from_vector(vec![
            Value::from_vector(vec![Value::from_int(2)]),
            Value::from_string("*"),
        ]);
        let source = vector_to_source(&val).unwrap();
        assert_eq!(source, "[ 2 ] *");
    }

    #[test]
    fn test_vector_to_source_symbols() {
        // [ DUP * ] → "DUP *"
        let val = Value::from_vector(vec![
            Value::from_string("DUP"),
            Value::from_string("*"),
        ]);
        let source = vector_to_source(&val).unwrap();
        assert_eq!(source, "DUP *");
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
        let val = Value::from_vector(vec![
            Value::from_fraction(Fraction::new(BigInt::from(1), BigInt::from(3))),
        ]);
        let source = vector_to_source(&val).unwrap();
        assert_eq!(source, "1/3");
    }
}
