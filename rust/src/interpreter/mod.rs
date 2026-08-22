pub mod algo_ops;
pub mod arithmetic;
pub(crate) mod arithmetic_meter;
pub(crate) mod bindings;
pub mod cast;
pub(crate) mod collection_meter;
pub mod comparison;
pub(crate) mod compiled_call;
pub mod compiled_plan;
pub mod control;
pub mod control_cond;
pub mod debug_diagnosis;
mod debug_next_checks;
pub(crate) mod declared_nil_contract;
pub mod epoch;
pub mod error_flow_trace;
pub mod execute_def;
pub mod execute_del;
pub mod execution_plan_set;
pub mod higher_order;
pub mod higher_order_fold;
pub mod host;
pub mod host_lookup;
mod host_profile_defaults;
pub mod io;
pub mod logic;
pub mod math_ops;
pub(crate) mod naming_convention_checker;
mod ordering_ops;
mod reflection;
mod resolve_cache;
pub mod runtime_limits;
mod session_lifecycle;
mod shape_ops;
pub(crate) mod simd_ops;
pub mod sort;
pub mod tensor_cmds;
pub mod tensor_ops;
pub(crate) mod value_extraction_helpers;
pub mod vector_ops;
mod word_candidates;
pub mod word_contract;
mod word_contract_flow;
mod word_contract_lattice;
#[cfg(test)]
mod word_contract_tests;
pub(crate) mod word_cost;
#[cfg(test)]
mod word_cost_tests;
// `pub(crate)`, not private: `agent::observation_digest` (Phase 1,
// competitive-advantage-work-order-2026-08.md) calls
// `word_identity::content_digest` and `word_identity::encode_token` directly,
// so the crate-wide agent boundary needs to name this module.
pub(crate) mod word_identity;
#[cfg(test)]
mod word_identity_tests;
pub mod word_space;
#[cfg(test)]
mod word_space_tests;
#[cfg(test)]
mod work_meter_calibration_tests;
// Re-exported only for the host-only `cli` consumers (receipt / lockfile source
// identity); `content_digest` itself is used internally by `word_identity`, so
// gate just this re-export to the same target as `cli` to stay wasm-clean.

pub mod interpreter_core;

mod resolve_word;

mod execution_loop;
mod vector_literal;

mod execute_builtin;

pub(crate) mod nil_diagnostics;

#[cfg(test)]
mod algo_ops_tests;
#[cfg(test)]
mod arithmetic_exact_div_tests;
#[cfg(test)]
mod arithmetic_meter_tests;
#[cfg(test)]
mod collection_meter_tests;
#[cfg(test)]
mod control_cond_tests;
#[cfg(test)]
mod control_exec_eval_tests;
#[cfg(test)]
mod control_or_else_tests;
#[cfg(test)]
mod debug_next_checks_tests;
#[cfg(test)]
mod dependents_index_tests;
#[cfg(test)]
mod dictionary_operation_tests;
#[cfg(test)]
mod dictionary_resolution_tests;
#[cfg(test)]
mod dictionary_tier_tests;
#[cfg(test)]
mod error_flow_trace_tests;
#[cfg(test)]
mod exact_vector_broadcast_tests;
#[cfg(test)]
mod higher_order_fold_tests;
#[cfg(test)]
mod higher_order_map_tests;
#[cfg(test)]
mod interpreter_definition_tests;
#[cfg(test)]
mod interpreter_execution_tests;
#[cfg(test)]
mod interpreter_mode_tests;
#[cfg(test)]
mod math_ops_tests;
#[cfg(test)]
mod nil_conformance_tests;
#[cfg(test)]
mod nil_contract_conformance_tests;
#[cfg(test)]
mod nil_diagnostics_tests;
#[cfg(test)]
mod nil_reason_tests;

pub use interpreter_core::*;
pub use runtime_limits::RuntimeLimits;

pub use host::{default_host_env, DefaultHostEnv, HostEffect, HostEnv, RecordingHostEnv};

pub use crate::types::WordDefinition;

pub use compiled_plan::{
    compile_word_definition, execute_compiled_plan, is_plan_valid, CompiledLine, CompiledOp,
    CompiledPlan, COMPILED_PLAN_SCHEMA_VERSION,
};
pub use epoch::EpochSnapshot;

#[cfg(test)]
mod compiled_clause_tests;
#[cfg(test)]
mod compiled_plan_tests;
#[cfg(test)]
mod cond_dispatch_tests;
#[cfg(test)]
mod core_word_canonicalization_tests;
#[cfg(test)]
mod scalar_fastpath_tests;
#[cfg(test)]
mod tail_call_tests;
#[cfg(test)]
mod vector_literal_tests;
