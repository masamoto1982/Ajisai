/// EvaluationUnit — the scheduling atom for the Elastic Engine.
///
/// Each unit represents one word invocation that the engine tracks.
/// In greedy mode every unit is created and immediately marked `Done`.
/// In elastic-safe mode the scheduler can reorder or cache pure units.

use std::sync::atomic::{AtomicU32, Ordering};
use std::collections::HashSet;

static UNIT_COUNTER: AtomicU32 = AtomicU32::new(1);

fn next_unit_id() -> u32 {
    UNIT_COUNTER.fetch_add(1, Ordering::Relaxed)
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitState {
    /// Created but dependencies not yet satisfied.
    Pending,
    /// All dependencies done; ready to schedule.
    Ready,
    /// Currently executing.
    Running,
    /// Execution completed successfully.
    Done,
    /// Execution raised an error.
    Failed,
    /// Skipped by the scheduler (e.g. COND short-circuit).
    Bypassed,
}

impl std::fmt::Display for UnitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            UnitState::Pending  => "pending",
            UnitState::Ready    => "ready",
            UnitState::Running  => "running",
            UnitState::Done     => "done",
            UnitState::Failed   => "failed",
            UnitState::Bypassed => "bypassed",
        };
        write!(f, "{}", s)
    }
}

// ── Struct ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct EvaluationUnit {
    /// Globally unique, monotonically increasing identifier.
    pub id: u32,
    /// The word name as it appears in source.
    pub word_name: String,
    pub state: UnitState,
    /// IDs of units that must complete before this one may run.
    pub depends_on: Vec<u32>,
    /// Estimated relative cost (lower = prefer earlier).
    pub estimated_cost: f64,
    /// `true` only when the word is provably referentially transparent.
    pub pure: bool,
    /// `true` when result depends on evaluation order (e.g. FOLD, I/O).
    pub order_sensitive: bool,
    /// `true` when the word must be evaluated immediately (e.g. PRINT).
    pub eager_required: bool,
    /// Pre-computed cache key for pure units; `None` when uncacheable.
    pub cache_key: Option<String>,
    /// Extra score subtracted from `priority_score` when this unit's
    /// early evaluation is expected to prune many successors (e.g. COND guard).
    pub pruning_bonus: f64,
    /// Append-only execution log for debugging / trace output.
    pub trace_log: Vec<String>,
}

impl EvaluationUnit {
    pub fn new(word_name: impl Into<String>) -> Self {
        EvaluationUnit {
            id: next_unit_id(),
            word_name: word_name.into(),
            state: UnitState::Pending,
            depends_on: Vec::new(),
            estimated_cost: 1.0,
            pure: false,
            order_sensitive: true,
            eager_required: false,
            cache_key: None,
            pruning_bonus: 0.0,
            trace_log: Vec::new(),
        }
    }

    /// Build a unit pre-populated from purity table data.
    pub fn from_purity(word_name: impl Into<String>, info: &crate::elastic::purity_table::PurityInfo) -> Self {
        use crate::elastic::purity_table::{EvalCost, Purity};
        let mut u = Self::new(word_name);
        u.pure             = info.purity == Purity::Pure;
        u.order_sensitive  = info.order_sensitive;
        u.eager_required   = info.purity == Purity::Impure;
        u.estimated_cost   = match info.cost {
            EvalCost::Trivial => 0.5,
            EvalCost::Light   => 1.0,
            EvalCost::Medium  => 3.0,
            EvalCost::Heavy   => 8.0,
        };
        u
    }

    // ── Scheduling predicates ─────────────────────────────────────────────

    /// `true` when all dependency units have completed.
    pub fn promotable(&self, completed_ids: &HashSet<u32>) -> bool {
        self.depends_on.iter().all(|id| completed_ids.contains(id))
    }

    /// `true` when this unit qualifies for elastic optimisation.
    ///
    /// Requirements: pure, not order-sensitive, not requiring eager evaluation.
    pub fn elastic_eligible(&self) -> bool {
        self.pure && !self.order_sensitive && !self.eager_required
    }

    /// Scheduling priority — **lower score = higher priority**.
    pub fn priority_score(&self) -> f64 {
        self.estimated_cost - self.pruning_bonus
    }

    // ── Logging ───────────────────────────────────────────────────────────

    pub fn log(&mut self, msg: impl Into<String>) {
        self.trace_log.push(msg.into());
    }
}

impl std::fmt::Display for EvaluationUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Unit#{} ({}) state={} cost={:.1} pure={}",
            self.id, self.word_name, self.state, self.estimated_cost, self.pure
        )
    }
}
