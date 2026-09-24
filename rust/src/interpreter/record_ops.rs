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
use crate::interpreter::Interpreter;
use crate::semantic::Recoverability;
use crate::types::{RecordBuildError, RecordData, Value};

/// Put a Word's consumed operands back, in order.
fn restore_all(interp: &mut Interpreter, operands: Vec<Value>) {
    for operand in operands {
        interp.stack.push(operand);
    }
}

fn push_record(interp: &mut Interpreter, record: RecordData) {
    interp.stack.push(Value::from_record(record));
}

/// The declared `nonRecord` condition, naming the position and what was
/// found there.
fn non_record(position: &str, got: &Value) -> AjisaiError {
    AjisaiError::declared(
        "nonRecord",
        format!("expected a Record as {position}, got {}", got.domain_name()),
    )
}

/// The `notFound` absence `AT` and `WITHOUT` project for a key the
/// Record does not hold.
fn not_found() -> Value {
    Value::nil_with_reason(NilReason::NotFound, Recoverability::Recoverable)
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
                    "the key at position {second} repeats the key at position {first}; \
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
        let err = non_record("its operand", &operand);
        restore(interp, operand);
        return Err(err);
    };
    let keys = Value::from_vector(record.keys().to_vec());
    interp.stack.push(keys);
    Ok(())
}

/// `VALUES ( [ record ] -> [ values ] )`.
pub fn op_values(interp: &mut Interpreter) -> Result<()> {
    let operand = take_operand(interp)?;
    let Some(record) = operand.as_record() else {
        let err = non_record("its operand", &operand);
        restore(interp, operand);
        return Err(err);
    };
    let values = Value::from_vector(record.values().to_vec());
    interp.stack.push(values);
    Ok(())
}

/// `AT ( [ record ] [ key ] -> [ value ] )`: projects `notFound`.
pub fn op_at(interp: &mut Interpreter) -> Result<()> {
    let operands = extract_operands(interp, 2)?;
    let Some(record) = operands[0].as_record() else {
        let err = non_record("the first operand", &operands[0]);
        restore_all(interp, operands);
        return Err(err);
    };
    let answer = match record.get(&operands[1]) {
        Some(value) => value.clone(),
        None => not_found(),
    };
    interp.stack.push(answer);
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
        let err = non_record("the first operand", &operands[0]);
        restore_all(interp, operands);
        return Err(err);
    };
    if operands[1].is_operational_nil() {
        let err = non_record("the key", &operands[1]);
        restore_all(interp, operands);
        return Err(err);
    }
    let next = record.with(operands[1].clone(), operands[2].clone());
    push_record(interp, next);
    Ok(())
}

/// `WITHOUT ( [ record ] [ key ] -> [ record ] )`: projects `notFound`.
pub fn op_without(interp: &mut Interpreter) -> Result<()> {
    let operands = extract_operands(interp, 2)?;
    let Some(record) = operands[0].as_record() else {
        let err = non_record("the first operand", &operands[0]);
        restore_all(interp, operands);
        return Err(err);
    };
    match record.without(&operands[1]) {
        Some(next) => {
            push_record(interp, next);
        }
        None => {
            interp.stack.push(not_found());
        }
    }
    Ok(())
}

/// `HAS? ( [ record ] [ key ] -> [ TRUE | FALSE ] )`.
pub fn op_has(interp: &mut Interpreter) -> Result<()> {
    let operands = extract_operands(interp, 2)?;
    let Some(record) = operands[0].as_record() else {
        let err = non_record("the first operand", &operands[0]);
        restore_all(interp, operands);
        return Err(err);
    };
    let present = record.has(&operands[1]);
    interp.stack.push(Value::from_bool(present));
    Ok(())
}

/// `MERGE ( [ record ] [ record ] -> [ record ] )`: right-biased union.
pub fn op_merge(interp: &mut Interpreter) -> Result<()> {
    let operands = extract_operands(interp, 2)?;
    let (Some(left), Some(right)) = (operands[0].as_record(), operands[1].as_record()) else {
        let (position, got) = if operands[0].as_record().is_none() {
            ("the first operand", &operands[0])
        } else {
            ("the second operand", &operands[1])
        };
        let err = non_record(position, got);
        restore_all(interp, operands);
        return Err(err);
    };
    if let Err(e) = charge_key_scan(interp, right.keys()) {
        restore_all(interp, operands);
        return Err(e);
    }
    let merged = left.merge(right);
    push_record(interp, merged);
    Ok(())
}
