use super::hedged_result::{HedgedCandidateResult, HedgedRejectReason, HedgedWinner};

pub fn validate_winner(
    winner: &HedgedCandidateResult,
    current_epoch: u64,
    expected_stack_len: usize,
) -> Result<HedgedWinner, HedgedRejectReason> {
    if winner.epoch_at_spawn != current_epoch {
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
