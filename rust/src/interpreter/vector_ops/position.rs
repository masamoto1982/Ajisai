use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::{extract_integer_from_value, normalize_index};
use crate::interpreter::Interpreter;
use crate::semantic::Recoverability;
use crate::types::Value;

/// `extract_integer_from_value`, with a structurally malformed index operand
/// reclassified as the declared `invalidInteger` — GET's own condition for "not
/// itself a well-formed index (non-integer, wrong shape)", distinct from
/// `indexOutOfBounds` (a well-formed index outside bounds, which is a NIL
/// projection, not this ERROR). The message names an index, which is what
/// this caller of the shared helper knows the integer was for.
fn require_index_operand(value: &Value) -> Result<i64> {
    extract_integer_from_value(value).map_err(|e| {
        AjisaiError::declared(
            "invalidInteger",
            format!("expected a well-formed index, got {}", e.got),
        )
    })
}

/// `GET ( [ container ] [ key ] -> [ value ] )`: the element of a Vector at an
/// index, or the value of a Record under a key — one Word for reading a
/// container, as `PUT` is one Word for writing one.
///
/// The key is a `leaf`, so a Vector of indices or keys lifts at dispatch
/// (LANG.COLLECTIONS.LIFT) and this primitive only ever sees one. What names
/// nothing projects: an index outside the Vector is `indexOutOfBounds`, a key
/// the Record does not hold is `notFound`.
pub fn op_get(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::stack_underflow());
    }
    let key = interp.stack.pop().expect("checked by len()");
    let target = interp.stack.pop().expect("checked by len()");
    let restore = |interp: &mut Interpreter, target: Value, key: Value| {
        interp.stack.push(target);
        interp.stack.push(key);
    };

    if let Some(record) = target.as_record() {
        let answer = match record.get(&key) {
            Some(value) => value.clone(),
            None => Value::nil_with_reason(NilReason::NotFound, Recoverability::Recoverable),
        };
        interp.stack.push(answer);
        return Ok(());
    }

    if !target.is_vector() {
        let got = target.domain_name();
        restore(interp, target, key);
        return Err(AjisaiError::declared(
            "nonContainer",
            format!("expected a Vector or a Record, got {got}"),
        ));
    }

    let index = match require_index_operand(&key) {
        Ok(index) => index,
        Err(e) => {
            restore(interp, target, key);
            return Err(e);
        }
    };

    // One element is selected, so one element is priced: `GET` is the one
    // Word in the family whose cost does not track the operand it is handed.
    if let Err(e) = crate::interpreter::collection_meter::charge_copy_of(interp, &target, 1) {
        restore(interp, target, key);
        return Err(e);
    }

    let len = target.len();
    let answer = if len == 0 {
        None
    } else {
        normalize_index(index, len)
    }
    .and_then(|position| target.child(position))
    .unwrap_or_else(|| {
        Value::nil_with_reason(NilReason::IndexOutOfBounds, Recoverability::Recoverable)
    });
    interp.stack.push(answer);
    Ok(())
}
