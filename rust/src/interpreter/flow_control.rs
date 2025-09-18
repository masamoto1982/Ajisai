// rust/src/interpreter/flow_control.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}, WordExecutionState};
use crate::types::ValueType;
use num_bigint::BigInt;
use num_traits::One;

pub fn op_goto(interp: &mut Interpreter) -> Result<()> {
    let line_num_val = interp.workspace.pop().ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    let line_num = match line_num_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => {
            if let ValueType::Number(n) = &v[0].val_type {
                if n.denominator == BigInt::one() {
                    n.to_i64().ok_or_else(|| AjisaiError::from("Line number is too large"))?
                } else {
                    return Err(AjisaiError::type_error("integer", "fraction"));
                }
            } else {
                return Err(AjisaiError::type_error("number", "other type"));
            }
        },
        _ => return Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    };

    if line_num <= 0 {
        return Err(AjisaiError::from("GOTO line number must be positive"));
    }

    if let Some(state) = interp.execution_state.as_mut() {
        // GOTOは次のイテレーションでジャンプするため、PCを(N-1)に設定し、ループを継続させる
        state.program_counter = (line_num - 1) as usize;
        state.continue_loop = true; // ループを抜けないようにフラグを立てる
    } else {
        return Err(AjisaiError::from("GOTO can only be used inside a custom word"));
    }

    Ok(())
}
