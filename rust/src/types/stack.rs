//! The execution stack as the single authority for top-level semantic roles.
//!
//! SPEC §12 observes each stack position as a `(data, role)` pair. Phase 4
//! collapses the historical two-place ownership (`Vec<Value>` for data plus a
//! parallel `SemanticRegistry.stack_hints` for roles) into one type that owns
//! both, so the role of a slot can never drift out of alignment with its value.
//!
//! Reads flow through `Deref` to `Vec<Value>`, so the vast majority of existing
//! `Vec<Value>`-shaped call sites (len/iter/last/get/index-read/…) keep working
//! unchanged. Mutation goes through inherent methods that maintain both vectors
//! together, and there is deliberately no `DerefMut`: growing the value vector
//! without also placing a role is unrepresentable.
//!
//! The default role of a pushed slot is the value's construction-time role
//! (`Value.hint`, SPEC §12.1). A module word that pops operands and pushes
//! freshly built results therefore adopts the results' construction roles
//! automatically, while slots it never touches keep the plane role a prior
//! position cast (`>CF`) assigned — the same outcome the retired fingerprint
//! resync produced, but without any pointer-identity comparison.

use super::{Interpretation, Value};
use std::ops::{Deref, Index, IndexMut, RangeBounds};

/// The interpreter's working stack: values with their top-level semantic roles.
///
/// Invariant: `values.len() == roles.len()`. Every mutating method preserves it,
/// and no public API can grow one vector without the other.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stack {
    values: Vec<Value>,
    roles: Vec<Interpretation>,
}

impl Stack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a stack from bare values, deriving each slot's role from the
    /// value's construction-time `hint` (SPEC §12.1).
    pub fn from_values(values: Vec<Value>) -> Self {
        let roles = values.iter().map(|value| value.hint).collect();
        Self { values, roles }
    }

    /// Build a stack from position-aligned values and roles. If the lengths
    /// disagree the roles are normalized to the value count (padding with
    /// `Unassigned`), preserving the length invariant.
    pub fn from_values_and_roles(values: Vec<Value>, mut roles: Vec<Interpretation>) -> Self {
        roles.resize(values.len(), Interpretation::Unassigned);
        Self { values, roles }
    }

    /// Push a value, adopting its construction-time role as the slot role.
    pub fn push(&mut self, value: Value) {
        self.roles.push(value.hint);
        self.values.push(value);
    }

    /// Push a value under an explicit plane role, overriding the value's
    /// construction-time role for this slot only.
    pub fn push_with_role(&mut self, value: Value, role: Interpretation) {
        self.roles.push(role);
        self.values.push(value);
    }

    /// Pop the top value, discarding its role.
    pub fn pop(&mut self) -> Option<Value> {
        self.roles.pop();
        self.values.pop()
    }

    /// Iterate the stack bottom-to-top as observable `(value, role)` slots —
    /// the `(data, role)` pairs of SPEC §12. This is the alignment-guaranteed
    /// source for every stack-rendering surface.
    pub fn iter_slots(&self) -> impl ExactSizeIterator<Item = (&Value, Interpretation)> + '_ {
        self.values.iter().zip(self.roles.iter().copied())
    }

    /// Pop the top slot as a `(value, role)` pair for callers that need to carry
    /// the role forward (e.g. re-push it after inspection).
    pub fn pop_slot(&mut self) -> Option<(Value, Interpretation)> {
        let value = self.values.pop()?;
        let role = self.roles.pop().unwrap_or(Interpretation::Unassigned);
        Some((value, role))
    }

    pub fn truncate(&mut self, len: usize) {
        self.values.truncate(len);
        self.roles.truncate(len);
    }

    pub fn clear(&mut self) {
        self.values.clear();
        self.roles.clear();
    }

    pub fn reverse(&mut self) {
        self.values.reverse();
        self.roles.reverse();
    }

    pub fn insert(&mut self, index: usize, value: Value) {
        self.roles.insert(index, value.hint);
        self.values.insert(index, value);
    }

    pub fn remove(&mut self, index: usize) -> Value {
        self.roles.remove(index);
        self.values.remove(index)
    }

    pub fn split_off(&mut self, at: usize) -> Stack {
        let values = self.values.split_off(at);
        let roles = self.roles.split_off(at);
        Stack { values, roles }
    }

    pub fn extend<I: IntoIterator<Item = Value>>(&mut self, iter: I) {
        for value in iter {
            self.push(value);
        }
    }

    /// Drain a range of values, dropping the aligned roles. Mirrors
    /// `Vec::drain` so existing `stack.drain(..).collect()` sites are unchanged.
    pub fn drain<R>(&mut self, range: R) -> std::vec::Drain<'_, Value>
    where
        R: RangeBounds<usize> + Clone,
    {
        self.roles.drain(range.clone());
        self.values.drain(range)
    }

    // --- Role plane (the former `SemanticRegistry.stack_hints` API) ---

    /// The values as a slice. Equivalent to dereferencing to `&[Value]`, but
    /// spelled out for range-indexing call sites (`stack.as_slice()[a..b]`),
    /// which the inherent `Index<usize>` would otherwise shadow.
    pub fn as_slice(&self) -> &[Value] {
        &self.values
    }

    /// The top-level role of every slot, in stack order.
    pub fn roles(&self) -> &[Interpretation] {
        &self.roles
    }

    /// The role of the slot at `index`, or `Unassigned` if out of range.
    pub fn role_at(&self, index: usize) -> Interpretation {
        self.roles
            .get(index)
            .copied()
            .unwrap_or(Interpretation::Unassigned)
    }

    /// The role of the top slot, or `Unassigned` if the stack is empty.
    pub fn last_role(&self) -> Interpretation {
        self.roles
            .last()
            .copied()
            .unwrap_or(Interpretation::Unassigned)
    }

    /// Retag the slot at `index` (a position cast such as `>CF`, or a core-word
    /// role override). Out-of-range indices are ignored, matching the legacy
    /// `update_hint_at`.
    pub fn set_role_at(&mut self, index: usize, role: Interpretation) {
        if index < self.roles.len() {
            self.roles[index] = role;
        }
    }

    /// Retag the top slot. No-op on an empty stack.
    pub fn set_last_role(&mut self, role: Interpretation) {
        if let Some(last) = self.roles.last_mut() {
            *last = role;
        }
    }

    /// Replace the whole role plane, normalizing to the value count so the
    /// length invariant holds. Used by save/restore boundaries.
    pub fn set_roles(&mut self, mut roles: Vec<Interpretation>) {
        roles.resize(self.values.len(), Interpretation::Unassigned);
        self.roles = roles;
    }

    /// Consume the stack into its aligned value and role vectors.
    pub fn into_parts(self) -> (Vec<Value>, Vec<Interpretation>) {
        (self.values, self.roles)
    }

    /// Consume the stack into just its values, dropping roles.
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

// In-place value mutation only; the slot's role is intentionally untouched.
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
    fn push_adopts_construction_role_and_pop_drops_it() {
        let mut stack = Stack::new();
        stack.push(Value::from_bool(true));
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.roles().len(), stack.len());
        // Overriding the top role leaves the value untouched.
        stack.set_last_role(Interpretation::TruthValue);
        assert_eq!(stack.last_role(), Interpretation::TruthValue);
        assert!(stack.pop().is_some());
        assert!(stack.roles().is_empty());
    }

    #[test]
    fn position_cast_survives_a_push_and_pop_above_it() {
        // Mirrors `x >CF <module-word>`: the cast retags a lower slot, and a
        // later slot built and removed above it must not disturb that role.
        let mut stack = Stack::new();
        stack.push(Value::from_int(5));
        stack.set_role_at(0, Interpretation::ContinuedFraction);
        stack.push(Value::from_int(9));
        assert_eq!(stack.pop().unwrap(), Value::from_int(9));
        assert_eq!(stack.role_at(0), Interpretation::ContinuedFraction);
    }

    #[test]
    fn length_invariant_holds_across_bulk_mutation() {
        let mut stack = Stack::from_values(vec![Value::from_int(1), Value::from_int(2)]);
        stack.extend(vec![Value::from_int(3), Value::from_int(4)]);
        assert_eq!(stack.roles().len(), stack.len());
        let tail = stack.split_off(1);
        assert_eq!(stack.roles().len(), stack.len());
        assert_eq!(tail.roles().len(), tail.len());
        let drained: Vec<Value> = stack.drain(..).collect();
        assert_eq!(drained.len(), 1);
        assert!(stack.is_empty());
        assert!(stack.roles().is_empty());
    }
}
