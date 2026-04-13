use crate::interpreter::epoch::EpochSnapshot;

use super::hedged_result::{HedgedCandidateResult, HedgedRejectReason, HedgedWinner};

/// Validate a hedged race winner before committing its result.
///
/// Only `dictionary_epoch` and `module_epoch` are compared — `execution_epoch`
/// changes during normal execution and must not trigger rejection.
pub fn validate_winner(
    winner: &HedgedCandidateResult,
    current_epoch: &EpochSnapshot,
    expected_stack_len: usize,
) -> Result<HedgedWinner, HedgedRejectReason> {
    if winner.epoch_at_spawn.dictionary_epoch != current_epoch.dictionary_epoch
        || winner.epoch_at_spawn.module_epoch != current_epoch.module_epoch
    {
        return Err(HedgedRejectReason::EpochMismatch);
    }
    if winner.stack.len() < expected_stack_len {
        return Err(HedgedRejectReason::StackShapeMismatch);
    }
    Ok(HedgedWinner {
        path: winner.path,
        stack: winner.stack.clone(),
    })
}
