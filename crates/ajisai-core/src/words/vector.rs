//! Vector words.
//!
//! Vectors are the aggregate of Ajisai, and they nest: a matrix is a vector of
//! vectors, so there is no separate rank-aware type and no shape metadata to
//! keep consistent with the data.
//!
//! Role propagation follows one rule, [`role::retain`]: a result keeps the
//! reading its source had whenever the result's shape still admits that
//! reading, and drops to `RAW` when it does not. `REST` of a text is still a
//! text; `REST` of an interval is not an interval, so it is raw.

use crate::error::{Error, Result};
use crate::number::Number;
use crate::role::{self, Role};
use crate::value::Value;

use super::support::{arity_bug, number_of, vector_of};

/// The largest vector a single word will build. A budget, not a semantics:
/// exceeding it is an error rather than a silent truncation.
pub const SIZE_LIMIT: usize = 1_000_000;

pub fn length(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [a] = args else {
        return Err(arity_bug(word, 1, args.len()));
    };
    let items = vector_of(word, a)?;
    Ok(vec![Value::integer(items.len() as i64)])
}

pub fn nth(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [a, i] = args else {
        return Err(arity_bug(word, 2, args.len()));
    };
    let items = vector_of(word, a)?;
    let number = number_of(word, i)?;
    let index = number.as_index().ok_or_else(|| Error::IndexOutOfRange {
        index: number.to_string(),
        length: items.len(),
    })?;
    items
        .get(index)
        .cloned()
        .map(|value| vec![value])
        .ok_or_else(|| Error::IndexOutOfRange {
            index: number.to_string(),
            length: items.len(),
        })
}

pub fn first(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [a] = args else {
        return Err(arity_bug(word, 1, args.len()));
    };
    let items = vector_of(word, a)?;
    // An empty vector has no first element. That is an absence, not a broken
    // rule: `NIL` is exactly the right answer.
    Ok(vec![items.first().cloned().unwrap_or_else(Value::nil)])
}

pub fn rest(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [a] = args else {
        return Err(arity_bug(word, 1, args.len()));
    };
    let items = vector_of(word, a)?;
    let tail = if items.is_empty() {
        Vec::new()
    } else {
        items[1..].to_vec()
    };
    Ok(vec![carry(a.role(), Value::vector(tail))])
}

pub fn reverse(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [a] = args else {
        return Err(arity_bug(word, 1, args.len()));
    };
    let mut items = vector_of(word, a)?.to_vec();
    items.reverse();
    Ok(vec![carry(a.role(), Value::vector(items))])
}

pub fn append(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [a, item] = args else {
        return Err(arity_bug(word, 2, args.len()));
    };
    let mut items = vector_of(word, a)?.to_vec();
    if items.len() >= SIZE_LIMIT {
        return Err(Error::SizeLimitExceeded { limit: SIZE_LIMIT });
    }
    items.push(item.clone());
    Ok(vec![carry(a.role(), Value::vector(items))])
}

pub fn concat(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [a, b] = args else {
        return Err(arity_bug(word, 2, args.len()));
    };
    let left = vector_of(word, a)?;
    let right = vector_of(word, b)?;
    if left.len() + right.len() > SIZE_LIMIT {
        return Err(Error::SizeLimitExceeded { limit: SIZE_LIMIT });
    }
    let mut items = left.to_vec();
    items.extend(right.iter().cloned());
    // Two containers only agree on a reading if they had the same one.
    Ok(vec![carry(a.role().join(b.role()), Value::vector(items))])
}

pub fn range(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [from, to] = args else {
        return Err(arity_bug(word, 2, args.len()));
    };
    let start = number_of(word, from)?;
    let end = number_of(word, to)?;
    if !start.is_integer() || !end.is_integer() {
        return Err(Error::TypeMismatch {
            word: word.to_string(),
            expected: "integer bounds".to_string(),
            found: format!("{start} and {end}"),
        });
    }
    if end <= start {
        return Ok(vec![Value::vector(Vec::new())]);
    }
    let span = end - start;
    let count = span
        .as_index()
        .ok_or(Error::SizeLimitExceeded { limit: SIZE_LIMIT })?;
    if count > SIZE_LIMIT {
        return Err(Error::SizeLimitExceeded { limit: SIZE_LIMIT });
    }
    let mut items = Vec::with_capacity(count);
    let mut current = start.clone();
    let one = Number::one();
    for _ in 0..count {
        items.push(Value::number(current.clone()));
        current = &current + &one;
    }
    Ok(vec![Value::vector(items)])
}

/// Carry a role onto a freshly built value, dropping it if the shape no longer
/// admits the reading.
fn carry(role: Role, value: Value) -> Value {
    let kept = role::retain(role, &value);
    value.with_role(kept)
}
