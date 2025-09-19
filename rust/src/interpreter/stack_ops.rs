// rust/src/interpreter/stack_ops.rs
use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};

pub fn op_dup(interp: &mut Interpreter) -> Result<()> {
    let top = interp.workspace.last()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    interp.workspace.push(top.clone());
    Ok(())
}

pub fn op_drop(interp: &mut Interpreter) -> Result<()> {
    interp.workspace.pop()
        .ok_or(AjisaiError::WorkspaceUnderflow)?;
    Ok(())
}

pub fn op_swap(interp: &mut Interpreter) -> Result<()> {
    let len = interp.workspace.len();
    if len < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    interp.workspace.swap(len - 1, len - 2);
    Ok(())
}

pub fn op_over(interp: &mut Interpreter) -> Result<()> {
    let len = interp.workspace.len();
    if len < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    let item = interp.workspace[len - 2].clone();
    interp.workspace.push(item);
    Ok(())
}

pub fn op_rot(interp: &mut Interpreter) -> Result<()> {
    let len = interp.workspace.len();
    if len < 3 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    let third = interp.workspace.remove(len - 3);
    interp.workspace.push(third);
    Ok(())
}

pub fn op_nip(interp: &mut Interpreter) -> Result<()> {
    let len = interp.workspace.len();
    if len < 2 {
        return Err(AjisaiError::WorkspaceUnderflow);
    }
    interp.workspace.remove(len - 2);
    Ok(())
}
