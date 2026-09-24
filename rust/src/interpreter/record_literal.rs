//! Building a Record literal (`{ key value … }`) from source tokens.
//!
//! The literal is the constructor written where the reader wants it: the
//! elements alternate key and value, and what they build is exactly what
//! `[ keys ] [ values ] RECORD` builds from the same values
//! (LANG.RECORDS.STRUCTURE). Because it *is* that construction, it raises the
//! constructor's own ERRORs rather than a second vocabulary of its own — an
//! odd element count is the key sequence being one longer than the value
//! sequence (`vectorLengthMismatch`), and a key written twice is
//! `duplicateKey`. Neither is a source error: a key is a value, so `1` and
//! `1/1` are one key, which is a question about values and not about text.
//!
//! The element scan itself lives in `vector_literal.rs` and is shared with
//! the Vector literal, so a Record nested in a Vector and a Vector nested in
//! a Record are one walk. Nothing here looks a name up.

use crate::error::{AjisaiError, Result};
use crate::types::{RecordBuildError, RecordData, Token, Value};

use super::vector_literal::LiteralKind;
use super::Interpreter;

impl Interpreter {
    /// The Record one `{ ... }` denotes, and how many tokens it spans.
    pub(crate) fn collect_record_literal(
        tokens: &[Token],
        start_index: usize,
        depth: usize,
    ) -> Result<(Value, usize)> {
        let (elements, consumed) =
            Self::collect_literal_elements(tokens, start_index, depth, LiteralKind::Record)?;
        let record = record_from_literal_elements(elements)?;
        Ok((Value::from_record(record), consumed))
    }
}

/// Split the elements into the two sequences a Record is, position by
/// position: evens are keys, odds are the values under them.
pub(crate) fn record_from_literal_elements(elements: Vec<Value>) -> Result<RecordData> {
    let mut keys = Vec::with_capacity(elements.len().div_ceil(2));
    let mut values = Vec::with_capacity(elements.len() / 2);
    for (position, element) in elements.into_iter().enumerate() {
        if position % 2 == 0 {
            keys.push(element);
        } else {
            values.push(element);
        }
    }
    RecordData::new(keys, values).map_err(|e| match e {
        RecordBuildError::LengthMismatch { keys, values } => AjisaiError::VectorLengthMismatch {
            len1: keys,
            len2: values,
        },
        RecordBuildError::DuplicateKey { first, second } => AjisaiError::declared(
            "duplicateKey",
            format!(
                "a Record literal: the key at position {second} repeats the key at position \
                 {first}; a Record holds each key once"
            ),
        ),
    })
}
