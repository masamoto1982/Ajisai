//! Lifting the arithmetic Words over Records
//! (LANG.COLLECTIONS.LIFT, LANG.RECORDS.STRUCTURE).
//!
//! A Record lifts in the value direction only: the keys are untouched and the
//! result is a Record over the same key sequence. Two Records combine when
//! their key sequences are equal, pairing values position by position; a
//! Record combines with anything else by applying the Word to each value and
//! that other operand. The comparison and logic Words lift over a Record
//! through `lane_lift`, and every other Word through the dispatcher
//! (`declared_lift`); this is the arithmetic family's entry to the same rule,
//! ahead of its tensor path.
//!
//! The lift is generic over the Word rather than written once per Word: the
//! Word's own entry point is run on each value pair on a scratch region of
//! the stack, so every scalar and Vector law, every NIL rule, every cost
//! charge and every declared ERROR is exactly the one the Word already has.
//! Nothing here restates a numeric law.

use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::extract_operands;
use crate::interpreter::Interpreter;
use crate::types::{RecordData, Value};

type WordOp<'a> = &'a dyn Fn(&mut Interpreter) -> Result<()>;

/// Lift a unary Word over a Record operand at the top of the stack. Answers
/// `Ok(false)` when the operand is not a Record, having touched nothing.
pub(crate) fn lift_unary(interp: &mut Interpreter, op: WordOp) -> Result<bool> {
    let is_record = interp
        .stack
        .last()
        .is_some_and(|top| top.as_record().is_some());
    if !is_record {
        return Ok(false);
    }
    let operands = extract_operands(interp, 1)?;
    run_lift(interp, operands, &|interp, operands| {
        let record = operands[0].as_record().expect("checked above");
        record
            .map_values(|value| leaf(interp, vec![value.clone()], op))
            .map(Value::from_record)
    })
}

/// Lift a binary Word over one or two Record operands at the top of the
/// stack. Answers `Ok(false)` when neither operand is a Record, having
/// touched nothing.
pub(crate) fn lift_binary(interp: &mut Interpreter, op: WordOp) -> Result<bool> {
    let len = interp.stack.len();
    if len < 2 {
        return Ok(false);
    }
    let slots = interp.stack.as_slice();
    if slots[len - 2].as_record().is_none() && slots[len - 1].as_record().is_none() {
        return Ok(false);
    }
    let operands = extract_operands(interp, 2)?;
    run_lift(interp, operands, &|interp, operands| {
        combine(interp, &operands[0], &operands[1], op)
    })
}

/// Run `lift` over consumed operands, then push the result or put the
/// operands back on an ERROR.
fn run_lift(
    interp: &mut Interpreter,
    operands: Vec<Value>,
    lift: &dyn Fn(&mut Interpreter, &[Value]) -> Result<Value>,
) -> Result<bool> {
    let result = lift(interp, &operands);
    match result {
        Ok(value) => {
            interp.stack.push(value);
            Ok(true)
        }
        Err(e) => {
            for operand in operands {
                interp.stack.push(operand);
            }
            Err(e)
        }
    }
}

fn combine(interp: &mut Interpreter, a: &Value, b: &Value, op: WordOp) -> Result<Value> {
    match (a.as_record(), b.as_record()) {
        (Some(left), Some(right)) => {
            if !left.same_keys(right) {
                return Err(AjisaiError::declared(
                    "shapeMismatch",
                    format!(
                        "two Records combine only when their key sequences are equal; \
                         got {} keys against {} keys that do not line up",
                        left.len(),
                        right.len()
                    ),
                ));
            }
            let values = left
                .values()
                .iter()
                .zip(right.values())
                .map(|(x, y)| combine(interp, x, y, op))
                .collect::<Result<Vec<_>>>()?;
            Ok(Value::from_record(
                RecordData::new(left.keys().to_vec(), values)
                    .expect("the keys are the left Record's own, so distinct and aligned"),
            ))
        }
        (Some(left), None) => left
            .map_values(|x| combine(interp, x, b, op))
            .map(Value::from_record),
        (None, Some(right)) => right
            .map_values(|y| combine(interp, a, y, op))
            .map(Value::from_record),
        (None, None) => leaf(interp, vec![a.clone(), b.clone()], op),
    }
}

/// Run the Word on `operands` in a scratch region above the current stack
/// and take its single result back off.
fn leaf(interp: &mut Interpreter, operands: Vec<Value>, op: WordOp) -> Result<Value> {
    let base = interp.stack.len();
    for operand in operands {
        interp.stack.push(operand);
    }
    let outcome = op(interp).map(|()| {
        // Every Word lifted over a Record's values has one declared output,
        // so this is the arity the registry states, not a check on input.
        assert_eq!(
            interp.stack.len(),
            base + 1,
            "a Word lifted over a Record's values leaves exactly one result"
        );
        interp
            .stack
            .pop()
            .expect("the result just asserted to be there")
    });
    interp.stack.truncate(base);
    outcome
}
