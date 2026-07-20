//! Query words for the DATA module: column projection, row filtering, and
//! grouping. Split from `mod.rs` to keep each file within the file-size budget;
//! the shared value helpers (`build_record`, `vector_of`, `extract_stack_value`,
//! `encoding_bubble`) live in the parent module.

use super::{build_record, encoding_bubble, extract_stack_value, vector_of};
use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::higher_order::{
    execute_executable_code, extract_executable_code, ExecutableCode,
};
use crate::interpreter::value_extraction_helpers::value_as_string;
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::{Interpretation, Value, ValueData};
use std::collections::HashMap;
use std::sync::Arc;

/// `DATA@SELECT`: `[ table ] [ columns ] SELECT` → a table keeping only the
/// named columns, in the given order. A column absent from a row yields a cell
/// that is NIL with reason `MissingField` (the "column does not exist" reason
/// preserved, §15.3), so the result stays rectangular. A non-table input, a
/// non-vector column list, or a non-Record row projects to a reasoned NIL.
pub fn op_select(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    let cols_val = extract_stack_value(interp, is_keep, 0)?;
    let table_val = extract_stack_value(interp, is_keep, usize::from(is_keep))?;

    interp
        .stack
        .push(select_columns(&table_val, &cols_val).unwrap_or_else(encoding_bubble));
    Ok(())
}

/// `DATA@WHERE`: `[ table ] 'column' { predicate } WHERE` → the rows for which
/// the predicate, run on that column's cell, is definitely true. A false,
/// UNKNOWN, or NIL result drops the row (so a missing column — a NIL cell —
/// drops the row, SQL-like). The result is always a table: no matching row
/// yields an empty table, not NIL. A non-table input projects to a reasoned NIL.
pub fn op_where(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;
    let len = interp.stack.len();
    if len < 3 {
        return Err(AjisaiError::StackUnderflow);
    }
    // Arguments from the top: predicate (0), column (1), table (2). Read by
    // clone so the inputs stay in place until the run succeeds.
    let code_val = interp.stack[len - 1].clone();
    let col_val = interp.stack[len - 2].clone();
    let table_val = interp.stack[len - 3].clone();

    let executable = extract_executable_code(interp, &code_val)?;
    let column = value_as_string(&col_val).unwrap_or_default();

    let result = filter_rows(interp, &table_val, &column, &executable)?;

    if !is_keep {
        interp.stack.truncate(len - 3);
    }
    interp.stack.push(result);
    Ok(())
}

/// `DATA@GROUP`: `[ table ] 'column' GROUP` → a vector of group Records
/// `{ 'key' <value> 'rows' <subtable> }`, one per distinct value of the column
/// in first-appearance order. Rows whose column is absent (or empty) share one
/// group, keyed by the NIL that carries the missing reason. A non-table input,
/// or a non-Record row, projects to a reasoned NIL.
pub fn op_group(interp: &mut Interpreter) -> Result<()> {
    let is_keep = interp.consumption_mode == ConsumptionMode::Keep;
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    let col_val = extract_stack_value(interp, is_keep, 0)?;
    let table_val = extract_stack_value(interp, is_keep, usize::from(is_keep))?;
    let column = value_as_string(&col_val).unwrap_or_default();

    interp
        .stack
        .push(group_by(&table_val, &column).unwrap_or_else(encoding_bubble));
    Ok(())
}

/// Project each Record onto `columns`, filling an absent column with a
/// reasoned NIL cell so every output row shares the requested shape.
fn select_columns(table: &Value, columns: &Value) -> Option<Value> {
    let ValueData::Vector(names) = &columns.data else {
        return None;
    };
    let names: Vec<String> = names
        .iter()
        .map(|c| value_as_string(c).unwrap_or_default())
        .collect();

    let ValueData::Vector(rows) = &table.data else {
        return None;
    };
    let mut out = Vec::with_capacity(rows.len());
    for row in rows.iter() {
        if !matches!(&row.data, ValueData::Record { .. }) {
            return None;
        }
        let pairs = names
            .iter()
            .map(|name| pair_with_value(name, record_get(row, name)))
            .collect();
        out.push(build_record(pairs));
    }
    Some(vector_of(out))
}

/// Group rows by the text of `column`'s cell, preserving first-appearance
/// order. Each group becomes `{ 'key' <cell> 'rows' <subtable> }`; the key cell
/// is the first row's cell for that bucket, so an absent column keeps its NIL
/// `MissingField` reason on the group key.
fn group_by(table: &Value, column: &str) -> Option<Value> {
    use std::collections::hash_map::Entry;
    let ValueData::Vector(rows) = &table.data else {
        return None;
    };
    let mut order: Vec<String> = Vec::new();
    let mut key_cell: HashMap<String, Value> = HashMap::new();
    let mut members: HashMap<String, Vec<Value>> = HashMap::new();
    for row in rows.iter() {
        if !matches!(&row.data, ValueData::Record { .. }) {
            return None;
        }
        let cell = record_get(row, column);
        let bucket = value_as_string(&cell).unwrap_or_default();
        match members.entry(bucket.clone()) {
            Entry::Vacant(slot) => {
                order.push(bucket.clone());
                key_cell.insert(bucket, cell);
                slot.insert(vec![row.clone()]);
            }
            Entry::Occupied(mut slot) => slot.get_mut().push(row.clone()),
        }
    }
    let groups = order
        .into_iter()
        .map(|bucket| {
            let key = key_cell.remove(&bucket).expect("bucket has a key cell");
            let members = members.remove(&bucket).expect("bucket has members");
            build_record(vec![
                pair_with_value("key", key),
                pair_with_value("rows", vector_of(members)),
            ])
        })
        .collect();
    Some(vector_of(groups))
}

/// Keep the rows whose predicate, run on `column`'s cell, is definitely true.
/// Runs each predicate on a scratch stack, restoring the caller's stack and
/// operation mode; a hard predicate error aborts and is propagated.
fn filter_rows(
    interp: &mut Interpreter,
    table: &Value,
    column: &str,
    executable: &ExecutableCode,
) -> Result<Value> {
    let ValueData::Vector(rows) = &table.data else {
        return Ok(encoding_bubble());
    };
    let rows: Vec<Value> = rows.iter().cloned().collect();

    let saved_stack = std::mem::take(&mut interp.stack);
    let saved_target = interp.operation_target_mode;
    let saved_no_change_check = interp.disable_no_change_check;
    interp.operation_target_mode = OperationTargetMode::StackTop;
    interp.disable_no_change_check = true;

    let mut kept = Vec::new();
    let mut error = None;
    for row in &rows {
        interp.stack.clear();
        interp.stack.push(record_get(row, column));
        match execute_executable_code(interp, executable) {
            Ok(()) => {
                if interp.stack.pop().as_ref().is_some_and(is_definitely_true) {
                    kept.push(row.clone());
                }
            }
            Err(e) => {
                error = Some(e);
                break;
            }
        }
    }

    interp.operation_target_mode = saved_target;
    interp.disable_no_change_check = saved_no_change_check;
    interp.stack = saved_stack;

    match error {
        Some(e) => Err(e),
        None => Ok(vector_of(kept)),
    }
}

/// A predicate result keeps its row only when it is a definite truth: a `true`
/// Boolean or a non-zero number. `false`, the logical UNKNOWN, and any NIL all
/// read as "not selected".
fn is_definitely_true(value: &Value) -> bool {
    if let Some(b) = value.as_truth() {
        return b;
    }
    value.as_scalar().is_some_and(|f| !f.is_zero())
}

/// The value stored under `column` in a Record, or a NIL cell carrying reason
/// `MissingField` when the column is absent (§15.3: "column does not exist").
fn record_get(record: &Value, column: &str) -> Value {
    if let ValueData::Record { pairs, .. } = &record.data {
        for pair in pairs.iter() {
            if let ValueData::Vector(kv) = &pair.data {
                if kv.len() == 2 && value_as_string(&kv[0]).unwrap_or_default() == column {
                    return kv[1].clone();
                }
            }
        }
    }
    Value::nil_with_reason(NilReason::MissingField)
}

/// A `[ key value ]` pair from a string key and an already-built value.
fn pair_with_value(key: &str, value: Value) -> Value {
    Value {
        data: ValueData::Vector(Arc::new(vec![Value::from_string(key), value])),
        hint: Interpretation::Unassigned,
        absence: None,
    }
}
