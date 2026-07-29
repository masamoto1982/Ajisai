use crate::error::{AjisaiError, Result};
use crate::interpreter::Interpreter;
use crate::types::Value;

pub(crate) fn op_exec(interp: &mut Interpreter) -> Result<()> {
    let target_vector: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    crate::interpreter::vector_exec::execute_vector_as_code(interp, &target_vector)
}
