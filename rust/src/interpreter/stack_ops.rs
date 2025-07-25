use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};

pub fn op_dup(interp: &mut Interpreter) -> Result<()> {
    let top = interp.stack.last()
        .ok_or(AjisaiError::StackUnderflow)?;
    interp.stack.push(top.clone());
    Ok(())
}

pub fn op_drop(interp: &mut Interpreter) -> Result<()> {
    interp.stack.pop()
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
    let item = interp.stack[len - 2].clone();
    interp.stack.push(item);
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
    let val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    interp.register = Some(val);
    Ok(())
}

pub fn op_from_r(interp: &mut Interpreter) -> Result<()> {
    let val = interp.register.take()
        .ok_or(AjisaiError::RegisterEmpty)?;
    interp.stack.push(val);
    Ok(())
}

pub fn op_r_fetch(interp: &mut Interpreter) -> Result<()> {
    let val = interp.register.as_ref()
        .ok_or(AjisaiError::RegisterEmpty)?;
    interp.stack.push(val.clone());
    Ok(())
}
