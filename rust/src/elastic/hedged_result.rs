use crate::types::Value;

use super::hedged_trace::HedgedPath;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HedgedRejectReason {
    EpochMismatch,
    StackShapeMismatch,
    ValidationFailed,
}

#[derive(Debug, Clone)]
pub struct HedgedCandidateResult {
    pub path: HedgedPath,
    pub stack: Vec<Value>,
    pub epoch_at_spawn: u64,
}

#[derive(Debug, Clone)]
pub struct HedgedWinner {
    pub path: HedgedPath,
    pub stack: Vec<Value>,
}
