//! Deterministic execution routing and checkpoint support.
//!
//! The purified runtime no longer keeps per-word failure history, retry
//! budgets, cooldown windows, or adaptive demotion state.  Routing decisions
//! are derived only from the current block's VTU hint:
//!
//! * `StrongCandidate` enters the SIMD/SoA-oriented route.
//! * every other hint enters the plain scalar route.
//!
//! Checkpoints remain available as a local stack-safety primitive for callers
//! that need to restore interpreter state after a single attempted execution;
//! they are not a budget or history mechanism.

use crate::elastic::hedged_snapshot::HedgedSnapshot;
use crate::error::Result;
use crate::interpreter::quantized_block::{VtuHint, VtuSuitability};
use crate::interpreter::Interpreter;

// ── Checkpoint ───────────────────────────────────────────────────────────────

/// Pre-execution interpreter snapshot.
///
/// Capturing this is cheap (stack + hints clone).  Restoring it lets a caller
/// recover from a failed single-path attempt without recording any historical
/// scheduling state.
#[derive(Clone)]
pub struct RedundancyCheckpoint {
    snapshot: HedgedSnapshot,
}

impl RedundancyCheckpoint {
    /// Capture current interpreter state.
    pub fn capture(interp: &Interpreter) -> Self {
        Self {
            snapshot: HedgedSnapshot::from_interpreter(interp),
        }
    }

    /// Restore interpreter to the captured state.
    ///
    /// Epoch counters are intentionally **not** rolled back — dictionary
    /// mutations (DEF / DEL / IMPORT) are irreversible.
    pub fn restore(&self, interp: &mut Interpreter) {
        interp.restore_hedged_snapshot(&self.snapshot);
        interp.runtime_metrics.redundancy_restore_count += 1;
    }
}

// ── Deterministic route selection ───────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeterministicExecutionRoute {
    /// Shape and kernel metadata strongly support the dense SIMD/SoA path.
    SimdSoa,
    /// Use the scalar/plain interpreter path immediately.
    Plain,
}

/// Select an execution route from the current data/block shape only.
///
/// This function intentionally ignores historical failures, retry budgets,
/// cooldown epochs, and accumulated statistics.
pub fn select_deterministic_route(vtu_hint: &VtuHint) -> DeterministicExecutionRoute {
    if vtu_hint.suitability == VtuSuitability::StrongCandidate {
        DeterministicExecutionRoute::SimdSoa
    } else {
        DeterministicExecutionRoute::Plain
    }
}

// ── Interpreter integration ───────────────────────────────────────────────────

impl Interpreter {
    /// Lightweight single-path checkpoint wrapper.
    ///
    /// Restores the stack if `f` returns an error.  Used for plain execution
    /// paths that need local rollback without adaptive degradation.
    pub(crate) fn with_stack_checkpoint<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Interpreter) -> Result<()>,
    {
        self.runtime_metrics.redundancy_checkpoint_count += 1;
        let checkpoint = RedundancyCheckpoint::capture(self);
        let result = f(self);
        if result.is_err() {
            checkpoint.restore(self);
        }
        result
    }
}
