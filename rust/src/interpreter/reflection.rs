use crate::error::{AjisaiError, Result};
use crate::interpreter::cast::cast_value_helpers::apply_unary_cast;
use crate::interpreter::Interpreter;
use crate::types::code_data::{code_data_to_tokens, tokens_to_code_data};
use crate::types::{Value, ValueData};

pub(crate) fn op_reflect(interp: &mut Interpreter) -> Result<()> {
    apply_unary_cast(interp, reflect)
}

fn reflect(value: &Value) -> Result<Value> {
    match &value.data {
        ValueData::CodeBlock(tokens) => Ok(tokens_to_code_data(tokens)),
        ValueData::Vector(_) => Ok(Value::from_code_block(code_data_to_tokens(value)?)),
        _ => Err(AjisaiError::from(
            "REFLECT requires a CodeBlock or canonical code-data Vector",
        )),
    }
}
