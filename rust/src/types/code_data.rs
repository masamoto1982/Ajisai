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
                if !crate::tokenizer::is_number_token_lexeme(payload) {
                    return Err(malformed("invalid number payload"));
                }
                Token::Number(payload.into())
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
                if !crate::tokenizer::is_symbol_token_lexeme(payload) {
                    return Err(malformed("invalid symbol payload"));
                }
                Token::Symbol(payload.into())
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
    use proptest::prelude::*;
    use std::sync::Arc;

    fn record(parts: &[&str]) -> Value {
        Value::from_vector(parts.iter().map(|part| Value::from_string(part)).collect())
    }

    fn data(records: Vec<Value>) -> Value {
        let mut values = vec![Value::from_string(CODE_DATA_VERSION)];
        values.extend(records);
        Value::from_vector(values)
    }

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
        let malformed = vec![
            Value::from_vector(vec![]),
            Value::from_vector(vec![Value::from_string("wrong-header")]),
            data(vec![Value::from_int(1)]),
            data(vec![Value::from_vector(vec![])]),
            data(vec![record(&["number"])]),
            data(vec![record(&["number", "1", "extra"])]),
            data(vec![Value::from_vector(vec![Value::from_int(1)])]),
            data(vec![Value::from_vector(vec![
                Value::from_string("number"),
                Value::from_int(1),
            ])]),
            data(vec![record(&["unknown"])]),
            data(vec![record(&["NUMBER", "1"])]),
            data(vec![record(&["number", "1 2"])]),
            data(vec![record(&["number", "ADD"])]),
            data(vec![record(&["symbol", ""])]),
            data(vec![record(&["symbol", "A B"])]),
            data(vec![record(&["vector-end"])]),
            data(vec![record(&["block-end"])]),
            data(vec![record(&["vector-start"])]),
            data(vec![record(&["block-start"])]),
            data(vec![record(&["vector-start"]), record(&["block-end"])]),
            data(vec![record(&["cond-clause-sep"])]),
        ];
        for value in malformed {
            assert!(code_data_to_tokens(&value).is_err(), "accepted {value:?}");
        }
    }

    #[test]
    fn lexical_spelling_and_string_symbol_distinction_are_preserved() {
        let tokens = vec![
            Token::Number("1".into()),
            Token::Number("1/1".into()),
            Token::Number("1.0".into()),
            Token::Symbol("ADD".into()),
            Token::Symbol("add".into()),
            Token::Symbol("+".into()),
            Token::String("ADD".into()),
            Token::String("".into()),
            Token::String(" whitespace\n🙂e\u{301}'[]{} ".into()),
        ];
        assert_eq!(
            code_data_to_tokens(&tokens_to_code_data(&tokens)).unwrap(),
            tokens
        );
    }

    proptest! {
        #[test]
        fn payload_token_sequences_round_trip(
            strings in prop::collection::vec(any::<String>(), 0..20),
            numbers in prop::collection::vec(prop::sample::select(vec!["0", "-1", "1/1", "1.0", ".5"]), 0..20),
            symbols in prop::collection::vec(prop::sample::select(vec!["ADD", "add", "+", "UNKNOWN-WORD", ">CF"]), 0..20),
        ) {
            let mut tokens = Vec::new();
            tokens.extend(strings.into_iter().map(|s| Token::String(s.into())));
            tokens.extend(numbers.into_iter().map(|s| Token::Number(s.into())));
            tokens.extend(symbols.into_iter().map(|s| Token::Symbol(s.into())));
            let encoded = tokens_to_code_data(&tokens);
            prop_assert_eq!(code_data_to_tokens(&encoded).unwrap(), tokens);
            prop_assert_eq!(tokens_to_code_data(&code_data_to_tokens(&encoded).unwrap()), encoded);
        }
    }
}
