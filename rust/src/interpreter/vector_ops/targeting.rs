use crate::error::{AjisaiError, Result};
use crate::interpreter::Interpreter;
use crate::types::Value;

pub(crate) fn with_stacktop_vector_target_with_arg<R, F>(
    interp: &mut Interpreter,
    arg_to_restore: &Value,
    preserve_source: bool,
    action: F,
) -> Result<R>
where
    F: FnOnce(&Value) -> Result<R>,
{
    let target_val = if preserve_source {
        interp.stack.last().cloned().ok_or_else(|| {
            interp.stack.push(arg_to_restore.clone());
            AjisaiError::StackUnderflow
        })?
    } else {
        interp.stack.pop().ok_or_else(|| {
            interp.stack.push(arg_to_restore.clone());
            AjisaiError::StackUnderflow
        })?
    };

    if !target_val.is_vector() {
        if !preserve_source {
            interp.stack.push(target_val);
        }
        interp.stack.push(arg_to_restore.clone());
        return Err(AjisaiError::create_structure_error("vector", "other format"));
    }

    match action(&target_val) {
        Ok(result) => Ok(result),
        Err(error) => {
            if !preserve_source {
                interp.stack.push(target_val);
            }
            interp.stack.push(arg_to_restore.clone());
            Err(error)
        }
    }
}

pub(crate) fn with_stacktop_vector_target_no_arg<R, F>(
    interp: &mut Interpreter,
    preserve_source: bool,
    action: F,
) -> Result<R>
where
    F: FnOnce(&Value) -> Result<R>,
{
    let target_val = if preserve_source {
        interp.stack.last().cloned().ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    if !target_val.is_vector() {
        if !preserve_source {
            interp.stack.push(target_val);
        }
        return Err(AjisaiError::create_structure_error("vector", "other format"));
    }

    match action(&target_val) {
        Ok(result) => Ok(result),
        Err(error) => {
            if !preserve_source {
                interp.stack.push(target_val);
            }
            Err(error)
        }
    }
}
