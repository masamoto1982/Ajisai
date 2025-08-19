// rust/src/interpreter/quotation.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType};

pub fn op_call(interp: &mut Interpreter) -> Result<()> {
    let quotation_val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    let tokens = match quotation_val.val_type {
        ValueType::Quotation(t) => t,
        _ => return Err(AjisaiError::type_error("quotation", "other type")),
    };
    
    interp.execute_tokens(&tokens)
}
