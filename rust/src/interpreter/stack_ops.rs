use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};

pub fn op_dup(interp: &mut Interpreter) -> Result<()> {
    let top = interp.peek_value()
        .ok_or(AjisaiError::StackUnderflow)?;
    interp.push_value(top.clone());
    Ok(())
}

pub fn op_drop(interp: &mut Interpreter) -> Result<()> {
    interp.pop_value()
        .ok_or(AjisaiError::StackUnderflow)?;
    Ok(())
}

pub fn op_swap(interp: &mut Interpreter) -> Result<()> {
    let len = interp.stack.len();
    if len < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    interp.stack.swap(len - 1, len - 2);
    Ok(())
}

pub fn op_over(interp: &mut Interpreter) -> Result<()> {
    let len = interp.stack.len();
    if len < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    let item = interp.stack[len - 2].value.clone();
    interp.push_value(item);
    Ok(())
}

pub fn op_rot(interp: &mut Interpreter) -> Result<()> {
    let len = interp.stack.len();
    if len < 3 {
        return Err(AjisaiError::StackUnderflow);
    }
    let third = interp.stack.remove(len - 3);
    interp.stack.push(third);
    Ok(())
}

pub fn op_nip(interp: &mut Interpreter) -> Result<()> {
    let len = interp.stack.len();
    if len < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    interp.stack.remove(len - 2);
    Ok(())
}

pub fn op_to_r(interp: &mut Interpreter) -> Result<()> {
    let val = interp.pop_value()
        .ok_or(AjisaiError::StackUnderflow)?;
    interp.register = Some(val);
    Ok(())
}

pub fn op_from_r(interp: &mut Interpreter) -> Result<()> {
    let val = interp.register.take()
        .ok_or(AjisaiError::RegisterEmpty)?;
    interp.push_value(val);
    Ok(())
}

pub fn op_r_fetch(interp: &mut Interpreter) -> Result<()> {
    let val = interp.register.as_ref()
        .ok_or(AjisaiError::RegisterEmpty)?;
    interp.push_value(val.clone());
    Ok(())
}
