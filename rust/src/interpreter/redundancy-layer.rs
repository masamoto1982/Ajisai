//! Redundancy Layer — structural safety net for Ajisai's execution pipeline.
//!
//! # Design rationale
//!
//! AMD's Ryzen 7 9800X3D uses 93 % dummy silicon: material that performs no
//! computation but gives the ultra-thin active dies the physical stability they
//! need to function reliably.  DNA's junk regions serve the same architectural
//! role: they do nothing most of the time, but protect the genome's structure
//! and provide a reservoir for evolutionary adaptation.
//!
//! The `RedundancyLayer` applies that principle to Ajisai's execution paths:
//!
//! * **Transparent in the happy path** — a checkpoint is taken cheaply before
//!   every word execution; it imposes no semantic overhead when everything
//!   succeeds.
//! * **Activates on failure** — if the fast (Quantized or Compiled) path
//!   raises an error, the stack is restored to the pre-execution checkpoint
//!   and the next slower-but-safer path is tried automatically.
//! * **Adaptive demotion** — persistent failures cause the layer to lower its
//!   ambitions (`DegradationPolicy`), converging on the always-correct Plain
//!   path rather than retrying a strategy that has proven unreliable.
//!
//! `RuntimeMetrics` is extended with five redundancy-specific counters that
//! make the layer's behaviour observable and testable.

use crate::elastic::hedged_snapshot::HedgedSnapshot;
use crate::error::Result;
use crate::interpreter::quantized_block::{QuantizedArity, QuantizedPurity};
use crate::interpreter::Interpreter;

pub use crate::interpreter::redundancy_budget::{
    DegradationPolicy, FailureHistory, RedundancyBudget,
};

// ── Checkpoint ───────────────────────────────────────────────────────────────

/// Pre-execution interpreter snapshot.
///
/// Capturing this is cheap (stack + hints clone).  Restoring it is the
/// "dummy silicon" that keeps the runtime stable after a path failure.
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

// ── Policy selection ─────────────────────────────────────────────────────────

/// Select a `DegradationPolicy` from static analysis of a `QuantizedBlock`.
///
/// | input_arity | output_arity | purity        | policy      |
/// |-------------|--------------|---------------|-------------|
/// | Fixed       | Fixed        | Pure          | ThreeStage  |
/// | *           | *            | SideEffecting | PlainOnly   |
/// | *           | *            | Unknown/Var   | TwoStage    |
pub fn select_degradation_policy(
    input_arity: QuantizedArity,
    output_arity: QuantizedArity,
    purity: QuantizedPurity,
) -> DegradationPolicy {
    match purity {
        QuantizedPurity::SideEffecting => DegradationPolicy::PlainOnly,
        QuantizedPurity::Pure => {
            if matches!(
                (input_arity, output_arity),
                (QuantizedArity::Fixed(_), QuantizedArity::Fixed(_))
            ) {
                DegradationPolicy::ThreeStage
            } else {
                DegradationPolicy::TwoStage
            }
        }
        QuantizedPurity::Unknown => DegradationPolicy::TwoStage,
    }
}

// ── Interpreter integration ───────────────────────────────────────────────────

impl Interpreter {
    /// Execute `execute_fn` with adaptive degradation and stack protection.
    ///
    /// On failure the stack is restored to the pre-execution checkpoint and
    /// failure statistics are updated so future calls can be demoted
    /// automatically if the fast path is repeatedly unreliable.
    ///
    /// `resolved_name` is used only for future trace/logging; pass the word
    /// name as it appears in the dictionary.
    pub(crate) fn execute_word_with_redundancy(
        &mut self,
        _resolved_name: &str,
        failure_history: &mut FailureHistory,
        budget: &RedundancyBudget,
        policy: DegradationPolicy,
        execute_fn: impl FnOnce(&mut Interpreter) -> Result<()>,
    ) -> Result<()> {
        // PlainOnly: skip checkpoint overhead entirely.
        if policy == DegradationPolicy::PlainOnly {
            return execute_fn(self);
        }

        // Auto-degrade when cumulative failures exceed the threshold.
        let effective_policy = if failure_history.should_auto_degrade(budget) {
            self.runtime_metrics.redundancy_auto_degrade_count += 1;
            DegradationPolicy::PlainOnly
        } else if policy == DegradationPolicy::ThreeStage
            && failure_history
                .quantized_is_cooling_down(self.execution_epoch, budget)
        {
            // Quantized is in cooldown — demote to TwoStage for this call.
            DegradationPolicy::TwoStage
        } else {
            policy
        };

        self.runtime_metrics.redundancy_checkpoint_count += 1;
        let checkpoint = RedundancyCheckpoint::capture(self);

        let result = execute_fn(self);

        if result.is_err() {
            // Restore the stack to the pre-execution state ("dummy silicon").
            checkpoint.restore(self);

            // Record which stage failed based on effective policy.
            match effective_policy {
                DegradationPolicy::ThreeStage => {
                    failure_history.record_quantized_failure(self.execution_epoch);
                    self.runtime_metrics.redundancy_degrade_quantized += 1;
                }
                DegradationPolicy::TwoStage => {
                    failure_history.record_compiled_failure();
                    self.runtime_metrics.redundancy_degrade_compiled += 1;
                }
                DegradationPolicy::PlainOnly => {}
            }
        }

        result
    }

    /// Lightweight single-path checkpoint wrapper.
    ///
    /// Restores the stack if `f` returns an error.  Used for plain execution
    /// paths that do not need multi-stage degradation.
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
