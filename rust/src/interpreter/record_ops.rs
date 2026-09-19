//! The Record Words: `RECORD`, `KEYS`, `VALUES`, `AT`, `WITH`, `WITHOUT`,
//! `HAS?`, `MERGE` (LANG.RECORDS.STRUCTURE).
//!
//! A Record is the seventh value domain: a keyed correspondence whose keys
//! keep the order they arrived in. It is the one shape the parallel-vector
//! idiom could only imitate — `INDEX-OF` then `GET` scans every key where
//! `AT` hashes one — and it is what structured data from a host arrives as.
//! Nothing here converts a Vector to a Record or back on its own: `RECORD`
//! and `KEYS`/`VALUES` are the only bridges, both explicit.
//!
//! Every Word puts its operands back before raising (the ERROR discipline of
//! every Core Word), and every Record it hands back is a new value: a Record
//! on the stack is never mutated in place.

use super::ordering_ops::{elements_of, restore, take_operand};
use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::collection_meter::ScanMeter;
use crate::interpreter::value_extraction_helpers::extract_operands;
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::semantic::Recoverability;
use crate::types::{Interpretation, RecordBuildError, RecordData, Value};

/// Put a Word's consumed operands back, in order, when it did consume them.
fn restore_all(interp: &mut Interpreter, operands: Vec<Value>) {
    if interp.consumption_mode != ConsumptionMode::Keep {
        for operand in operands {
            interp.stack.push(operand);
        }
    }
}

fn push_record(interp: &mut Interpreter, record: RecordData) {
    interp
        .stack
        .push_with_role(Value::from_record(record), Interpretation::Unassigned);
}

/// The declared `nonRecord` condition, naming the Word and the position.
fn non_record(word: &str, position: &str) -> AjisaiError {
    AjisaiError::declared(
        "nonRecord",
        format!("{word}: expected a Record as {position}, got a non-record value"),
    )
}

/// The `missingField` absence `AT` and `WITHOUT` project for a key the
/// Record does not hold.
fn missing_field() -> Value {
    Value::nil_with_reason(NilReason::MissingField, Recoverability::Recoverable)
}

/// Charge for hashing every key of a Record being built or rebuilt: one
/// hash-keyed scan, the same price `UNIQUE` pays for the same work.
fn charge_key_scan(interp: &mut Interpreter, keys: &[Value]) -> Result<()> {
    let meter = ScanMeter::new(keys);
    for completed in 0..keys.len() {
        meter.charge_scan_of(interp, completed)?;
        meter.charge_retained(interp, completed)?;
    }
    Ok(())
}

/// `RECORD ( [ keys ] [ values ] -> [ record ] )`.
pub fn op_record(interp: &mut Interpreter) -> Result<()> {
    let operands = extract_operands(interp, 2)?;
    let outcome = (|| {
        let keys = elements_of(&operands[0], "a Vector of keys as the first operand")?;
        let values = elements_of(&operands[1], "a Vector of values as the second operand")?;
        charge_key_scan(interp, &keys)?;
        RecordData::new(keys, values).map_err(|e| match e {
            RecordBuildError::LengthMismatch { keys, values } => {
                AjisaiError::VectorLengthMismatch {
                    len1: keys,
                    len2: values,
                }
            }
            RecordBuildError::DuplicateKey { first, second } => AjisaiError::declared(
                "duplicateKey",
                format!(
                    "RECORD: the key at position {second} repeats the key at position {first}; \
                     a Record holds each key once"
                ),
            ),
        })
    })();
    match outcome {
        Ok(record) => {
            push_record(interp, record);
            Ok(())
        }
        Err(e) => {
            restore_all(interp, operands);
            Err(e)
        }
    }
}

/// `KEYS ( [ record ] -> [ keys ] )`.
pub fn op_keys(interp: &mut Interpreter) -> Result<()> {
    let operand = take_operand(interp)?;
    let Some(record) = operand.as_record() else {
        restore(interp, operand);
        return Err(non_record("KEYS", "its operand"));
    };
    let keys = Value::from_vector(record.keys().to_vec());
    interp
        .stack
        .push_with_role(keys, Interpretation::Unassigned);
    Ok(())
}

/// `VALUES ( [ record ] -> [ values ] )`.
pub fn op_values(interp: &mut Interpreter) -> Result<()> {
    let operand = take_operand(interp)?;
    let Some(record) = operand.as_record() else {
        restore(interp, operand);
        return Err(non_record("VALUES", "its operand"));
    };
    let values = Value::from_vector(record.values().to_vec());
    interp
        .stack
        .push_with_role(values, Interpretation::Unassigned);
    Ok(())
}

/// `AT ( [ record ] [ key ] -> [ value ] )`: projects `missingField`.
pub fn op_at(interp: &mut Interpreter) -> Result<()> {
    let operands = extract_operands(interp, 2)?;
    let Some(record) = operands[0].as_record() else {
        restore_all(interp, operands);
        return Err(non_record("AT", "the first operand"));
    };
    let answer = match record.get(&operands[1]) {
        Some(value) => value.clone(),
        None => missing_field(),
    };
    let role = if answer.is_nil() {
        Interpretation::Nil
    } else {
        answer.hint
    };
    interp.stack.push_with_role(answer, role);
    Ok(())
}

/// `WITH ( [ record ] [ key ] [ value ] -> [ record ] )`.
///
/// Declared `consumeNil` rather than `rejectNil`: a NIL *value* is a stored
/// absence and belongs under a key, so only a NIL Record or a NIL key is
/// malformed use, and the Word decides that itself.
pub fn op_with(interp: &mut Interpreter) -> Result<()> {
    let operands = extract_operands(interp, 3)?;
    let Some(record) = operands[0].as_record() else {
        restore_all(interp, operands);
        return Err(non_record("WITH", "the first operand"));
    };
    if operands[1].is_operational_nil() {
        restore_all(interp, operands);
        return Err(non_record("WITH", "the key (got NIL)"));
    }
    let next = record.with(operands[1].clone(), operands[2].clone());
    push_record(interp, next);
    Ok(())
}

/// `WITHOUT ( [ record ] [ key ] -> [ record ] )`: projects `missingField`.
pub fn op_without(interp: &mut Interpreter) -> Result<()> {
    let operands = extract_operands(interp, 2)?;
    let Some(record) = operands[0].as_record() else {
        restore_all(interp, operands);
        return Err(non_record("WITHOUT", "the first operand"));
    };
    match record.without(&operands[1]) {
        Some(next) => {
            push_record(interp, next);
        }
        None => {
            interp
                .stack
                .push_with_role(missing_field(), Interpretation::Nil);
        }
    }
    Ok(())
}

/// `HAS? ( [ record ] [ key ] -> [ TRUE | FALSE ] )`.
pub fn op_has(interp: &mut Interpreter) -> Result<()> {
    let operands = extract_operands(interp, 2)?;
    let Some(record) = operands[0].as_record() else {
        restore_all(interp, operands);
        return Err(non_record("HAS?", "the first operand"));
    };
    let present = record.has(&operands[1]);
    interp
        .stack
        .push_with_role(Value::from_bool(present), Interpretation::TruthValue);
    Ok(())
}

/// `MERGE ( [ record ] [ record ] -> [ record ] )`: right-biased union.
pub fn op_merge(interp: &mut Interpreter) -> Result<()> {
    let operands = extract_operands(interp, 2)?;
    let (Some(left), Some(right)) = (operands[0].as_record(), operands[1].as_record()) else {
        let position = if operands[0].as_record().is_none() {
            "the first operand"
        } else {
            "the second operand"
        };
        restore_all(interp, operands);
        return Err(non_record("MERGE", position));
    };
    if let Err(e) = charge_key_scan(interp, right.keys()) {
        restore_all(interp, operands);
        return Err(e);
    }
    let merged = left.merge(right);
    push_record(interp, merged);
    Ok(())
}
