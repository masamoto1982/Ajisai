use crate::error::{AjisaiError, Result};
use crate::types::{Token, Value};

pub(crate) const CODE_DATA_VERSION: &str = "AJISAI-CODE-1";

pub(crate) fn tokens_to_code_data(tokens: &[Token]) -> Value {
    let mut values = Vec::with_capacity(tokens.len() + 1);
    values.push(Value::from_string(CODE_DATA_VERSION));
    values.extend(tokens.iter().map(|token| {
        let (tag, payload): (&str, Option<&str>) = match token {
            Token::Number(value) => ("number", Some(value)),
            Token::String(value) => ("string", Some(value)),
            Token::Symbol(value) => ("symbol", Some(value)),
            Token::VectorStart => ("vector-start", None),
            Token::VectorEnd => ("vector-end", None),
            Token::BlockStart => ("block-start", None),
            Token::BlockEnd => ("block-end", None),
            Token::Pipeline => ("pipeline", None),
            Token::NilCoalesce => ("nil-coalesce", None),
            Token::CondClauseSep => ("cond-clause-sep", None),
            Token::LineBreak => ("line-break", None),
        };
        let mut record = vec![Value::from_string(tag)];
        if let Some(payload) = payload {
            record.push(Value::from_string(payload));
        }
        Value::from_vector(record)
    }));
    Value::from_vector(values)
}

fn malformed(message: impl Into<String>) -> AjisaiError {
    AjisaiError::from(format!("malformed canonical code data: {}", message.into()))
}

pub(crate) fn code_data_to_tokens(value: &Value) -> Result<Vec<Token>> {
    let values = value
        .as_vector()
        .ok_or_else(|| malformed("expected Vector"))?;
    if values.first().and_then(Value::as_text) != Some(CODE_DATA_VERSION) {
        return Err(malformed("missing AJISAI-CODE-1 header"));
    }
    let mut tokens = Vec::with_capacity(values.len().saturating_sub(1));
    for value in &values[1..] {
        let record = value
            .as_vector()
            .ok_or_else(|| malformed("record is not a Vector"))?;
        let tag = record
            .first()
            .and_then(Value::as_text)
            .ok_or_else(|| malformed("record tag is not a String"))?;
        let token = match (tag, record.len()) {
            ("number", 2) => {
                let payload = record[1]
                    .as_text()
                    .ok_or_else(|| malformed("number payload is not a String"))?;
                match crate::tokenizer::tokenize(payload).ok().as_deref() {
                    Some([Token::Number(n)]) if n.as_ref() == payload => {
                        Token::Number(payload.into())
                    }
                    _ => return Err(malformed("invalid number payload")),
                }
            }
            ("string", 2) => Token::String(
                record[1]
                    .as_text()
                    .ok_or_else(|| malformed("string payload is not a String"))?
                    .into(),
            ),
            ("symbol", 2) => {
                let payload = record[1]
                    .as_text()
                    .ok_or_else(|| malformed("symbol payload is not a String"))?;
                match crate::tokenizer::tokenize(payload).ok().as_deref() {
                    Some([Token::Symbol(s)]) if s.as_ref() == payload => {
                        Token::Symbol(payload.into())
                    }
                    _ => return Err(malformed("invalid symbol payload")),
                }
            }
            ("vector-start", 1) => Token::VectorStart,
            ("vector-end", 1) => Token::VectorEnd,
            ("block-start", 1) => Token::BlockStart,
            ("block-end", 1) => Token::BlockEnd,
            ("pipeline", 1) => Token::Pipeline,
            ("nil-coalesce", 1) => Token::NilCoalesce,
            ("cond-clause-sep", 1) => Token::CondClauseSep,
            ("line-break", 1) => Token::LineBreak,
            ("number" | "string" | "symbol", _) => {
                return Err(malformed("payload record must contain exactly two fields"))
            }
            (
                "vector-start" | "vector-end" | "block-start" | "block-end" | "pipeline"
                | "nil-coalesce" | "cond-clause-sep" | "line-break",
                _,
            ) => {
                return Err(malformed(
                    "structural record must contain exactly one field",
                ))
            }
            _ => return Err(malformed("unknown token tag")),
        };
        tokens.push(token);
    }
    crate::tokenizer::validate_code_tokens(&tokens).map_err(malformed)?;
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn all_token_variants_round_trip() {
        let tokens = vec![
            Token::Number(Arc::from("1/1")),
            Token::String(Arc::from("🙂\n'")),
            Token::Symbol(Arc::from("add")),
            Token::VectorStart,
            Token::VectorEnd,
            Token::BlockStart,
            Token::Number(Arc::from("2")),
            Token::CondClauseSep,
            Token::Number(Arc::from("3")),
            Token::BlockEnd,
            Token::Pipeline,
            Token::NilCoalesce,
            Token::LineBreak,
        ];
        assert_eq!(
            code_data_to_tokens(&tokens_to_code_data(&tokens)).unwrap(),
            tokens
        );
    }

    #[test]
    fn malformed_records_are_rejected() {
        assert!(code_data_to_tokens(&Value::from_vector(vec![])).is_err());
        assert!(code_data_to_tokens(&Value::from_vector(vec![
            Value::from_string(CODE_DATA_VERSION),
            Value::from_vector(vec![Value::from_string("unknown")])
        ]))
        .is_err());
    }
}
