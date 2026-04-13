

use crate::error::{AjisaiError, Result};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::types::Value;
use std::fmt::Write;

fn extract_value_for_print(interp: &mut Interpreter, keep_mode: bool) -> Result<Value> {
    if keep_mode {
        return interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow);
    }
    interp.stack.pop().ok_or(AjisaiError::StackUnderflow)
}


pub fn op_print(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let val = extract_value_for_print(interp, is_keep_mode)?;
    write!(&mut interp.output_buffer, "{} ", val)
        .map_err(|e| AjisaiError::from(format!("PRINT failed: {}", e)))?;
    Ok(())
}
