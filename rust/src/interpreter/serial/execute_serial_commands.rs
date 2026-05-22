//! Outbound `SERIAL` command words (Phase 1).
//!
//! Stack discipline follows the `MUSIC` precedent: each word pops its operands
//! and writes one `SERIAL:{json}` line to `interp.output_buffer`. Words that
//! represent an ongoing connection push the opaque port-id handle back so it
//! can be threaded through a pipeline.

use super::super::Interpreter;
use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{extract_integer_from_value, value_as_string};
use crate::types::{Value, ValueData};
use serde_json::json;

fn pop(interp: &mut Interpreter) -> Result<Value> {
    interp.stack.pop().ok_or(AjisaiError::StackUnderflow)
}

/// The handle is an opaque port id, surfaced as a text value. A Phase-2
/// dedicated handle type may replace this; until then a string is the handle.
///
/// A bare scalar would decode to a single codepoint, so the shape is guarded
/// here: only a text-shaped (vector) value is a valid port id. Passing a number
/// is misuse and raises a `StructureError` per the Bubble Rule.
fn require_port_id(val: &Value) -> Result<String> {
    if !matches!(&val.data, ValueData::Vector(_) | ValueData::Tensor { .. }) {
        return Err(AjisaiError::create_structure_error(
            "serial port-id text",
            "non-text value",
        ));
    }
    value_as_string(val)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AjisaiError::create_structure_error("serial port-id text", "non-text value"))
}

fn extract_bytes(val: &Value) -> Result<Vec<u8>> {
    let fractions = val.collect_fractions_flat();
    if fractions.is_empty() {
        return Err(AjisaiError::create_structure_error(
            "non-empty byte vector",
            "empty or non-numeric value",
        ));
    }
    let mut bytes = Vec::with_capacity(fractions.len());
    for f in fractions {
        if !f.is_integer() {
            return Err(AjisaiError::create_structure_error(
                "integer byte 0-255",
                "fraction",
            ));
        }
        let n = f
            .to_i64()
            .ok_or_else(|| AjisaiError::from("Serial byte value is too large"))?;
        if !(0..=255).contains(&n) {
            return Err(AjisaiError::create_structure_error(
                "integer byte 0-255",
                "out-of-range integer",
            ));
        }
        bytes.push(n as u8);
    }
    Ok(bytes)
}

fn emit(interp: &mut Interpreter, command: serde_json::Value) {
    if !interp.output_buffer.is_empty() && !interp.output_buffer.ends_with('\n') {
        interp.output_buffer.push('\n');
    }
    interp.output_buffer.push_str("SERIAL:");
    interp.output_buffer.push_str(&command.to_string());
    interp.output_buffer.push('\n');
}

/// `-- ` : ask the host adapter to enumerate available ports. The result is
/// surfaced by the adapter (program output); a stack-returning form is Phase 2.
pub fn op_list_ports(interp: &mut Interpreter) -> Result<()> {
    emit(interp, json!({ "op": "listPorts" }));
    Ok(())
}

/// `port-id -- handle` : open the named port; leaves the port-id on the stack
/// as the connection handle.
pub fn op_open(interp: &mut Interpreter) -> Result<()> {
    let handle = pop(interp)?;
    let id = require_port_id(&handle)?;
    emit(interp, json!({ "op": "open", "portId": id }));
    interp.stack.push(handle);
    Ok(())
}

/// `handle baud-rate -- handle` : set the baud rate of an open port.
pub fn op_configure(interp: &mut Interpreter) -> Result<()> {
    let options = pop(interp)?;
    let handle = pop(interp)?;
    let id = require_port_id(&handle)?;
    let baud = extract_integer_from_value(&options)?;
    if baud <= 0 {
        return Err(AjisaiError::from("Serial baud rate must be positive"));
    }
    emit(
        interp,
        json!({ "op": "configure", "portId": id, "baudRate": baud }),
    );
    interp.stack.push(handle);
    Ok(())
}

/// `handle bytes -- handle` : write a byte vector to an open port.
pub fn op_write(interp: &mut Interpreter) -> Result<()> {
    let payload = pop(interp)?;
    let handle = pop(interp)?;
    let id = require_port_id(&handle)?;
    let bytes = extract_bytes(&payload)?;
    emit(
        interp,
        json!({ "op": "write", "portId": id, "bytes": bytes }),
    );
    interp.stack.push(handle);
    Ok(())
}

/// `handle -- handle` : flush the port's outgoing buffer.
pub fn op_flush(interp: &mut Interpreter) -> Result<()> {
    let handle = pop(interp)?;
    let id = require_port_id(&handle)?;
    emit(interp, json!({ "op": "flush", "portId": id }));
    interp.stack.push(handle);
    Ok(())
}

/// `handle -- ` : close the port and release the connection.
pub fn op_close(interp: &mut Interpreter) -> Result<()> {
    let handle = pop(interp)?;
    let id = require_port_id(&handle)?;
    emit(interp, json!({ "op": "close", "portId": id }));
    Ok(())
}
