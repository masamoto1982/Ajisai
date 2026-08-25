use std::sync::Arc;

use super::compiled_plan::CompiledPlan;
use super::EpochSnapshot;

#[derive(Debug, Clone)]
pub struct ExecutionPlanSet {
    pub plain_available: bool,
    pub compiled: Option<Arc<CompiledPlan>>,
    pub built_at: EpochSnapshot,
    pub validated_until_epoch: u64,
    pub validation_failures: u64,
}

impl ExecutionPlanSet {
    pub fn new(snapshot: EpochSnapshot) -> Self {
        Self {
            plain_available: true,
            compiled: None,
            built_at: snapshot,
            validated_until_epoch: 0,
            validation_failures: 0,
        }
    }
}
