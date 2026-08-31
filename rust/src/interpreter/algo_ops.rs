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

fn pop_vector_and_target(interp: &mut Interpreter, _word: &str) -> Result<(Vec<Value>, Value)> {
    let operands = extract_operands(interp, 2)?;
    match operands[0].as_vector_view() {
        Some(view) => {
            let vector = view.into_owned();
            Ok((vector, operands[1].clone()))
        }
        None => {
            restore_operands(interp, operands);
            // A noun phrase, not a sentence: the template around it already
            // says "expected _, got _", and the failing Word's name is the
            // diagnosis locus rather than part of the message.
            Err(AjisaiError::create_structure_error(
                "vector as first operand",
                "non-vector value",
            ))
        }
    }
}

/// `vector value -- index`. Index of the first element equal to the target.
/// A well-formed miss (value absent from a valid vector) projects to
/// NIL with `reason = missingField` per the NIL Projection Rule.
pub fn op_index_of(interp: &mut Interpreter) -> Result<()> {
    require_stack_top(interp, "INDEX-OF")?;
    let (vector, target) = pop_vector_and_target(interp, "INDEX-OF")?;
    // A linear search, priced at its worst case — the miss, which is the only
    // outcome that has to walk the whole vector. The count is known before the
    // scan starts, unlike the distinct-value scans, so this is a pre-charge.
    let units = crate::interpreter::collection_meter::element_cost_of_slice(&vector)
        .probe()
        .saturating_mul(vector.len() as u64);
    if let Err(e) = crate::interpreter::collection_meter::charge(interp, units) {
        restore_operands(interp, vec![Value::from_vector(vector), target]);
        return Err(e);
    }
    match vector.iter().position(|elem| elem == &target) {
        Some(index) => {
            push_result(interp, Value::from_int(index as i64));
            interp.stack.set_last_role(Interpretation::RawNumber);
        }
        None => {
            push_result(
                interp,
                Value::nil_with_reason_and_recoverability(
                    NilReason::MissingField,
                    Recoverability::Recoverable,
                ),
            );
        }
    }
    Ok(())
}
