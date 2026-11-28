//! The sheet view of the User dictionary (spec/gui-semantics.md, Sheet): a
//! User Word shown as a cell — its name, its body as the formula, and, for a
//! Word that takes nothing and leaves one value, that value.
//!
//! A cell is not a new kind of thing. Its value is what running the Word on
//! an empty stack leaves, worked out on a separate interpreter that shares
//! the dictionary and the ceilings and nothing else, so reading a sheet runs
//! no program of the session: its stack, output, meters and dictionary are
//! as they were.

use super::word_types::{first_misfit, DeclaredType};
use super::Interpreter;
use crate::types::{Value, ValueData};

/// What the sheet shows for one User Word without running it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetCell {
    /// The body as written, without its brackets.
    pub formula: String,
    /// How many values it takes and leaves, where inference fixes both.
    pub effect: Option<(u16, u16)>,
    /// Its declared signature (LANG.CONTRACT.TYPES), as `#:contract` writes it.
    pub signature: Option<String>,
    /// Its content identity (LANG.DICTIONARY.MUTATION), which changes when
    /// it or anything it calls does: the key a host can keep a value under.
    pub identity: Option<String>,
}

/// What a cell's Word leaves on an empty stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellValue {
    Value {
        display: String,
        /// The narrowest declared type the value fits, offered when the
        /// Word declares none.
        suggested_type: Option<String>,
        /// The value laid out as a table, when it is one.
        table: Option<SheetTable>,
    },
    Error(String),
}

/// A table as the sheet lays it out: a Record whose values are Vectors of
/// one length, its keys the columns (LANG.RECORDS.STRUCTURE). Only the first
/// [`TABLE_ROWS`] rows are written out; `rows` counts them all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetTable {
    pub columns: Vec<String>,
    pub cells: Vec<Vec<String>>,
    pub rows: usize,
}

pub const TABLE_ROWS: usize = 100;

/// `value` as a table, when it is a Record of columns of one length.
pub fn table_of(value: &Value) -> Option<SheetTable> {
    let record = value.as_record()?;
    let mut rows = None;
    for column in record.values() {
        let is_column = matches!(
            column.data,
            ValueData::Vector(_) | ValueData::Tensor { .. } | ValueData::Truths(_)
        );
        if !is_column || *rows.get_or_insert(column.len()) != column.len() {
            return None;
        }
    }
    let rows = rows?;
    let columns = record
        .keys()
        .iter()
        .map(|key| {
            key.as_text()
                .map_or_else(|| key.to_string(), str::to_string)
        })
        .collect();
    let cells = (0..rows.min(TABLE_ROWS))
        .map(|row| {
            record
                .values()
                .iter()
                .map(|column| {
                    column
                        .child(row)
                        .map_or_else(String::new, |v| v.to_string())
                })
                .collect()
        })
        .collect();
    Some(SheetTable {
        columns,
        cells,
        rows,
    })
}

/// The narrowest declared type `value` fits: a NIL alone fits every type and
/// so suggests none. A Record with String keys suggests a Record type, field
/// by field.
pub fn suggest_type(value: &Value) -> Option<DeclaredType> {
    if value.is_nil() {
        return None;
    }
    if let Some(record) = value.as_record() {
        return record
            .keys()
            .iter()
            .zip(record.values())
            .map(|(key, field)| Some((key.as_text()?.to_string(), suggest_type(field)?)))
            .collect::<Option<Vec<_>>>()
            .map(DeclaredType::Record);
    }
    let scalars = [
        DeclaredType::Int,
        DeclaredType::Rat,
        DeclaredType::Bool,
        DeclaredType::Str,
    ];
    let mut candidates: Vec<DeclaredType> = scalars.to_vec();
    for _ in 0..3 {
        let deeper: Vec<DeclaredType> = candidates[candidates.len() - scalars.len()..]
            .iter()
            .map(|ty| DeclaredType::Vector(Box::new(ty.clone())))
            .collect();
        candidates.extend(deeper);
    }
    candidates
        .into_iter()
        .find(|ty| first_misfit(value, ty).is_none())
}

impl Interpreter {
    /// Every typed User Word's signature (LANG.CONTRACT.TYPES), by name.
    #[cfg_attr(not(feature = "wasm"), allow(dead_code))]
    pub(crate) fn declared_signatures(
        &self,
    ) -> std::collections::HashMap<String, super::word_types::Signature> {
        self.user_words
            .iter()
            .filter_map(|(name, def)| Some((name.clone(), (**def.signature.as_ref()?).clone())))
            .collect()
    }

    /// The sheet's view of the User Word `name`, or `None` when no User Word
    /// holds it.
    pub fn sheet_cell(&self, name: &str) -> Option<SheetCell> {
        let upper = name.to_uppercase();
        let def = self.user_words.get(&upper)?;
        Some(SheetCell {
            formula: self.lookup_word_definition_tokens(&upper)?,
            effect: self.stack_effect_of_word(&upper),
            signature: def.signature.as_ref().map(|sig| sig.to_string()),
            identity: self.word_identity(&upper).cloned(),
        })
    }

    /// Run the User Word `name` on an empty stack apart from the session,
    /// and answer what it leaves: its one value, or every value it leaves
    /// written in a row, or the error it stopped with.
    pub async fn evaluate_sheet_cell(&self, name: &str) -> CellValue {
        let upper = name.to_uppercase();
        let Some(def) = self.user_words.get(&upper) else {
            return CellValue::Error(format!("Word '{upper}' is not defined"));
        };
        let mut apart = Interpreter::new();
        apart.user_words = self.user_words.clone();
        apart.dependents = self.dependents.clone();
        apart.set_runtime_limits(*self.runtime_limits());
        apart.set_max_execution_steps(self.max_execution_steps());
        apart.bump_dictionary_epoch();
        if let Err(error) = apart.execute(&upper).await {
            return CellValue::Error(error.to_string());
        }
        let stack = apart.get_stack().as_slice();
        let suggested_type = match (stack, &def.signature) {
            ([value], None) => suggest_type(value).map(|ty| ty.to_string()),
            _ => None,
        };
        CellValue::Value {
            table: match stack {
                [value] => table_of(value),
                _ => None,
            },
            display: stack
                .iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join(" "),
            suggested_type,
        }
    }
}
