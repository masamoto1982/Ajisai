use crate::interpreter::Interpreter;
use crate::types::{SemanticStack, SemanticStackError};

impl Interpreter {
    /// Capture the current observable stack as Phase 4 `(value, role)` slots.
    ///
    /// This is a compatibility adapter: the runtime still stores stack values
    /// and top-level semantic roles separately, but new migration code can use
    /// this boundary to avoid open-coded parallel-vector handling.
    pub fn semantic_stack_snapshot(&self) -> Result<SemanticStack, SemanticStackError> {
        SemanticStack::from_parts(
            self.get_stack().to_vec(),
            self.collect_stack_hints().to_vec(),
        )
    }

    /// Replace the legacy stack vectors from a Phase 4 semantic-stack value.
    ///
    /// Callers that already hold a `SemanticStack` should use this method
    /// instead of splitting values and roles by hand. The public wire/protocol
    /// representation remains unchanged because `update_stack_with_hints` keeps
    /// the existing interpreter storage model for now.
    pub fn replace_semantic_stack(&mut self, semantic_stack: SemanticStack) {
        let (stack, hints) = semantic_stack.into_parts();
        self.update_stack_with_hints(stack, hints);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Interpretation, Value};

    #[tokio::test]
    async fn snapshot_roundtrips_cf_retagged_stack() {
        let mut source = Interpreter::new();
        source.execute("'anchor' 5/2 >CF").await.unwrap();

        let snapshot = source.semantic_stack_snapshot().unwrap();
        let roles = snapshot.roles().collect::<Vec<_>>();
        assert_eq!(
            roles,
            [Interpretation::Text, Interpretation::ContinuedFraction]
        );

        let mut restored = Interpreter::new();
        restored.replace_semantic_stack(snapshot);
        assert_eq!(restored.get_stack(), source.get_stack());
        assert_eq!(restored.collect_stack_hints(), source.collect_stack_hints());
    }

    #[test]
    fn replace_semantic_stack_preserves_slot_roles() {
        let semantic_stack = SemanticStack::from_parts(
            vec![Value::from_bool(true), Value::from_int(7)],
            vec![Interpretation::TruthValue, Interpretation::RawNumber],
        )
        .unwrap();

        let mut interp = Interpreter::new();
        interp.replace_semantic_stack(semantic_stack);

        assert_eq!(interp.get_stack().len(), 2);
        assert_eq!(
            interp.collect_stack_hints(),
            [Interpretation::TruthValue, Interpretation::RawNumber]
        );
    }
}
