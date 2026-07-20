//! `DATA` module — tabular data words (Phase 8C, unit 1).
//!
//! Implemented as ordinary Module words over the existing value model (Vector,
//! Record, RecordShape); no new Core syntax is added (SPEC handoff §15.3). This
//! first unit provides the CSV ↔ Record-vector conversion:
//!
//! - `DATA@CSV-PARSE`     text → a vector of Records (first CSV row = header).
//! - `DATA@CSV-STRINGIFY` a vector of Records → CSV text.
//!
//! Both are **pure transforms**: no file I/O (reading a file is left to the
//! existing IO / Hosted capability), and a malformed input never raises — it
//! projects to a reasoned Bubble/NIL, the same projection `JSON@PARSE` uses for
//! unparseable text (SPEC §11.2). A CSV table is rectangular: a row whose field
//! count differs from the header, or an unterminated quoted field, makes the
//! whole parse project to NIL rather than silently corrupting the data. Cells
//! are text; numeric interpretation is a later unit's concern.

use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::value_as_string;
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::semantic::{AbsenceOrigin, Recoverability};
use crate::types::record_shape::intern_record_shape;
use crate::types::{Interpretation, Value, ValueData};
use std::collections::HashMap;
use std::sync::Arc;

fn extract_stack_value(interp: &mut Interpreter, keep_mode: bool) -> Result<Value> {
    if keep_mode {
        interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)
    }
}

fn encoding_bubble() -> Value {
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
    let val = extract_stack_value(interp, is_keep)?;
    let text = value_as_string(&val).unwrap_or_default();

    let parsed = parse_csv_rows(&text).and_then(rows_to_record_vector);
    interp.stack.push(parsed.unwrap_or_else(encoding_bubble));
    Ok(())
}

/// `DATA@CSV-STRINGIFY`: a vector of Records → CSV text. A non-table input, or
/// records that do not share one column shape, projects to a reasoned NIL.
pub fn op_csv_stringify(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;
    let val = extract_stack_value(interp, is_keep)?;

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

fn build_record(pairs: Vec<Value>) -> Value {
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

fn vector_of(items: Vec<Value>) -> Value {
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
mod tests {
    use super::*;

    #[test]
    fn parses_a_simple_table() {
        let rows = parse_csv_rows("a,b\n1,2\n3,4").unwrap();
        assert_eq!(rows, vec![vec!["a", "b"], vec!["1", "2"], vec!["3", "4"]]);
    }

    #[test]
    fn trailing_newline_does_not_add_a_row() {
        let rows = parse_csv_rows("a,b\n1,2\n").unwrap();
        assert_eq!(rows, vec![vec!["a", "b"], vec!["1", "2"]]);
    }

    #[test]
    fn crlf_ends_a_line() {
        let rows = parse_csv_rows("a,b\r\n1,2\r\n").unwrap();
        assert_eq!(rows, vec![vec!["a", "b"], vec!["1", "2"]]);
    }

    #[test]
    fn quoted_fields_carry_commas_quotes_and_newlines() {
        let rows = parse_csv_rows("name,note\n\"Doe, John\",\"a \"\"quote\"\"\"\n").unwrap();
        assert_eq!(
            rows,
            vec![
                vec!["name".to_string(), "note".to_string()],
                vec!["Doe, John".to_string(), "a \"quote\"".to_string()],
            ]
        );
    }

    #[test]
    fn empty_text_is_zero_rows() {
        assert_eq!(parse_csv_rows("").unwrap(), Vec::<Vec<String>>::new());
    }

    #[test]
    fn unterminated_quote_is_rejected() {
        assert!(parse_csv_rows("a,b\n\"oops,1\n").is_none());
    }

    #[test]
    fn ragged_row_makes_the_table_none() {
        let rows = parse_csv_rows("a,b\n1,2,3\n").unwrap();
        assert!(rows_to_record_vector(rows).is_none());
    }

    #[test]
    fn header_only_is_an_empty_table() {
        let rows = parse_csv_rows("a,b\n").unwrap();
        let v = rows_to_record_vector(rows).unwrap();
        assert!(matches!(&v.data, ValueData::Vector(items) if items.is_empty()));
    }

    #[test]
    fn round_trips_through_records() {
        let csv = "a,b\n1,2\n3,4\n";
        let value = rows_to_record_vector(parse_csv_rows(csv).unwrap()).unwrap();
        assert_eq!(record_vector_to_csv(&value).as_deref(), Some(csv));
    }

    #[test]
    fn round_trips_fields_needing_quotes() {
        let csv = "name,note\n\"Doe, John\",\"a \"\"quote\"\"\"\n";
        let value = rows_to_record_vector(parse_csv_rows(csv).unwrap()).unwrap();
        assert_eq!(record_vector_to_csv(&value).as_deref(), Some(csv));
    }

    #[test]
    fn stringify_rejects_a_non_table() {
        assert!(record_vector_to_csv(&Value::from_int(5)).is_none());
    }

    #[test]
    fn empty_vector_stringifies_to_empty_text() {
        assert_eq!(
            record_vector_to_csv(&vector_of(Vec::new())).as_deref(),
            Some("")
        );
    }

    #[test]
    fn encode_field_quotes_only_when_needed() {
        assert_eq!(encode_field("plain"), "plain");
        assert_eq!(encode_field("a,b"), "\"a,b\"");
        assert_eq!(encode_field("a\"b"), "\"a\"\"b\"");
    }

    // Execution-level tests: the words are reachable through IMPORT and drive
    // the production interpreter path.

    #[tokio::test]
    async fn csv_round_trips_through_the_interpreter() {
        let mut interp = Interpreter::new();
        interp
            .execute("'a,b\n1,2\n3,4' 'DATA' IMPORT CSV-PARSE CSV-STRINGIFY PRINT")
            .await
            .unwrap();
        assert_eq!(interp.collect_output().trim_end(), "a,b\n1,2\n3,4");
    }

    #[tokio::test]
    async fn csv_parse_of_ragged_input_bubbles_to_nil() {
        let mut interp = Interpreter::new();
        interp
            .execute("'a,b\n1,2,3' 'DATA' IMPORT CSV-PARSE")
            .await
            .unwrap();
        let top = interp.get_stack().last().expect("a value on the stack");
        assert!(top.is_absent(), "expected a NIL, got {:?}", top);
    }
}
