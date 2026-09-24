//! The execution stack.
//!
//! A slot holds a value and nothing else: every observation of a slot is
//! derived from the value it holds (LANG.VALUES.DENOTATION), so there is no
//! per-slot state beside it for a Word to set or a surface to read.
//!
//! Reads flow through `Deref` to `Vec<Value>`. Mutation goes through the
//! inherent methods below; there is deliberately no `DerefMut`.

use super::Value;
use std::ops::{Deref, Index, IndexMut, RangeBounds};

/// The interpreter's working stack.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stack {
    values: Vec<Value>,
}

impl Stack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_values(values: Vec<Value>) -> Self {
        Self { values }
    }

    pub fn push(&mut self, value: Value) {
        self.values.push(value);
    }

    pub fn pop(&mut self) -> Option<Value> {
        self.values.pop()
    }

    pub fn truncate(&mut self, len: usize) {
        self.values.truncate(len);
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn reverse(&mut self) {
        self.values.reverse();
    }

    pub fn insert(&mut self, index: usize, value: Value) {
        self.values.insert(index, value);
    }

    pub fn remove(&mut self, index: usize) -> Value {
        self.values.remove(index)
    }

    pub fn split_off(&mut self, at: usize) -> Stack {
        Stack {
            values: self.values.split_off(at),
        }
    }

    pub fn extend<I: IntoIterator<Item = Value>>(&mut self, iter: I) {
        self.values.extend(iter);
    }

    /// Drain a range of values. Mirrors `Vec::drain`.
    pub fn drain<R>(&mut self, range: R) -> std::vec::Drain<'_, Value>
    where
        R: RangeBounds<usize>,
    {
        self.values.drain(range)
    }

    /// The values as a slice. Equivalent to dereferencing to `&[Value]`, but
    /// spelled out for range-indexing call sites (`stack.as_slice()[a..b]`),
    /// which the inherent `Index<usize>` would otherwise shadow.
    pub fn as_slice(&self) -> &[Value] {
        &self.values
    }

    /// Consume the stack into its values.
    pub fn into_values(self) -> Vec<Value> {
        self.values
    }
}

impl Deref for Stack {
    type Target = Vec<Value>;
    fn deref(&self) -> &Vec<Value> {
        &self.values
    }
}

impl Index<usize> for Stack {
    type Output = Value;
    fn index(&self, index: usize) -> &Value {
        &self.values[index]
    }
}

// In-place value mutation.
impl IndexMut<usize> for Stack {
    fn index_mut(&mut self, index: usize) -> &mut Value {
        &mut self.values[index]
    }
}

impl IntoIterator for Stack {
    type Item = Value;
    type IntoIter = std::vec::IntoIter<Value>;
    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<'a> IntoIterator for &'a Stack {
    type Item = &'a Value;
    type IntoIter = std::slice::Iter<'a, Value>;
    fn into_iter(self) -> Self::IntoIter {
        self.values.iter()
    }
}

impl From<Vec<Value>> for Stack {
    fn from(values: Vec<Value>) -> Self {
        Stack::from_values(values)
    }
}

impl FromIterator<Value> for Stack {
    fn from_iter<I: IntoIterator<Item = Value>>(iter: I) -> Self {
        Stack::from_values(iter.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bulk_mutation_keeps_values_in_order() {
        let mut stack = Stack::from_values(vec![Value::from_int(1), Value::from_int(2)]);
        stack.extend(vec![Value::from_int(3), Value::from_int(4)]);
        let tail = stack.split_off(1);
        assert_eq!(stack.len(), 1);
        assert_eq!(tail.len(), 3);
        let drained: Vec<Value> = stack.drain(..).collect();
        assert_eq!(drained, vec![Value::from_int(1)]);
        assert!(stack.is_empty());
    }
}
