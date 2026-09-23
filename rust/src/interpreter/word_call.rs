//! The two ways a User Word call runs its body (LANG.SOURCE.FRAME), and how
//! each settles `KEEP` (LANG.MODIFIERS.CONSUMPTION).
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
//! runs in the default consuming mode. What the call gives back differs by
//! kind: a Word written with a parameter header puts back exactly the operands
//! its header declares, a fact of the text; one written without puts back
//! whatever a depth watch saw the call reach, a fact of the run.

use crate::error::{AjisaiError, Result};
use crate::types::{Interpretation, Value};

use super::compiled_plan::execute_compiled_plan;
use super::{ConsumptionMode, Interpreter};

impl Interpreter {
    /// A call to a Word written without a parameter header: the body sees the
    /// whole stack, and `KEEP` restores whatever the call turned out to reach.
    pub(super) fn run_whole_stack_call(
        &mut self,
        compiled_plan: Option<&std::sync::Arc<super::compiled_plan::CompiledPlan>>,
        def: &crate::types::WordDefinition,
    ) -> Result<()> {
        let keep_call = self.consumption_mode == ConsumptionMode::Keep;
        let kept_operands: Option<Vec<(Value, Interpretation)>> = keep_call.then(|| {
            self.stack
                .iter_slots()
                .map(|(value, role)| (value.clone(), role))
                .collect()
        });
        self.consumption_mode = ConsumptionMode::Consume;
        let enclosing_watch = self.stack.begin_depth_watch();

        // A Word call is a barrier frame: its body names its own locals and
        // reads none of the caller's, so what a Word means depends on its
        // operands and its dictionary and nothing else.
        self.open_binding_scope(true);

        // Compiling a body is unobservable (LANG.AUTHORITY.FREEDOM): a run
        // produces the same result whether it went through the compiled plan
        // or the plain guard structure.
        let result = match compiled_plan {
            Some(compiled) => execute_compiled_plan(self, compiled),
            None => self.execute_guard_structure(&def.lines),
        };

        self.close_binding_scope();

        let operand_floor = self.stack.end_depth_watch(enclosing_watch);
        if let (Some(operands), true) = (kept_operands, result.is_ok()) {
            self.restore_kept_operands(operands, operand_floor);
        }
        result
    }

    /// A call to a Word with a parameter header (`[ A B | … ]`): take exactly
    /// the declared operands, bind them, and run the body on an empty stack
    /// (LANG.SOURCE.FRAME). Whatever the body leaves is the call's result. The
    /// arity is written, so `KEEP` restores exactly those operands — a fact
    /// of the text, not of how far the body happened to reach.
    pub(super) fn run_header_call(
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

    /// Put a `KEEP`-ed call's operands back underneath its results.
    ///
    /// After the call the stack is `survivors ++ results`, where `survivors` is
    /// the part below `operand_floor` — the shallowest depth the call reached.
    /// `operands` is the whole stack as it stood before the call, so everything
    /// from `operand_floor` up is what the call ate. Splicing that region back
    /// in leaves `operands ++ results`: operands preserved, result appended.
    pub(crate) fn restore_kept_operands(
        &mut self,
        operands: Vec<(Value, Interpretation)>,
        operand_floor: usize,
    ) {
        if operand_floor >= operands.len() {
            return;
        }
        let results = self.stack.split_off(operand_floor.min(self.stack.len()));
        for (value, role) in operands.into_iter().skip(operand_floor) {
            self.stack.push_with_role(value, role);
        }
        let (values, roles) = results.into_parts();
        for (value, role) in values.into_iter().zip(roles) {
            self.stack.push_with_role(value, role);
        }
    }
}
