use super::{Interpretation, Value};

/// A single observable stack position: data plus its semantic-plane role.
///
/// Phase 4 keeps this as a small, typed abstraction so migration work can move
/// call sites from parallel `Vec<Value>` / `Vec<Interpretation>` handling toward
/// a single ownership boundary without changing Ajisai surface syntax or wire
/// formats.
#[derive(Debug, Clone, PartialEq)]
pub struct StackSlot {
    value: Value,
    role: Interpretation,
}

impl StackSlot {
    pub fn new(value: Value, role: Interpretation) -> Self {
        Self { value, role }
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn role(&self) -> Interpretation {
        self.role
    }

    pub fn into_parts(self) -> (Value, Interpretation) {
        (self.value, self.role)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticStackError {
    LengthMismatch { values: usize, roles: usize },
}

/// Private-by-construction façade for stack values and top-level roles.
///
/// The current runtime still stores values and roles separately in a few legacy
/// paths. New Phase 4 migration code should prefer this type at boundaries that
/// need to keep stack slots position-aligned. It deliberately exposes values and
/// roles through iterators or `into_parts`, not by mutable access to parallel
/// vectors.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SemanticStack {
    slots: Vec<StackSlot>,
}

impl SemanticStack {
    pub fn new() -> Self {
        Self { slots: Vec::new() }
    }

    pub fn from_slots(slots: Vec<StackSlot>) -> Self {
        Self { slots }
    }

    pub fn from_parts(
        values: Vec<Value>,
        roles: Vec<Interpretation>,
    ) -> Result<Self, SemanticStackError> {
        if values.len() != roles.len() {
            return Err(SemanticStackError::LengthMismatch {
                values: values.len(),
                roles: roles.len(),
            });
        }
        let slots = values
            .into_iter()
            .zip(roles)
            .map(|(value, role)| StackSlot::new(value, role))
            .collect();
        Ok(Self { slots })
    }

    pub fn from_values_with_default_roles(values: Vec<Value>) -> Self {
        let slots = values
            .into_iter()
            .map(|value| StackSlot::new(value, Interpretation::Unassigned))
            .collect();
        Self { slots }
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    pub fn push(&mut self, value: Value, role: Interpretation) {
        self.slots.push(StackSlot::new(value, role));
    }

    pub fn pop(&mut self) -> Option<StackSlot> {
        self.slots.pop()
    }

    pub fn truncate(&mut self, len: usize) {
        self.slots.truncate(len);
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &StackSlot> {
        self.slots.iter()
    }

    pub fn values(&self) -> impl ExactSizeIterator<Item = &Value> {
        self.slots.iter().map(StackSlot::value)
    }

    pub fn roles(&self) -> impl ExactSizeIterator<Item = Interpretation> + '_ {
        self.slots.iter().map(StackSlot::role)
    }

    pub fn into_parts(self) -> (Vec<Value>, Vec<Interpretation>) {
        self.slots.into_iter().map(StackSlot::into_parts).unzip()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_parts_rejects_length_mismatch() {
        let values = vec![Value::from_int(1), Value::from_int(2)];
        let roles = vec![Interpretation::RawNumber];
        assert_eq!(
            SemanticStack::from_parts(values, roles),
            Err(SemanticStackError::LengthMismatch {
                values: 2,
                roles: 1
            })
        );
    }

    #[test]
    fn stack_operations_keep_values_and_roles_together() {
        let mut stack = SemanticStack::new();
        stack.push(Value::from_int(1), Interpretation::RawNumber);
        stack.push(Value::nil(), Interpretation::Nil);
        assert_eq!(stack.len(), 2);
        assert_eq!(
            stack.roles().collect::<Vec<_>>(),
            [Interpretation::RawNumber, Interpretation::Nil]
        );

        let popped = stack.pop().expect("slot should pop as one unit");
        assert!(popped.value().is_nil());
        assert_eq!(popped.role(), Interpretation::Nil);
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn into_parts_is_the_only_parallel_vector_escape_hatch() {
        let values = vec![Value::from_bool(true), Value::from_int(7)];
        let roles = vec![Interpretation::TruthValue, Interpretation::RawNumber];
        let semantic_stack = SemanticStack::from_parts(values.clone(), roles.clone()).unwrap();

        let observed_values = semantic_stack.values().cloned().collect::<Vec<_>>();
        let observed_roles = semantic_stack.roles().collect::<Vec<_>>();
        assert_eq!(observed_values, values);
        assert_eq!(observed_roles, roles);

        let (roundtrip_values, roundtrip_roles) = semantic_stack.into_parts();
        assert_eq!(roundtrip_values, values);
        assert_eq!(roundtrip_roles, roles);
    }
}
