use crate::interpreter::EpochSnapshot;
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::{DisplayHint, Value};

#[derive(Debug, Clone)]
pub struct HedgedSnapshot {
    pub stack: Vec<Value>,
    pub display_hints: Vec<DisplayHint>,
    pub operation_target_mode: OperationTargetMode,
    pub consumption_mode: ConsumptionMode,
    pub safe_mode: bool,
    pub epoch_snapshot: EpochSnapshot,
}

impl HedgedSnapshot {
    pub fn from_interpreter(interpreter: &Interpreter) -> Self {
        Self {
            stack: interpreter.stack.clone(),
            display_hints: interpreter.collect_stack_hints().to_vec(),
            operation_target_mode: interpreter.operation_target_mode,
            consumption_mode: interpreter.consumption_mode,
            safe_mode: interpreter.safe_mode,
            epoch_snapshot: interpreter.current_epoch_snapshot(),
        }
    }
}

impl Interpreter {
    pub(crate) fn restore_hedged_snapshot(
        &mut self,
        snapshot: &crate::elastic::hedged_snapshot::HedgedSnapshot,
    ) {
        self.stack = snapshot.stack.clone();
        self.semantic_registry.stack_hints = snapshot.display_hints.clone();
        self.operation_target_mode = snapshot.operation_target_mode;
        self.consumption_mode = snapshot.consumption_mode;
        self.safe_mode = snapshot.safe_mode;
    }
}
