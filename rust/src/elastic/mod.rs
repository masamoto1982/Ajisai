/// Ajisai Elastic Evaluation Engine — MVP modules
///
/// Provides purity analysis, evaluation unit tracking, tracing,
/// pure-result caching, and execution-mode management for the
/// elastic-safe execution path.
///
/// All functionality is additive: the existing greedy evaluator is
/// never rewritten — only wrapped.
pub mod cache_manager;
pub mod evaluation_unit;
pub mod execution_mode;
pub mod fallback_bridge;
pub mod hedged_executor;
pub mod hedged_policy;
pub mod hedged_result;
pub mod hedged_snapshot;
pub mod hedged_trace;
pub mod purity_table;
pub mod tracer;

pub use cache_manager::CacheManager;
pub use evaluation_unit::{EvaluationUnit, UnitState};
pub use execution_mode::ElasticMode;
pub use fallback_bridge::{FallbackBridge, FallbackLogEntry, FallbackReason};
pub use purity_table::{infer_purity, purity_by_name, EvalCost, Purity, PurityInfo};

#[cfg(test)]
#[path = "elastic-engine-tests.rs"]
mod elastic_engine_tests;

pub use hedged_executor::validate_winner as validate_hedged_winner;
pub use hedged_policy::{
    can_hedge_code_block, can_hedge_cond_guard, can_hedge_hof_kernel, can_hedge_word,
};
pub use hedged_result::{HedgedCandidateResult, HedgedRejectReason, HedgedWinner};
pub use hedged_snapshot::HedgedSnapshot;
pub use hedged_trace::{HedgedPath, HedgedTrace, HedgedTraceEvent};
