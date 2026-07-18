//! Opt-in execution provenance recorder for receipts (Phase 6).
//!
//! An execution receipt records not just a run's result but what it was based
//! on: which content-identified words executed, which host capabilities were
//! required, and (elsewhere) the reference-validation outcome. Recording is off
//! by default and enabled only when a receipt is requested, so ordinary
//! execution pays nothing beyond a single boolean check at each recording site.
//!
//! The recorder is strictly observational: it never influences values, effects,
//! control flow, or identity. Enabling it must not change a run's result.

use std::collections::{BTreeMap, BTreeSet};

use super::{HostCapability, Interpreter};

/// Aggregated provenance for one executed word. A word invoked many times (a
/// loop body, a recursive call) collapses to a single entry — the receipt
/// records that it ran and how often, not an unbounded per-call event list.
#[derive(Debug, Clone, Default)]
pub struct ExecutedWord {
    /// 0-based position of this word's first execution among distinct words.
    pub first_seen_order: u64,
    /// How many times the word was invoked.
    pub call_count: u64,
}

/// Records execution provenance while enabled. Cleared by a session reset; the
/// recording flag persists so a caller that opted in before a reset keeps
/// recording afterward.
#[derive(Debug, Default)]
pub struct ReceiptRecorder {
    recording: bool,
    next_order: u64,
    executed_words: BTreeMap<String, ExecutedWord>,
    required_capabilities: BTreeSet<HostCapability>,
}

impl ReceiptRecorder {
    pub fn set_recording(&mut self, on: bool) {
        self.recording = on;
    }

    pub fn is_recording(&self) -> bool {
        self.recording
    }

    /// Record one execution of `resolved_name` (a fully-qualified resolved
    /// name). No-op unless recording is enabled.
    pub fn record_executed(&mut self, resolved_name: &str) {
        if !self.recording {
            return;
        }
        if let Some(entry) = self.executed_words.get_mut(resolved_name) {
            entry.call_count += 1;
            return;
        }
        let order = self.next_order;
        self.next_order += 1;
        self.executed_words.insert(
            resolved_name.to_string(),
            ExecutedWord {
                first_seen_order: order,
                call_count: 1,
            },
        );
    }

    /// Record that `capability` was required by a Hosted word, whether or not
    /// the host granted it. No-op unless recording is enabled.
    pub fn record_required(&mut self, capability: HostCapability) {
        if !self.recording {
            return;
        }
        self.required_capabilities.insert(capability);
    }

    pub fn executed_words(&self) -> &BTreeMap<String, ExecutedWord> {
        &self.executed_words
    }

    pub fn required_capabilities(&self) -> &BTreeSet<HostCapability> {
        &self.required_capabilities
    }

    /// Clear recorded data (on session reset). The recording flag is preserved.
    pub fn clear(&mut self) {
        self.next_order = 0;
        self.executed_words.clear();
        self.required_capabilities.clear();
    }
}

impl Interpreter {
    /// Enable or disable execution provenance recording for a receipt (Phase 6).
    /// Recording is observational; enabling it does not change a run's result.
    pub fn set_receipt_recording(&mut self, on: bool) {
        self.receipt_recorder.set_recording(on);
    }

    /// The provenance recorded so far (executed words, required capabilities).
    pub fn receipt_recorder(&self) -> &ReceiptRecorder {
        &self.receipt_recorder
    }

    /// Capabilities the active host grants, in a stable order — the
    /// `grantedCapabilities` set for an execution receipt (Phase 6).
    pub fn granted_host_capabilities(&self) -> Vec<HostCapability> {
        HostCapability::ALL
            .iter()
            .copied()
            .filter(|cap| self.host_env.has_capability(*cap))
            .collect()
    }

    /// Execution step budget in force (water level, SPEC §5.3). Reported as the
    /// receipt's `water.stepLimit`.
    pub fn max_execution_steps(&self) -> usize {
        self.max_execution_steps
    }

    /// Steps consumed by the run so far. Reported as the receipt's
    /// `water.stepsUsed`.
    pub fn execution_step_count(&self) -> usize {
        self.execution_step_count
    }
}
