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
//! interpretation role assigned — the same outcome the retired fingerprint
//! resync produced, but without any pointer-identity comparison.

use super::{Interpretation, Value};
use std::ops::{Deref, Index, IndexMut, RangeBounds};

/// The interpreter's working stack: values with their top-level semantic roles.
///
/// Invariant: `values.len() == roles.len()`. Every mutating method preserves it,
/// and no public API can grow one vector without the other.
#[derive(Debug, Clone, Default)]
pub struct Stack {
    values: Vec<Value>,
    roles: Vec<Interpretation>,
    /// Shallowest depth the stack has reached since the enclosing depth watch
    /// began — the *operand region* whatever is running has reached into.
    ///
    /// This is what lets `KEEP` name the operands of a whole call rather than
    /// the operands of the first consuming Word inside it: a caller opens a
    /// watch, runs the call, and the mark that comes back is exactly the floor
    /// the call touched. Derived bookkeeping, never part of the stack's value;
    /// `PartialEq` ignores it for that reason.
    low_water: usize,
}

impl Stack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a stack from bare values, deriving each slot's role from the
    /// value's construction-time `hint` (SPEC §12.1).
    pub fn from_values(values: Vec<Value>) -> Self {
        let roles = values.iter().map(|value| value.hint).collect();
        let low_water = values.len();
        Self {
            values,
            roles,
            low_water,
        }
    }

    /// Build a stack from position-aligned values and roles. If the lengths
    /// disagree the roles are normalized to the value count (padding with
    /// `Unassigned`), preserving the length invariant.
    pub fn from_values_and_roles(values: Vec<Value>, mut roles: Vec<Interpretation>) -> Self {
        roles.resize(values.len(), Interpretation::Unassigned);
        let low_water = values.len();
        Self {
            values,
            roles,
            low_water,
        }
    }

    /// Record the depth after a shrink. Growing the stack never lowers the mark,
    /// so only the shrinking methods call this.
    fn note_depth(&mut self) {
        if self.values.len() < self.low_water {
            self.low_water = self.values.len();
        }
    }

    /// Begin watching how deep the stack is reached into, returning the
    /// enclosing watch's mark so the caller can restore it.
    pub fn begin_depth_watch(&mut self) -> usize {
        std::mem::replace(&mut self.low_water, self.values.len())
    }

    /// End the watch opened by [`Stack::begin_depth_watch`], returning the
    /// shallowest depth reached while it was open. The enclosing watch inherits
    /// that floor: a region this one reached into is a region the caller's call
    /// reached into as well.
    pub fn end_depth_watch(&mut self, enclosing: usize) -> usize {
        let reached = self.low_water;
        self.low_water = enclosing.min(reached);
        reached
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
        let value = self.values.pop();
        self.note_depth();
        value
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
        self.note_depth();
        Some((value, role))
    }

    pub fn truncate(&mut self, len: usize) {
        self.values.truncate(len);
        self.roles.truncate(len);
        self.note_depth();
    }

    pub fn clear(&mut self) {
        self.values.clear();
        self.roles.clear();
        self.note_depth();
    }

    pub fn reverse(&mut self) {
        self.values.reverse();
        self.roles.reverse();
        // Reversal keeps the depth but not the identity of any slot, so no
        // region below can still be called untouched.
        self.low_water = 0;
    }

    pub fn insert(&mut self, index: usize, value: Value) {
        self.roles.insert(index, value.hint);
        self.values.insert(index, value);
        if index < self.low_water {
            self.low_water = index;
        }
    }

    pub fn remove(&mut self, index: usize) -> Value {
        self.roles.remove(index);
        let value = self.values.remove(index);
        if index < self.low_water {
            self.low_water = index;
        }
        self.note_depth();
        value
    }

    pub fn split_off(&mut self, at: usize) -> Stack {
        let values = self.values.split_off(at);
        let roles = self.roles.split_off(at);
        self.note_depth();
        let low_water = values.len();
        Stack {
            values,
            roles,
            low_water,
        }
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
        if let std::ops::Bound::Included(&start) = range.start_bound() {
            if start < self.low_water {
                self.low_water = start;
            }
        } else if matches!(range.start_bound(), std::ops::Bound::Unbounded) {
            self.low_water = 0;
        }
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

    /// Retag the slot at `index` (a core-word
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

/// Two stacks are equal when they observe the same `(value, role)` slots. The
/// depth watermark is bookkeeping about how a stack was reached into, not part
/// of what it holds, so it is deliberately excluded — shadow validation
/// compares the compiled and plain routes with this, and the two may reach the
/// same stack by different routes.
impl PartialEq for Stack {
    fn eq(&self, other: &Self) -> bool {
        self.values == other.values && self.roles == other.roles
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
        // A retag of a lower slot, where a
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

    #[test]
    fn insert_remove_truncate_reverse_keep_values_and_roles_aligned() {
        let mut stack = Stack::from_values(vec![Value::from_int(1), Value::from_int(3)]);
        // A cast on slot 0, then an insert in the middle: roles must move with
        // their values, so the cast still names the original value.
        stack.set_role_at(0, Interpretation::ContinuedFraction);
        let inserted = Value::from_int(2);
        let inserted_role = inserted.hint;
        stack.insert(1, inserted);
        assert_eq!(stack.roles().len(), stack.len());
        // The cast slot's role stays with its value; the new slot adopts the
        // inserted value's construction role.
        assert_eq!(stack.role_at(0), Interpretation::ContinuedFraction);
        assert_eq!(stack.role_at(1), inserted_role);

        stack.remove(1);
        assert_eq!(stack.roles().len(), stack.len());
        assert_eq!(stack.role_at(0), Interpretation::ContinuedFraction);

        stack.reverse();
        assert_eq!(stack.roles().len(), stack.len());
        // The cast slot is now on top after reversal.
        assert_eq!(stack.last_role(), Interpretation::ContinuedFraction);

        stack.truncate(1);
        assert_eq!(stack.roles().len(), stack.len());

        stack.clear();
        assert!(stack.roles().is_empty());
    }

    #[test]
    fn pop_slot_then_re_push_round_trips_the_role() {
        let mut stack = Stack::new();
        stack.push_with_role(Value::from_int(7), Interpretation::ContinuedFraction);
        let (value, role) = stack.pop_slot().unwrap();
        assert_eq!(role, Interpretation::ContinuedFraction);
        stack.push_with_role(value, role);
        assert_eq!(stack.last_role(), Interpretation::ContinuedFraction);
        assert_eq!(stack.roles().len(), stack.len());
    }
}
