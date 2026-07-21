use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::{extract_operands, push_result};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::semantic::{AbsenceOrigin, Recoverability};
use crate::types::{Interpretation, Value};

fn require_stack_top(interp: &Interpreter, word: &str) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::from(format!(
            "{}: Stack mode is not supported",
            word
        )));
    }
    Ok(())
}

fn restore_operands(interp: &mut Interpreter, operands: Vec<Value>) {
    if interp.consumption_mode != ConsumptionMode::Keep {
        interp.stack.extend(operands);
    }
}

/// `vector -- vector`. Remove duplicate elements, keeping the first
/// occurrence and preserving order. An empty result projects to NIL,
/// matching `ALGO@SORT`.
pub fn op_unique(interp: &mut Interpreter) -> Result<()> {
    require_stack_top(interp, "UNIQUE")?;
    let operands = extract_operands(interp, 1)?;
    let view = match operands[0].as_vector_view() {
        Some(v) => v,
        None => {
            restore_operands(interp, operands);
            return Err(AjisaiError::create_structure_error(
                "UNIQUE: expected vector",
                "non-vector value",
            ));
        }
    };

    let mut seen: Vec<Value> = Vec::new();
    for elem in view.iter() {
        if !seen.iter().any(|kept| kept == elem) {
            seen.push(elem.clone());
        }
    }

    if seen.is_empty() {
        interp.stack.push(Value::nil());
    } else {
        interp.stack.push(Value::from_vector(seen));
    }
    Ok(())
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

/// `vector value -- bool`. True if the vector contains an element equal to
/// the target value.
pub fn op_contains(interp: &mut Interpreter) -> Result<()> {
    require_stack_top(interp, "CONTAINS")?;
    let (vector, target) = pop_vector_and_target(interp, "CONTAINS")?;
    let found = vector.iter().any(|elem| elem == &target);
    interp.stack.push(Value::from_bool(found));
    interp.stack.set_last_role(Interpretation::TruthValue);
    Ok(())
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
                Value::bubble_with_reason(
                    NilReason::MissingField,
                    AbsenceOrigin::ExecutionFailure,
                    Recoverability::Recoverable,
                ),
            );
        }
    }
    Ok(())
}
