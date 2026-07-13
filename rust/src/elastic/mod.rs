/// Ajisai Elastic Evaluation Engine — MVP modules
///
/// Provides purity analysis, evaluation unit tracking, tracing,
/// pure-result caching, and execution-mode management for the
/// elastic-safe execution path.
///
/// All functionality is additive: the existing greedy evaluator is
/// never rewritten — only wrapped.
///
/// # Feature gating
///
/// The hedged/elastic *engine* (races, prefetch policy, fallback bridge) is
/// isolated behind the opt-in `elastic-engine` cargo feature so the default
/// build ships a smaller trusted core. The submodules below that are compiled
/// unconditionally are the ones the plain greedy path itself depends on:
/// `purity_table` and `evaluation_unit` gate quantized/parallel kernels,
/// `cache_manager` backs the `MEMO` word, `tracer` implements `AJISAI_TRACE`,
/// and `execution_mode` keeps the `ElasticMode` type (and its string API)
/// stable across both configurations.
pub mod cache_manager;
pub mod evaluation_unit;
pub mod execution_mode;
#[cfg(feature = "elastic-engine")]
pub mod fallback_bridge;
#[cfg(feature = "elastic-engine")]
pub mod hedged_executor;
#[cfg(feature = "elastic-engine")]
pub mod hedged_policy;
#[cfg(feature = "elastic-engine")]
pub mod hedged_result;
#[cfg(feature = "elastic-engine")]
pub mod hedged_trace;
pub mod purity_table;
pub mod tracer;

pub use cache_manager::CacheManager;
pub use evaluation_unit::{EvaluationUnit, ParallelGate, UnitState, MIN_PARALLEL_WORK_SCORE};
pub use execution_mode::ElasticMode;
#[cfg(feature = "elastic-engine")]
pub use fallback_bridge::{FallbackBridge, FallbackLogEntry, FallbackReason};
pub use purity_table::{infer_purity, purity_by_name, EvalCost, Purity, PurityInfo};

#[cfg(test)]
mod elastic_engine_tests;

#[cfg(feature = "elastic-engine")]
pub use hedged_executor::validate_winner as validate_hedged_winner;
#[cfg(feature = "elastic-engine")]
pub use hedged_policy::{
    can_hedge_code_block, can_hedge_cond_guard, can_hedge_hof_kernel, can_hedge_word,
};
#[cfg(feature = "elastic-engine")]
pub use hedged_result::{HedgedCandidateResult, HedgedRejectReason, HedgedWinner};
#[cfg(feature = "elastic-engine")]
pub use hedged_trace::{HedgedPath, HedgedTrace, HedgedTraceEvent};
