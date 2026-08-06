use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::{extract_integer_from_value, normalize_index};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::semantic::Recoverability;
use crate::types::Value;

/// Every index the operand names, in the order it names them.
///
/// The index operand is written as a Vector (`[ 0 ]`), so a Vector of several
/// indices is the shape a reader already has in hand — and `[ 10 20 30 40 ] [ 0 2 ] GET`
/// used to be an error, which made "select these positions" a question the
/// language could not ask. Selecting a permutation of a Vector, or the labels
/// that go with a set of distances, is not an exotic operation; without it
/// every such program has to encode the pairing into the numbers themselves
/// and decode it afterwards, and the encoding's soundness condition is one the
/// language can neither express nor check.
///
/// A single index is unchanged and still answers with the element itself, so
/// this is a generalization rather than a new rule: one index selects a value,
/// several select a Vector of values.
fn index_list(value: &Value) -> Result<Vec<i64>> {
    let count = match &value.data {
        crate::types::ValueData::Vector(children) => children.len(),
        crate::types::ValueData::Tensor { data, .. } => data.len(),
        _ => return Ok(vec![extract_integer_from_value(value)?]),
    };
    if count <= 1 {
        return Ok(vec![extract_integer_from_value(value)?]);
    }
    (0..count)
        .map(|position| {
            let child = value
                .child(position)
                .ok_or_else(|| AjisaiError::create_structure_error("integer", "absent element"))?;
            extract_integer_from_value(&child)
        })
        .collect()
}

fn pop_index_operand(interp: &mut Interpreter) -> Result<(Value, Vec<i64>)> {
    let index_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let indices = match index_list(&index_val) {
        Ok(value) => value,
        Err(error) => {
            interp.stack.push(index_val);
            return Err(error);
        }
    };
    Ok((index_val, indices))
}

pub fn op_get(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let (index_val, indices) = pop_index_operand(interp)?;

    // `GET` declares `consumption: eat` with `[ vec ] [ idx ] -> [ elem ]`, so
    // the vector operand leaves the stack unless `KEEP` is in force.
    let target_val = match if is_keep_mode {
        interp.stack.last().cloned()
    } else {
        interp.stack.pop()
    } {
        Some(value) => value,
        None => {
            interp.stack.push(index_val);
            return Err(AjisaiError::StackUnderflow);
        }
    };

    if !target_val.is_vector() {
        if !is_keep_mode {
            interp.stack.push(target_val);
        }
        interp.stack.push(index_val);
        return Err(AjisaiError::create_structure_error(
            "vector",
            "other format",
        ));
    }

    // An index that names nothing projects where it stands: with one index
    // that is the whole result, and with several it is one NIL among the
    // selected elements, so the answer keeps the shape of the request and the
    // miss stays attached to the position that missed it.
    let len = target_val.len();
    let select = |index: i64| -> Value {
        if len == 0 { None } else { normalize_index(index, len) }
            .and_then(|idx| target_val.child(idx))
            .unwrap_or_else(|| {
                Value::bubble_with_reason(NilReason::IndexOutOfBounds, Recoverability::Recoverable)
            })
    };
    let result_elem = match indices.as_slice() {
        [only] => select(*only),
        several => Value::from_vector(several.iter().map(|index| select(*index)).collect()),
    };

    if is_keep_mode {
        interp.stack.push(index_val);
    }
    interp.stack.push(result_elem);
    Ok(())
}
