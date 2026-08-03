use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::{extract_operands, push_result};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::semantic::Recoverability;
use crate::types::{Interpretation, Value};

fn require_stack_top(_interp: &Interpreter, _word: &str) -> Result<()> {
    Ok(())
}

fn restore_operands(interp: &mut Interpreter, operands: Vec<Value>) {
    if interp.consumption_mode != ConsumptionMode::Keep {
        interp.stack.extend(operands);
    }
}

fn pop_vector_and_target(interp: &mut Interpreter, word: &str) -> Result<(Vec<Value>, Value)> {
    let operands = extract_operands(interp, 2)?;
    match operands[0].as_vector_view() {
        Some(view) => {
            let vector = view.into_owned();
            Ok((vector, operands[1].clone()))
        }
        None => {
            restore_operands(interp, operands);
            Err(AjisaiError::create_structure_error(
                &format!("{}: expected vector as first operand", word),
                "non-vector value",
            ))
        }
    }
}

/// `vector value -- index`. Index of the first element equal to the target.
/// A well-formed miss (value absent from a valid vector) projects to
/// Bubble/NIL with `reason = missingField` per the Bubble Rule.
pub fn op_index_of(interp: &mut Interpreter) -> Result<()> {
    require_stack_top(interp, "INDEX-OF")?;
    let (vector, target) = pop_vector_and_target(interp, "INDEX-OF")?;
    match vector.iter().position(|elem| elem == &target) {
        Some(index) => {
            push_result(interp, Value::from_int(index as i64));
            interp.stack.set_last_role(Interpretation::RawNumber);
        }
        None => {
            push_result(
                interp,
                Value::bubble_with_reason(NilReason::MissingField, Recoverability::Recoverable),
            );
        }
    }
    Ok(())
}
