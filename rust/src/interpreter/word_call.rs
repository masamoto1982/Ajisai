//! How a User Word call runs its body (LANG.SOURCE.FRAME), and how it
//! settles `KEEP` (LANG.MODIFIERS.CONSUMPTION).
//!
//! `KEEP` modifies the *call*, not the first consuming Word inside the
//! body (LANG.MODIFIERS.CONSUMPTION). Both readings agree for a Core Word, because a Core
//! Word has no inside; they disagree for a User Word, and the body
//! reading is the wrong one — `{ 2 * } 'TWICE' DEF` under `5 KEEP TWICE`
//! let the modifier reach `*`, which then preserved the body's own
//! literal `2` as if the caller had written it. The answer was `5 2 10`
//! with no error and no NIL: a silently wrong result from the one
//! modifier the language has, which is exactly what
//! LANG.FAILURE.TRICHOTOMY rules out.
//!
//! So the modifier is settled here, at the boundary it names, and the body
//! runs in the default consuming mode. Every User Word is written with a
//! parameter header, so what a call puts back is exactly the operands its
//! header declares — a fact of the text, never of how far a run reached.

use crate::error::{AjisaiError, Result};

use super::compiled_plan::execute_compiled_plan;
use super::{ConsumptionMode, Interpreter};

impl Interpreter {
    /// A User Word call (`[ A B | … ]`): take exactly the declared operands,
    /// bind them, and run the body on an empty stack (LANG.SOURCE.FRAME).
    /// Whatever the body leaves is the call's result, and `KEEP` restores
    /// exactly the operands the header declares.
    pub(super) fn run_word_call(
        &mut self,
        params: &[String],
        compiled_plan: Option<&std::sync::Arc<super::compiled_plan::CompiledPlan>>,
        def: &crate::types::WordDefinition,
    ) -> Result<()> {
        let arity = params.len();
        if self.stack.len() < arity {
            return Err(AjisaiError::StackUnderflow);
        }
        let keep_call = self.consumption_mode == ConsumptionMode::Keep;
        self.consumption_mode = ConsumptionMode::Consume;
        let operands = self.stack.split_off(self.stack.len() - arity);
        let caller = std::mem::replace(&mut self.stack, crate::types::Stack::new());

        self.open_binding_scope(true);
        for (name, (value, role)) in params.iter().zip(operands.iter_slots()) {
            self.bind_local(name.clone(), value.clone(), role);
        }
        let result = match compiled_plan {
            Some(compiled) => execute_compiled_plan(self, compiled),
            None => self.execute_guard_structure(&def.lines),
        };
        self.close_binding_scope();

        let frame = std::mem::replace(&mut self.stack, caller);
        // A failed call puts its operands back, as a failed MAP does: the
        // ERROR halts evaluation, and the stack it is reported against is the
        // one the call was given.
        if keep_call || result.is_err() {
            let (values, roles) = operands.into_parts();
            for (value, role) in values.into_iter().zip(roles) {
                self.stack.push_with_role(value, role);
            }
        }
        if result.is_ok() {
            let (values, roles) = frame.into_parts();
            for (value, role) in values.into_iter().zip(roles) {
                self.stack.push_with_role(value, role);
            }
        }
        result
    }
}
