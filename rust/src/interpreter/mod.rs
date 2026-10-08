pub mod arithmetic;
pub(crate) mod arithmetic_meter;
pub(crate) mod bindings;
mod body_symbols;
pub(crate) mod broadcast_tree;
pub mod cast;
pub(crate) mod collection_meter;
pub mod comparison;
pub mod compiled_plan;
#[cfg(test)]
mod compiled_plan_tests;
mod contract_record;
pub mod control;
mod debug_declared_checks;
pub mod debug_diagnosis;
mod debug_next_checks;
pub(crate) mod declared_nil_contract;
pub(crate) mod dense_kernels;
#[cfg(test)]
mod dense_kernels_tests;
#[cfg(test)]
mod dense_slice_route_tests;
pub mod error_flow_trace;
#[cfg(test)]
mod error_message_format_tests;
#[cfg(test)]
mod error_operand_restore_tests;
pub mod execute_def;
#[cfg(test)]
mod format_json_tests;
mod format_ops;
pub(crate) mod fused_block;
#[cfg(test)]
mod fused_block_cache_tests;
mod fused_block_general;
mod fused_block_int;
#[cfg(test)]
mod fused_block_lane_call_tests;
pub(crate) mod fused_block_lower;
mod fused_block_rat;
mod fused_block_reg;
#[cfg(test)]
mod fused_block_tests;
pub mod higher_order;
pub mod higher_order_fold;
#[cfg(test)]
mod higher_order_tests;
pub mod host;
mod quickened;
#[cfg(test)]
mod quickened_tests;
#[cfg(test)]
mod route_observation;
pub(crate) mod segment;
mod segment_lower;
#[cfg(test)]
mod segment_tests;
pub mod trace_diagnosis;
// The host-side Word lookup lives in `host.rs`; the module path it used to
// have is kept for the hosts (`wasm_interpreter_bindings`, `rust/tests`) that
// import it by that name.
pub use self::host as host_lookup;
pub(crate) mod host_profile_defaults;
pub mod io;
mod json_decode;
mod json_encode;
pub(crate) mod lane_lift;
// The limit-profile enumeration lives in `host_profile_defaults.rs`; the
// module path it used to have is kept for `agent::execution_receipt` and the
// wasm bindings, which import it by that name.
pub(crate) use self::host_profile_defaults as limit_profile;
pub mod logic;
pub mod math_ops;
pub(crate) mod naming_convention_checker;
mod ordering_ops;
#[cfg(test)]
mod ordering_ops_tests;
#[cfg(test)]
mod power_words_tests;
pub(crate) mod predict_program_outcomes;
mod record_ops;
#[cfg(test)]
mod record_words_tests;
mod reflection_ops;
#[cfg(test)]
mod reflection_words_tests;
pub mod runtime_limits;
mod search_ops;
mod session_lifecycle;
mod shape_words;
#[cfg(test)]
mod shape_words_tests;
pub(crate) mod simd_ops;
pub mod sort;
mod space_projection;
pub mod tensor_cmds;
pub(crate) mod tensor_lane_ops;
pub mod tensor_ops;
// The upstream-NIL link lives in `nil_diagnostics.rs`; the module path it
// used to have is kept for `agent::report`, which imports it by that name.
pub(crate) use self::nil_diagnostics as upstream_nil_link;
pub(crate) mod value_extraction_helpers;
pub mod vector_ops;
mod word_candidates;
pub mod word_contract;
mod word_contract_flow;
#[cfg(test)]
mod word_contract_tests;
mod word_contract_widen;
pub(crate) mod word_cost;
#[cfg(test)]
mod word_cost_tests;
pub(crate) mod word_outcome_vocabulary;
// `pub(crate)`, not private: `agent::observation_digest` calls
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
#[cfg(test)]
mod execution_step_parity_tests;
mod value_as_code;

mod execute_builtin;

pub(crate) mod fusion_contract;
#[cfg(test)]
mod fusion_contract_route_tests;
#[cfg(test)]
mod fusion_contract_tests;
pub(crate) mod nil_diagnostics;

#[cfg(test)]
mod arithmetic_meter_tests;
#[cfg(test)]
mod collection_meter_tests;
#[cfg(test)]
mod debug_diagnosis_tests;
#[cfg(test)]
mod debug_next_checks_tests;
#[cfg(test)]
mod declared_condition_tests;
#[cfg(test)]
mod definable_name_tests;
#[cfg(test)]
mod definition_source_tests;
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
mod higher_order_block_plan_tests;
#[cfg(test)]
mod higher_order_column_tests;
#[cfg(test)]
mod index_projection_tests;
#[cfg(test)]
mod interpreter_definition_tests;
#[cfg(test)]
mod interpreter_execution_tests;
#[cfg(test)]
mod interpreter_mode_tests;
#[cfg(test)]
mod kleene_truth_conformance_tests;
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
    compile_token_block, compile_word_definition, execute_compiled_plan, is_plan_valid,
    CompiledLine, CompiledOp, CompiledPlan,
};

#[cfg(test)]
mod builtin_dispatch_tests;
#[cfg(test)]
mod core_word_canonicalization_tests;
#[cfg(test)]
mod scalar_fastpath_tests;
#[cfg(test)]
mod word_domains_tests;
