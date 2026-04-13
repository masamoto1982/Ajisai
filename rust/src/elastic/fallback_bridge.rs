/// Greedy-fallback decision logic for the Elastic Engine (M5).
///
/// When running in `ElasticSafe` mode the bridge inspects each
/// `EvaluationUnit` and decides whether it is safe to optimise or
/// whether it must fall back to the standard greedy path.
///
/// Every fallback decision is logged (when tracing is enabled) and
/// appended to `fallback_log` for post-run analysis.

use crate::elastic::evaluation_unit::EvaluationUnit;
use crate::elastic::execution_mode::ElasticMode;
use crate::elastic::tracer;

// ── Reason codes ──────────────────────────────────────────────────────────────

/// Reason a unit was sent back to the greedy path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackReason {
    /// Word is impure or its purity could not be determined statically.
    UnknownPurity,
    /// Evaluation order relative to siblings must be preserved.
    OrderSensitive,
    /// Dependency graph could not be fully resolved.
    DependencyUnresolvable,
    /// A side effect was detected at runtime that was not anticipated.
    UnexpectedSideEffect,
    /// `ElasticForce` is not active and a safety rule would be violated.
    ElasticForceDisabled,
}

impl FallbackReason {
    pub fn as_str(self) -> &'static str {
        match self {
            FallbackReason::UnknownPurity            => "unknown_purity",
            FallbackReason::OrderSensitive           => "order_sensitive",
            FallbackReason::DependencyUnresolvable   => "dependency_unresolvable",
            FallbackReason::UnexpectedSideEffect     => "unexpected_side_effect",
            FallbackReason::ElasticForceDisabled     => "elastic_force_disabled",
        }
    }
}

impl std::fmt::Display for FallbackReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── Log entry ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FallbackLogEntry {
    pub unit_id:   u32,
    pub word_name: String,
    pub reason:    FallbackReason,
}

// ── Bridge ────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct FallbackBridge {
    pub fallback_log: Vec<FallbackLogEntry>,
}

impl FallbackBridge {
    pub fn new() -> Self {
        Self::default()
    }

    /// Decide whether `unit` must fall back to greedy execution.
    ///
    /// Returns `Some(reason)` when greedy is required, `None` when
    /// elastic optimisation is safe for this unit.
    ///
    /// Rules (checked in order):
    /// 1. `ElasticForce` → never fall back (debug mode, skip all guards).
    /// 2. `!pure`              → `UnknownPurity`
    /// 3. `order_sensitive`    → `OrderSensitive`
    /// 4. `eager_required`     → `OrderSensitive` (I/O must be immediate)
    pub fn should_fallback(
        &mut self,
        unit:  &EvaluationUnit,
        mode:  ElasticMode,
    ) -> Option<FallbackReason> {
        // ElasticForce skips all safety gates (debug / benchmarking only).
        if mode == ElasticMode::ElasticForce {
            return None;
        }

        if !unit.pure {
            return Some(self.log_fallback(unit, FallbackReason::UnknownPurity));
        }

        if unit.order_sensitive {
            return Some(self.log_fallback(unit, FallbackReason::OrderSensitive));
        }

        if unit.eager_required {
            // I/O and similar must always be evaluated in order.
            return Some(self.log_fallback(unit, FallbackReason::OrderSensitive));
        }

        None
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    fn log_fallback(&mut self, unit: &EvaluationUnit, reason: FallbackReason) -> FallbackReason {
        if tracer::is_enabled() {
            eprintln!(
                "[elastic] fallback unit={} word={} reason={}",
                unit.id, unit.word_name, reason
            );
        }
        self.fallback_log.push(FallbackLogEntry {
            unit_id:   unit.id,
            word_name: unit.word_name.clone(),
            reason,
        });
        reason
    }
}
