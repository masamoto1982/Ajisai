use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};

pub fn op_dot(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    interp.append_output(&format!("{} ", val));
    Ok(())
}

pub fn op_print(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.last()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    interp.append_output(&format!("{} ", val));
    Ok(())
}

pub fn op_cr(interp: &mut Interpreter) -> Result<()> {
    interp.append_output("\n");
    Ok(())
}
