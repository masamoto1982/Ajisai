use crate::error::{AjisaiError, Result};
use crate::types::{Token, Value, ValueData};

fn value_to_literal_tokens(value: &Value) -> Result<Vec<Token>> {
    match &value.data {
        ValueData::Scalar(n) => Ok(vec![Token::Number(n.to_string().into())]),
        ValueData::Vector(values) => {
            let mut tokens = vec![Token::VectorStart];
            for child in values.iter() {
                match &child.data {
                    ValueData::Scalar(n) => tokens.push(Token::Number(n.to_string().into())),
                    _ => {
                        return Err(AjisaiError::from(
                            "PRECOMPUTE failed: result contains unsupported value type",
                        ))
                    }
                }
            }
            tokens.push(Token::VectorEnd);
            Ok(tokens)
        }
        _ => Err(AjisaiError::from(
            "PRECOMPUTE failed: result contains unsupported value type",
        )),
    }
}

pub(crate) fn stack_to_literal_tokens(stack: &[Value]) -> Result<Vec<Token>> {
    let mut out = Vec::new();
    for value in stack {
        out.extend(value_to_literal_tokens(value)?);
    }
    Ok(out)
}
