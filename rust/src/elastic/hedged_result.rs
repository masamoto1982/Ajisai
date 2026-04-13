use crate::interpreter::epoch::EpochSnapshot;
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
    /// Full epoch snapshot captured at race spawn time.
    /// Validation compares only dictionary_epoch and module_epoch.
    pub epoch_at_spawn: EpochSnapshot,
}

#[derive(Debug, Clone)]
pub struct HedgedWinner {
    pub path: HedgedPath,
    pub stack: Vec<Value>,
}
