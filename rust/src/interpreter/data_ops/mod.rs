//! `DATA` module — tabular data words (Phase 8C).
//!
//! Implemented as ordinary Module words over the existing value model (Vector,
//! Record, RecordShape); no new Core syntax is added (SPEC handoff §15.3).
//!
//! Unit 1 — CSV ↔ Record-vector conversion:
//! - `DATA@CSV-PARSE`     text → a vector of Records (first CSV row = header).
//! - `DATA@CSV-STRINGIFY` a vector of Records → CSV text.
//!
//! Unit 2 — column and row selection:
//! - `DATA@SELECT` `[ table ] [ columns ] SELECT` → keep only the named columns.
//! - `DATA@WHERE`  `[ table ] 'col' { pred } WHERE` → keep the rows whose
//!   predicate on that column is true.
//!
//! Unit 3 — grouping:
//! - `DATA@GROUP` `[ table ] 'col' GROUP` → a vector of `{ 'key' 'rows' }` group
//!   records, one per distinct column value in first-appearance order.
//!
//! Unit 4 — joining:
//! - `DATA@JOIN` `[ left ] [ right ] 'key' JOIN` → a left/lookup join enriching
//!   each left row with the matching right row; no match fills the added
//!   columns with NIL `MissingField` cells ("join key does not exist").
//!
//! All are **pure transforms**: no file I/O (reading a file is left to the
//! existing IO / Hosted capability), and a malformed input never raises — it
//! projects to a reasoned Bubble/NIL, the same projection `JSON@PARSE` uses for
//! unparseable text (SPEC §11.2). A CSV table is rectangular: a row whose field
//! count differs from the header, or an unterminated quoted field, makes the
//! whole parse project to NIL rather than silently corrupting the data. Cells
//! are text; numeric interpretation is a later unit's concern. Missing values
//! keep a distinct reason: an absent column reads as NIL `MissingField`
//! ("column does not exist", §15.3), never collapsed into a generic absence.

mod query;

pub use query::{op_group, op_join, op_select, op_where};

use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::value_as_string;
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::semantic::{AbsenceOrigin, Recoverability};
use crate::types::record_shape::intern_record_shape;
use crate::types::{Interpretation, Value, ValueData};
use std::collections::HashMap;
use std::sync::Arc;

/// Read an argument `from_top` positions below the stack top. In keep mode the
/// value is peeked (left in place); in consume mode the caller pops the top
/// argument first, so each consumed read is `from_top == 0`.
pub(super) fn extract_stack_value(
    interp: &mut Interpreter,
    keep_mode: bool,
    from_top: usize,
) -> Result<Value> {
    if keep_mode {
        let len = interp.stack.len();
        if len <= from_top {
            return Err(AjisaiError::StackUnderflow);
        }
        Ok(interp.stack[len - 1 - from_top].clone())
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)
    }
}

pub(super) fn encoding_bubble() -> Value {
    Value::bubble_with_reason(
        NilReason::InvalidEncoding,
        AbsenceOrigin::InvalidEncoding,
        Recoverability::Recoverable,
    )
}

/// `DATA@CSV-PARSE`: text → a vector of Records. Malformed CSV (an unterminated
/// quote or a ragged row) projects to a reasoned NIL.
pub fn op_csv_parse(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;
    let val = extract_stack_value(interp, is_keep, 0)?;
    let text = value_as_string(&val).unwrap_or_default();

    let parsed = parse_csv_rows(&text).and_then(rows_to_record_vector);
    interp.stack.push(parsed.unwrap_or_else(encoding_bubble));
    Ok(())
}

/// `DATA@CSV-STRINGIFY`: a vector of Records → CSV text. A non-table input, or
/// records that do not share one column shape, projects to a reasoned NIL.
pub fn op_csv_stringify(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;
    let val = extract_stack_value(interp, is_keep, 0)?;

    match record_vector_to_csv(&val) {
        // An empty table stringifies to empty text, which is NIL in this value
        // model (empty sequence); a non-empty table becomes a Text vector.
        Some(text) => interp.stack.push(Value::from_string(&text)),
        None => interp.stack.push(encoding_bubble()),
    }
    Ok(())
}

// --- pure CSV core (RFC 4180), independent of the interpreter ---------------

/// Parse CSV text into rows of string fields. Returns `None` on an unterminated
/// quoted field. Empty text yields zero rows. `\r\n` and `\n` both end a line.
fn parse_csv_rows(text: &str) -> Option<Vec<Vec<String>>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                '"' => in_quotes = false,
                other => field.push(other),
            }
        } else {
            match c {
                '"' => in_quotes = true,
                ',' => row.push(std::mem::take(&mut field)),
                '\n' => {
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                }
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                }
                other => field.push(other),
            }
        }
    }
    if in_quotes {
        return None;
    }
    // Flush a final line that had no trailing newline.
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    Some(rows)
}

/// Build a vector of Records from parsed rows: the first row is the header, and
/// every data row must have the same field count. A ragged row → `None`.
fn rows_to_record_vector(rows: Vec<Vec<String>>) -> Option<Value> {
    let mut it = rows.into_iter();
    let header = match it.next() {
        Some(header) => header,
        None => return Some(vector_of(Vec::new())),
    };
    let ncol = header.len();

    let mut records = Vec::new();
    for row in it {
        if row.len() != ncol {
            return None;
        }
        let pairs = header
            .iter()
            .zip(row.iter())
            .map(|(key, value)| make_pair(key, value))
            .collect();
        records.push(build_record(pairs));
    }
    Some(vector_of(records))
}

/// Render a vector of Records as CSV text. All records must share the same
/// ordered column keys; the first record fixes the header. A non-vector input,
/// a non-Record element, or a shape mismatch → `None`. An empty vector → `""`.
fn record_vector_to_csv(val: &Value) -> Option<String> {
    let ValueData::Vector(items) = &val.data else {
        return None;
    };
    if items.is_empty() {
        return Some(String::new());
    }

    let header: Vec<String> = record_pairs(&items[0])?
        .into_iter()
        .map(|(key, _)| key)
        .collect();

    let mut out = String::new();
    out.push_str(&encode_row(&header));
    out.push('\n');
    for item in items.iter() {
        let pairs = record_pairs(item)?;
        let keys: Vec<String> = pairs.iter().map(|(key, _)| key.clone()).collect();
        if keys != header {
            return None;
        }
        let values: Vec<String> = pairs.into_iter().map(|(_, value)| value).collect();
        out.push_str(&encode_row(&values));
        out.push('\n');
    }
    Some(out)
}

/// A record's `(key, value)` pairs as text, in pair order. `None` if the value
/// is not a Record of well-formed `[key value]` pairs. An empty or NIL cell
/// reads as the empty string.
fn record_pairs(val: &Value) -> Option<Vec<(String, String)>> {
    let ValueData::Record { pairs, .. } = &val.data else {
        return None;
    };
    let mut out = Vec::with_capacity(pairs.len());
    for pair in pairs.iter() {
        let ValueData::Vector(kv) = &pair.data else {
            return None;
        };
        if kv.len() != 2 {
            return None;
        }
        out.push((
            value_as_string(&kv[0]).unwrap_or_default(),
            value_as_string(&kv[1]).unwrap_or_default(),
        ));
    }
    Some(out)
}

fn make_pair(key: &str, value: &str) -> Value {
    Value {
        data: ValueData::Vector(Arc::new(vec![
            Value::from_string(key),
            Value::from_string(value),
        ])),
        hint: Interpretation::Unassigned,
        absence: None,
    }
}

pub(super) fn build_record(pairs: Vec<Value>) -> Value {
    let mut index: HashMap<String, usize> = HashMap::new();
    for (i, pair) in pairs.iter().enumerate() {
        if let ValueData::Vector(kv) = &pair.data {
            if kv.len() == 2 {
                index.insert(value_as_string(&kv[0]).unwrap_or_default(), i);
            }
        }
    }
    Value {
        data: ValueData::Record {
            pairs: Arc::new(pairs),
            shape: intern_record_shape(index),
        },
        hint: Interpretation::Unassigned,
        absence: None,
    }
}

pub(super) fn vector_of(items: Vec<Value>) -> Value {
    Value {
        data: ValueData::Vector(Arc::new(items)),
        hint: Interpretation::Unassigned,
        absence: None,
    }
}

fn encode_row(fields: &[String]) -> String {
    fields
        .iter()
        .map(|f| encode_field(f))
        .collect::<Vec<_>>()
        .join(",")
}

/// Quote a field per RFC 4180 when it contains a comma, quote, or line break;
/// internal quotes are doubled.
fn encode_field(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

#[cfg(test)]
mod tests;
