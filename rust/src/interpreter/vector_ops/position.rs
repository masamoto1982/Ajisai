use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::{extract_integer_from_value, normalize_index};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::semantic::Recoverability;
use crate::types::Value;

fn pop_index_operand(interp: &mut Interpreter) -> Result<(Value, i64)> {
    let index_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let index = match extract_integer_from_value(&index_val) {
        Ok(value) => value,
        Err(error) => {
            interp.stack.push(index_val);
            return Err(error);
        }
    };
    Ok((index_val, index))
}

pub fn op_get(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let (index_val, index) = pop_index_operand(interp)?;

    let target_val = match interp.stack.last().cloned() {
        Some(value) => value,
        None => {
            interp.stack.push(index_val);
            return Err(AjisaiError::StackUnderflow);
        }
    };

    if !target_val.is_vector() {
        interp.stack.push(index_val);
        return Err(AjisaiError::create_structure_error(
            "vector",
            "other format",
        ));
    }

    let result_elem = {
        let len = target_val.len();
        let actual_index = if len == 0 {
            None
        } else {
            normalize_index(index, len)
        };

        actual_index
            .and_then(|idx| target_val.child(idx))
            .unwrap_or_else(|| {
                Value::bubble_with_reason(NilReason::IndexOutOfBounds, Recoverability::Recoverable)
            })
    };

    if is_keep_mode {
        interp.stack.push(index_val);
    }
    interp.stack.push(result_elem);
    Ok(())
}
