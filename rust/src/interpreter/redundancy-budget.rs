/// Cost accounting for redundant execution attempts.
///
/// This limits how many times a given code path will be retried with the
/// faster (but potentially unsafe) representations before permanently
/// downgrading to the safe plain path.
#[derive(Debug, Clone, Copy)]
pub struct RedundancyBudget {
    /// Maximum quantized-path attempts before the block is considered
    /// incompatible with quantized execution for the current epoch.
    pub max_quantized_attempts: u32,

    /// Number of epochs that must pass after a failure before the quantized
    /// path is tried again for the same block.
    pub cooldown_epochs: u64,

    /// If the cumulative quantized failure count for a word reaches this
    /// threshold the word is permanently demoted to `TwoStage` (Compiled →
    /// Plain only).
    pub auto_degrade_threshold: u32,
}

impl Default for RedundancyBudget {
    fn default() -> Self {
        Self {
            max_quantized_attempts: 3,
            cooldown_epochs: 2,
            auto_degrade_threshold: 5,
        }
    }
}

/// How aggressively to attempt faster execution representations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DegradationPolicy {
    /// Quantized → Compiled → Plain (all three stages).
    /// Used when arity is fully resolved and purity is `Pure`.
    ThreeStage,

    /// Compiled → Plain only (Quantized is skipped).
    /// Used when arity is `Variable` or purity is `Unknown`.
    TwoStage,

    /// Plain only (maximum safety).
    /// Used when purity is `SideEffecting` or the auto-degrade threshold
    /// has been exceeded.
    PlainOnly,
}

/// Per-word accumulated failure history used to make adaptive demotion
/// decisions.
#[derive(Debug, Clone, Default)]
pub struct FailureHistory {
    pub quantized_failures: u32,
    pub compiled_failures: u32,
    /// Epoch at which the most recent quantized failure was recorded.
    pub last_quantized_failure_epoch: u64,
}

impl FailureHistory {
    /// Record a quantized-path failure at `current_epoch`.
    pub fn record_quantized_failure(&mut self, current_epoch: u64) {
        self.quantized_failures += 1;
        self.last_quantized_failure_epoch = current_epoch;
    }

    pub fn record_compiled_failure(&mut self) {
        self.compiled_failures += 1;
    }

    /// Returns `true` when the quantized path is in its cooldown window and
    /// should be skipped.
    pub fn quantized_is_cooling_down(
        &self,
        current_epoch: u64,
        budget: &RedundancyBudget,
    ) -> bool {
        if self.last_quantized_failure_epoch == 0 {
            return false;
        }
        current_epoch.saturating_sub(self.last_quantized_failure_epoch)
            < budget.cooldown_epochs
    }

    /// Returns `true` when cumulative failures exceed the auto-degrade
    /// threshold defined in `budget`.
    pub fn should_auto_degrade(&self, budget: &RedundancyBudget) -> bool {
        self.quantized_failures >= budget.auto_degrade_threshold
    }
}
